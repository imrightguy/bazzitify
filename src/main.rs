//! bazzitify — GUI and CLI for Bazzite-style gaming optimization.

use bazzitify::module::{Module, ModuleGraph};
use bazzitify::profile::{Profile, ProfileError};
use bazzitify::runner::{RunOpts, run_module_opts};
use slint::Model;
use std::env;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;

slint::include_modules!();

fn modules_dir() -> PathBuf {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules");
    if dev.is_dir() {
        return dev;
    }
    PathBuf::from("/usr/share/bazzitify/modules")
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bazzitify")
        .join("profiles")
}

fn distro_info() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
        .unwrap_or_else(|| "unknown distro".into())
}

enum Event {
    Log(String),
    Status(String, String),
    Done,
}

fn set_module_status(app: &AppWindow, name: &str, status: &str) {
    let model = app.get_modules();
    for i in 0..model.row_count() {
        if let Some(mut row) = model.row_data(i).filter(|r| r.name == name) {
            row.status = status.into();
            model.set_row_data(i, row);
            break;
        }
    }
}

fn spawn_worker(
    handle: &slint::Weak<AppWindow>,
    tx: mpsc::Sender<Event>,
    dir: PathBuf,
    selected: Vec<Module>,
    action: &'static str,
    dry_run: bool,
) {
    if let Some(app) = handle.upgrade() {
        app.set_running(true);
    }
    std::thread::spawn(move || {
        if selected.is_empty() {
            tx.send(Event::Log("nothing selected".into())).ok();
            tx.send(Event::Done).ok();
            return;
        }

        // Sort selected modules by dependency order
        let sorted = if action == "undo" {
            ModuleGraph::reverse_topological_sort(&selected)
        } else {
            ModuleGraph::topological_sort(&selected)
        };

        let sorted = match sorted {
            Ok(m) => m,
            Err(e) => {
                tx.send(Event::Log(format!("dependency error: {e}"))).ok();
                tx.send(Event::Done).ok();
                return;
            }
        };

        // Dry-run: show planned execution order
        if dry_run {
            let order: Vec<String> = sorted.iter().map(|m| m.name.clone()).collect();
            tx.send(Event::Log(format!(
                "DRY RUN — planned {} order: {}",
                action,
                order.join(" → ")
            )))
            .ok();
        }

        for m in &sorted {
            if matches!(action, "undo") && !m.has_undo {
                tx.send(Event::Log(format!(
                    "[{action}:{}] no undo function; skipped",
                    m.name
                )))
                .ok();
                continue;
            }
            if action == "apply" && !m.has_apply {
                tx.send(Event::Log(format!(
                    "[{action}:{}] no apply function; skipped",
                    m.name
                )))
                .ok();
                continue;
            }
            tx.send(Event::Status(m.name.clone(), "running…".into()))
                .ok();
            tx.send(Event::Log(format!("── {action} {} ──", m.name)))
                .ok();
            match run_module_opts(&dir, m, action, RunOpts { dry_run }) {
                Ok(r) => {
                    for line in r.output.lines() {
                        tx.send(Event::Log(line.to_string())).ok();
                    }
                    if r.success && !dry_run {
                        // Persist applied-state so the GUI shows it on next launch
                        let sp = bazzitify::state::state_path();
                        let mut st = bazzitify::state::load(&sp);
                        if action == "apply" {
                            st.mark_applied(&m.name, &bazzitify::state::now_rfc3339());
                        } else {
                            st.unmark(&m.name);
                        }
                        if let Err(e) = bazzitify::state::save(&sp, &st) {
                            tx.send(Event::Log(format!("warn: could not save state: {e}")))
                                .ok();
                        }
                    }
                    if r.success {
                        let status = if action == "apply" {
                            "applied"
                        } else {
                            "undone"
                        };
                        tx.send(Event::Status(m.name.clone(), status.into())).ok();
                        tx.send(Event::Log(format!("✓ {} ok (exit 0)", m.name)))
                            .ok();
                    } else {
                        tx.send(Event::Status(m.name.clone(), "✗ failed".into()))
                            .ok();
                        tx.send(Event::Log(format!(
                            "✗ {} failed (exit {:?})",
                            m.name, r.exit_code
                        )))
                        .ok();
                    }
                }
                Err(e) => {
                    tx.send(Event::Status(m.name.clone(), "✗ failed".into()))
                        .ok();
                    tx.send(Event::Log(format!("error running {}: {e}", m.name)))
                        .ok();
                }
            }
        }
        tx.send(Event::Done).ok();
    });
}

fn run_one(
    handle: &slint::Weak<AppWindow>,
    tx: &mpsc::Sender<Event>,
    dir: &std::path::Path,
    mods: &[Module],
    action: &'static str,
    index: i32,
) {
    if index < 0 {
        return;
    }
    let idx = index as usize;
    let Some(m) = mods.get(idx) else { return };
    let dry = handle.upgrade().map(|a| a.get_dry_run()).unwrap_or(false);
    spawn_worker(
        handle,
        tx.clone(),
        dir.to_path_buf(),
        vec![m.clone()],
        action,
        dry,
    );
}

/// CLI: export selected modules as a profile
fn cli_export(modules: &[Module], name: &str) -> Result<(), ProfileError> {
    let selected: Vec<String> = modules
        .iter()
        .filter(|m| m.has_apply)
        .map(|m| m.name.clone())
        .collect();
    let profile = Profile {
        version: 1,
        distro: detect_distro(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        modules: selected,
    };
    let dir = config_dir();
    let file_path = profile.export(&dir, name)?;
    println!("Exported profile to {}", file_path.display());
    Ok(())
}

/// CLI: import a profile and print its modules
fn cli_import(file: &str) -> Result<(), ProfileError> {
    let modules_dir = modules_dir();
    let file_path = PathBuf::from(file);
    let (profile, warnings) = Profile::import(&file_path, &modules_dir)?;

    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }

    println!("Imported profile: {} modules", profile.modules.len());
    for module in &profile.modules {
        println!("  - {}", module);
    }

    // Save to config dir with a name based on the file stem
    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    let dir = config_dir();
    profile.export(&dir, name)?;
    println!("Saved as profile '{}'", name);

    Ok(())
}

/// CLI: list available profiles
fn cli_list() -> Result<(), ProfileError> {
    let dir = config_dir();
    let profiles = Profile::list_profiles(&dir)?;
    if profiles.is_empty() {
        println!("No profiles found in {}", dir.display());
    } else {
        println!("Available profiles in {}:", dir.display());
        for profile in profiles {
            println!("  - {}", profile);
        }
    }
    Ok(())
}

fn detect_distro() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                return line.trim_start_matches("ID=").trim_matches('"').to_string();
            }
        }
    }
    "unknown".to_string()
}

fn print_usage() {
    eprintln!("bazzitify — Bazzite-style gaming optimization for mutable distros");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  bazzitify                         Launch GUI");
    eprintln!("  bazzitify profile export <name>   Export current selection as profile");
    eprintln!("  bazzitify profile import <file>   Import profile from file");
    eprintln!("  bazzitify profile list            List available profiles");
    eprintln!("  bazzitify --list                  List available modules");
    eprintln!("  bazzitify --dry-run               Show planned execution order");
    eprintln!("  bazzitify --all                   Apply all modules");
    eprintln!("  bazzitify undo <module>           Undo a module");
    eprintln!("  bazzitify <module>                Apply a single module");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // If no args or first arg is not a known CLI flag, run GUI
    if args.len() == 1
        || (args.len() > 1
            && !args[1].starts_with('-')
            && args[1] != "profile"
            && args[1] != "undo")
    {
        run_gui();
        return;
    }

    // CLI mode
    let dir = modules_dir();
    let discovered = Module::discover(&dir).unwrap_or_default();

    match args.get(1).map(String::as_str) {
        Some("profile") => match args.get(2).map(String::as_str) {
            Some("export") => {
                if args.len() < 4 {
                    print_usage();
                    process::exit(1);
                }
                if let Err(e) = cli_export(&discovered, &args[3]) {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
            Some("import") => {
                if args.len() < 4 {
                    print_usage();
                    process::exit(1);
                }
                if let Err(e) = cli_import(&args[3]) {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
            Some("list") => {
                if let Err(e) = cli_list() {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
            _ => {
                print_usage();
                process::exit(1);
            }
        },
        Some("--list") | Some("-l") => {
            println!("Available modules:");
            for m in &discovered {
                let deps = m.depends.join(" ");
                if deps.is_empty() {
                    println!(
                        "  {:<20} {}",
                        m.name,
                        m.description.clone().unwrap_or_default()
                    );
                } else {
                    println!(
                        "  {:<20} {} (depends: {})",
                        m.name,
                        m.description.clone().unwrap_or_default(),
                        deps
                    );
                }
            }
        }
        Some("--dry-run") | Some("-n") => {
            println!("DRY RUN — modules that would run (in dependency order):");
            if let Ok(sorted) = ModuleGraph::topological_sort(&discovered) {
                for m in sorted {
                    println!("  {}", m.name);
                }
            }
        }
        Some("--all") => {
            println!("Applying all modules in dependency order...");
            if let Ok(sorted) = ModuleGraph::topological_sort(&discovered) {
                for m in sorted {
                    if m.has_apply {
                        match run_module_opts(&dir, &m, "apply", RunOpts::default()) {
                            Ok(r) => {
                                for line in r.output.lines() {
                                    println!("{}", line);
                                }
                                if r.success {
                                    println!("✓ {} ok", m.name);
                                } else {
                                    eprintln!("✗ {} failed", m.name);
                                }
                            }
                            Err(e) => eprintln!("error running {}: {}", m.name, e),
                        }
                    }
                }
            }
        }
        Some("undo") => {
            if args.len() < 3 {
                print_usage();
                process::exit(1);
            }
            let name = &args[2];
            if let Some(m) = discovered.iter().find(|m| m.name == *name) {
                if m.has_undo {
                    match run_module_opts(&dir, m, "undo", RunOpts::default()) {
                        Ok(r) => {
                            for line in r.output.lines() {
                                println!("{}", line);
                            }
                        }
                        Err(e) => eprintln!("error running {}: {}", m.name, e),
                    }
                } else {
                    eprintln!("Module {} has no undo — skipping.", name);
                }
            } else {
                eprintln!("Unknown module: {}", name);
                process::exit(1);
            }
        }
        Some(module_name) => {
            // Single module apply
            if let Some(m) = discovered.iter().find(|m| m.name == module_name) {
                if m.has_apply {
                    match run_module_opts(&dir, m, "apply", RunOpts::default()) {
                        Ok(r) => {
                            for line in r.output.lines() {
                                println!("{}", line);
                            }
                        }
                        Err(e) => eprintln!("error running {}: {}", m.name, e),
                    }
                } else {
                    eprintln!("Module {} has no apply — skipping.", module_name);
                }
            } else {
                eprintln!("Unknown module: {}", module_name);
                process::exit(1);
            }
        }
        None => {
            run_gui();
        }
    }
}

fn run_gui() {
    let dir = modules_dir();
    let discovered = Module::discover(&dir)
        .map_err(|e| eprintln!("failed to discover modules in {}: {e}", dir.display()))
        .unwrap_or_default();

    let app = AppWindow::new().expect("failed to create window");
    app.set_distro_info(distro_info().into());

    // Restore applied-state from previous runs
    let applied = bazzitify::state::load(&bazzitify::state::state_path());

    let items: Vec<ModuleInfo> = discovered
        .iter()
        .map(|m| {
            let status = if applied.is_applied(&m.name) {
                "applied"
            } else {
                "available"
            };
            ModuleInfo {
                name: m.name.clone().into(),
                description: m.description.clone().unwrap_or_default().into(),
                details: m.long_description.join("\n").into(),
                selected: false,
                status: status.into(),
            }
        })
        .collect();
    app.set_modules(items.as_slice().into());
    app.set_current_page(-1);

    // Load initial profiles list
    let config_dir = config_dir();
    if let Ok(profiles) = Profile::list_profiles(&config_dir) {
        let profile_vec: Vec<slint::SharedString> =
            profiles.into_iter().map(|s| s.into()).collect();
        app.set_profiles(profile_vec.as_slice().into());
    }

    let handle = app.as_weak();
    let (tx, rx) = mpsc::channel::<Event>();

    // UI updater thread: applies engine events on the Slint event loop.
    {
        let weak = handle.clone();
        std::thread::spawn(move || {
            while let Ok(ev) = rx.recv() {
                let weak = weak.clone();
                let _ = slint::invoke_from_event_loop(move || match ev {
                    Event::Log(line) => {
                        if let Some(app) = weak.upgrade() {
                            app.invoke_append_log(slint::SharedString::from(&line));
                        }
                    }
                    Event::Status(name, status) => {
                        if let Some(app) = weak.upgrade() {
                            set_module_status(&app, &name, &status);
                        }
                    }
                    Event::Done => {
                        if let Some(app) = weak.upgrade() {
                            app.set_running(false);
                        }
                    }
                });
            }
        });
    }

    {
        let handle = handle.clone();
        app.on_select_module(move |i| {
            if let Some(app) = handle.upgrade() {
                app.set_current_page(i);
            }
        });
    }

    {
        let handle = handle.clone();
        app.on_select_all(move |v| {
            if let Some(app) = handle.upgrade() {
                let model = app.get_modules();
                for i in 0..model.row_count() {
                    if let Some(mut row) = model.row_data(i) {
                        row.selected = v;
                        model.set_row_data(i, row);
                    }
                }
            }
        });
    }

    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir2 = dir.clone();
        let mods = discovered.clone();
        app.on_apply_module(move |i| run_one(&handle, &tx, &dir2, &mods, "apply", i));
    }
    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir2 = dir.clone();
        let mods = discovered.clone();
        app.on_undo_module(move |i| run_one(&handle, &tx, &dir2, &mods, "undo", i));
    }

    // Profile export callback
    {
        let handle = handle.clone();
        let tx = tx.clone();
        let _dir2 = dir.clone();
        let mods = discovered.clone();
        let config_dir2 = config_dir.clone();
        app.on_export_profile(move |name: slint::SharedString| {
            if name.is_empty() {
                if let Some(app) = handle.upgrade() {
                    app.invoke_append_log(slint::SharedString::from(
                        "Error: profile name cannot be empty",
                    ));
                }
                return;
            }
            let selected: Vec<Module> = mods.iter().filter(|m| m.has_apply).cloned().collect();
            let tx = tx.clone();
            let config_dir = config_dir2.clone();
            // Do the export on the event loop thread (it's fast)
            let profile = Profile {
                version: 1,
                distro: detect_distro(),
                date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                modules: selected.iter().map(|m| m.name.clone()).collect(),
            };
            match profile.export(&config_dir, name.as_str()) {
                Ok(path) => {
                    tx.send(Event::Log(format!(
                        "Exported profile to {}",
                        path.display()
                    )))
                    .ok();
                    // Refresh profiles list directly (we're on the event loop thread)
                    if let Ok(profiles) = Profile::list_profiles(&config_dir)
                        && let Some(app) = handle.upgrade()
                    {
                        let profile_vec: Vec<slint::SharedString> =
                            profiles.into_iter().map(|s| s.into()).collect();
                        app.set_profiles(profile_vec.as_slice().into());
                    }
                }
                Err(e) => {
                    tx.send(Event::Log(format!("Export failed: {}", e))).ok();
                }
            }
        });
    }

    // Profile import callback
    {
        let _handle = handle.clone();
        let tx = tx.clone();
        let _dir2 = dir.clone();
        let _config_dir2 = config_dir.clone();
        app.on_import_profile(move |_file: slint::SharedString| {
            // For now, just log that file dialog would be needed
            // In a real implementation, we'd open a file dialog
            tx.send(Event::Log(
                "Import: file dialog not yet implemented; use CLI for now".into(),
            ))
            .ok();
        });
    }

    // Profile load/refresh callback
    {
        let _handle = handle.clone();
        let config_dir2 = config_dir.clone();
        app.on_load_profiles(move || {
            if let Ok(profiles) = Profile::list_profiles(&config_dir2) {
                // We need to use invoke_from_event_loop here since this callback might be called from anywhere
                // Actually, since it's a callback from the UI, it's on the event loop thread
                if let Some(app) = _handle.upgrade() {
                    let profile_vec: Vec<slint::SharedString> =
                        profiles.into_iter().map(|s| s.into()).collect();
                    app.set_profiles(profile_vec.as_slice().into());
                }
            }
        });
    }

    app.run().expect("slint event loop failed");
}

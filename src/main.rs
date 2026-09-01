//! bazzitify — GUI and CLI for Bazzite-style gaming optimization.

use bazzitify::distro::{detect_distro, distro_pretty_name};
use bazzitify::module::{Module, ModuleGraph};
use bazzitify::profile::{Profile, ProfileError};
use bazzitify::runner::{RunOpts, run_module_opts};
use bazzitify::wizard::{
    detect_hardware_profile, generate_dry_run_preview_from_modules,
    get_suggested_modules_for_distro_and_hardware, mark_wizard_complete, should_show_wizard,
    wizard_marker_path,
};
use serde::Serialize;
use slint::Model;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc;

slint::include_modules!();

fn modules_dir() -> PathBuf {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules");
    modules_dir_from(
        env::var_os("APPDIR").as_deref().map(Path::new),
        &dev,
        Path::new("/usr/share/bazzitify/modules"),
    )
}

fn modules_dir_from(appdir: Option<&Path>, dev: &Path, system: &Path) -> PathBuf {
    if let Some(appdir) = appdir {
        let bundled = appdir.join("usr/share/bazzitify/modules");
        if bundled.is_dir() {
            return bundled;
        }
    }
    if dev.is_dir() {
        return dev.to_path_buf();
    }
    system.to_path_buf()
}

#[cfg(test)]
mod module_path_tests {
    use super::*;

    #[test]
    fn wizard_returns_to_confirmation_when_apply_fails() {
        assert_eq!(wizard_apply_result_step(false), 4);
        assert_eq!(wizard_apply_result_step(true), 6);
    }

    #[test]
    fn appimage_modules_take_precedence_over_a_present_build_tree() {
        let temp = tempfile::tempdir().expect("temp dir");
        let appdir = temp.path().join("AppDir");
        let appimage_modules = appdir.join("usr/share/bazzitify/modules");
        let build_tree_modules = temp.path().join("source/modules");
        std::fs::create_dir_all(&appimage_modules).expect("AppImage module directory");
        std::fs::create_dir_all(&build_tree_modules).expect("build tree module directory");

        assert_eq!(
            modules_dir_from(
                Some(&appdir),
                &build_tree_modules,
                Path::new("/missing/system/modules")
            ),
            appimage_modules
        );
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bazzitify")
        .join("profiles")
}

/// JSON output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonMode {
    Off,
    Compact,
    Pretty,
}

/// Stable module metadata for JSON output.
#[derive(Debug, Serialize)]
struct ModuleJson {
    name: String,
    desc: Option<String>,
    long_desc: Vec<String>,
    applied_status: bool,
    requires: Vec<String>,
}

/// Result for apply/undo in JSON mode.
#[derive(Debug, Serialize)]
struct ApplyResultJson {
    success: bool,
    module: String,
    action: String,
    output: String,
    error: Option<String>,
    duration_ms: u64,
}

/// Dry-run plan entry for JSON output.
#[derive(Debug, Serialize)]
struct DryRunPlanJson {
    module: String,
    would_apply: bool,
    commands: Vec<String>,
    warnings: Vec<String>,
}

enum Event {
    Log(String),
    Status(String, String),
    Done { success: bool },
}

fn wizard_apply_result_step(success: bool) -> i32 {
    if success { 6 } else { 4 }
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
            tx.send(Event::Done { success: true }).ok();
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
                tx.send(Event::Done { success: false }).ok();
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

        let mut all_succeeded = true;
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
                        all_succeeded = false;
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
                    all_succeeded = false;
                    tx.send(Event::Status(m.name.clone(), "✗ failed".into()))
                        .ok();
                    tx.send(Event::Log(format!("error running {}: {e}", m.name)))
                        .ok();
                }
            }
        }
        tx.send(Event::Done {
            success: all_succeeded,
        })
        .ok();
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
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --json              Output JSON (compact)");
    eprintln!("  --json=pretty       Output JSON (pretty-printed)");
}

fn parse_json_mode(args: &[String]) -> (JsonMode, Vec<String>) {
    let mut mode = JsonMode::Off;
    let mut remaining = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--json" {
            mode = JsonMode::Compact;
        } else if arg == "--json=pretty" {
            mode = JsonMode::Pretty;
        } else if arg.starts_with("--json=") {
            // Handle --json=compact explicitly
            if arg == "--json=compact" {
                mode = JsonMode::Compact;
            } else {
                eprintln!(
                    "Unknown --json value: {}. Use --json, --json=compact, or --json=pretty",
                    arg
                );
                process::exit(2);
            }
        } else {
            remaining.push(arg.clone());
        }
        i += 1;
    }
    (mode, remaining)
}

fn output_json<T: Serialize>(value: &T, mode: JsonMode) {
    let json = match mode {
        JsonMode::Compact => serde_json::to_string(value).unwrap(),
        JsonMode::Pretty => serde_json::to_string_pretty(value).unwrap(),
        JsonMode::Off => unreachable!(),
    };
    println!("{}", json);
}

fn cli_list_modules(
    discovered: &[Module],
    applied: &bazzitify::state::AppliedState,
    json_mode: JsonMode,
) {
    if json_mode != JsonMode::Off {
        let modules: Vec<ModuleJson> = discovered
            .iter()
            .map(|m| ModuleJson {
                name: m.name.clone(),
                desc: m.description.clone(),
                long_desc: m.long_description.clone(),
                applied_status: applied.is_applied(&m.name),
                requires: m.depends.clone(),
            })
            .collect();
        output_json(&modules, json_mode);
    } else {
        println!("Available modules:");
        for m in discovered {
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
}

fn cli_dry_run(discovered: &[Module], json_mode: JsonMode) {
    if let Ok(sorted) = ModuleGraph::topological_sort(discovered) {
        if json_mode != JsonMode::Off {
            let plan: Vec<DryRunPlanJson> = sorted
                .iter()
                .map(|m| DryRunPlanJson {
                    module: m.name.clone(),
                    would_apply: m.has_apply,
                    commands: vec![format!(
                        "module_apply  # {}",
                        m.description.clone().unwrap_or_default()
                    )],
                    warnings: if m.depends.is_empty() {
                        vec![]
                    } else {
                        vec![format!("Depends on: {}", m.depends.join(", "))]
                    },
                })
                .collect();
            output_json(&plan, json_mode);
        } else {
            println!("DRY RUN — modules that would run (in dependency order):");
            for m in sorted {
                println!("  {}", m.name);
            }
        }
    }
}

fn cli_apply_all(dir: &Path, discovered: &[Module], json_mode: JsonMode) -> i32 {
    let mut exit_code = 0;
    let mut results = Vec::new();

    match ModuleGraph::topological_sort(discovered) {
        Ok(sorted) => {
            for m in sorted {
                if m.has_apply {
                    match run_module_opts(dir, &m, "apply", RunOpts::default()) {
                        Ok(r) => {
                            if !r.success {
                                exit_code = 1;
                            }
                            if json_mode != JsonMode::Off {
                                results.push(ApplyResultJson {
                                    success: r.success,
                                    module: m.name.clone(),
                                    action: "apply".into(),
                                    output: r.output.clone(),
                                    error: if r.success {
                                        None
                                    } else {
                                        Some(r.output.clone())
                                    },
                                    duration_ms: r.duration_ms,
                                });
                            } else {
                                for line in r.output.lines() {
                                    println!("{}", line);
                                }
                                if r.success {
                                    println!("✓ {} ok", m.name);
                                } else {
                                    eprintln!("✗ {} failed", m.name);
                                }
                            }
                        }
                        Err(e) => {
                            exit_code = 1;
                            if json_mode != JsonMode::Off {
                                results.push(ApplyResultJson {
                                    success: false,
                                    module: m.name.clone(),
                                    action: "apply".into(),
                                    output: String::new(),
                                    error: Some(e.to_string()),
                                    duration_ms: 0,
                                });
                            } else {
                                eprintln!("error running {}: {}", m.name, e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            // Dependency error: cyclic dependency or missing dependency
            exit_code = 3;
            if json_mode != JsonMode::Off {
                let err = serde_json::json!({
                    "error": e.to_string()
                });
                output_json(&err, json_mode);
            } else {
                eprintln!("Dependency error: {}", e);
            }
        }
    }

    if json_mode != JsonMode::Off && exit_code == 0 {
        output_json(&results, json_mode);
    }
    exit_code
}

fn cli_apply_single(dir: &Path, discovered: &[Module], name: &str, json_mode: JsonMode) -> i32 {
    let mut exit_code = 0;
    if let Some(m) = discovered.iter().find(|m| m.name == name) {
        if m.has_apply {
            match run_module_opts(dir, m, "apply", RunOpts::default()) {
                Ok(r) => {
                    if !r.success {
                        exit_code = 1;
                    }
                    if json_mode != JsonMode::Off {
                        let result = ApplyResultJson {
                            success: r.success,
                            module: m.name.clone(),
                            action: "apply".into(),
                            output: r.output.clone(),
                            error: if r.success {
                                None
                            } else {
                                Some(r.output.clone())
                            },
                            duration_ms: r.duration_ms,
                        };
                        output_json(&result, json_mode);
                    } else {
                        for line in r.output.lines() {
                            println!("{}", line);
                        }
                    }
                }
                Err(e) => {
                    exit_code = 1;
                    if json_mode != JsonMode::Off {
                        let result = ApplyResultJson {
                            success: false,
                            module: m.name.clone(),
                            action: "apply".into(),
                            output: String::new(),
                            error: Some(e.to_string()),
                            duration_ms: 0,
                        };
                        output_json(&result, json_mode);
                    } else {
                        eprintln!("error running {}: {}", m.name, e);
                    }
                }
            }
        } else {
            if json_mode != JsonMode::Off {
                let result = ApplyResultJson {
                    success: false,
                    module: m.name.clone(),
                    action: "apply".into(),
                    output: String::new(),
                    error: Some("Module has no apply function".into()),
                    duration_ms: 0,
                };
                output_json(&result, json_mode);
            } else {
                eprintln!("Module {} has no apply — skipping.", name);
            }
            exit_code = 1;
        }
    } else {
        if json_mode != JsonMode::Off {
            let result = ApplyResultJson {
                success: false,
                module: name.into(),
                action: "apply".into(),
                output: String::new(),
                error: Some("Unknown module".into()),
                duration_ms: 0,
            };
            output_json(&result, json_mode);
        } else {
            eprintln!("Unknown module: {}", name);
        }
        // Unknown module is a module error (exit code 1).
        exit_code = 1;
    }
    exit_code
}

fn cli_undo(dir: &Path, discovered: &[Module], name: &str, json_mode: JsonMode) -> i32 {
    let mut exit_code = 0;
    if let Some(m) = discovered.iter().find(|m| m.name == name) {
        if m.has_undo {
            match run_module_opts(dir, m, "undo", RunOpts::default()) {
                Ok(r) => {
                    if !r.success {
                        exit_code = 1;
                    }
                    if json_mode != JsonMode::Off {
                        let result = ApplyResultJson {
                            success: r.success,
                            module: m.name.clone(),
                            action: "undo".into(),
                            output: r.output.clone(),
                            error: if r.success {
                                None
                            } else {
                                Some(r.output.clone())
                            },
                            duration_ms: r.duration_ms,
                        };
                        output_json(&result, json_mode);
                    } else {
                        for line in r.output.lines() {
                            println!("{}", line);
                        }
                    }
                }
                Err(e) => {
                    exit_code = 1;
                    if json_mode != JsonMode::Off {
                        let result = ApplyResultJson {
                            success: false,
                            module: m.name.clone(),
                            action: "undo".into(),
                            output: String::new(),
                            error: Some(e.to_string()),
                            duration_ms: 0,
                        };
                        output_json(&result, json_mode);
                    } else {
                        eprintln!("error running {}: {}", m.name, e);
                    }
                }
            }
        } else {
            if json_mode != JsonMode::Off {
                let result = ApplyResultJson {
                    success: false,
                    module: m.name.clone(),
                    action: "undo".into(),
                    output: String::new(),
                    error: Some("Module has no undo function".into()),
                    duration_ms: 0,
                };
                output_json(&result, json_mode);
            } else {
                eprintln!("Module {} has no undo — skipping.", name);
            }
            exit_code = 1;
        }
    } else {
        if json_mode != JsonMode::Off {
            let result = ApplyResultJson {
                success: false,
                module: name.into(),
                action: "undo".into(),
                output: String::new(),
                error: Some("Unknown module".into()),
                duration_ms: 0,
            };
            output_json(&result, json_mode);
        } else {
            eprintln!("Unknown module: {}", name);
        }
        // Unknown module is a module error (exit code 1).
        exit_code = 1;
    }
    exit_code
}

fn main() {
    // Initialize translations for @tr() macro in Slint UI
    slint::init_translations!(concat!(env!("CARGO_MANIFEST_DIR"), "/translations/"));

    let args: Vec<String> = env::args().collect();

    // Parse --json flag first
    let (json_mode, args) = parse_json_mode(&args);

    // If no args, run GUI
    if args.len() == 1 {
        run_gui();
        return;
    }

    // CLI mode
    let dir = modules_dir();
    let discovered = Module::discover(&dir).unwrap_or_default();
    let applied = bazzitify::state::load(&bazzitify::state::state_path());

    let exit_code = match args.get(1).map(String::as_str) {
        Some("profile") => match args.get(2).map(String::as_str) {
            Some("export") => {
                if args.len() < 4 {
                    print_usage();
                    2
                } else {
                    if let Err(e) = cli_export(&discovered, &args[3]) {
                        if json_mode != JsonMode::Off {
                            let err = serde_json::json!({"error": e.to_string()});
                            output_json(&err, json_mode);
                        } else {
                            eprintln!("Error: {}", e);
                        }
                        1
                    } else {
                        0
                    }
                }
            }
            Some("import") => {
                if args.len() < 4 {
                    print_usage();
                    2
                } else {
                    if let Err(e) = cli_import(&args[3]) {
                        if json_mode != JsonMode::Off {
                            let err = serde_json::json!({"error": e.to_string()});
                            output_json(&err, json_mode);
                        } else {
                            eprintln!("Error: {}", e);
                        }
                        1
                    } else {
                        0
                    }
                }
            }
            Some("list") => {
                if let Err(e) = cli_list() {
                    if json_mode != JsonMode::Off {
                        let err = serde_json::json!({"error": e.to_string()});
                        output_json(&err, json_mode);
                    } else {
                        eprintln!("Error: {}", e);
                    }
                    1
                } else {
                    0
                }
            }
            _ => {
                print_usage();
                2
            }
        },
        Some("--list") | Some("-l") => {
            cli_list_modules(&discovered, &applied, json_mode);
            0
        }
        Some("--dry-run") | Some("-n") => {
            cli_dry_run(&discovered, json_mode);
            0
        }
        Some("--all") => cli_apply_all(&dir, &discovered, json_mode),
        Some("undo") => {
            if args.len() < 3 {
                print_usage();
                2
            } else {
                cli_undo(&dir, &discovered, &args[2], json_mode)
            }
        }
        Some(module_name) => cli_apply_single(&dir, &discovered, module_name, json_mode),
        None => {
            run_gui();
            0
        }
    };

    process::exit(exit_code);
}

fn run_gui() {
    let dir = modules_dir();
    let discovered = Module::discover(&dir)
        .map_err(|e| eprintln!("failed to discover modules in {}: {e}", dir.display()))
        .unwrap_or_default();

    let app = AppWindow::new().expect("failed to create window");
    app.set_distro_info(distro_pretty_name().into());

    // Check if wizard should be shown
    let marker_path = wizard_marker_path();
    let show_wizard = should_show_wizard(&marker_path);
    app.set_wizard_active(show_wizard);
    app.set_wizard_step(0);
    app.set_wizard_confirmed(false);
    app.set_wizard_error(slint::SharedString::from(""));

    if show_wizard {
        // Detect distro for the wizard
        let _distro = detect_distro();
        app.set_wizard_distro(slint::SharedString::from(distro_pretty_name()));
    }

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
                    Event::Done { success } => {
                        if let Some(app) = weak.upgrade() {
                            app.set_running(false);
                            if app.get_wizard_active() && app.get_wizard_step() == 5 {
                                app.set_wizard_step(wizard_apply_result_step(success));
                                if !success {
                                    app.set_wizard_error(slint::SharedString::from(
                                        "One or more modules failed. Review the log and try again.",
                                    ));
                                }
                            }
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
                // Fetch current power profile if power-profiles module is selected
                if i >= 0 {
                    let model = app.get_modules();
                    let idx: usize = i as usize;
                    let row_opt = model.row_data(idx);
                    #[allow(clippy::collapsible_if)]
                    if let Some(row) = row_opt {
                        if row.name == "power-profiles" {
                            app.invoke_get_power_profile();
                        }
                    }
                }
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
                app.set_selected_count(if v { model.row_count() as i32 } else { 0 });
            }
        });
    }

    // Clear selection callback (Escape key)
    {
        let handle = handle.clone();
        app.on_clear_selection(move || {
            if let Some(app) = handle.upgrade() {
                let model = app.get_modules();
                for i in 0..model.row_count() {
                    if let Some(mut row) = model.row_data(i) {
                        row.selected = false;
                        model.set_row_data(i, row);
                    }
                }
                app.set_selected_count(0);
            }
        });
    }

    // Selection changed callback - sync select-all-checkbox with individual selections
    {
        let handle = handle.clone();
        app.on_selection_changed(move || {
            if let Some(app) = handle.upgrade() {
                let model = app.get_modules();
                let mut all_selected = true;
                let mut any_selected = false;
                let mut selected_count = 0;
                for i in 0..model.row_count() {
                    if let Some(row) = model.row_data(i) {
                        if row.selected {
                            any_selected = true;
                            selected_count += 1;
                        } else {
                            all_selected = false;
                        }
                    }
                }
                app.set_selected_count(selected_count);
                // select-all-checkbox should be true only if ALL modules are selected
                // and there's at least one module
                app.set_select_all_checkbox(any_selected && all_selected);
            }
        });
    }

    // Batch apply selected modules
    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir2 = dir.clone();
        let mods = discovered.clone();
        app.on_apply_selected(move || {
            if let Some(app) = handle.upgrade() {
                let model = app.get_modules();
                let mut selected_indices = Vec::new();
                for i in 0..model.row_count() {
                    if let Some(row) = model.row_data(i)
                        && row.selected
                    {
                        selected_indices.push(i);
                    }
                }
                if !selected_indices.is_empty() {
                    let selected_modules: Vec<Module> = selected_indices
                        .iter()
                        .filter_map(|&idx| mods.get(idx).cloned())
                        .collect();
                    spawn_worker(
                        &handle,
                        tx.clone(),
                        dir2.clone(),
                        selected_modules,
                        "apply",
                        app.get_dry_run(),
                    );
                }
            }
        });
    }

    // Batch undo selected modules (reverse order for dependency safety)
    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir2 = dir.clone();
        let mods = discovered.clone();
        app.on_undo_selected(move || {
            if let Some(app) = handle.upgrade() {
                let model = app.get_modules();
                let mut selected_indices = Vec::new();
                for i in 0..model.row_count() {
                    if let Some(row) = model.row_data(i)
                        && row.selected
                    {
                        selected_indices.push(i);
                    }
                }
                if !selected_indices.is_empty() {
                    let selected_modules: Vec<Module> = selected_indices
                        .iter()
                        .filter_map(|&idx| mods.get(idx).cloned())
                        .collect();
                    spawn_worker(
                        &handle,
                        tx.clone(),
                        dir2.clone(),
                        selected_modules,
                        "undo",
                        app.get_dry_run(),
                    );
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

    // Get current power profile callback
    {
        let handle = handle.clone();
        app.on_get_power_profile(move || {
            if let Some(app) = handle.upgrade() {
                // Run powerprofilesctl get to fetch current profile
                let output = std::process::Command::new("powerprofilesctl")
                    .arg("get")
                    .output();
                let profile_str = match output {
                    Ok(out) if out.status.success() => {
                        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if s.is_empty() {
                            "unknown".to_string()
                        } else {
                            s
                        }
                    }
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr);
                        format!("error: {}", err.trim())
                    }
                    Err(e) => format!("error: {}", e),
                };
                app.set_power_profile(slint::SharedString::from(profile_str));
            }
        });
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

    // Wizard callbacks
    {
        let handle = handle.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        let _marker_path = wizard_marker_path();
        app.on_wizard_next(move || {
            if let Some(app) = handle.upgrade() {
                let step = app.get_wizard_step();
                if step < 5 {
                    app.set_wizard_step(step + 1);

                    // Special handling for step transitions
                    match step {
                        0 => {
                            // Welcome -> Distro detection
                            let _distro = detect_distro();
                            app.set_wizard_distro(slint::SharedString::from(distro_pretty_name()));
                        }
                        1 => {
                            // Distro detection -> Module selection
                            let distro = app.get_wizard_distro().to_string();
                            let hardware = detect_hardware_profile();
                            let suggested =
                                get_suggested_modules_for_distro_and_hardware(&distro, &hardware);
                            // Convert suggested modules to WizardModule and set the model
                            let wizard_modules: Vec<crate::WizardModule> = suggested
                                .iter()
                                .map(|m| crate::WizardModule {
                                    name: m.name.clone().into(),
                                    description: m.description.clone().into(),
                                    reason: m.reason.clone().into(),
                                    selected: m.selected,
                                })
                                .collect();
                            app.set_wizard_modules(wizard_modules.as_slice().into());
                            app.invoke_append_log(slint::SharedString::from(format!(
                                "Found {} suggested modules",
                                suggested.len()
                            )));
                        }
                        2 => {
                            // Module selection -> Dry-run preview using the same runner as apply.
                            let model = app.get_wizard_modules();
                            let selected_modules = (0..model.row_count())
                                .filter_map(|index| model.row_data(index))
                                .filter(|row| row.selected)
                                .filter_map(|row| {
                                    mods.iter().find(|module| module.name == row.name.as_str())
                                })
                                .cloned()
                                .collect::<Vec<_>>();
                            match generate_dry_run_preview_from_modules(&dir, &selected_modules) {
                                Ok(preview) => {
                                    app.set_wizard_preview(slint::SharedString::from(preview));
                                    app.invoke_append_log(slint::SharedString::from(
                                        "Dry-run preview generated",
                                    ));
                                }
                                Err(error) => {
                                    app.set_wizard_preview(slint::SharedString::from(format!(
                                        "Dry-run preview failed: {error}"
                                    )));
                                    app.invoke_append_log(slint::SharedString::from(format!(
                                        "Dry-run preview failed: {error}"
                                    )));
                                }
                            }
                        }
                        3 => {
                            // Dry-run -> Confirm
                            app.set_wizard_confirmed(false);
                        }
                        4 => {
                            // Confirm -> Complete (actual apply happens in wizard_confirm)
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    {
        let handle = handle.clone();
        app.on_wizard_prev(move || {
            if let Some(_app) = handle.upgrade() {
                let step = _app.get_wizard_step();
                if step > 0 {
                    _app.set_wizard_step(step - 1);
                }
            }
        });
    }

    {
        let handle = handle.clone();
        app.on_wizard_toggle_module(move |_i: i32| {
            if let Some(_app) = handle.upgrade() {
                // The two-way binding in Slint handles this automatically
            }
        });
    }

    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        app.on_wizard_confirm(move || {
            if let Some(app) = handle.upgrade() {
                let step = app.get_wizard_step();
                if step == 4 {
                    // Apply step - run the selected modules
                    let model = app.get_wizard_modules();
                    let mut selected_modules = Vec::new();
                    for i in 0..model.row_count() {
                        if let Some(row) = model.row_data(i)
                            && row.selected
                            && let Some(module) = mods.iter().find(|m| m.name == row.name.as_str())
                        {
                            selected_modules.push(module.clone());
                        }
                    }
                    if !selected_modules.is_empty() {
                        app.set_wizard_error(slint::SharedString::from(""));
                        app.set_running(true);
                        app.set_wizard_step(5); // Applying step; worker advances only on success
                        let tx = tx.clone();
                        let dir = dir.clone();
                        spawn_worker(&handle, tx, dir, selected_modules, "apply", false);
                    }
                } else if step == 6 {
                    // Complete step - mark wizard done and go to main UI
                    let marker_path = wizard_marker_path();
                    let tx = tx.clone();
                    if let Err(e) = mark_wizard_complete(&marker_path) {
                        tx.send(Event::Log(format!("Failed to mark wizard complete: {}", e)))
                            .ok();
                    }
                    app.set_wizard_active(false);
                    app.set_current_page(-1); // Overview page
                    app.invoke_append_log(slint::SharedString::from(
                        "Wizard complete! You can now manage modules from the main screen.",
                    ));
                }
            }
        });
    }

    {
        let handle = handle.clone();
        let tx = tx.clone();
        let marker_path = wizard_marker_path();
        app.on_wizard_skip(move || {
            if let Some(app) = handle.upgrade() {
                if let Err(e) = mark_wizard_complete(&marker_path) {
                    let tx = tx.clone();
                    tx.send(Event::Log(format!("Failed to mark wizard complete: {}", e)))
                        .ok();
                }
                app.set_wizard_active(false);
                app.set_current_page(-1); // Overview page
                app.invoke_append_log(slint::SharedString::from(
                    "Wizard skipped. You can run it again by deleting the config file.",
                ));
            }
        });
    }

    // Re-run Wizard callback
    {
        let handle = handle.clone();
        app.on_rerun_wizard(move || {
            if let Some(app) = handle.upgrade() {
                app.set_wizard_active(true);
                app.set_wizard_step(0);
                app.set_wizard_distro(slint::SharedString::from(""));
                app.set_wizard_preview(slint::SharedString::from(""));
                app.set_wizard_error(slint::SharedString::from(""));
                app.set_wizard_confirmed(false);
                app.invoke_append_log(slint::SharedString::from(
                    "Wizard restarted. Detecting distribution...",
                ));
            }
        });
    }

    app.run().expect("slint event loop failed");
}

//! bazzitify GUI — Slint frontend over the module engine.

use slint::Model;
use std::path::PathBuf;
use std::sync::mpsc;

slint::include_modules!();

fn modules_dir() -> PathBuf {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules");
    if dev.is_dir() {
        return dev;
    }
    PathBuf::from("/usr/share/bazzitify/modules")
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
        if let Some(mut row) = model.row_data(i) {
            if row.name == name {
                row.status = status.into();
                model.set_row_data(i, row);
                break;
            }
        }
    }
}

fn spawn_worker(
    handle: &slint::Weak<AppWindow>,
    tx: mpsc::Sender<Event>,
    dir: PathBuf,
    selected: Vec<bazzitify::module::Module>,
    action: &'static str,
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
        for m in &selected {
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
            match bazzitify::runner::run_module(&dir, m, action) {
                Ok(r) => {
                    for line in r.output.lines() {
                        tx.send(Event::Log(line.to_string())).ok();
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
    mods: &[bazzitify::module::Module],
    action: &'static str,
    index: i32,
) {
    if index < 0 {
        return;
    }
    let idx = index as usize;
    let Some(m) = mods.get(idx) else { return };
    spawn_worker(
        handle,
        tx.clone(),
        dir.to_path_buf(),
        vec![m.clone()],
        action,
    );
}

fn main() {
    let dir = modules_dir();
    let discovered = bazzitify::module::Module::discover(&dir)
        .map_err(|e| eprintln!("failed to discover modules in {}: {e}", dir.display()))
        .unwrap_or_default();

    let app = AppWindow::new().expect("failed to create window");
    app.set_distro_info(distro_info().into());

    let items: Vec<ModuleInfo> = discovered
        .iter()
        .map(|m| ModuleInfo {
            name: m.name.clone().into(),
            description: m.description.clone().unwrap_or_default().into(),
            details: m.long_description.join("\n").into(),
            selected: false,
            status: "available".into(),
        })
        .collect();
    app.set_modules(items.as_slice().into());
    app.set_current_page(-1);

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

    app.run().expect("slint event loop failed");
}

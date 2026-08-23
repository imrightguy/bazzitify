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
            selected: false,
            status: "available".into(),
        })
        .collect();
    app.set_modules(items.as_slice().into());

    let (tx, rx) = mpsc::channel::<Event>();
    let handle = app.as_weak();

    // UI updater thread: applies engine events on the Slint event loop.
    {
        let weak = handle.clone();
        std::thread::spawn(move || {
            while let Ok(ev) = rx.recv() {
                let weak = weak.clone();
                slint::invoke_from_event_loop(move || match ev {
                    Event::Log(line) => {
                        if let Some(app) = weak.upgrade() {
                            app.invoke_append_log(slint::SharedString::from(&line));
                        }
                    }
                    Event::Status(name, status) => {
                        if let Some(app) = weak.upgrade() {
                            let model = app.get_modules();
                            for i in 0..model.row_count() {
                                if model.row_data(i).map(|r| r.name == name).unwrap_or(false) {
                                    if let Some(mut row) = model.row_data(i) {
                                        row.status = status.into();
                                        model.set_row_data(i, row);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Event::Done => {
                        if let Some(app) = weak.upgrade() {
                            app.set_running(false);
                        }
                    }
                })
                .ok();
            }
        });
    }

    fn set_all_selected(handle: &slint::Weak<AppWindow>, value: bool) {
        if let Some(app) = handle.upgrade() {
            let model = app.get_modules();
            for i in 0..model.row_count() {
                if let Some(mut row) = model.row_data(i) {
                    row.selected = value;
                    model.set_row_data(i, row);
                }
            }
        }
    }

    {
        let handle = handle.clone();
        app.on_select_all(move |v| set_all_selected(&handle, v));
    }

    fn run_modules(
        handle: &slint::Weak<AppWindow>,
        tx: &mpsc::Sender<Event>,
        dir: &std::path::Path,
        mods: &[bazzitify::module::Module],
        action: &'static str,
    ) {
        if let Some(app) = handle.upgrade() {
            app.set_running(true);
        }
        let tx = tx.clone();
        let dir: PathBuf = dir.to_path_buf();
        let selected: Vec<bazzitify::module::Module> = {
            let names: Vec<String> = handle
                .upgrade()
                .map(|app| {
                    app.get_modules()
                        .iter()
                        .filter(|m| m.selected)
                        .map(|m| m.name.to_string())
                        .collect()
                })
                .unwrap_or_default();
            mods.iter()
                .filter(|m| names.contains(&m.name))
                .cloned()
                .collect()
        };

        std::thread::spawn(move || {
            if selected.is_empty() {
                tx.send(Event::Log("nothing selected".into())).ok();
                tx.send(Event::Done).ok();
                return;
            }
            for m in &selected {
                if action == "undo" && !m.has_undo {
                    tx.send(Event::Log(format!(
                        "[{}:{}] no undo function; skipped",
                        action, m.name
                    )))
                    .ok();
                    continue;
                }
                if action == "apply" && !m.has_apply {
                    tx.send(Event::Log(format!(
                        "[{}:{}] no apply function; skipped",
                        action, m.name
                    )))
                    .ok();
                    continue;
                }
                tx.send(Event::Status(m.name.clone(), "running…".into()))
                    .ok();
                tx.send(Event::Log(format!("── {} {} ──", action, m.name)))
                    .ok();
                match bazzitify::runner::run_module(&dir, m, action) {
                    Ok(r) => {
                        for line in r.output.lines() {
                            tx.send(Event::Log(line.to_string())).ok();
                        }
                        let (status, verdict) = if r.success {
                            ("applied", format!("✓ {} ok (exit 0)", m.name))
                        } else {
                            (
                                "failed",
                                format!("✗ {} failed (exit {:?})", m.name, r.exit_code),
                            )
                        };
                        let _ = status;
                        tx.send(Event::Status(
                            m.name.clone(),
                            if r.success {
                                if action == "apply" {
                                    "applied".into()
                                } else {
                                    "undone".into()
                                }
                            } else {
                                "✗ failed".into()
                            },
                        ))
                        .ok();
                        tx.send(Event::Log(verdict)).ok();
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

    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        app.on_apply_selected(move || run_modules(&handle, &tx, &dir, &mods, "apply"));
    }
    {
        let handle = handle.clone();
        let tx = tx.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        app.on_undo_selected(move || run_modules(&handle, &tx, &dir, &mods, "undo"));
    }

    app.run().expect("slint event loop failed");
}

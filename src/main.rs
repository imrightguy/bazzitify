//! bazzitify GUI — Slint frontend over the module engine.

use slint::Model;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;

slint::include_modules!();

fn modules_dir() -> PathBuf {
    // Prefer ./modules next to the binary's project root (dev), else /usr/share/bazzitify/modules
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules");
    if dev.is_dir() {
        return dev;
    }
    PathBuf::from("/usr/share/bazzitify/modules")
}

fn main() {
    let dir = modules_dir();
    let discovered = bazzitify::module::Module::discover(&dir)
        .map_err(|e| eprintln!("failed to discover modules in {}: {e}", dir.display()))
        .unwrap_or_default();

    let app = AppWindow::new().expect("failed to create window");

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

    let (tx, rx) = mpsc::channel::<String>();
    let handle = app.as_weak();

    // Background worker: drains run requests, streams log lines back via channel.
    let weak_for_worker = handle.clone();
    std::thread::spawn(move || {
        while let Ok(line) = rx.recv() {
            let weak = weak_for_worker.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(app) = weak.upgrade() {
                    let mut log: String = app.get_log_text().to_string();
                    log.push_str(&line);
                    log.push('\n');
                    app.set_log_text(log.into());
                }
            })
            .ok();
        }
    });

    let send = Rc::new(move |msg: String| {
        tx.send(msg).ok();
    });

    {
        let send = send.clone();
        let weak = handle.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        app.on_apply_selected(move || {
            let selected: Vec<String> = selected_names(&weak);
            for name in &selected {
                if let Some(m) = mods.iter().find(|m| &m.name == name) {
                    match bazzitify::runner::run_module(&dir, m, "apply") {
                        Ok(r) => {
                            let status = if r.success {
                                "✓ applied"
                            } else {
                                "✗ failed"
                            };
                            send(format!("[apply:{}]\n{}{}", name, r.output, status));
                        }
                        Err(e) => send(format!("[apply:{name}] error: {e}")),
                    }
                }
            }
        });
    }

    {
        let send = send.clone();
        let weak = handle.clone();
        let dir = dir.clone();
        let mods = discovered.clone();
        app.on_undo_selected(move || {
            let selected: Vec<String> = selected_names(&weak);
            for name in &selected {
                if let Some(m) = mods.iter().find(|m| &m.name == name) {
                    match bazzitify::runner::run_module(&dir, m, "undo") {
                        Ok(r) => {
                            let status = if r.success {
                                "✓ undone"
                            } else {
                                "✗ failed"
                            };
                            send(format!("[undo:{}]\n{}{}", name, r.output, status));
                        }
                        Err(e) => send(format!("[undo:{name}] error: {e}")),
                    }
                }
            }
        });
    }

    app.run().expect("slint event loop failed");
}

fn selected_names(weak: &slint::Weak<AppWindow>) -> Vec<String> {
    weak.upgrade()
        .map(|app| {
            app.get_modules()
                .iter()
                .filter(|m| m.selected)
                .map(|m| m.name.to_string())
                .collect()
        })
        .unwrap_or_default()
}

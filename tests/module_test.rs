use bazzitify::module::{Module, ModuleStatus};
use std::path::Path;

#[test]
fn parses_module_with_desc_and_apply() {
    let script = "#!/bin/bash\n# desc: Install gaming packages\nmodule_apply() { echo hi; }\n";
    let m = Module::parse("gaming-packages", script).unwrap();
    assert_eq!(m.name, "gaming-packages");
    assert_eq!(m.description.as_deref(), Some("Install gaming packages"));
    assert!(m.has_apply);
}

#[test]
fn module_without_apply_is_not_applicable() {
    let script = "#!/bin/bash\n# desc: broken\n";
    let m = Module::parse("broken", script).unwrap();
    assert!(!m.has_apply);
}

#[test]
fn parse_rejects_nonexistent_name() {
    assert!(Module::parse("", "x").is_err());
}

#[test]
fn discovers_modules_from_directory() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.sh"), "# desc: A\nmodule_apply() { :; }\n").unwrap();
    std::fs::write(dir.join("b.txt"), "not a module").unwrap();

    let mods = Module::discover(&dir).unwrap();
    assert_eq!(mods.len(), 1);
    assert_eq!(mods[0].name, "a");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_dir_is_error() {
    assert!(Module::discover(Path::new("/nonexistent/bazzitify/modules")).is_err());
}

#[test]
fn status_display_is_readable() {
    assert_eq!(ModuleStatus::Applied.to_string(), "applied");
    assert_eq!(ModuleStatus::Available.to_string(), "available");
}

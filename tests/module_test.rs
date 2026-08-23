use bazzitify::module::{Module, ModuleGraph, ModuleStatus};
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
fn parses_module_with_dependencies() {
    let script = "#!/bin/bash\n# desc: Kernel params\n# depends: gpu-drivers sysctl\nmodule_apply() { echo hi; }\n";
    let m = Module::parse("kernel-params", script).unwrap();
    assert_eq!(m.depends, vec!["gpu-drivers", "sysctl"]);
}

#[test]
fn parses_module_without_dependencies() {
    let script = "#!/bin/bash\n# desc: Gaming packages\nmodule_apply() { echo hi; }\n";
    let m = Module::parse("gaming-packages", script).unwrap();
    assert!(m.depends.is_empty());
}

#[test]
fn discovers_modules_with_dependencies() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}-deps", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sh"),
        "# desc: A\n# depends: b\nmodule_apply() { :; }\n",
    )
    .unwrap();
    std::fs::write(dir.join("b.sh"), "# desc: B\nmodule_apply() { :; }\n").unwrap();
    std::fs::write(
        dir.join("c.sh"),
        "# desc: C\n# depends: a b\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let mods = Module::discover(&dir).unwrap();
    assert_eq!(mods.len(), 3);

    let a = mods.iter().find(|m| m.name == "a").unwrap();
    let b = mods.iter().find(|m| m.name == "b").unwrap();
    let c = mods.iter().find(|m| m.name == "c").unwrap();

    assert_eq!(a.depends, vec!["b"]);
    assert!(b.depends.is_empty());
    assert_eq!(c.depends, vec!["a", "b"]);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn topological_sort_orders_by_dependencies() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}-topo", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sh"),
        "# desc: A\n# depends: b\nmodule_apply() { :; }\n",
    )
    .unwrap();
    std::fs::write(dir.join("b.sh"), "# desc: B\nmodule_apply() { :; }\n").unwrap();
    std::fs::write(
        dir.join("c.sh"),
        "# desc: C\n# depends: a b\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let mods = Module::discover(&dir).unwrap();
    let sorted = ModuleGraph::topological_sort(&mods).unwrap();

    // b should come before a, a and b before c
    let b_idx = sorted.iter().position(|m| m.name == "b").unwrap();
    let a_idx = sorted.iter().position(|m| m.name == "a").unwrap();
    let c_idx = sorted.iter().position(|m| m.name == "c").unwrap();

    assert!(b_idx < a_idx, "b should come before a");
    assert!(a_idx < c_idx, "a should come before c");
    assert!(b_idx < c_idx, "b should come before c");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn topological_sort_detects_cycle() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}-cycle", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sh"),
        "# desc: A\n# depends: b\nmodule_apply() { :; }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.sh"),
        "# desc: B\n# depends: a\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let mods = Module::discover(&dir).unwrap();
    let result = ModuleGraph::topological_sort(&mods);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("cycle") || err.contains("Cycle"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn topological_sort_errors_on_missing_dependency() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}-missing", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sh"),
        "# desc: A\n# depends: nonexistent\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let mods = Module::discover(&dir).unwrap();
    let result = ModuleGraph::topological_sort(&mods);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonexistent") || err.contains("missing") || err.contains("unknown"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reverse_topological_sort_for_undo() {
    let dir = std::env::temp_dir().join(format!("bazzitify-test-{}-reverse", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.sh"),
        "# desc: A\n# depends: b\nmodule_apply() { :; }\n",
    )
    .unwrap();
    std::fs::write(dir.join("b.sh"), "# desc: B\nmodule_apply() { :; }\n").unwrap();
    std::fs::write(
        dir.join("c.sh"),
        "# desc: C\n# depends: a b\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let mods = Module::discover(&dir).unwrap();
    let apply_order = ModuleGraph::topological_sort(&mods).unwrap();
    let undo_order = ModuleGraph::reverse_topological_sort(&mods).unwrap();

    // undo order should be reverse of apply order
    assert_eq!(apply_order.len(), undo_order.len());
    for i in 0..apply_order.len() {
        assert_eq!(
            apply_order[i].name,
            undo_order[apply_order.len() - 1 - i].name
        );
    }

    std::fs::remove_dir_all(&dir).unwrap();
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

#[test]
fn discovers_hdr_vrr_module() {
    let dir = std::path::Path::new("modules");
    let mods = bazzitify::module::Module::discover(dir).unwrap();
    let hdr = mods.iter().find(|m| m.name == "hdr-vrr").expect("hdr-vrr module should be discovered");
    assert!(hdr.description.is_some(), "hdr-vrr should have description");
    assert!(!hdr.long_description.is_empty(), "hdr-vrr should have long description");
    assert!(hdr.has_apply, "hdr-vrr should have module_apply");
    assert!(hdr.has_undo, "hdr-vrr should have module_undo");
    assert_eq!(hdr.depends, vec!["gpu-drivers", "display-gpu-control"]);
}

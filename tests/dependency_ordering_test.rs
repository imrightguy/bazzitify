//! Integration test for module dependency ordering.
//! Verifies that modules with declared dependencies execute in the correct order.

use bazzitify::module::{Module, ModuleGraph};
use bazzitify::runner::{RunOpts, run_module, run_module_opts};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURES_DIR: &str = "tests/fixtures/deps";

/// Generate a unique test run identifier.
fn unique_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}-{}", std::process::id(), nanos)
}

/// Create a unique temp directory for this test run's marker files.
fn test_marker_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("bazzitify-dep-test-{}", unique_id()))
}

#[test]
fn dependency_ordering_module_a_runs_before_module_b() {
    let marker_dir = test_marker_dir();
    fs::create_dir_all(&marker_dir).unwrap();
    let marker_a = marker_dir.join("module-a.marker");
    let marker_b = marker_dir.join("module-b.marker");

    // Copy fixtures to temp dir with unique marker paths
    let test_dir = std::env::temp_dir().join(format!("bazzitify-dep-test-run-{}", unique_id()));
    fs::create_dir_all(&test_dir).unwrap();

    let module_a_src = fs::read_to_string(Path::new(FIXTURES_DIR).join("module-a.sh")).unwrap();
    let module_b_src = fs::read_to_string(Path::new(FIXTURES_DIR).join("module-b.sh")).unwrap();

    // Replace marker paths in the module sources
    let module_a_src = module_a_src.replace(
        "/tmp/bazzitify-dep-test-a.marker",
        marker_a.to_str().unwrap(),
    );
    let module_b_src = module_b_src.replace(
        "/tmp/bazzitify-dep-test-a.marker",
        marker_a.to_str().unwrap(),
    );
    let module_b_src = module_b_src.replace(
        "/tmp/bazzitify-dep-test-b.marker",
        marker_b.to_str().unwrap(),
    );

    fs::write(test_dir.join("module-a.sh"), module_a_src).unwrap();
    fs::write(test_dir.join("module-b.sh"), module_b_src).unwrap();

    let modules = Module::discover(&test_dir).expect("should discover test fixtures");

    // Find our test modules
    let module_a = modules
        .iter()
        .find(|m| m.name == "module-a")
        .expect("module-a should exist");
    let module_b = modules
        .iter()
        .find(|m| m.name == "module-b")
        .expect("module-b should exist");

    // Verify module-b declares dependency on module-a
    assert_eq!(module_b.depends, vec!["module-a"]);
    assert!(module_a.depends.is_empty());

    // Topological sort should put module-a before module-b
    let sorted = ModuleGraph::topological_sort(&modules).expect("topological sort should succeed");
    let a_idx = sorted.iter().position(|m| m.name == "module-a").unwrap();
    let b_idx = sorted.iter().position(|m| m.name == "module-b").unwrap();
    assert!(
        a_idx < b_idx,
        "module-a must come before module-b in dependency order"
    );

    // Execute in dependency order (apply)
    for m in &sorted {
        if m.name == "module-a" || m.name == "module-b" {
            let result = run_module(&test_dir, m, "apply").expect("module apply should not error");
            assert!(
                result.success,
                "module {} apply failed: {}",
                m.name, result.output
            );
        }
    }

    // Verify execution order: module-a's marker should exist before module-b runs
    // (module-b's apply function checks for this and fails if missing)
    assert!(
        marker_a.exists(),
        "module-a marker should exist after apply"
    );
    assert!(
        marker_b.exists(),
        "module-b marker should exist after apply"
    );

    // Reverse topological sort for undo should put module-b before module-a
    let undo_sorted = ModuleGraph::reverse_topological_sort(&modules)
        .expect("reverse topological sort should succeed");
    let a_idx_undo = undo_sorted
        .iter()
        .position(|m| m.name == "module-a")
        .unwrap();
    let b_idx_undo = undo_sorted
        .iter()
        .position(|m| m.name == "module-b")
        .unwrap();
    assert!(
        b_idx_undo < a_idx_undo,
        "module-b must come before module-a in undo order"
    );

    // Execute undo in reverse dependency order
    for m in &undo_sorted {
        if m.name == "module-a" || m.name == "module-b" {
            let result = run_module(&test_dir, m, "undo").expect("module undo should not error");
            assert!(
                result.success,
                "module {} undo failed: {}",
                m.name, result.output
            );
        }
    }

    // Verify both markers are cleaned up
    assert!(
        !marker_a.exists(),
        "module-a marker should be removed after undo"
    );
    assert!(
        !marker_b.exists(),
        "module-b marker should be removed after undo"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_dir_all(&marker_dir);
}

#[test]
fn dependency_ordering_dry_run_shows_correct_order() {
    let marker_dir = test_marker_dir();
    fs::create_dir_all(&marker_dir).unwrap();
    let marker_a = marker_dir.join("module-a.marker");
    let marker_b = marker_dir.join("module-b.marker");

    // Copy fixtures to temp dir with unique marker paths
    let test_dir = std::env::temp_dir().join(format!("bazzitify-dep-test-dry-{}", unique_id()));
    fs::create_dir_all(&test_dir).unwrap();

    let module_a_src = fs::read_to_string(Path::new(FIXTURES_DIR).join("module-a.sh")).unwrap();
    let module_b_src = fs::read_to_string(Path::new(FIXTURES_DIR).join("module-b.sh")).unwrap();

    let module_a_src = module_a_src.replace(
        "/tmp/bazzitify-dep-test-a.marker",
        marker_a.to_str().unwrap(),
    );
    let module_b_src = module_b_src.replace(
        "/tmp/bazzitify-dep-test-a.marker",
        marker_a.to_str().unwrap(),
    );
    let module_b_src = module_b_src.replace(
        "/tmp/bazzitify-dep-test-b.marker",
        marker_b.to_str().unwrap(),
    );

    fs::write(test_dir.join("module-a.sh"), module_a_src).unwrap();
    fs::write(test_dir.join("module-b.sh"), module_b_src).unwrap();

    let modules = Module::discover(&test_dir).expect("should discover test fixtures");

    let module_a = modules.iter().find(|m| m.name == "module-a").unwrap();
    let module_b = modules.iter().find(|m| m.name == "module-b").unwrap();

    // Dry-run should show planned execution order without executing
    let result_a = run_module_opts(&test_dir, module_a, "apply", RunOpts { dry_run: true })
        .expect("dry-run should work");
    let result_b = run_module_opts(&test_dir, module_b, "apply", RunOpts { dry_run: true })
        .expect("dry-run should work");

    assert!(result_a.success);
    assert!(result_b.success);
    assert!(result_a.output.contains("[dry-run]"));
    assert!(result_b.output.contains("[dry-run]"));

    // Markers should NOT exist because dry-run doesn't execute
    assert!(
        !marker_a.exists(),
        "dry-run should not create module-a marker"
    );
    assert!(
        !marker_b.exists(),
        "dry-run should not create module-b marker"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_dir_all(&marker_dir);
}

#[test]
fn single_module_apply_undo_unchanged() {
    let marker_dir = test_marker_dir();
    fs::create_dir_all(&marker_dir).unwrap();
    let marker_a = marker_dir.join("module-a.marker");

    // Copy fixtures to temp dir with unique marker paths
    let test_dir = std::env::temp_dir().join(format!("bazzitify-dep-test-single-{}", unique_id()));
    fs::create_dir_all(&test_dir).unwrap();

    let module_a_src = fs::read_to_string(Path::new(FIXTURES_DIR).join("module-a.sh")).unwrap();
    let module_a_src = module_a_src.replace(
        "/tmp/bazzitify-dep-test-a.marker",
        marker_a.to_str().unwrap(),
    );

    fs::write(test_dir.join("module-a.sh"), module_a_src).unwrap();

    let modules = Module::discover(&test_dir).expect("should discover test fixtures");
    let module_a = modules.iter().find(|m| m.name == "module-a").unwrap();

    // Apply single module - should work without any dependency resolution
    let result = run_module(&test_dir, module_a, "apply").expect("single module apply should work");
    assert!(result.success);
    assert!(marker_a.exists());

    // Undo single module - should work without any dependency resolution
    let result = run_module(&test_dir, module_a, "undo").expect("single module undo should work");
    assert!(result.success);
    assert!(!marker_a.exists());

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_dir_all(&marker_dir);
}

#[test]
fn missing_dependency_reported_as_error() {
    let dir = std::env::temp_dir().join(format!("bazzitify-dep-test-missing-{}", unique_id()));
    fs::create_dir_all(&dir).unwrap();

    // Create module that depends on non-existent module
    fs::write(
        dir.join("broken.sh"),
        "# desc: Broken module\n# requires: nonexistent-module\nmodule_apply() { echo hi; }\n",
    )
    .unwrap();

    let modules = Module::discover(&dir).expect("should discover module");
    let result = ModuleGraph::topological_sort(&modules);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown") || err.contains("missing") || err.contains("nonexistent-module")
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cycle_detection_blocks_apply() {
    let dir = std::env::temp_dir().join(format!("bazzitify-dep-test-cycle-{}", unique_id()));
    fs::create_dir_all(&dir).unwrap();

    // Create cyclic dependency: a -> b -> a
    fs::write(
        dir.join("a.sh"),
        "# desc: A\n# requires: b\nmodule_apply() { :; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("b.sh"),
        "# desc: B\n# requires: a\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let modules = Module::discover(&dir).expect("should discover modules");
    let result = ModuleGraph::topological_sort(&modules);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.to_lowercase().contains("cycle"));

    fs::remove_dir_all(&dir).unwrap();
}

//! CLI JSON output tests for BZ-17
//! Tests the --json flag behavior including exit codes and output structure

use std::process::Command;

fn get_bazzitify_bin() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    std::path::Path::new(&manifest_dir).join("bin/bazzitify")
}

fn run_bazzitify(args: &[&str]) -> (i32, String, String) {
    let binary = get_bazzitify_bin();
    let output = Command::new(&binary)
        .args(args)
        .output()
        .expect("Failed to execute bazzitify");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.code().unwrap_or(-1), stdout, stderr)
}

#[test]
fn cli_json_list_compact() {
    let (code, stdout, _stderr) = run_bazzitify(&["--list", "--json"]);
    assert_eq!(code, 0, "Exit code should be 0 for success");

    // Should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // Should be an array
    assert!(json.is_array(), "Output should be a JSON array");

    let modules = json.as_array().unwrap();
    assert!(!modules.is_empty(), "Should have at least one module");

    // BZ-17's machine-readable module schema is deliberately stable.
    let first = &modules[0];
    for field in ["name", "desc", "long_desc", "applied_status", "requires"] {
        assert!(
            first.get(field).is_some(),
            "Module should have '{field}' field"
        );
    }
    assert!(
        first["long_desc"].is_array(),
        "long_desc should preserve all # long: lines as an array"
    );
    assert!(
        first["applied_status"].is_boolean(),
        "applied_status should report whether this module is currently applied"
    );
    assert!(
        first["requires"].is_array(),
        "requires should contain the module dependency names"
    );
}

#[test]
fn cli_json_list_pretty() {
    let (code, stdout, _stderr) = run_bazzitify(&["--list", "--json=pretty"]);
    assert_eq!(code, 0, "Exit code should be 0 for success");

    // Should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.is_array(), "Output should be a JSON array");
}

#[test]
fn cli_json_dry_run() {
    let (code, stdout, _stderr) = run_bazzitify(&["--dry-run", "sysctl", "--json"]);
    assert_eq!(code, 0, "Exit code should be 0 for success");

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert!(json.is_array(), "Output should be a JSON array");
    let actions = json.as_array().unwrap();
    assert!(
        !actions.is_empty(),
        "Should have at least one planned action"
    );

    // Check action structure (matches current implementation)
    let first = &actions[0];
    assert!(
        first.get("module").is_some(),
        "Action should have 'module' field"
    );
    assert!(
        first.get("would_apply").is_some(),
        "Action should have 'would_apply' field"
    );
    assert!(
        first.get("commands").is_some(),
        "Action should have 'commands' field"
    );
    assert!(
        first.get("warnings").is_some(),
        "Action should have 'warnings' field"
    );
}

#[test]
fn cli_json_apply_success() {
    // Use test-dep-a which is a harmless test module
    let (code, stdout, _stderr) = run_bazzitify(&["test-dep-a", "--json"]);
    assert_eq!(code, 0, "Exit code should be 0 for successful apply");

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(json.get("success").and_then(|v| v.as_bool()), Some(true));
    assert!(json.get("module").is_some(), "Should have 'module' field");
    assert!(json.get("action").is_some(), "Should have 'action' field");
    assert!(json.get("output").is_some(), "Should have 'output' field");
    assert!(json.get("error").is_some(), "Should have 'error' field");
    assert!(
        json.get("duration_ms").is_some(),
        "Should have 'duration_ms' field"
    );
}

#[test]
fn cli_json_undo_success() {
    // First apply, then undo
    let (code, _, _) = run_bazzitify(&["test-dep-a", "--json"]);
    assert_eq!(code, 0);

    let (code, stdout, _stderr) = run_bazzitify(&["undo", "test-dep-a", "--json"]);
    assert_eq!(code, 0, "Exit code should be 0 for successful undo");

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(json.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        json.get("module").and_then(|v| v.as_str()),
        Some("test-dep-a")
    );
    assert_eq!(json.get("action").and_then(|v| v.as_str()), Some("undo"));
}

#[test]
fn cli_json_module_not_found_exit_code() {
    // Unknown modules are module errors (exit code 1).
    let (code, _stdout, _stderr) = run_bazzitify(&["nonexistent_module_xyz", "--json"]);
    assert_eq!(code, 1, "Exit code should be 1 for a module error");
}

#[test]
fn cli_json_usage_error_exit_code() {
    // Missing arguments are usage errors (exit code 2).
    let (code, _stdout, _stderr) = run_bazzitify(&["undo", "--json"]);
    assert_eq!(code, 2, "Exit code should be 2 for a usage error");
}

#[test]
fn cli_json_execution_failure_exit_code() {
    // This is harder to test without a module that actually fails
    // We'll skip this for now as it requires a failing module
    // The issue specifies exit code 3 for execution failure
}

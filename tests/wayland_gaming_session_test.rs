//! Tests for the wayland-gaming-session module (issue #6 / BZ-?)

use std::fs;
use std::path::Path;

/// Test that the wayland-gaming-session module file exists and has required structure
#[test]
fn test_wayland_gaming_session_module_exists() {
    let module_path = Path::new("modules/wayland-gaming-session.sh");
    assert!(
        module_path.exists(),
        "modules/wayland-gaming-session.sh should exist"
    );
}

/// Test that module has required headers
#[test]
fn test_wayland_gaming_session_headers() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    // Check for required headers
    assert!(
        content.contains("# desc:"),
        "Module must have # desc: header"
    );
    assert!(
        content.contains("# long:"),
        "Module should have # long: header"
    );
    assert!(
        content.contains("# depends:"),
        "Module must have # depends: header"
    );

    // Check depends includes display-gpu-control and gpu-drivers
    let depends_line = content
        .lines()
        .find(|l| l.starts_with("# depends:"))
        .expect("Should have # depends: line");
    assert!(
        depends_line.contains("display-gpu-control"),
        "Should depend on display-gpu-control"
    );
    assert!(
        depends_line.contains("gpu-drivers"),
        "Should depend on gpu-drivers"
    );
}

/// Test that module has module_apply function
#[test]
fn test_wayland_gaming_session_has_apply() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    assert!(
        content.contains("module_apply()"),
        "Module must have module_apply() function"
    );
}

/// Test that module has module_undo function
#[test]
fn test_wayland_gaming_session_has_undo() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    assert!(
        content.contains("module_undo()"),
        "Module must have module_undo() function"
    );
}

/// Test that module creates gamescope.desktop session file
#[test]
fn test_wayland_gaming_session_creates_session_file() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    // Should create session file in wayland-sessions or xsessions
    assert!(
        content.contains("gamescope.desktop"),
        "Should create gamescope.desktop session"
    );
    assert!(
        content.contains("wayland-sessions"),
        "Should target wayland-sessions directory"
    );
}

/// Test that module creates environment.d snippet
#[test]
fn test_wayland_gaming_session_creates_env_file() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    // Should create environment.d snippet with Wayland gaming env vars
    assert!(
        content.contains("environment.d"),
        "Should create environment.d snippet"
    );
    assert!(
        content.contains("SDL_VIDEODRIVER=wayland"),
        "Should set SDL_VIDEODRIVER=wayland"
    );
    assert!(
        content.contains("MOZ_ENABLE_WAYLAND=1"),
        "Should set MOZ_ENABLE_WAYLAND=1"
    );
    assert!(
        content.contains("QT_QPA_PLATFORM=wayland"),
        "Should set QT_QPA_PLATFORM=wayland"
    );
    assert!(
        content.contains("WINE_D3D_CONFIG=dxvk"),
        "Should set WINE_D3D_CONFIG=dxvk"
    );
    assert!(content.contains("DXVK_ASYNC=1"), "Should set DXVK_ASYNC=1");
    assert!(content.contains("VKD3D_CONFIG"), "Should set VKD3D_CONFIG");
    assert!(
        content.contains("RADV_PERFTEST"),
        "Should set RADV_PERFTEST"
    );
}

/// Test that module_undo removes created files
#[test]
fn test_wayland_gaming_session_undo_removes_files() {
    let content = fs::read_to_string("modules/wayland-gaming-session.sh")
        .expect("Should be able to read module file");

    // Should remove session file and environment.d snippet
    assert!(
        content.contains("rm"),
        "module_undo should remove created files"
    );
    assert!(
        content.contains("gamescope.desktop"),
        "Should remove gamescope.desktop"
    );
    assert!(
        content.contains("99-bazzitify-gaming.conf"),
        "Should remove environment.d snippet"
    );
}

/// Test module passes bash syntax check
#[test]
fn test_wayland_gaming_session_bash_syntax() {
    let output = std::process::Command::new("bash")
        .args(["-n", "modules/wayland-gaming-session.sh"])
        .output()
        .expect("Should be able to run bash -n");

    assert!(
        output.status.success(),
        "bash -n should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

//! Acceptance tests for issue #42 / BZ-18 HDR/VRR helpers.

use std::fs;
use std::process::Command;

fn module_source() -> String {
    fs::read_to_string("modules/hdr-vrr.sh").expect("HDR/VRR module should be readable")
}

#[test]
fn hdr_vrr_installs_the_requested_arch_packages() {
    let source = module_source();

    assert!(
        source.contains(
            "resolve_package_list kwin-effects-hdr color-management gamescope libdisplay-info edid-decode"
        ),
        "module_apply should resolve KWin, gamescope, libdisplay-info, and edid-decode"
    );
}

#[test]
fn hdr_vrr_detail_explains_monitor_gpu_and_cable_requirements() {
    let source = module_source();

    assert!(
        source.contains("HDR requires an HDR-capable monitor, GPU support, and a suitable cable"),
        "the GUI detail text must state the HDR monitor, GPU, and cable requirements"
    );
    assert!(
        source.contains("VRR requires a VRR-capable display"),
        "the GUI detail text must state the VRR-capable display requirement"
    );
}

#[test]
fn hdr_vrr_arch_packages_are_mapped() {
    let output = Command::new("bash")
        .args([
            "-c",
            "source modules/lib/distro.sh && source modules/lib/packages.sh && resolve_package_for_distro arch edid-decode",
        ])
        .output()
        .expect("bash should be available");

    assert!(
        output.status.success(),
        "edid-decode must have an Arch mapping"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "edid-decode"
    );
}

#[test]
fn hdr_vrr_module_is_valid_bash() {
    let output = Command::new("bash")
        .args(["-n", "modules/hdr-vrr.sh"])
        .output()
        .expect("bash should be available");

    assert!(
        output.status.success(),
        "bash -n should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

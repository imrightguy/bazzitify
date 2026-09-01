//! Tests for first-run wizard functionality.

use bazzitify::module::Module;
use bazzitify::wizard::{
    FormFactor, GpuVendor, HardwareProfile, SuggestedModule, WizardState, WizardStep,
    detect_hardware_profile_from, generate_dry_run_preview, generate_dry_run_preview_from_modules,
    get_suggested_modules_for_distro, get_suggested_modules_for_distro_and_hardware,
    mark_wizard_complete, should_show_wizard, wizard_marker_path,
};
use std::fs;

#[test]
fn wizard_marker_path_is_correct() {
    // The wizard marker should be in the config directory
    let path = wizard_marker_path();
    assert!(path.to_string_lossy().contains("bazzitify"));
    assert!(path.to_string_lossy().contains("wizard"));
}

#[test]
fn should_show_wizard_returns_true_when_no_marker() {
    // Use the actual wizard marker path
    let marker_path = wizard_marker_path();

    // Clean up any existing marker
    if marker_path.exists() {
        fs::remove_file(&marker_path).ok();
    }

    // No marker file exists - should show wizard
    assert!(should_show_wizard(&marker_path));
}

#[test]
fn should_show_wizard_returns_false_when_marker_exists() {
    let marker_path = wizard_marker_path();

    // Clean up any existing marker
    if marker_path.exists() {
        fs::remove_file(&marker_path).ok();
    }

    // Create marker
    mark_wizard_complete(&marker_path).unwrap();

    // Marker file exists - should NOT show wizard
    assert!(!should_show_wizard(&marker_path));

    // Clean up
    fs::remove_file(&marker_path).ok();
}

#[test]
fn wizard_state_default_is_welcome() {
    let state = WizardState::default();
    assert_eq!(state.step, WizardStep::Welcome);
    assert!(state.suggested_modules.is_empty());
    assert!(!state.dry_run_previewed);
    assert!(!state.confirmed);
}

#[test]
fn suggested_module_has_required_fields() {
    let suggested = SuggestedModule {
        name: "test-module".to_string(),
        description: "Test description".to_string(),
        reason: "Required for distro".to_string(),
        selected: true,
    };

    assert_eq!(suggested.name, "test-module");
    assert_eq!(suggested.description, "Test description");
    assert_eq!(suggested.reason, "Required for distro");
    assert!(suggested.selected);
}

#[test]
fn get_suggested_modules_for_arch_returns_core_modules() {
    let suggested = get_suggested_modules_for_distro("arch");
    // Should include core modules for Arch/CachyOS
    let names: Vec<&str> = suggested.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"gpu-drivers"), "Arch needs gpu-drivers");
    assert!(names.contains(&"kernel-params"), "Arch needs kernel-params");
    assert!(names.contains(&"sysctl"), "Arch needs sysctl");
    assert!(names.contains(&"services"), "Arch needs services");
}

#[test]
fn get_suggested_modules_for_fedora_returns_core_modules() {
    let suggested = get_suggested_modules_for_distro("fedora");
    let names: Vec<&str> = suggested.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"gpu-drivers"), "Fedora needs gpu-drivers");
    assert!(
        names.contains(&"kernel-params"),
        "Fedora needs kernel-params"
    );
    assert!(names.contains(&"codecs"), "Fedora needs codecs");
}

#[test]
fn get_suggested_modules_for_opensuse_returns_core_modules() {
    let suggested = get_suggested_modules_for_distro("opensuse");
    let names: Vec<&str> = suggested.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"gpu-drivers"), "openSUSE needs gpu-drivers");
    assert!(
        names.contains(&"kernel-params"),
        "openSUSE needs kernel-params"
    );
    assert!(names.contains(&"codecs"), "openSUSE needs codecs");
}

#[test]
fn hardware_detection_interprets_pci_vendor_and_laptop_chassis() {
    let hardware = detect_hardware_profile_from(["0x10de"], Some("10"));

    assert_eq!(hardware.gpu_vendor, GpuVendor::Nvidia);
    assert_eq!(hardware.form_factor, FormFactor::Laptop);
}

#[test]
fn laptop_nvidia_profile_adds_hardware_relevant_suggestions() {
    let hardware = HardwareProfile {
        gpu_vendor: GpuVendor::Nvidia,
        form_factor: FormFactor::Laptop,
    };

    let suggestions = get_suggested_modules_for_distro_and_hardware("arch", &hardware);
    let names: Vec<&str> = suggestions
        .iter()
        .map(|module| module.name.as_str())
        .collect();

    assert!(names.contains(&"power-profiles"));
    assert!(names.contains(&"display-gpu-control"));
    assert!(
        suggestions
            .iter()
            .any(|module| module.name == "power-profiles" && module.reason.contains("Laptop"))
    );
}

#[test]
fn desktop_unknown_hardware_does_not_add_hardware_specific_modules() {
    let hardware = HardwareProfile {
        gpu_vendor: GpuVendor::Unknown,
        form_factor: FormFactor::Desktop,
    };

    let suggestions = get_suggested_modules_for_distro_and_hardware("arch", &hardware);
    let names: Vec<&str> = suggestions
        .iter()
        .map(|module| module.name.as_str())
        .collect();

    assert!(!names.contains(&"power-profiles"));
    assert!(!names.contains(&"display-gpu-control"));
}

#[test]
fn wizard_can_generate_dry_run_preview() {
    let suggested = vec![
        SuggestedModule {
            name: "gpu-drivers".to_string(),
            description: "Install GPU drivers".to_string(),
            reason: "Required for graphics".to_string(),
            selected: true,
        },
        SuggestedModule {
            name: "kernel-params".to_string(),
            description: "Optimize kernel parameters".to_string(),
            reason: "Performance tuning".to_string(),
            selected: true,
        },
    ];

    let preview = generate_dry_run_preview(&suggested);
    assert!(preview.contains("gpu-drivers"));
    assert!(preview.contains("kernel-params"));
    assert!(preview.contains("[dry-run]") || preview.contains("dry-run"));
}

#[test]
fn wizard_dry_run_preview_includes_dependency_ordered_module_bodies() {
    let temp_dir = std::env::temp_dir().join(format!(
        "bazzitify-wizard-dry-run-test-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let dependency_source = "# desc: Dependency\nmodule_apply() { echo dependency-body; }\n";
    let dependent_source =
        "# desc: Dependent\n# requires: dependency\nmodule_apply() { echo dependent-body; }\n";
    fs::write(temp_dir.join("dependency.sh"), dependency_source).unwrap();
    fs::write(temp_dir.join("dependent.sh"), dependent_source).unwrap();
    let dependent = Module::parse("dependent", dependent_source).unwrap();
    let dependency = Module::parse("dependency", dependency_source).unwrap();

    let preview =
        generate_dry_run_preview_from_modules(&temp_dir, &[dependent, dependency]).unwrap();

    let dependency_index = preview.find("dependency-body").unwrap();
    let dependent_index = preview.find("dependent-body").unwrap();
    assert!(dependency_index < dependent_index);
    assert!(preview.contains("[dry-run]"));

    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn wizard_can_mark_complete() {
    let temp_dir =
        std::env::temp_dir().join(format!("bazzitify-wizard-test-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let marker_path = temp_dir.join("wizard_done");
    assert!(!marker_path.exists());

    mark_wizard_complete(&marker_path).unwrap();

    assert!(marker_path.exists());
    let content = fs::read_to_string(&marker_path).unwrap();
    assert!(content.contains("done") || content.contains("complete"));

    fs::remove_dir_all(&temp_dir).unwrap();
}

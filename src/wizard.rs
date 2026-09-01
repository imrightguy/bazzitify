//! First-run wizard functionality.

use crate::module::{Module, ModuleGraph};
use crate::runner::{RunOpts, run_module_opts};
use std::fs;
use std::path::{Path, PathBuf};

/// Wizard step enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    DistroDetect,
    ModuleSelection,
    DryRunPreview,
    Confirm,
    Complete,
}

/// Suggested module for the wizard.
#[derive(Debug, Clone)]
pub struct SuggestedModule {
    pub name: String,
    pub description: String,
    pub reason: String,
    pub selected: bool,
}

/// Wizard state machine.
#[derive(Debug, Clone)]
pub struct WizardState {
    pub step: WizardStep,
    pub detected_distro: String,
    pub suggested_modules: Vec<SuggestedModule>,
    pub dry_run_previewed: bool,
    pub confirmed: bool,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            step: WizardStep::Welcome,
            detected_distro: String::new(),
            suggested_modules: Vec::new(),
            dry_run_previewed: false,
            confirmed: false,
        }
    }
}

/// Get the wizard marker file path.
pub fn wizard_marker_path() -> PathBuf {
    config_dir().join("wizard_done")
}

/// Check if the wizard should be shown (no marker file exists).
pub fn should_show_wizard(marker_path: &Path) -> bool {
    !marker_path.exists()
}

/// Mark the wizard as complete by creating the marker file.
pub fn mark_wizard_complete(marker_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker_path, "complete")
}

/// Hardware traits that affect the wizard's safe default module selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    Unknown,
}

/// Form factor used to avoid recommending laptop-specific tuning to desktops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFactor {
    Laptop,
    Desktop,
    Unknown,
}

/// Detectable hardware inputs for the wizard suggestion policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareProfile {
    pub gpu_vendor: GpuVendor,
    pub form_factor: FormFactor,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            gpu_vendor: GpuVendor::Unknown,
            form_factor: FormFactor::Unknown,
        }
    }
}

/// Convert PCI vendor IDs and a DMI chassis type into the profile used by the
/// suggestion policy. Kept pure so the policy is testable without host hardware.
pub fn detect_hardware_profile_from<I, S>(
    vendor_ids: I,
    chassis_type: Option<&str>,
) -> HardwareProfile
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let gpu_vendor = vendor_ids
        .into_iter()
        .find_map(
            |id| match id.as_ref().trim().to_ascii_lowercase().as_str() {
                "0x10de" => Some(GpuVendor::Nvidia),
                "0x1002" | "0x1022" => Some(GpuVendor::Amd),
                "0x8086" => Some(GpuVendor::Intel),
                _ => None,
            },
        )
        .unwrap_or(GpuVendor::Unknown);
    let form_factor = match chassis_type.map(str::trim) {
        Some("8" | "9" | "10" | "14") => FormFactor::Laptop,
        Some(_) => FormFactor::Desktop,
        None => FormFactor::Unknown,
    };

    HardwareProfile {
        gpu_vendor,
        form_factor,
    }
}

/// Detect local hardware conservatively. Missing sysfs information is treated
/// as unknown so it can never trigger a hardware-specific recommendation.
pub fn detect_hardware_profile() -> HardwareProfile {
    let vendor_ids = fs::read_dir("/sys/class/drm")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_to_string(entry.path().join("device/vendor")).ok())
        .collect::<Vec<_>>();
    let chassis_type = fs::read_to_string("/sys/class/dmi/id/chassis_type").ok();
    detect_hardware_profile_from(vendor_ids, chassis_type.as_deref())
}

/// Get suggested modules for a given distro using conservative hardware-neutral defaults.
pub fn get_suggested_modules_for_distro(distro: &str) -> Vec<SuggestedModule> {
    get_suggested_modules_for_distro_and_hardware(distro, &HardwareProfile::default())
}

/// Get suggested modules for a distro and explicit hardware profile.
///
/// Hardware-specific recommendations remain opt-in through the wizard checklist;
/// this function only selects safe defaults relevant to the detected device.
pub fn get_suggested_modules_for_distro_and_hardware(
    distro: &str,
    hardware: &HardwareProfile,
) -> Vec<SuggestedModule> {
    let mut suggestions = vec![
        // Core modules for all distros
        SuggestedModule {
            name: "gpu-drivers".to_string(),
            description: "Install and configure GPU drivers (NVIDIA/AMD/Intel)".to_string(),
            reason: "Required for gaming and hardware acceleration".to_string(),
            selected: true,
        },
        SuggestedModule {
            name: "kernel-params".to_string(),
            description: "Optimize kernel parameters for gaming performance".to_string(),
            reason: "Reduces latency and improves frame pacing".to_string(),
            selected: true,
        },
        SuggestedModule {
            name: "sysctl".to_string(),
            description: "System control parameters for performance".to_string(),
            reason: "Tunes kernel for gaming workloads".to_string(),
            selected: true,
        },
        SuggestedModule {
            name: "services".to_string(),
            description: "Enable and configure gaming-related system services".to_string(),
            reason: "Ensures required services are running".to_string(),
            selected: true,
        },
    ];

    // Distro-specific additions
    match distro.to_lowercase().as_str() {
        d if d.contains("arch") || d.contains("cachy") || d.contains("bazzite") => {
            suggestions.push(SuggestedModule {
                name: "gaming-packages".to_string(),
                description: "Install gaming libraries and tools (Steam, Lutris, etc.)".to_string(),
                reason: "Arch-based distros need gaming packages installed".to_string(),
                selected: true,
            });
            suggestions.push(SuggestedModule {
                name: "flatpak".to_string(),
                description: "Enable Flatpak support for sandboxed apps".to_string(),
                reason: "Flatpak provides additional gaming applications".to_string(),
                selected: true,
            });
        }
        d if d.contains("fedora") => {
            suggestions.push(SuggestedModule {
                name: "codecs".to_string(),
                description: "Install multimedia codecs for video playback".to_string(),
                reason: "Fedora requires separate codec installation".to_string(),
                selected: true,
            });
            suggestions.push(SuggestedModule {
                name: "gaming-packages".to_string(),
                description: "Install gaming libraries and tools".to_string(),
                reason: "Fedora needs gaming packages from RPM Fusion".to_string(),
                selected: true,
            });
        }
        d if d.contains("opensuse") || d.contains("suse") => {
            suggestions.push(SuggestedModule {
                name: "codecs".to_string(),
                description: "Install multimedia codecs for video playback".to_string(),
                reason: "openSUSE requires separate codec installation".to_string(),
                selected: true,
            });
            suggestions.push(SuggestedModule {
                name: "gaming-packages".to_string(),
                description: "Install gaming libraries and tools".to_string(),
                reason: "openSUSE needs gaming packages".to_string(),
                selected: true,
            });
        }
        _ => {}
    }

    match hardware.gpu_vendor {
        GpuVendor::Amd | GpuVendor::Intel | GpuVendor::Nvidia => {
            suggestions.push(SuggestedModule {
                name: "display-gpu-control".to_string(),
                description: "Install display and GPU control tools".to_string(),
                reason: "Recommended for the detected GPU".to_string(),
                selected: true,
            })
        }
        GpuVendor::Unknown => {}
    }

    if hardware.form_factor == FormFactor::Laptop {
        suggestions.push(SuggestedModule {
            name: "power-profiles".to_string(),
            description: "Configure power profiles for gaming and battery use".to_string(),
            reason: "Laptop detected; provides a reversible gaming/battery profile".to_string(),
            selected: true,
        });
    }

    suggestions
}

/// Generate a dry-run preview string for the selected modules.
pub fn generate_dry_run_preview(suggested: &[SuggestedModule]) -> String {
    let selected: Vec<&SuggestedModule> = suggested.iter().filter(|s| s.selected).collect();
    if selected.is_empty() {
        return "No modules selected for dry-run.".to_string();
    }

    let mut preview = String::from("DRY RUN — planned apply order:\n");
    for (i, module) in selected.iter().enumerate() {
        preview.push_str(&format!(
            "  {}. {} — {}\n",
            i + 1,
            module.name,
            module.description
        ));
    }
    preview.push_str("\nRun with --dry-run to see full dependency-ordered execution plan.");
    preview
}

/// Generate a dependency-ordered dry-run preview using the same module runner as apply.
///
/// This returns the actual `module_apply` function bodies with the runner's
/// `[dry-run]` prefix and never executes a module.
pub fn generate_dry_run_preview_from_modules(
    modules_dir: &Path,
    selected: &[Module],
) -> Result<String, String> {
    if selected.is_empty() {
        return Ok("No modules selected for dry-run.".to_string());
    }

    let sorted = ModuleGraph::topological_sort(selected).map_err(|error| error.to_string())?;
    let mut preview = String::from("DRY RUN — dependency-ordered module changes:\n");
    for module in sorted {
        preview.push_str(&format!("\n── apply {} ──\n", module.name));
        let result = run_module_opts(modules_dir, &module, "apply", RunOpts { dry_run: true })
            .map_err(|error| format!("could not inspect {}: {error}", module.name))?;
        preview.push_str(&result.output);
        if !result.success {
            return Err(format!(
                "dry-run inspection failed for {} (exit {:?})\n{}",
                module.name, result.exit_code, result.output
            ));
        }
    }
    Ok(preview)
}

/// Get config directory path (shared with main.rs)
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bazzitify")
        .join("profiles")
}

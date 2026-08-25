//! First-run wizard functionality.

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

/// Get suggested modules for a given distro.
pub fn get_suggested_modules_for_distro(distro: &str) -> Vec<SuggestedModule> {
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

/// Get config directory path (shared with main.rs)
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bazzitify")
        .join("profiles")
}

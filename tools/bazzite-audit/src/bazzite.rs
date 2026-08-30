//! Bazzite repository handling

use crate::{AuditError, BazziteTweak, Result, TweakCategory};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BazziteRepo {
    path: PathBuf,
}

impl BazziteRepo {
    pub fn clone(shallow: bool) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("bazzite-audit");
        let repo_path = temp_dir.join("bazzite");

        // Clean up previous clone if exists
        if repo_path.exists() {
            fs::remove_dir_all(&repo_path)?;
        }
        fs::create_dir_all(&temp_dir)?;

        let mut cmd = Command::new("git");
        cmd.arg("clone");
        if shallow {
            cmd.arg("--depth=1");
        }
        cmd.arg("https://github.com/ublue-os/bazzite.git")
            .arg(&repo_path);

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(AuditError::Git(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(Self { path: repo_path })
    }

    pub fn get_head_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(["rev-parse", "HEAD"])
            .output()?;

        if !output.status.success() {
            return Err(AuditError::Git(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn get_version(&self) -> Result<String> {
        // Try to get version from Containerfile or other source
        let containerfile = self.path.join("Containerfile");
        if containerfile.exists() {
            let content = fs::read_to_string(&containerfile)?;
            for line in content.lines() {
                if line.contains("FEDORA_VERSION") && line.contains("=") {
                    // Extract version like "44" from ARG FEDORA_VERSION="44"
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            return Ok(format!("Fedora {}", &line[start + 1..start + 1 + end]));
                        }
                    }
                }
            }
        }
        Ok("unknown".to_string())
    }

    pub fn extract_tweaks(&self) -> Result<Vec<BazziteTweak>> {
        let mut tweaks = Vec::new();

        // Extract from system_files
        tweaks.extend(self.extract_from_system_files()?);

        // Extract from build_files
        tweaks.extend(self.extract_from_build_files()?);

        // Extract from spec_files (package lists)
        tweaks.extend(self.extract_from_spec_files()?);

        // Extract from Containerfile
        tweaks.extend(self.extract_from_containerfile()?);

        Ok(tweaks)
    }

    fn extract_from_system_files(&self) -> Result<Vec<BazziteTweak>> {
        let mut tweaks = Vec::new();
        let system_files = self.path.join("system_files");

        if !system_files.exists() {
            return Ok(tweaks);
        }

        // Walk system_files directory
        for entry in walkdir::WalkDir::new(&system_files)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let rel_path = path.strip_prefix(&self.path).unwrap_or(path);

            // Skip binary files, only process text configs
            if self.is_text_file(path) {
                if let Ok(content) = fs::read_to_string(path) {
                    if !content.trim().is_empty() {
                        let category = self.categorize_system_file(rel_path);
                        tweaks.push(BazziteTweak {
                            category,
                            name: self.extract_name_from_path(rel_path),
                            description: self.generate_description(category, &content),
                            source_file: rel_path.to_string_lossy().to_string(),
                            raw_content: content,
                        });
                    }
                }
            }
        }

        Ok(tweaks)
    }

    fn extract_from_build_files(&self) -> Result<Vec<BazziteTweak>> {
        let mut tweaks = Vec::new();
        let build_files = self.path.join("build_files");

        if !build_files.exists() {
            return Ok(tweaks);
        }

        for entry in walkdir::WalkDir::new(&build_files)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let rel_path = path.strip_prefix(&self.path).unwrap_or(path);

            if self.is_text_file(path) {
                if let Ok(content) = fs::read_to_string(path) {
                    if !content.trim().is_empty() && !content.starts_with("#!/") {
                        // Skip shebang scripts for now, focus on config
                        let category = TweakCategory::Other;
                        tweaks.push(BazziteTweak {
                            category,
                            name: self.extract_name_from_path(rel_path),
                            description: format!(
                                "Build script: {}",
                                path.file_name().unwrap().to_string_lossy()
                            ),
                            source_file: rel_path.to_string_lossy().to_string(),
                            raw_content: content,
                        });
                    }
                }
            }
        }

        Ok(tweaks)
    }

    fn extract_from_spec_files(&self) -> Result<Vec<BazziteTweak>> {
        let mut tweaks = Vec::new();
        let spec_files = self.path.join("spec_files");

        if !spec_files.exists() {
            return Ok(tweaks);
        }

        // Each spec file directory represents a package
        for entry in walkdir::WalkDir::new(&spec_files)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            let path = entry.path();
            if path == spec_files {
                continue;
            }

            let package_name = path.file_name().unwrap().to_string_lossy().to_string();
            tweaks.push(BazziteTweak {
                category: TweakCategory::PackageLists,
                name: package_name.clone(),
                description: format!("RPM package: {}", package_name),
                source_file: path
                    .strip_prefix(&self.path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                raw_content: format!("Package: {}", package_name),
            });
        }

        Ok(tweaks)
    }

    fn extract_from_containerfile(&self) -> Result<Vec<BazziteTweak>> {
        let mut tweaks = Vec::new();
        let containerfile = self.path.join("Containerfile");

        if !containerfile.exists() {
            return Ok(tweaks);
        }

        let content = fs::read_to_string(&containerfile)?;

        // Extract RUN commands that install packages or configure system
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("RUN ") || trimmed.starts_with("COPY ") {
                tweaks.push(BazziteTweak {
                    category: TweakCategory::Other,
                    name: format!("Containerfile_step_{}", i),
                    description: trimmed.chars().take(100).collect::<String>(),
                    source_file: "Containerfile".to_string(),
                    raw_content: trimmed.to_string(),
                });
            }
        }

        Ok(tweaks)
    }

    fn is_text_file(&self, path: &Path) -> bool {
        // Skip binary files
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(
                ext.as_str(),
                "conf"
                    | "service"
                    | "rules"
                    | "toml"
                    | "yaml"
                    | "yml"
                    | "json"
                    | "sh"
                    | "bash"
                    | "py"
                    | "txt"
                    | "md"
                    | "override"
                    | "preset"
                    | "config"
                    | "ini"
                    | "gschema"
                    | "dconf"
                    | "modprobe"
                    | "sysctl"
                    | "udev"
            )
        } else {
            // Check for common config file names without extensions
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            matches!(
                name.as_str(),
                "justfile"
                    | "makefile"
                    | "dockerfile"
                    | "containerfile"
                    | "profile"
                    | "environment"
                    | "limits"
                    | "fstab"
                    | "crypttab"
            )
        }
    }

    fn categorize_system_file(&self, path: &Path) -> TweakCategory {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains("sysctl") || path_str.ends_with(".sysctl") {
            TweakCategory::Sysctl
        } else if path_str.contains("systemd")
            || path_str.contains(".service")
            || path_str.contains(".timer")
            || path_str.contains(".socket")
        {
            TweakCategory::SystemdUnits
        } else if path_str.contains("udev") || path_str.ends_with(".rules") {
            TweakCategory::UdevRules
        } else if path_str.contains("kwin")
            || path_str.contains("hyprland")
            || path_str.contains("sway")
            || path_str.contains("gamescope")
            || path_str.contains("compositor")
            || path_str.contains("wayland")
        {
            TweakCategory::CompositorConfigs
        } else if path_str.contains("power")
            || path_str.contains("tuned")
            || path_str.contains("cpupower")
            || path_str.contains("ppd")
        {
            TweakCategory::PowerProfiles
        } else if path_str.contains("pipewire")
            || path_str.contains("wireplumber")
            || path_str.contains("alsa")
            || path_str.contains("pulse")
        {
            TweakCategory::PipewireConfig
        } else if path_str.contains("modprobe") || path_str.ends_with(".modprobe") {
            TweakCategory::ModprobeConfig
        } else if path_str.contains("dracut") || path_str.contains("initramfs") {
            TweakCategory::DracutConfig
        } else if path_str.contains("environment")
            || path_str.contains("env.d")
            || path_str.contains("profile.d")
        {
            TweakCategory::EnvironmentVars
        } else if path_str.contains("font") || path_str.contains("fontconfig") {
            TweakCategory::FontConfig
        } else if path_str.contains("kernel")
            || path_str.contains("cmdline")
            || path_str.contains("grub")
            || path_str.contains("bootloader")
        {
            TweakCategory::KernelParams
        } else {
            TweakCategory::Other
        }
    }

    fn extract_name_from_path(&self, path: &Path) -> String {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    fn generate_description(&self, category: TweakCategory, content: &str) -> String {
        let first_line = content.lines().next().unwrap_or("").trim();
        let prefix = match category {
            TweakCategory::KernelParams => "Kernel parameter: ",
            TweakCategory::Sysctl => "Sysctl setting: ",
            TweakCategory::SystemdUnits => "Systemd unit: ",
            TweakCategory::PackageLists => "Package: ",
            TweakCategory::UdevRules => "Udev rule: ",
            TweakCategory::CompositorConfigs => "Compositor config: ",
            TweakCategory::PowerProfiles => "Power profile: ",
            TweakCategory::PipewireConfig => "PipeWire config: ",
            TweakCategory::EnvironmentVars => "Environment variable: ",
            TweakCategory::DracutConfig => "Dracut config: ",
            TweakCategory::ModprobeConfig => "Modprobe config: ",
            TweakCategory::FontConfig => "Font config: ",
            TweakCategory::Other => "Config: ",
        };

        if first_line.is_empty() {
            format!("{}<empty>", prefix)
        } else if first_line.len() > 80 {
            format!("{} {}...", prefix, &first_line[..77])
        } else {
            format!("{} {}", prefix, first_line)
        }
    }
}

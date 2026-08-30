//! Module catalog loading and management

use crate::{AuditError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub desc: String,
    pub long_desc: Vec<String>,
    pub provides: Vec<String>,
    pub category: String,
}

pub struct ModuleCatalog {
    pub modules: HashMap<String, ModuleInfo>,
    pub by_category: HashMap<String, Vec<String>>,
}

impl ModuleCatalog {
    pub fn load(modules_dir: &str) -> Result<Self> {
        let mut modules = HashMap::new();
        let mut by_category: HashMap<String, Vec<String>> = HashMap::new();

        let dir = Path::new(modules_dir);
        if !dir.exists() {
            return Err(AuditError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Modules directory not found: {}", modules_dir),
            )));
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            #[allow(clippy::collapsible_if)]
            if path.extension().and_then(|s| s.to_str()) == Some("sh") {
                if let Ok(module) = Self::parse_module(&path) {
                    let category = module.category.clone();
                    by_category
                        .entry(category)
                        .or_default()
                        .push(module.name.clone());
                    modules.insert(module.name.clone(), module);
                }
            }
        }

        Ok(Self {
            modules,
            by_category,
        })
    }

    fn parse_module(path: &Path) -> Result<ModuleInfo> {
        let content = fs::read_to_string(path)?;
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut desc = String::new();
        let mut long_desc = Vec::new();
        let mut provides = Vec::new();
        let mut category = "uncategorized".to_string();

        for line in content.lines() {
            let trimmed = line.trim();

            if let Some(stripped) = trimmed.strip_prefix("# desc:") {
                desc = stripped.trim().to_string();
            } else if let Some(stripped) = trimmed.strip_prefix("# long:") {
                long_desc.push(stripped.trim().to_string());
            } else if let Some(stripped) = trimmed.strip_prefix("# provides:") {
                let prov = stripped.trim();
                provides = prov.split_whitespace().map(|s| s.to_string()).collect();
            } else if let Some(stripped) = trimmed.strip_prefix("# category:") {
                category = stripped.trim().to_string();
            }
        }

        // If no explicit category, infer from name
        if category == "uncategorized" {
            category = Self::infer_category(&name);
        }

        Ok(ModuleInfo {
            name,
            desc,
            long_desc,
            provides,
            category,
        })
    }

    fn infer_category(name: &str) -> String {
        let name = name.to_lowercase();
        if name.contains("kernel") || name.contains("boot") || name.contains("grub") {
            "kernel".to_string()
        } else if name.contains("sysctl") || name.contains("sysfs") {
            "sysctl".to_string()
        } else if name.contains("service") || name.contains("systemd") {
            "services".to_string()
        } else if name.contains("udev") || name.contains("hwdb") {
            "udev".to_string()
        } else if name.contains("kwin")
            || name.contains("hypr")
            || name.contains("gamescope")
            || name.contains("display")
            || name.contains("gpu")
            || name.contains("hdr")
            || name.contains("vrr")
        {
            "display".to_string()
        } else if name.contains("power")
            || name.contains("cpu")
            || name.contains("battery")
            || name.contains("tuned")
        {
            "power".to_string()
        } else if name.contains("pipewire") || name.contains("audio") || name.contains("codec") {
            "audio".to_string()
        } else if name.contains("font") {
            "fonts".to_string()
        } else if name.contains("nvidia") || name.contains("amd") || name.contains("intel") {
            "gpu".to_string()
        } else if name.contains("flatpak") || name.contains("podman") || name.contains("distrobox")
        {
            "containers".to_string()
        } else if name.contains("network") || name.contains("dns") || name.contains("firewall") {
            "network".to_string()
        } else if name.contains("security") || name.contains("selinux") || name.contains("apparmor")
        {
            "security".to_string()
        } else {
            "other".to_string()
        }
    }

    pub fn find_by_category(&self, category: &str) -> Vec<&ModuleInfo> {
        self.by_category
            .get(category)
            .map(|names| names.iter().filter_map(|n| self.modules.get(n)).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_module() {
        let dir = tempdir().unwrap();
        let module_path = dir.path().join("test-module.sh");
        let mut file = fs::File::create(&module_path).unwrap();
        writeln!(file, "#!/bin/bash").unwrap();
        writeln!(file, "# desc: Test module description").unwrap();
        writeln!(file, "# long: Detailed line 1").unwrap();
        writeln!(file, "# long: Detailed line 2").unwrap();
        writeln!(file, "# depends: dep1 dep2").unwrap();
        writeln!(file, "# provides: feature1").unwrap();
        writeln!(file, "# category: test").unwrap();
        writeln!(file, "module_apply() {{ echo apply; }}").unwrap();
        writeln!(file, "module_undo() {{ echo undo; }}").unwrap();

        let module = ModuleCatalog::parse_module(&module_path).unwrap();
        assert_eq!(module.name, "test-module");
        assert_eq!(module.desc, "Test module description");
        assert_eq!(module.long_desc, vec!["Detailed line 1", "Detailed line 2"]);
        assert_eq!(module.provides, vec!["feature1"]);
        assert_eq!(module.category, "test");
    }

    #[test]
    fn test_infer_category() {
        assert_eq!(ModuleCatalog::infer_category("kernel-params"), "kernel");
        assert_eq!(ModuleCatalog::infer_category("sysctl-tweaks"), "sysctl");
        assert_eq!(ModuleCatalog::infer_category("hdr-vrr"), "display");
        assert_eq!(ModuleCatalog::infer_category("power-profiles"), "power");
        assert_eq!(ModuleCatalog::infer_category("pipewire-config"), "audio");
        assert_eq!(ModuleCatalog::infer_category("nvidia-drivers"), "gpu");
        assert_eq!(ModuleCatalog::infer_category("flatpak-setup"), "containers");
        assert_eq!(ModuleCatalog::infer_category("unknown-module"), "other");
    }
}

//! Audit engine - maps Bazzite tweaks to bazzitify modules

use crate::module::ModuleInfo;
use crate::{BazziteTweak, MappingStatus, ModuleCatalog, ModuleMapping, TweakCategory};
use std::collections::HashMap;

pub struct AuditEngine {
    catalog: ModuleCatalog,
    // Keywords that indicate a module implements a certain tweak
    module_keywords: HashMap<String, Vec<String>>,
}

impl AuditEngine {
    pub fn new(catalog: ModuleCatalog) -> Self {
        let mut module_keywords = HashMap::new();

        // Define keywords for each module category/tweak type
        module_keywords.insert(
            "kernel-params".to_string(),
            vec![
                "kernel".to_string(),
                "cmdline".to_string(),
                "boot".to_string(),
                "grub".to_string(),
            ],
        );
        module_keywords.insert(
            "sysctl".to_string(),
            vec![
                "sysctl".to_string(),
                "sysfs".to_string(),
                "vm.".to_string(),
                "net.".to_string(),
            ],
        );
        module_keywords.insert(
            "services".to_string(),
            vec![
                "systemd".to_string(),
                "service".to_string(),
                "socket".to_string(),
                "timer".to_string(),
            ],
        );
        module_keywords.insert(
            "udev".to_string(),
            vec!["udev".to_string(), "rules".to_string(), "hwdb".to_string()],
        );
        module_keywords.insert(
            "display".to_string(),
            vec![
                "kwin".to_string(),
                "hyprland".to_string(),
                "gamescope".to_string(),
                "wayland".to_string(),
                "drm".to_string(),
                "hdr".to_string(),
                "vrr".to_string(),
                "compositor".to_string(),
                "display".to_string(),
                "gpu".to_string(),
            ],
        );
        module_keywords.insert(
            "power".to_string(),
            vec![
                "power".to_string(),
                "tuned".to_string(),
                "cpupower".to_string(),
                "battery".to_string(),
                "ppd".to_string(),
                "governor".to_string(),
            ],
        );
        module_keywords.insert(
            "audio".to_string(),
            vec![
                "pipewire".to_string(),
                "wireplumber".to_string(),
                "alsa".to_string(),
                "pulse".to_string(),
                "codec".to_string(),
                "audio".to_string(),
            ],
        );
        module_keywords.insert(
            "fonts".to_string(),
            vec!["font".to_string(), "fontconfig".to_string()],
        );
        module_keywords.insert(
            "gpu".to_string(),
            vec![
                "nvidia".to_string(),
                "amd".to_string(),
                "intel".to_string(),
                "vulkan".to_string(),
                "opengl".to_string(),
            ],
        );
        module_keywords.insert(
            "containers".to_string(),
            vec![
                "flatpak".to_string(),
                "podman".to_string(),
                "distrobox".to_string(),
                "container".to_string(),
            ],
        );
        module_keywords.insert(
            "network".to_string(),
            vec![
                "network".to_string(),
                "dns".to_string(),
                "firewall".to_string(),
                "resolved".to_string(),
            ],
        );
        module_keywords.insert(
            "security".to_string(),
            vec![
                "selinux".to_string(),
                "apparmor".to_string(),
                "security".to_string(),
            ],
        );

        Self {
            catalog,
            module_keywords,
        }
    }

    pub fn map_tweaks(&self, tweaks: Vec<BazziteTweak>) -> Vec<ModuleMapping> {
        tweaks
            .into_iter()
            .map(|tweak| self.map_tweak(tweak))
            .collect()
    }

    fn map_tweak(&self, tweak: BazziteTweak) -> ModuleMapping {
        // First, try exact module name match
        if let Some(module) = self.catalog.modules.get(&tweak.name) {
            return ModuleMapping {
                bazzite_tweak: tweak,
                bazzitify_module: Some(module.name.clone()),
                status: MappingStatus::Implemented,
                confidence: 0.95,
            };
        }

        // Try category-based matching
        let category_key = format!("{:?}", tweak.category).to_lowercase();
        let category_module = self.catalog.find_by_category(&category_key);
        if !category_module.is_empty() {
            // Check if any module in this category matches keywords
            for module in &category_module {
                if self.module_matches_tweak(module, &tweak) {
                    return ModuleMapping {
                        bazzite_tweak: tweak,
                        bazzitify_module: Some(module.name.clone()),
                        status: MappingStatus::Implemented,
                        confidence: 0.8,
                    };
                }
            }
            // Category matches but no keyword match - partial
            return ModuleMapping {
                bazzite_tweak: tweak,
                bazzitify_module: Some(category_module[0].name.clone()),
                status: MappingStatus::Partial,
                confidence: 0.5,
            };
        }

        // Try keyword matching across all modules
        let mut best_match: Option<(&ModuleInfo, f32)> = None;
        for module in self.catalog.modules.values() {
            let score = self.calculate_match_score(module, &tweak);
            if score > 0.3 && (best_match.is_none() || score > best_match.unwrap().1) {
                best_match = Some((module, score));
            }
        }

        if let Some((module, score)) = best_match {
            let status = if score > 0.7 {
                MappingStatus::Implemented
            } else if score > 0.4 {
                MappingStatus::Partial
            } else {
                MappingStatus::Missing
            };
            ModuleMapping {
                bazzite_tweak: tweak,
                bazzitify_module: Some(module.name.clone()),
                status,
                confidence: score,
            }
        } else {
            // No match found
            let status = if matches!(
                tweak.category,
                TweakCategory::PackageLists | TweakCategory::Other
            ) {
                MappingStatus::NotApplicable
            } else {
                MappingStatus::Missing
            };
            ModuleMapping {
                bazzite_tweak: tweak,
                bazzitify_module: None,
                status,
                confidence: 0.0,
            }
        }
    }

    fn module_matches_tweak(&self, module: &ModuleInfo, tweak: &BazziteTweak) -> bool {
        // Check module name, description, and provides against tweak
        let haystack = format!(
            "{} {} {}",
            module.name,
            module.desc,
            module.long_desc.join(" ")
        )
        .to_lowercase();

        let keywords = self
            .module_keywords
            .get(&format!("{:?}", tweak.category).to_lowercase())
            .cloned()
            .unwrap_or_default();

        for keyword in &keywords {
            if haystack.contains(&keyword.to_lowercase()) {
                return true;
            }
        }

        // Also check tweak name and content
        let tweak_haystack = format!("{} {}", tweak.name, tweak.raw_content).to_lowercase();
        for keyword in &keywords {
            if tweak_haystack.contains(&keyword.to_lowercase()) {
                return true;
            }
        }

        false
    }

    fn calculate_match_score(&self, module: &ModuleInfo, tweak: &BazziteTweak) -> f32 {
        let mut score: f32 = 0.0;

        let module_text = format!(
            "{} {} {} {}",
            module.name,
            module.desc,
            module.long_desc.join(" "),
            module.provides.join(" ")
        )
        .to_lowercase();

        let tweak_text = format!("{} {}", tweak.name, tweak.raw_content).to_lowercase();

        // Check category keywords
        let keywords = self
            .module_keywords
            .get(&format!("{:?}", tweak.category).to_lowercase())
            .cloned()
            .unwrap_or_default();

        for keyword in keywords {
            let kw = keyword.to_lowercase();
            if module_text.contains(&kw) && tweak_text.contains(&kw) {
                score += 0.2;
            } else if module_text.contains(&kw) || tweak_text.contains(&kw) {
                score += 0.1;
            }
        }

        // Direct name similarity
        if module.name == tweak.name {
            score += 0.5;
        } else if module.name.contains(&tweak.name) || tweak.name.contains(&module.name) {
            score += 0.3;
        }

        // Check provides
        for provide in &module.provides {
            if tweak_text.contains(provide) {
                score += 0.15;
            }
        }

        score.min(1.0)
    }
}

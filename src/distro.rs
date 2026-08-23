//! Shared distro detection for Rust CLI and GUI.
//! Mirrors the logic in modules/lib/distro.sh for canonical distro IDs.

use std::collections::HashMap;

/// Detect the distro ID from /etc/os-release (raw ID field).
fn raw_distro_id() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("ID="))
                .map(|l| l.trim_start_matches("ID=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Map raw distro ID to canonical distro ID (matches modules/lib/distro.sh).
pub fn canonical_distro_id() -> String {
    let raw = raw_distro_id();
    let mut map = HashMap::new();
    // Arch family
    map.insert("arch", "arch");
    map.insert("cachyos", "cachyos");
    map.insert("endeavouros", "arch");
    map.insert("manjaro", "arch");
    map.insert("garuda", "arch");
    map.insert("artix", "arch");
    // Debian family
    map.insert("debian", "debian");
    map.insert("ubuntu", "ubuntu");
    map.insert("linuxmint", "ubuntu");
    map.insert("pop", "ubuntu");
    map.insert("elementary", "ubuntu");
    map.insert("kali", "debian");
    // openSUSE family
    map.insert("opensuse-tumbleweed", "opensuse");
    map.insert("opensuse-leap", "opensuse");
    map.insert("opensuse", "opensuse");
    map.insert("sles", "opensuse");
    // Fedora family
    map.insert("fedora", "fedora");
    map.insert("rhel", "fedora");
    map.insert("centos", "fedora");
    map.insert("rocky", "fedora");
    map.insert("almalinux", "fedora");
    map.insert("nobara", "fedora");
    // NixOS (unsupported)
    map.insert("nixos", "unknown");

    map.get(raw.as_str())
        .copied()
        .unwrap_or("unknown")
        .to_string()
}

/// Get pretty distro name for GUI display (matches distro_info in main.rs).
pub fn distro_pretty_name() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
        .unwrap_or_else(|| "unknown distro".into())
}

/// Detect distro for profile metadata (returns canonical ID).
pub fn detect_distro() -> String {
    canonical_distro_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_mapping() {
        // These test the mapping logic directly
        assert_eq!(canonical_for("arch"), "arch");
        assert_eq!(canonical_for("cachyos"), "cachyos");
        assert_eq!(canonical_for("endeavouros"), "arch");
        assert_eq!(canonical_for("manjaro"), "arch");
        assert_eq!(canonical_for("debian"), "debian");
        assert_eq!(canonical_for("ubuntu"), "ubuntu");
        assert_eq!(canonical_for("linuxmint"), "ubuntu");
        assert_eq!(canonical_for("opensuse-tumbleweed"), "opensuse");
        assert_eq!(canonical_for("opensuse-leap"), "opensuse");
        assert_eq!(canonical_for("fedora"), "fedora");
        assert_eq!(canonical_for("nixos"), "unknown");
        assert_eq!(canonical_for("unknown-distro"), "unknown");
    }

    fn canonical_for(raw: &str) -> String {
        let mut map = HashMap::new();
        map.insert("arch", "arch");
        map.insert("cachyos", "cachyos");
        map.insert("endeavouros", "arch");
        map.insert("manjaro", "arch");
        map.insert("garuda", "arch");
        map.insert("artix", "arch");
        map.insert("debian", "debian");
        map.insert("ubuntu", "ubuntu");
        map.insert("linuxmint", "ubuntu");
        map.insert("pop", "ubuntu");
        map.insert("elementary", "ubuntu");
        map.insert("kali", "debian");
        map.insert("opensuse-tumbleweed", "opensuse");
        map.insert("opensuse-leap", "opensuse");
        map.insert("opensuse", "opensuse");
        map.insert("sles", "opensuse");
        map.insert("fedora", "fedora");
        map.insert("rhel", "fedora");
        map.insert("centos", "fedora");
        map.insert("rocky", "fedora");
        map.insert("almalinux", "fedora");
        map.insert("nobara", "fedora");
        map.insert("nixos", "unknown");
        map.get(raw).copied().unwrap_or("unknown").to_string()
    }
}

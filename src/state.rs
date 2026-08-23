//! Applied-state persistence.
//!
//! Records which modules were applied (and when) in
//! `$XDG_STATE_HOME/bazzitify/applied.toml` so the GUI can show
//! "applied" status across launches.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedEntry {
    pub applied_at: String, // RFC 3339
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppliedState {
    #[serde(default)]
    pub modules: BTreeMap<String, AppliedEntry>,
}

impl AppliedState {
    pub fn mark_applied(&mut self, module: &str, now_rfc3339: &str) {
        self.modules.insert(
            module.to_string(),
            AppliedEntry {
                applied_at: now_rfc3339.to_string(),
            },
        );
    }

    pub fn unmark(&mut self, module: &str) {
        self.modules.remove(module);
    }

    pub fn is_applied(&self, module: &str) -> bool {
        self.modules.contains_key(module)
    }
}

pub fn state_path() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".local/state"));
    base.join("bazzitify").join("applied.toml")
}

pub fn load(path: &Path) -> AppliedState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, state: &AppliedState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml = toml::to_string_pretty(state).map_err(std::io::Error::other)?;
    std::fs::write(path, toml)
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_query() {
        let mut s = AppliedState::default();
        assert!(!s.is_applied("sysctl"));
        s.mark_applied("sysctl", &now_rfc3339());
        assert!(s.is_applied("sysctl"));
        s.unmark("sysctl");
        assert!(!s.is_applied("sysctl"));
    }

    #[test]
    fn roundtrip_through_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("applied.toml");
        let mut s = AppliedState::default();
        s.mark_applied("codecs", "2026-08-23T12:00:00Z");
        s.mark_applied("zram", "2026-08-23T13:00:00Z");
        save(&p, &s).unwrap();
        let loaded = load(&p);
        assert_eq!(loaded, s);
        assert!(loaded.is_applied("codecs"));
        assert!(!loaded.is_applied("flatpak"));
    }

    #[test]
    fn load_missing_or_corrupt_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("nope.toml")), AppliedState::default());
        let bad = dir.path().join("bad.toml");
        std::fs::write(&bad, "not [ valid toml {{{").unwrap();
        assert_eq!(load(&bad), AppliedState::default());
    }
}

//! Profile serialization and validation for bazzitify module sets.

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml::{Value, from_str, to_string_pretty};

/// Current profile format version. Increment when making breaking changes.
const PROFILE_VERSION: u32 = 1;

/// Errors that can occur during profile operations.
#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("missing required field: {field}")]
    MissingField { field: String },
    #[error("unsupported profile version: {version} (current: {current})", current = PROFILE_VERSION)]
    UnsupportedVersion { version: u32 },
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("profile not found: {name}")]
    NotFound { name: String },
}

/// A portable module profile containing a selected set of modules and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Profile format version.
    pub version: u32,
    /// Distribution name/identifier (informational, not used for validation).
    pub distro: String,
    /// ISO 8601 date when the profile was exported.
    pub date: String,
    /// List of module names in this profile.
    pub modules: Vec<String>,
}

impl Profile {
    /// Serialize the profile to a TOML string.
    pub fn to_toml(&self) -> Result<String, ProfileError> {
        #[derive(serde::Serialize)]
        struct TomlProfile<'a> {
            version: u32,
            distro: &'a str,
            date: &'a str,
            modules: &'a [String],
        }
        let tp = TomlProfile {
            version: self.version,
            distro: &self.distro,
            date: &self.date,
            modules: &self.modules,
        };
        Ok(to_string_pretty(&tp)?)
    }

    /// Deserialize a profile from a TOML string.
    pub fn from_toml(toml: &str) -> Result<Self, ProfileError> {
        let value: Value = from_str(toml)?;

        let version = value
            .get("version")
            .and_then(|v| v.as_integer())
            .ok_or_else(|| ProfileError::MissingField {
                field: "version".into(),
            })? as u32;

        if version != PROFILE_VERSION {
            return Err(ProfileError::UnsupportedVersion { version });
        }

        let distro = value
            .get("distro")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProfileError::MissingField {
                field: "distro".into(),
            })?
            .to_string();

        let date = value
            .get("date")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProfileError::MissingField {
                field: "date".into(),
            })?
            .to_string();

        let modules = value
            .get("modules")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ProfileError::MissingField {
                field: "modules".into(),
            })?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        Ok(Self {
            version,
            distro,
            date,
            modules,
        })
    }

    /// Validate that all modules in this profile exist locally.
    /// Returns warnings for missing modules (non-fatal).
    pub fn validate_modules(&self, modules_dir: &Path) -> Result<Vec<String>, ProfileError> {
        let mut warnings = Vec::new();
        for module_name in &self.modules {
            let module_path = modules_dir.join(format!("{module_name}.sh"));
            if !module_path.exists() {
                warnings.push(format!(
                    "module '{}' not found locally — will be skipped on import",
                    module_name
                ));
            }
        }
        Ok(warnings)
    }

    /// Export the profile to the config directory with the given name.
    /// Returns the path to the created file.
    pub fn export(&self, config_dir: &Path, name: &str) -> Result<PathBuf, ProfileError> {
        fs::create_dir_all(config_dir)?;
        let file_path = config_dir.join(format!("{name}.toml"));
        let toml = self.to_toml()?;
        fs::write(&file_path, toml)?;
        Ok(file_path)
    }

    /// Import a profile from a file, validating against local modules.
    /// Returns the profile and any warnings about missing modules.
    pub fn import(
        import_file: &Path,
        modules_dir: &Path,
    ) -> Result<(Self, Vec<String>), ProfileError> {
        let content = fs::read_to_string(import_file)?;
        let profile = Self::from_toml(&content)?;
        let warnings = profile.validate_modules(modules_dir)?;
        Ok((profile, warnings))
    }

    /// List available profile names in the config directory (without .toml extension).
    pub fn list_profiles(config_dir: &Path) -> Result<Vec<String>, ProfileError> {
        if !config_dir.exists() {
            return Ok(Vec::new());
        }
        let mut profiles = Vec::new();
        for entry in fs::read_dir(config_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    profiles.push(stem.to_string());
                }
            }
        }
        profiles.sort();
        Ok(profiles)
    }

    /// Get the default config directory following XDG Base Directory spec.
    pub fn default_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("bazzitify")
            .join("profiles")
    }
}

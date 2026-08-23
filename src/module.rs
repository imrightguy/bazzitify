//! Module model: parse bash module files and discover them in a directory.

use std::fmt;
use std::io;
use std::path::Path;

/// A single tweakable unit backed by a bash script with `module_apply` / `module_undo`.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub description: Option<String>,
    pub long_description: Vec<String>,
    pub has_apply: bool,
    pub has_undo: bool,
}

impl Module {
    /// Parse a module from its name and raw bash source.
    pub fn parse(name: &str, source: &str) -> Result<Self, ParseError> {
        if name.is_empty() {
            return Err(ParseError("empty module name".into()));
        }
        let description = source
            .lines()
            .find_map(|l| l.trim().strip_prefix("# desc:"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        // Long description: consecutive "# long:" lines (rendered as a detail block in the GUI)
        let mut long_description = Vec::new();
        for line in source.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("# long:") {
                let rest = rest.trim();
                if !rest.is_empty() {
                    long_description.push(rest.to_string());
                }
            }
        }
        let has_apply = contains_fn(source, "module_apply");
        let has_undo = contains_fn(source, "module_undo");
        Ok(Self {
            name: name.into(),
            description,
            long_description,
            has_apply,
            has_undo,
        })
    }

    /// Discover all `*.sh` modules in a directory, sorted by name.
    pub fn discover(dir: &Path) -> io::Result<Vec<Module>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sh") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let source = std::fs::read_to_string(&path)?;
                out.push(
                    Module::parse(&stem, &source)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

fn contains_fn(source: &str, fname: &str) -> bool {
    source.lines().any(|l| {
        let t = l.trim();
        t.starts_with(fname)
            && (t.contains(&format!("{fname}()")) || t.contains(&format!("{fname} ()")))
    })
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "module parse error: {}", self.0)
    }
}
impl std::error::Error for ParseError {}

/// Whether a module has been applied on this system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    Applied,
    Available,
}

impl fmt::Display for ModuleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleStatus::Applied => write!(f, "applied"),
            ModuleStatus::Available => write!(f, "available"),
        }
    }
}

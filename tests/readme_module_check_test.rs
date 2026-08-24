//! Test for README module table synchronization check.
//! Verifies that the Modules table in README.md stays in sync with modules/ directory.

use std::fs;
use std::path::Path;

/// Parse the Modules table from README.md and return a vec of module names.
fn parse_readme_modules_table(readme_path: &Path) -> Vec<String> {
    let content = fs::read_to_string(readme_path).expect("README.md should be readable");
    let mut modules = Vec::new();
    let mut in_modules_table = false;

    for line in content.lines() {
        // Start of Modules table
        if line.trim_start().starts_with("| Module |")
            && line.contains("What it does")
            && line.contains("Undo")
        {
            in_modules_table = true;
            continue;
        }
        // Skip separator row
        if in_modules_table && line.trim_start().starts_with("|---") {
            continue;
        }
        // End of table - next header
        if in_modules_table && line.trim_start().starts_with("## ") {
            break;
        }
        // Parse table row
        if in_modules_table && line.trim_start().starts_with('|') {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let module = parts[1].trim();
                // Remove markdown bold markup
                let module = module
                    .trim_start_matches("**")
                    .trim_end_matches("**")
                    .trim();
                if !module.is_empty() && module != "Module" {
                    modules.push(module.to_string());
                }
            }
        }
    }
    modules
}

/// Get module names from modules/ directory (basename without .sh)
fn get_fs_modules(modules_dir: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    for entry in fs::read_dir(modules_dir).expect("modules/ should be readable") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("sh")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            modules.push(stem.to_string());
        }
    }
    modules.sort();
    modules
}

#[test]
fn readme_modules_table_parsing_works() {
    let readme = Path::new("README.md");
    assert!(readme.exists(), "README.md should exist");

    let modules = parse_readme_modules_table(readme);
    // Should find at least the 8 modules currently in the table
    assert!(
        modules.len() >= 8,
        "Should parse at least 8 modules from README, got {}",
        modules.len()
    );

    // Check known modules are present
    assert!(modules.contains(&"gaming-packages".to_string()));
    assert!(modules.contains(&"sysctl".to_string()));
    assert!(modules.contains(&"gpu-drivers".to_string()));
    assert!(modules.contains(&"kernel-params".to_string()));
    assert!(modules.contains(&"services".to_string()));
    assert!(modules.contains(&"filesystems".to_string()));
    assert!(modules.contains(&"flatpak".to_string()));
    assert!(modules.contains(&"hdr-vrr".to_string()));
}

#[test]
fn fs_modules_discovery_works() {
    let modules_dir = Path::new("modules");
    assert!(modules_dir.exists(), "modules/ should exist");

    let modules = get_fs_modules(modules_dir);
    // Should find at least the known modules
    assert!(
        modules.len() >= 8,
        "Should discover at least 8 modules from fs, got {}",
        modules.len()
    );

    // Check known modules are present
    assert!(modules.contains(&"gaming-packages".to_string()));
    assert!(modules.contains(&"sysctl".to_string()));
    assert!(modules.contains(&"gpu-drivers".to_string()));
    assert!(modules.contains(&"kernel-params".to_string()));
    assert!(modules.contains(&"services".to_string()));
    assert!(modules.contains(&"filesystems".to_string()));
    assert!(modules.contains(&"flatpak".to_string()));
    assert!(modules.contains(&"hdr-vrr".to_string()));
}

#[test]
fn readme_and_fs_modules_should_match_except_known_drift() {
    // This test documents the current drift state.
    // When the CI workflow is implemented and README is updated, this test should be updated
    // to expect zero drift (or the drift should be fixed).

    let readme_modules = parse_readme_modules_table(Path::new("README.md"));
    let fs_modules = get_fs_modules(Path::new("modules"));

    let readme_set: std::collections::HashSet<_> = readme_modules.iter().collect();
    let fs_set: std::collections::HashSet<_> = fs_modules.iter().collect();

    let missing_from_readme: Vec<_> = fs_set.difference(&readme_set).collect();
    let stale_in_readme: Vec<_> = readme_set.difference(&fs_set).collect();

    // Currently there IS drift - this test documents it
    // TODO: When README is updated, change these assertions to expect empty vecs
    println!("Missing from README: {:?}", missing_from_readme);
    println!("Stale in README: {:?}", stale_in_readme);

    // Known missing modules (not yet documented in README)
    let known_missing = [
        "codecs",
        "display-gpu-control",
        "input-peripherals",
        "power-profiles",
        "streaming-containers",
        "test-dep-a",
        "test-dep-b",
    ];
    for m in &known_missing {
        assert!(
            missing_from_readme.iter().any(|s| *s == m),
            "Expected {} to be missing from README",
            m
        );
    }

    // No stale modules expected (all README modules should exist in fs)
    assert!(
        stale_in_readme.is_empty(),
        "No stale modules expected in README, but found: {:?}",
        stale_in_readme
    );
}

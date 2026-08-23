//! CLI for profile export/import/list operations.

use bazzitify::profile::{Profile, ProfileError};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bazzitify-profile <export|import|list> [args...]");
        process::exit(1);
    }

    let command = &args[1];
    let modules_dir = default_modules_dir();
    let config_dir = Profile::default_config_dir();

    let result = match command.as_str() {
        "export" => {
            if args.len() < 3 {
                eprintln!("Usage: bazzitify-profile export <name>");
                process::exit(1);
            }
            let name = &args[2];
            do_export(&config_dir, &modules_dir, name)
        }
        "import" => {
            if args.len() < 3 {
                eprintln!("Usage: bazzitify-profile import <file>");
                process::exit(1);
            }
            let file = PathBuf::from(&args[2]);
            do_import(&config_dir, &modules_dir, &file)
        }
        "list" => do_list(&config_dir),
        _ => {
            eprintln!("Unknown command: {}. Use export, import, or list.", command);
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn default_modules_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("../modules"))
        .unwrap_or_else(|| PathBuf::from("modules"))
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("modules"))
}

fn do_export(config_dir: &Path, modules_dir: &Path, name: &str) -> Result<(), ProfileError> {
    // Discover available modules
    let modules = bazzitify::module::Module::discover(modules_dir)?;
    let selected_modules: Vec<String> = modules.iter().map(|m| m.name.clone()).collect();

    // For now, export all discovered modules. In the future, this could read from GUI state.
    let profile = Profile {
        version: 1,
        distro: detect_distro(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        modules: selected_modules,
    };

    let file_path = profile.export(config_dir, name)?;
    println!("Exported profile to {}", file_path.display());
    Ok(())
}

fn do_import(config_dir: &Path, modules_dir: &Path, file: &Path) -> Result<(), ProfileError> {
    let (profile, warnings) = Profile::import(file, modules_dir)?;

    for warning in &warnings {
        eprintln!("Warning: {}", warning);
    }

    println!("Imported profile: {} modules", profile.modules.len());
    for module in &profile.modules {
        println!("  - {}", module);
    }

    // Save to config dir with a name based on the file stem
    let name = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    profile.export(config_dir, name)?;
    println!("Saved as profile '{}'", name);

    Ok(())
}

fn do_list(config_dir: &Path) -> Result<(), ProfileError> {
    let profiles = Profile::list_profiles(config_dir)?;
    if profiles.is_empty() {
        println!("No profiles found in {}", config_dir.display());
    } else {
        println!("Available profiles in {}:", config_dir.display());
        for profile in profiles {
            println!("  - {}", profile);
        }
    }
    Ok(())
}

fn detect_distro() -> String {
    // Try to read /etc/os-release
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                return line.trim_start_matches("ID=").trim_matches('"').to_string();
            }
        }
    }
    "unknown".to_string()
}

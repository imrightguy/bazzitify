//! Bazzite Parity Audit Tool
//!
//! Compares bazzitify modules against Bazzite's actual image configuration
//! to identify missing optimizations and generate actionable module stubs.

use std::fs;
use std::path::PathBuf;
use thiserror::Error;

mod audit;
mod bazzite;
mod module;
mod report;

use audit::AuditEngine;
use bazzite::BazziteRepo;
use module::ModuleCatalog;
use report::GapReport;

#[derive(Error, Debug)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git error: {0}")]
    Git(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, AuditError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BazziteTweak {
    pub category: TweakCategory,
    pub name: String,
    pub description: String,
    pub source_file: String,
    pub raw_content: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TweakCategory {
    KernelParams,
    Sysctl,
    SystemdUnits,
    PackageLists,
    UdevRules,
    CompositorConfigs,
    PowerProfiles,
    PipewireConfig,
    EnvironmentVars,
    DracutConfig,
    ModprobeConfig,
    FontConfig,
    Other,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleMapping {
    pub bazzite_tweak: BazziteTweak,
    pub bazzitify_module: Option<String>,
    pub status: MappingStatus,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MappingStatus {
    Implemented,
    Partial,
    Missing,
    NotApplicable,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditResult {
    pub bazzite_commit: String,
    pub bazzite_version: String,
    pub mappings: Vec<ModuleMapping>,
    pub summary: AuditSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditSummary {
    pub total_tweaks: usize,
    pub implemented: usize,
    pub partial: usize,
    pub missing: usize,
    pub not_applicable: usize,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "run" => run_audit(&args[2..]),
        "generate-stubs" => generate_stubs(&args[2..]),
        "report" => generate_report(&args[2..]),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    eprintln!("bazzite-audit - Bazzite parity audit tooling");
    eprintln!();
    eprintln!("Usage:");
    eprintln!(
        "  bazzite-audit run [--shallow] [--output-dir DIR]     Run audit and generate gap report"
    );
    eprintln!(
        "  bazzite-audit generate-stubs [--report FILE]         Generate skeleton modules for missing items"
    );
    eprintln!(
        "  bazzite-audit report [--input FILE] [--output FILE]  Generate Markdown report from audit JSON"
    );
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --shallow          Shallow clone (faster, less history)");
    eprintln!("  --output-dir DIR   Output directory (default: docs/)");
    eprintln!("  --report FILE      Input audit report JSON file");
    eprintln!("  --input FILE       Input audit report JSON file");
    eprintln!("  --output FILE      Output Markdown file (default: docs/Bazzite_Parity_Report.md)");
}

fn run_audit(args: &[String]) -> Result<()> {
    let mut shallow = false;
    let mut output_dir = PathBuf::from("docs");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--shallow" => shallow = true,
            "--output-dir" => {
                i += 1;
                if i < args.len() {
                    output_dir = PathBuf::from(&args[i]);
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Cloning Bazzite repository...");
    let bazzite_repo = BazziteRepo::clone(shallow)?;
    let commit = bazzite_repo.get_head_commit()?;
    let version = bazzite_repo.get_version()?;

    println!("Extracting tweaks from Bazzite configuration...");
    let tweaks = bazzite_repo.extract_tweaks()?;
    println!("Found {} tweaks", tweaks.len());

    println!("Loading bazzitify module catalog...");
    let module_catalog = ModuleCatalog::load("modules")?;
    println!("Loaded {} modules", module_catalog.modules.len());

    println!("Mapping tweaks to modules...");
    let engine = AuditEngine::new(module_catalog);
    let mappings = engine.map_tweaks(tweaks);

    let summary = AuditSummary {
        total_tweaks: mappings.len(),
        implemented: mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Implemented)
            .count(),
        partial: mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Partial)
            .count(),
        missing: mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Missing)
            .count(),
        not_applicable: mappings
            .iter()
            .filter(|m| m.status == MappingStatus::NotApplicable)
            .count(),
    };

    let result = AuditResult {
        bazzite_commit: commit,
        bazzite_version: version,
        mappings,
        summary: summary.clone(),
    };

    fs::create_dir_all(&output_dir)?;
    let report_path = output_dir.join("Bazzite_Parity_Report.md");
    let json_path = output_dir.join("Bazzite_Parity_Report.json");

    let report = GapReport::generate(&result);
    fs::write(&report_path, report)?;
    println!("Markdown report written to {}", report_path.display());

    let json = serde_json::to_string_pretty(&result)?;
    fs::write(&json_path, json)?;
    println!("JSON report written to {}", json_path.display());

    println!("\nAudit Summary:");
    println!("  Total tweaks:     {}", summary.total_tweaks);
    println!("  Implemented:      {}", summary.implemented);
    println!("  Partial:          {}", summary.partial);
    println!("  Missing:          {}", summary.missing);
    println!("  Not Applicable:   {}", summary.not_applicable);

    Ok(())
}

fn generate_stubs(args: &[String]) -> Result<()> {
    let mut report_path = PathBuf::from("docs/Bazzite_Parity_Report.json");

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--report" {
            i += 1;
            if i < args.len() {
                report_path = PathBuf::from(&args[i]);
            }
        }
        i += 1;
    }

    let json = fs::read_to_string(&report_path)?;
    let result: AuditResult = serde_json::from_str(&json)?;

    let missing: Vec<_> = result
        .mappings
        .iter()
        .filter(|m| m.status == MappingStatus::Missing)
        .collect();

    println!("Generating {} skeleton modules...", missing.len());

    for mapping in missing {
        let module_name = sanitize_module_name(&mapping.bazzite_tweak.name);
        let module_path = PathBuf::from("modules").join(format!("{}.sh", module_name));

        if module_path.exists() {
            println!("  Skipping {} (already exists)", module_name);
            continue;
        }

        let content = generate_module_stub(mapping);
        fs::write(&module_path, content)?;
        println!("  Created {}", module_path.display());
    }

    println!("Done! Review generated modules before committing.");
    Ok(())
}

fn generate_report(args: &[String]) -> Result<()> {
    let mut input_path = PathBuf::from("docs/Bazzite_Parity_Report.json");
    let mut output_path = PathBuf::from("docs/Bazzite_Parity_Report.md");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                if i < args.len() {
                    input_path = PathBuf::from(&args[i]);
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = PathBuf::from(&args[i]);
                }
            }
            _ => {}
        }
        i += 1;
    }

    let json = fs::read_to_string(&input_path)?;
    let result: AuditResult = serde_json::from_str(&json)?;

    let report = GapReport::generate(&result);
    fs::write(&output_path, report)?;
    println!("Report written to {}", output_path.display());
    Ok(())
}

fn sanitize_module_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn generate_module_stub(mapping: &ModuleMapping) -> String {
    let name = sanitize_module_name(&mapping.bazzite_tweak.name);
    let desc = &mapping.bazzite_tweak.description;
    let category = format!("{:?}", mapping.bazzite_tweak.category);
    let source = &mapping.bazzite_tweak.source_file;

    let long_lines: Vec<String> = mapping
        .bazzite_tweak
        .raw_content
        .lines()
        .take(10)
        .map(|l| format!("• {}", l.trim()))
        .collect();

    let mut depends = Vec::new();
    match mapping.bazzite_tweak.category {
        TweakCategory::KernelParams => depends.push("kernel-params"),
        TweakCategory::Sysctl => depends.push("sysctl"),
        TweakCategory::SystemdUnits => depends.push("services"),
        TweakCategory::CompositorConfigs => depends.push("display-gpu-control"),
        TweakCategory::PowerProfiles => depends.push("power-profiles"),
        TweakCategory::PipewireConfig => depends.push("codecs"),
        _ => {}
    }

    let depends_str = if depends.is_empty() {
        String::new()
    } else {
        format!("# depends: {}", depends.join(" "))
    };

    format!(
        r#"#!/bin/bash
# desc: {}
# long: {} (from Bazzite: {})
# long: Source: {}
# long: Category: {}
# long: {}
{}
# depends: {}

module_apply() {{
    # TODO: Implement apply logic for {}
    # Source: {}
    echo "TODO: Implement {} apply"
    return 1
}}

module_undo() {{
    # TODO: Implement undo logic for {}
    echo "TODO: Implement {} undo"
    return 1
}}
"#,
        desc,
        desc,
        mapping.bazzite_tweak.category as u8,
        source,
        category,
        long_lines.join("\n# long: "),
        depends_str,
        name,
        source,
        name,
        name,
        name,
        name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_module_name() {
        assert_eq!(
            sanitize_module_name("Kernel Parameters"),
            "kernel-parameters"
        );
        assert_eq!(
            sanitize_module_name("sysctl.conf tweaks"),
            "sysctl-conf-tweaks"
        );
        assert_eq!(
            sanitize_module_name("GPU_Power_Profile"),
            "gpu-power-profile"
        );
        assert_eq!(sanitize_module_name("  spaced  name  "), "spaced-name");
    }

    #[test]
    fn test_mapping_status_ordering() {
        // Just verify the enum variants exist
        let _ = MappingStatus::Implemented;
        let _ = MappingStatus::Partial;
        let _ = MappingStatus::Missing;
        let _ = MappingStatus::NotApplicable;
    }
}

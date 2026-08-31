//! Report generation

use crate::{AuditResult, AuditSummary, MappingStatus, ModuleMapping, TweakCategory};
use std::collections::HashMap;

pub struct GapReport;

impl GapReport {
    pub fn generate(result: &AuditResult) -> String {
        let mut report = String::new();

        // Header
        report.push_str(&format!(
            "# Bazzite Parity Audit Report\n\n\
            **Bazzite Version:** {}  \n\
            **Bazzite Commit:** {}  \n\
            **Generated:** {}  \n\n",
            result.bazzite_version,
            &result.bazzite_commit[..12.min(result.bazzite_commit.len())],
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Summary
        report.push_str(&Self::generate_summary(&result.summary));

        // Missing tweaks (highest priority)
        report.push_str(&Self::generate_missing_section(result));

        // Partial matches
        report.push_str(&Self::generate_partial_section(result));

        // Implemented tweaks
        report.push_str(&Self::generate_implemented_section(result));

        // Not applicable
        report.push_str(&Self::generate_not_applicable_section(result));

        // Category breakdown
        report.push_str(&Self::generate_category_breakdown(result));

        // Recommendations
        report.push_str(&Self::generate_recommendations(result));

        report
    }

    fn generate_summary(summary: &AuditSummary) -> String {
        let mut s = String::new();
        s.push_str("## Summary\n\n");
        s.push_str("| Status | Count | Percentage |\n");
        s.push_str("|--------|-------|------------|\n");
        let total = summary.total_tweaks as f64;
        if total > 0.0 {
            s.push_str(&format!(
                "| ✅ Implemented | {} | {:.1}% |\n",
                summary.implemented,
                summary.implemented as f64 / total * 100.0
            ));
            s.push_str(&format!(
                "| ⚠️ Partial | {} | {:.1}% |\n",
                summary.partial,
                summary.partial as f64 / total * 100.0
            ));
            s.push_str(&format!(
                "| ❌ Missing | {} | {:.1}% |\n",
                summary.missing,
                summary.missing as f64 / total * 100.0
            ));
            s.push_str(&format!(
                "| ➖ Not Applicable | {} | {:.1}% |\n",
                summary.not_applicable,
                summary.not_applicable as f64 / total * 100.0
            ));
            s.push_str(&format!(
                "| **Total** | **{}** | **100%** |\n",
                summary.total_tweaks
            ));
        }
        s.push('\n');
        s
    }

    fn generate_missing_section(result: &AuditResult) -> String {
        let missing: Vec<_> = result
            .mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Missing)
            .collect();

        if missing.is_empty() {
            return String::new();
        }

        let mut s = String::new();
        s.push_str("## ❌ Missing Tweaks (Not Implemented)\n\n");
        s.push_str("These Bazzite tweaks have no corresponding bazzitify module. ");
        s.push_str("Consider creating modules for these high-priority items.\n\n");

        // Group by category
        let mut by_category: HashMap<TweakCategory, Vec<&ModuleMapping>> = HashMap::new();
        for m in missing {
            by_category
                .entry(m.bazzite_tweak.category)
                .or_default()
                .push(m);
        }

        for (category, mappings) in by_category {
            s.push_str(&format!("### {:?}\n\n", category));
            s.push_str("| Bazzite Tweak | Source | Description |\n");
            s.push_str("|---------------|--------|-------------|\n");
            for m in mappings {
                let desc = m
                    .bazzite_tweak
                    .description
                    .chars()
                    .take(80)
                    .collect::<String>();
                let source = &m.bazzite_tweak.source_file;
                s.push_str(&format!(
                    "| {} | {} | {} |\n",
                    m.bazzite_tweak.name, source, desc
                ));
            }
            s.push('\n');
        }

        s
    }

    fn generate_partial_section(result: &AuditResult) -> String {
        let partial: Vec<_> = result
            .mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Partial)
            .collect();

        if partial.is_empty() {
            return String::new();
        }

        let mut s = String::new();
        s.push_str("## ⚠️ Partial Matches (Needs Review)\n\n");
        s.push_str("These tweaks have a related module but may not fully implement the Bazzite configuration.\n\n");

        s.push_str("| Bazzite Tweak | bazzitify Module | Confidence | Source |\n");
        s.push_str("|---------------|------------------|------------|--------|\n");
        for m in partial {
            let module = m.bazzitify_module.as_deref().unwrap_or("N/A");
            let source = &m.bazzite_tweak.source_file;
            s.push_str(&format!(
                "| {} | {} | {:.0}% | {} |\n",
                m.bazzite_tweak.name,
                module,
                m.confidence * 100.0,
                source
            ));
        }
        s.push('\n');

        s
    }

    fn generate_implemented_section(result: &AuditResult) -> String {
        let implemented: Vec<_> = result
            .mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Implemented)
            .collect();

        if implemented.is_empty() {
            return String::new();
        }

        let mut s = String::new();
        s.push_str("## ✅ Implemented Tweaks\n\n");
        s.push_str("These Bazzite tweaks are fully covered by bazzitify modules.\n\n");

        s.push_str("| Bazzite Tweak | bazzitify Module | Confidence | Source |\n");
        s.push_str("|---------------|------------------|------------|--------|\n");
        for m in implemented {
            let module = m.bazzitify_module.as_deref().unwrap_or("N/A");
            let source = &m.bazzite_tweak.source_file;
            s.push_str(&format!(
                "| {} | {} | {:.0}% | {} |\n",
                m.bazzite_tweak.name,
                module,
                m.confidence * 100.0,
                source
            ));
        }
        s.push('\n');

        s
    }

    fn generate_not_applicable_section(result: &AuditResult) -> String {
        let na: Vec<_> = result
            .mappings
            .iter()
            .filter(|m| m.status == MappingStatus::NotApplicable)
            .collect();

        if na.is_empty() {
            return String::new();
        }

        let mut s = String::new();
        s.push_str("## ➖ Not Applicable\n\n");
        s.push_str("These items are not applicable to bazzitify (e.g., RPM packages, build-time only configs).\n\n");

        s.push_str("| Bazzite Tweak | Category | Reason |\n");
        s.push_str("|---------------|----------|--------|\n");
        for m in na {
            let reason = match m.bazzite_tweak.category {
                TweakCategory::PackageLists => "RPM package (build-time only)",
                TweakCategory::Other => "Containerfile/build step (not runtime config)",
                _ => "Not applicable to mutable distro",
            };
            s.push_str(&format!(
                "| {} | {:?} | {} |\n",
                m.bazzite_tweak.name, m.bazzite_tweak.category, reason
            ));
        }
        s.push('\n');

        s
    }

    fn generate_category_breakdown(result: &AuditResult) -> String {
        let mut by_category: HashMap<TweakCategory, Vec<&ModuleMapping>> = HashMap::new();
        for m in &result.mappings {
            by_category
                .entry(m.bazzite_tweak.category)
                .or_default()
                .push(m);
        }

        let mut s = String::new();
        s.push_str("## Category Breakdown\n\n");
        s.push_str("| Category | Total | Implemented | Partial | Missing | N/A |\n");
        s.push_str("|----------|-------|-------------|---------|---------|-----|\n");

        let mut categories: Vec<_> = by_category.keys().collect();
        categories.sort();

        for cat in categories {
            let mappings = &by_category[cat];
            let impl_count = mappings
                .iter()
                .filter(|m| m.status == MappingStatus::Implemented)
                .count();
            let part_count = mappings
                .iter()
                .filter(|m| m.status == MappingStatus::Partial)
                .count();
            let miss_count = mappings
                .iter()
                .filter(|m| m.status == MappingStatus::Missing)
                .count();
            let na_count = mappings
                .iter()
                .filter(|m| m.status == MappingStatus::NotApplicable)
                .count();

            s.push_str(&format!(
                "| {:?} | {} | {} | {} | {} | {} |\n",
                cat,
                mappings.len(),
                impl_count,
                part_count,
                miss_count,
                na_count
            ));
        }
        s.push('\n');

        s
    }

    fn generate_recommendations(result: &AuditResult) -> String {
        let missing: Vec<_> = result
            .mappings
            .iter()
            .filter(|m| m.status == MappingStatus::Missing)
            .collect();

        if missing.is_empty() {
            return String::new();
        }

        let mut s = String::new();
        s.push_str("## Recommendations\n\n");

        // Top 10 missing by category priority
        let priority_categories = [
            TweakCategory::KernelParams,
            TweakCategory::Sysctl,
            TweakCategory::SystemdUnits,
            TweakCategory::CompositorConfigs,
            TweakCategory::PowerProfiles,
            TweakCategory::PipewireConfig,
            TweakCategory::UdevRules,
            TweakCategory::EnvironmentVars,
            TweakCategory::DracutConfig,
            TweakCategory::ModprobeConfig,
        ];

        let mut count = 0;
        for cat in priority_categories {
            let cat_missing: Vec<_> = missing
                .iter()
                .filter(|m| m.bazzite_tweak.category == cat)
                .collect();

            if !cat_missing.is_empty() && count < 10 {
                s.push_str(&format!("### {:?} (Top Priority)\n\n", cat));
                for m in cat_missing.iter().take(3) {
                    s.push_str(&format!(
                        "- **{}**: {} (from `{}`)\n",
                        m.bazzite_tweak.name,
                        m.bazzite_tweak.description,
                        m.bazzite_tweak.source_file
                    ));
                }
                s.push('\n');
                count += cat_missing.len().min(3);
            }
        }

        s.push_str(
            "Run `bazzite-audit generate-stubs` to create skeleton modules for missing items.\n\n",
        );

        s
    }
}

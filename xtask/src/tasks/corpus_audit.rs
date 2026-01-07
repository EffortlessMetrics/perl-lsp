//! Corpus audit task implementation
//!
//! This module provides comprehensive corpus coverage analysis including:
//! - Corpus inventory and structure analysis
//! - NodeKind reachability analysis
//! - GA feature-to-fixture alignment
//! - Timeout/hang risk detection
//! - Machine-readable report generation

use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use perl_parser::Parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

mod corpus;
mod ga_alignment;
mod nodekind_analysis;
mod report;
mod timeout_detection;

use corpus::{CorpusFile, CorpusLayer, parse_corpus_files};
use ga_alignment::{GAFeature, GAFeatureCoverage, check_ga_feature_alignment};
use nodekind_analysis::{NodeKindStats, analyze_nodekind_coverage};
use report::{AuditReport, generate_report};
use timeout_detection::{ParseOutcome, TimeoutRisk, detect_timeout_risks, parse_with_timeout};

/// Default timeout for parsing individual files
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum nesting depth to prevent stack overflow
const MAX_NESTING_DEPTH: usize = 100;

/// Maximum regex operations to prevent exponential backtracking
const MAX_REGEX_OPERATIONS: usize = 10_000;

/// Maximum heredoc nesting depth
const MAX_HEREDOC_DEPTH: usize = 10;

/// Maximum heredoc content size (1MB)
const MAX_HEREDOC_SIZE: usize = 1_000_000;

/// Configuration for corpus audit
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Path to corpus directory
    pub corpus_path: PathBuf,
    /// Output path for JSON report
    pub output_path: PathBuf,
    /// Timeout for parsing individual files
    pub timeout: Duration,
    /// Whether to regenerate reports (--fresh flag)
    pub fresh: bool,
    /// Whether to run in check mode for CI (--check flag)
    pub check: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            corpus_path: PathBuf::from("crates/perl-corpus"),
            output_path: PathBuf::from("target/corpus-audit-report.json"),
            timeout: DEFAULT_TIMEOUT,
            fresh: false,
            check: false,
        }
    }
}

/// Run corpus audit with the given configuration
pub fn run(config: AuditConfig) -> Result<()> {
    let start_time = Instant::now();

    println!("🔍 Starting corpus audit...");
    println!("   Corpus path: {}", config.corpus_path.display());
    println!("   Output path: {}", config.output_path.display());
    println!("   Timeout: {:?}", config.timeout);
    println!("   Mode: {}", if config.check { "check (CI)" } else { "full" });

    // Create output directory if needed
    if let Some(parent) = config.output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }

    // Check if report already exists and not in fresh mode
    if !config.fresh && config.output_path.exists() {
        if config.check {
            println!("ℹ️  Using existing report (use --fresh to regenerate)");
            let report_content = fs::read_to_string(&config.output_path)
                .context("Failed to read existing report")?;
            let report: AuditReport = serde_json::from_str(&report_content)
                .context("Failed to parse existing report")?;

            // In check mode, validate the report and exit
            return validate_report_for_ci(&report);
        }
    }

    // Step 1: Parse corpus files with timeout protection
    println!("\n📂 Step 1: Parsing corpus files...");
    let corpus_files = parse_corpus_files(&config.corpus_path)?;
    let parse_results = parse_corpus_with_timeout(&corpus_files, config.timeout)?;

    // Step 2: Analyze NodeKind coverage
    println!("\n🔢 Step 2: Analyzing NodeKind coverage...");
    let nodekind_stats = analyze_nodekind_coverage(&parse_results);

    // Step 3: Check GA feature alignment
    println!("\n🎯 Step 3: Checking GA feature alignment...");
    let ga_coverage = check_ga_feature_alignment(&corpus_files)?;

    // Step 4: Detect timeout/hang risks
    println!("\n⏱️  Step 4: Detecting timeout/hang risks...");
    let timeout_risks = detect_timeout_risks(&corpus_files);

    // Step 5: Generate report
    println!("\n📊 Step 5: Generating report...");
    let report = generate_report(
        corpus_files,
        parse_results,
        nodekind_stats,
        ga_coverage,
        timeout_risks,
        start_time.elapsed(),
    );

    // Write report to file
    let report_json = serde_json::to_string_pretty(&report)
        .context("Failed to serialize report")?;
    fs::write(&config.output_path, report_json)
        .context("Failed to write report file")?;

    println!("\n✅ Corpus audit completed successfully!");
    println!("   Report written to: {}", config.output_path.display());

    // Print summary
    print_audit_summary(&report);

    // In check mode, validate and exit with appropriate code
    if config.check {
        return validate_report_for_ci(&report);
    }

    Ok(())
}

/// Parse all corpus files with timeout protection
fn parse_corpus_with_timeout(
    corpus_files: &[CorpusFile],
    timeout: Duration,
) -> Result<HashMap<PathBuf, ParseOutcome>> {
    let spinner = ProgressBar::new(corpus_files.len() as u64);
    spinner.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut results = HashMap::new();

    for file in corpus_files {
        spinner.set_message(format!("Parsing {}", file.path.display()));

        let outcome = parse_with_timeout(&file.path, &file.content, timeout);

        results.insert(file.path.clone(), outcome);

        spinner.inc(1);
    }

    spinner.finish_with_message("Parsing complete");

    Ok(results)
}

/// Print a summary of the audit results
fn print_audit_summary(report: &AuditReport) {
    println!("\n📋 Audit Summary:");
    println!("   Total files: {}", report.inventory.total_files);
    println!("   Parse results:");
    println!("     - OK: {} ✅", report.parse_outcomes.ok);
    println!("     - Error: {} ❌", report.parse_outcomes.error);
    println!("     - Timeout: {} ⏱️", report.parse_outcomes.timeout);
    println!("     - Panic: {} 💥", report.parse_outcomes.panic);
    println!("   NodeKind coverage: {}/{} ({:.1}%)",
        report.nodekind_coverage.covered_count,
        report.nodekind_coverage.total_count,
        report.nodekind_coverage.coverage_percentage
    );
    println!("   Never-seen NodeKinds: {}", report.nodekind_coverage.never_seen.len());
    println!("   At-risk NodeKinds (<5 occurrences): {}", report.nodekind_coverage.at_risk.len());
    println!("   GA features covered: {}/{} ({:.1}%)",
        report.ga_coverage.covered_count,
        report.ga_coverage.total_count,
        report.ga_coverage.coverage_percentage
    );
    println!("   Timeout/hang risks: {}", report.timeout_risks.len());

    if !report.timeout_risks.is_empty() {
        println!("\n⚠️  Timeout/Hang Risks:");
        for risk in &report.timeout_risks {
            println!("   - P{}: {} ({})",
                risk.priority,
                risk.description,
                risk.file_path.display()
            );
        }
    }
}

/// Validate report for CI gate
///
/// Returns Ok(()) if report passes validation, otherwise returns error
fn validate_report_for_ci(report: &AuditReport) -> Result<()> {
    println!("\n🔬 Validating report for CI gate...");

    let mut failures = Vec::new();

    // Check for parse failures
    if report.parse_outcomes.error > 0 {
        failures.push(format!(
            "Parse errors: {} files failed to parse",
            report.parse_outcomes.error
        ));
    }

    // Check for timeouts
    if report.parse_outcomes.timeout > 0 {
        failures.push(format!(
            "Parse timeouts: {} files timed out",
            report.parse_outcomes.timeout
        ));
    }

    // Check for panics
    if report.parse_outcomes.panic > 0 {
        failures.push(format!(
            "Parse panics: {} files caused panics",
            report.parse_outcomes.panic
        ));
    }

    // Check for critical timeout risks
    let critical_risks: Vec<_> = report.timeout_risks.iter()
        .filter(|r| r.priority == 0)
        .collect();

    if !critical_risks.is_empty() {
        failures.push(format!(
            "Critical timeout risks: {} P0 risks detected",
            critical_risks.len()
        ));
    }

    // Check GA feature coverage
    if report.ga_coverage.coverage_percentage < 80.0 {
        failures.push(format!(
            "Low GA feature coverage: {:.1}% (target: 80%)",
            report.ga_coverage.coverage_percentage
        ));
    }

    if failures.is_empty() {
        println!("✅ CI gate passed!");
        Ok(())
    } else {
        println!("\n❌ CI gate failed:");
        for failure in &failures {
            println!("   - {}", failure);
        }
        Err(color_eyre::eyre::eyre!(
            "CI gate validation failed: {}",
            failures.join("; ")
        ))
    }
}

/// Test function to verify corpus audit functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuditConfig::default();
        assert_eq!(config.corpus_path, PathBuf::from("crates/perl-corpus"));
        assert_eq!(config.timeout, DEFAULT_TIMEOUT);
        assert!(!config.fresh);
        assert!(!config.check);
    }

    #[test]
    fn test_timeout_constants() {
        assert_eq!(DEFAULT_TIMEOUT.as_secs(), 30);
        assert_eq!(MAX_NESTING_DEPTH, 100);
        assert_eq!(MAX_REGEX_OPERATIONS, 10_000);
        assert_eq!(MAX_HEREDOC_DEPTH, 10);
        assert_eq!(MAX_HEREDOC_SIZE, 1_000_000);
    }
}

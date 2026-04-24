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
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod corpus;
mod ga_alignment;
mod nodekind_analysis;
mod report;
mod timeout_detection;

use corpus::{CorpusFile, parse_corpus_files};
use ga_alignment::check_ga_feature_alignment;
use nodekind_analysis::analyze_nodekind_coverage;
use report::generate_report;
use timeout_detection::{ParseOutcome, detect_timeout_risks, parse_with_timeout};

pub use report::{AuditReport, FailingFile, ParseOutcomesSummary};

#[derive(Debug, serde::Deserialize)]
struct ParserScorecardBaseline {
    floor_metrics: BTreeMap<String, Option<f64>>,
    improvement_metrics: BTreeMap<String, Option<f64>>,
}

fn load_parser_metric_floor(metric: &str) -> Option<f64> {
    let baseline_path = Path::new(".ci/metrics/baselines/parser.json");
    let raw = fs::read_to_string(baseline_path).ok()?;
    let baseline: ParserScorecardBaseline = serde_json::from_str(&raw).ok()?;

    baseline
        .floor_metrics
        .get(metric)
        .and_then(|v| *v)
        .or_else(|| baseline.improvement_metrics.get(metric).and_then(|v| *v))
}

/// Default timeout for parsing individual files
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum nesting depth to prevent stack overflow
const MAX_NESTING_DEPTH: usize = 100;

/// Maximum regex operations to prevent exponential backtracking
const MAX_REGEX_OPERATIONS: usize = 10_000;

/// Maximum heredoc nesting depth
const MAX_HEREDOC_DEPTH: usize = 100;

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

/// Lightweight corpus snapshot for status reporting.
#[derive(Debug, Clone)]
pub struct StatusSummary {
    pub total_files: usize,
    pub ok_files: usize,
    pub error_files: usize,
    pub timeout_files: usize,
    pub panic_files: usize,
    pub test_corpus_files: usize,
    pub perl_corpus_files: usize,
    pub nodekind_covered: usize,
    pub nodekind_total: usize,
    pub ga_covered: usize,
    pub ga_total: usize,
}

/// Compute a lightweight repo-corpus summary for `update-status`.
///
/// This reuses the same corpus discovery and parse logic as `parser-audit`, but
/// stops after the metrics needed for `docs/project/status/parser.md`.
pub fn compute_status_summary(corpus_path: &Path, timeout: Duration) -> Result<StatusSummary> {
    let corpus_files = parse_corpus_files(corpus_path)?;
    let inventory = corpus::generate_inventory(&corpus_files);
    let parse_results = parse_corpus_with_timeout(&corpus_files, timeout)?;
    let nodekind_stats = analyze_nodekind_coverage(&parse_results);
    let ga_coverage = check_ga_feature_alignment(&corpus_files)?;

    let mut ok_files = 0usize;
    let mut error_files = 0usize;
    let mut timeout_files = 0usize;
    let mut panic_files = 0usize;

    for outcome in parse_results.values() {
        match outcome {
            ParseOutcome::Ok { .. } => ok_files += 1,
            ParseOutcome::Error { .. } => error_files += 1,
            ParseOutcome::Timeout { .. } => timeout_files += 1,
            ParseOutcome::Panic { .. } => panic_files += 1,
        }
    }

    let mut test_corpus_files = 0usize;
    let mut perl_corpus_files = 0usize;
    for layer_count in inventory.files_by_layer {
        match layer_count.layer {
            corpus::CorpusLayer::TestCorpus => test_corpus_files = layer_count.count,
            corpus::CorpusLayer::PerlCorpus => perl_corpus_files = layer_count.count,
            _ => {}
        }
    }

    Ok(StatusSummary {
        total_files: inventory.total_files,
        ok_files,
        error_files,
        timeout_files,
        panic_files,
        test_corpus_files,
        perl_corpus_files,
        nodekind_covered: nodekind_stats.covered_count,
        nodekind_total: nodekind_stats.total_count,
        ga_covered: ga_coverage.covered_count,
        ga_total: ga_coverage.total_count,
    })
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
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    // Check if report already exists and not in fresh mode
    if !config.fresh && config.output_path.exists() && config.check {
        println!("ℹ️  Using existing report (use --fresh to regenerate)");
        let report_content =
            fs::read_to_string(&config.output_path).context("Failed to read existing report")?;
        let report: AuditReport =
            serde_json::from_str(&report_content).context("Failed to parse existing report")?;

        // In check mode, validate the report and exit
        return validate_report_for_ci(&report);
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
    let report_json =
        serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
    fs::write(&config.output_path, report_json).context("Failed to write report file")?;

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
            .unwrap_or_else(|_| ProgressStyle::default_bar())
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
    println!(
        "   NodeKind coverage: {}/{} ({:.1}%)",
        report.nodekind_coverage.covered_count,
        report.nodekind_coverage.total_count,
        report.nodekind_coverage.coverage_percentage
    );
    println!("   Never-seen NodeKinds: {}", report.nodekind_coverage.never_seen.len());
    println!("   At-risk NodeKinds (<5 occurrences): {}", report.nodekind_coverage.at_risk.len());
    println!(
        "   GA features covered: {}/{} ({:.1}%)",
        report.ga_coverage.covered_count,
        report.ga_coverage.total_count,
        report.ga_coverage.coverage_percentage
    );
    println!("   Timeout/hang risks: {}", report.timeout_risks.len());

    if !report.timeout_risks.is_empty() {
        println!("\n⚠️  Timeout/Hang Risks:");
        for risk in &report.timeout_risks {
            println!(
                "   - {:?}: {} ({})",
                risk.priority,
                risk.description,
                risk.file_path.display()
            );
        }
    }
}

/// Validate report for CI gate with baseline ratcheting (Issue #180)
///
/// Returns Ok(()) if report passes validation, otherwise returns error.
/// Parse errors use baseline ratcheting (can only decrease, never increase).
fn validate_report_for_ci(report: &AuditReport) -> Result<()> {
    println!("\n🔬 Validating report for CI gate...");

    let mut failures = Vec::new();

    // Parse error ratchet: read baseline and compare (Issue #180)
    let baseline_path = std::path::Path::new("ci/parse_errors_baseline.txt");
    let current_errors = report.parse_outcomes.error;

    if baseline_path.exists() {
        let baseline_str =
            fs::read_to_string(baseline_path).context("Failed to read parse errors baseline")?;
        let baseline: usize =
            baseline_str.trim().parse().context("Failed to parse baseline as number")?;

        println!("   Parse errors: {} (baseline: {})", current_errors, baseline);

        if current_errors > baseline {
            failures.push(format!(
                "Parse error regression: {} > {} baseline. Fix parser or update baseline.",
                current_errors, baseline
            ));
        } else if current_errors < baseline {
            println!(
                "   📉 IMPROVEMENT: {} fewer errors! Update baseline: echo {} > ci/parse_errors_baseline.txt",
                baseline - current_errors,
                current_errors
            );
        }
    } else {
        // No baseline file - just report the count
        println!("   Parse errors: {} (no baseline file)", current_errors);
    }

    // Timeouts should always be zero
    if report.parse_outcomes.timeout > 0 {
        failures.push(format!("Parse timeouts: {} files timed out", report.parse_outcomes.timeout));
    }

    // Panics should always be zero
    if report.parse_outcomes.panic > 0 {
        failures.push(format!("Parse panics: {} files caused panics", report.parse_outcomes.panic));
    }

    // Check for critical timeout risks
    let critical_risks: Vec<_> = report
        .timeout_risks
        .iter()
        .filter(|r| r.priority == timeout_detection::RiskPriority::P0)
        .collect();

    if !critical_risks.is_empty() {
        failures
            .push(format!("Critical timeout risks: {} P0 risks detected", critical_risks.len()));
    }

    // Check GA feature coverage
    if report.ga_coverage.coverage_percentage < 80.0 {
        failures.push(format!(
            "Low GA feature coverage: {:.1}% (target: 80%)",
            report.ga_coverage.coverage_percentage
        ));
    }

    // Parser scorecard ratchets.
    let current_nodekind_coverage = if report.nodekind_coverage.total_count == 0 {
        0.0
    } else {
        report.nodekind_coverage.covered_count as f64 / report.nodekind_coverage.total_count as f64
    };
    if let Some(nodekind_floor) = load_parser_metric_floor("node_kind_coverage")
        && current_nodekind_coverage + f64::EPSILON < nodekind_floor
    {
        failures.push(format!(
            "NodeKind coverage regression: {:.4} < {:.4} floor",
            current_nodekind_coverage, nodekind_floor
        ));
    }

    let current_valid_parser_gap_count = report.parse_outcomes.error as f64;
    if let Some(valid_gap_ceiling) = load_parser_metric_floor("valid_parser_gap_count")
        && current_valid_parser_gap_count > valid_gap_ceiling
    {
        failures.push(format!(
            "Valid parser gap regression: {:.0} > {:.0} floor ceiling",
            current_valid_parser_gap_count, valid_gap_ceiling
        ));
    }

    let current_recovery_salvage_rate = if report.parse_outcomes.total == 0 {
        1.0
    } else {
        report.parse_outcomes.ok as f64 / report.parse_outcomes.total as f64
    };
    if let Some(salvage_floor) = load_parser_metric_floor("recovery_salvage_rate")
        && current_recovery_salvage_rate + f64::EPSILON < salvage_floor
    {
        failures.push(format!(
            "Recovery salvage regression: {:.4} < {:.4} floor",
            current_recovery_salvage_rate, salvage_floor
        ));
    }

    // Print error category breakdown if there are errors (Issue #180)
    if current_errors > 0 && !report.parse_outcomes.error_by_category.is_empty() {
        println!("\n   Error breakdown by category:");
        let mut categories: Vec<_> = report.parse_outcomes.error_by_category.iter().collect();
        categories.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending
        for (category, count) in categories {
            println!("     - {}: {}", category, count);
        }
    }

    if failures.is_empty() {
        println!("\n✅ CI gate passed!");
        Ok(())
    } else {
        println!("\n❌ CI gate failed:");
        for failure in &failures {
            println!("   - {}", failure);
        }
        Err(color_eyre::eyre::eyre!("CI gate validation failed: {}", failures.join("; ")))
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
        assert_eq!(MAX_HEREDOC_DEPTH, 100);
        assert_eq!(MAX_HEREDOC_SIZE, 1_000_000);
    }
}

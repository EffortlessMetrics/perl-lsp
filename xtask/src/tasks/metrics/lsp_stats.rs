//! LSP editor-UX scorecard subcommand.
//!
//! Reports fixture inventory and surfaces pass-rate metrics from the
//! headless test suite.  The actual measurements are produced by the
//! integration-test suite in
//! `crates/perl-lsp-rs/tests/editor_intelligence_scorecard.rs`; this command
//! reads those results and emits `.ci/metrics/editor_ux.json`.
//!
//! ## Usage
//!
//! ```bash
//! # Print fixture inventory and last-run pass rates
//! cargo xtask metrics lsp-stats
//!
//! # Write receipt to .ci/metrics/editor_ux.json
//! cargo xtask metrics lsp-stats --json
//!
//! # Aggregate measured UX scenario receipts
//! cargo xtask metrics lsp-stats --json --receipt-dir target/receipts/editor-ux
//! ```
//!
//! ## Top-line UX metrics (three numbers)
//!
//! - `workflow_pass_rate` — fraction of canonical editor workflows that
//!   complete with the expected result
//! - `workflow_stability_rate` — fraction that avoid spurious extra
//!   diagnostics, empty results, or regressions while typing / reindexing
//! - `p95_time_to_first_useful_result_ms` — p95 latency to first useful
//!   hover / completion / goto-definition result

use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use perl_corpus::gold::{
    load_completion_gold_fixtures, load_goto_gold_fixtures, load_hover_gold_fixtures,
};
use perl_lsp_ux_tests::recorder::UxScenarioRunReceipt;
use perl_lsp_ux_tests::taxonomy::{MetricState, UxScenarioResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Output schema for .ci/metrics/editor_ux.json
// ---------------------------------------------------------------------------

/// Top-level receipt written to `.ci/metrics/editor_ux.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct EditorUxMetrics {
    pub schema_version: u32,
    pub measured_at: String,
    pub subsystem: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run: Option<LastRunMetrics>,
    pub metrics: UxMetrics,
}

/// UX metric values.  `None` means not-yet-instrumented.
#[derive(Debug, Serialize, Deserialize)]
pub struct UxMetrics {
    /// Fraction of canonical editor workflows completing with the expected
    /// result.  Phase 1: derived from hover + goto + completion pass rates.
    pub workflow_pass_rate: Option<f64>,
    /// Fraction of workflows that avoid spurious extra diagnostics, empty
    /// results, flicker, or regressions while typing / reindexing.
    /// Phase 2.
    pub workflow_stability_rate: Option<f64>,
    /// p95 latency (ms) to first useful hover / completion / goto result.
    /// Phase 2 (latency instrumentation).
    pub p95_time_to_first_useful_result_ms: Option<u64>,

    // --- Feature drill-down rows (Phase 1 fills the first three) ---
    pub hover_correctness_rate: Option<f64>,
    /// Top-1 completion relevance against gold fixtures.
    /// Phase 2 (ranking-aware fixture assertions).
    pub completion_top1_relevance: Option<f64>,
    /// Top-5 completion relevance against gold fixtures.
    /// Phase 1 currently approximates this from completion pass rate.
    pub completion_top5_relevance: Option<f64>,
    /// Backward-compatible alias kept while downstream consumers migrate.
    /// Prefer `completion_top5_relevance`.
    pub completion_top5_usefulness: Option<f64>,
    pub completion_empty_when_should_not_be_empty_rate: Option<f64>,
    pub goto_definition_exact_hit_rate: Option<f64>,
    /// Phase 2+
    pub rename_success_rate: Option<f64>,
    /// Phase 2+
    pub settled_diagnostics_correctness_after_edit: Option<f64>,
    /// Phase 2+
    pub module_resolution_workflow_success: Option<f64>,
    /// Phase 2+
    pub multi_root_workspace_navigation_success: Option<f64>,
    /// Phase 3 (DAP lane)
    pub dap_happy_path_success_rate: Option<f64>,
}

// ---------------------------------------------------------------------------
// Internal: pass-rate data computed from gold fixtures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LastRunMetrics {
    pub hover_passed: usize,
    pub hover_total: usize,
    pub goto_passed: usize,
    pub goto_total: usize,
    pub completion_passed: usize,
    pub completion_total: usize,
}

impl LastRunMetrics {
    fn hover_rate(&self) -> Option<f64> {
        if self.hover_total == 0 {
            None
        } else {
            Some(self.hover_passed as f64 / self.hover_total as f64)
        }
    }
    fn goto_rate(&self) -> Option<f64> {
        if self.goto_total == 0 {
            None
        } else {
            Some(self.goto_passed as f64 / self.goto_total as f64)
        }
    }
    fn completion_rate(&self) -> Option<f64> {
        if self.completion_total == 0 {
            None
        } else {
            Some(self.completion_passed as f64 / self.completion_total as f64)
        }
    }

    /// Weighted average across all instrumented workflows.
    fn workflow_pass_rate(&self) -> Option<f64> {
        let total = self.hover_total + self.goto_total + self.completion_total;
        if total == 0 {
            return None;
        }
        let passed = self.hover_passed + self.goto_passed + self.completion_passed;
        Some(passed as f64 / total as f64)
    }
}

#[derive(Debug, Clone, Default)]
struct ObservedUxRates {
    workflow_pass_rate: Option<f64>,
    hover_correctness_rate: Option<f64>,
    goto_definition_exact_hit_rate: Option<f64>,
    completion_top5_usefulness: Option<f64>,
}

impl ObservedUxRates {
    fn from_last_run(last_run: &LastRunMetrics) -> Self {
        Self {
            workflow_pass_rate: last_run.workflow_pass_rate(),
            hover_correctness_rate: last_run.hover_rate(),
            goto_definition_exact_hit_rate: last_run.goto_rate(),
            completion_top5_usefulness: last_run.completion_rate(),
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run `cargo xtask metrics lsp-stats`, optionally aggregating scenario receipts.
///
/// When `receipt_dir` is provided, the command reads `ux_scenario_run` receipts
/// from that directory and writes the measured `.ci/metrics/editor_ux.json`
/// scorecard when `json` is true. Without `receipt_dir`, the command preserves
/// the legacy fixture-inventory output.
pub fn run_with_receipt_dir(json: bool, receipt_dir: Option<&Path>) -> Result<()> {
    let root = project_root()?;

    if let Some(receipts_dir) = receipt_dir {
        let fixture_matrix = root
            .join("crates")
            .join("perl-lsp-ux-tests")
            .join("fixtures")
            .join("editor_ux_fixture_matrix.json");
        let output_path = root.join(".ci").join("metrics").join("editor_ux.json");
        let scorecard = aggregate_from_receipts(receipts_dir, &fixture_matrix, None)?;

        print_measured_scorecard_summary(&scorecard, receipts_dir);

        if json {
            write_json_receipt(&output_path, &scorecard)
                .with_context(|| format!("writing receipt to {}", output_path.display()))?;
            println!("\nWrote receipt: {}", output_path.display());
        }

        return Ok(());
    }

    let gold_root = root.join("test_corpus").join("gold");

    // Count fixtures
    let hover_fixtures = load_hover_gold_fixtures(&gold_root).unwrap_or_default();
    let goto_fixtures = load_goto_gold_fixtures(&gold_root).unwrap_or_default();
    let completion_fixtures = load_completion_gold_fixtures(&gold_root).unwrap_or_default();

    let hover_assertions: usize = hover_fixtures.iter().map(|f| f.hover_assertions.len()).sum();
    let goto_assertions: usize = goto_fixtures.iter().map(|f| f.goto_assertions.len()).sum();
    let completion_assertions: usize =
        completion_fixtures.iter().map(|f| f.completion_assertions.len()).sum();

    // Try to load a previous run receipt for pass-rate data
    let receipt_path = root.join(".ci").join("metrics").join("editor_ux.json");
    let observed_rates = load_observed_rates(&receipt_path);
    let last_run = load_last_run(&receipt_path);

    print_table(
        hover_fixtures.len(),
        hover_assertions,
        goto_fixtures.len(),
        goto_assertions,
        completion_fixtures.len(),
        completion_assertions,
        observed_rates.as_ref(),
    );

    if json {
        let metrics = build_metrics(observed_rates.as_ref());
        let output = EditorUxMetrics {
            schema_version: 1,
            measured_at: Utc::now().to_rfc3339(),
            subsystem: "editor_ux",
            last_run: last_run.clone(),
            metrics,
        };
        write_json_receipt(&receipt_path, &output)
            .with_context(|| format!("writing receipt to {}", receipt_path.display()))?;
        println!("\nWrote receipt: {}", receipt_path.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_metrics(observed_rates: Option<&ObservedUxRates>) -> UxMetrics {
    let (hover_rate, goto_rate, completion_rate, workflow_rate) = match observed_rates {
        Some(r) => (
            r.hover_correctness_rate,
            r.goto_definition_exact_hit_rate,
            r.completion_top5_usefulness,
            r.workflow_pass_rate,
        ),
        None => (None, None, None, None),
    };

    // completion_empty_rate: inverse of non-empty completion pass rate.
    // Phase 1: not yet computed separately; deferred to Phase 2.
    UxMetrics {
        workflow_pass_rate: workflow_rate,
        workflow_stability_rate: None,            // Phase 2
        p95_time_to_first_useful_result_ms: None, // Phase 2
        hover_correctness_rate: hover_rate,
        completion_top1_relevance: None, // Phase 2
        completion_top5_relevance: completion_rate,
        completion_top5_usefulness: completion_rate,
        completion_empty_when_should_not_be_empty_rate: None, // Phase 2
        goto_definition_exact_hit_rate: goto_rate,
        rename_success_rate: None,
        settled_diagnostics_correctness_after_edit: None,
        module_resolution_workflow_success: None,
        multi_root_workspace_navigation_success: None,
        dap_happy_path_success_rate: None,
    }
}

fn load_observed_rates(path: &Path) -> Option<ObservedUxRates> {
    let raw = fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&raw).ok()?;
    // Accept both top-level `last_run` key (legacy) and the current schema
    // where pass-rate data is inside `metrics` as individual rates.
    // Prefer `last_run` when available because it carries numerator/denominator
    // data and avoids rounding loss.
    if let Some(last) = doc.get("last_run")
        && let Ok(parsed) = serde_json::from_value::<LastRunMetrics>(last.clone())
    {
        return Some(ObservedUxRates::from_last_run(&parsed));
    }

    let metrics = doc.get("metrics")?;
    Some(ObservedUxRates {
        workflow_pass_rate: metrics.get("workflow_pass_rate").and_then(serde_json::Value::as_f64),
        hover_correctness_rate: metrics
            .get("hover_correctness_rate")
            .and_then(serde_json::Value::as_f64),
        goto_definition_exact_hit_rate: metrics
            .get("goto_definition_exact_hit_rate")
            .and_then(serde_json::Value::as_f64),
        completion_top5_usefulness: metrics
            .get("completion_top5_usefulness")
            .and_then(serde_json::Value::as_f64),
    })
}

/// Load the raw `last_run` block (pass/total counts) from a receipt file, if
/// present. Returns `None` when the receipt is missing, unreadable, or lacks
/// a well-formed `last_run` entry.
fn load_last_run(path: &Path) -> Option<LastRunMetrics> {
    let raw = fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let last = doc.get("last_run")?;
    serde_json::from_value::<LastRunMetrics>(last.clone()).ok()
}

fn write_json_receipt<T: Serialize>(path: &Path, output: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(output)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn print_measured_scorecard_summary(scorecard: &MeasuredEditorUxScorecard, receipts_dir: &Path) {
    println!("\nMeasured Editor UX Scorecard");
    println!("{}", "=".repeat(60));
    println!("Receipts: {}", receipts_dir.display());
    println!("Workflows: {}", scorecard.workflows.len());
    println!("workflow_pass_rate: {}", format_rate_metric(&scorecard.top_line.workflow_pass_rate));
    println!(
        "workflow_stability_rate: {}",
        format_rate_metric(&scorecard.top_line.workflow_stability_rate)
    );
    println!(
        "p95_time_to_first_useful_result_ms: {}",
        format_latency_metric(&scorecard.top_line.p95_time_to_first_useful_result_ms)
    );
}

fn format_rate_metric(metric: &RateMetric) -> String {
    if metric.confidence == "low" {
        return format!("insufficient_data ({})", metric.basis.join("; "));
    }
    format!("{:.1}% from {}", metric.value * 100.0, metric.basis.join("; "))
}

fn format_latency_metric(metric: &LatencyMetric) -> String {
    if metric.confidence == "low" {
        return format!("insufficient_data ({})", metric.basis.join("; "));
    }
    format!("{:.1} ms from {}", metric.value, metric.basis.join("; "))
}

#[allow(clippy::too_many_arguments)]
fn print_table(
    hover_fixtures: usize,
    hover_assertions: usize,
    goto_fixtures: usize,
    goto_assertions: usize,
    completion_fixtures: usize,
    completion_assertions: usize,
    observed_rates: Option<&ObservedUxRates>,
) {
    println!("\nEditor UX Scorecard (Phase 1)");
    println!("{}", "=".repeat(60));
    println!("{:<20} {:>10} {:>12}", "Kind", "Fixtures", "Assertions");
    println!("{}", "-".repeat(44));
    println!("{:<20} {:>10} {:>12}", "Hover", hover_fixtures, hover_assertions);
    println!("{:<20} {:>10} {:>12}", "Goto-Definition", goto_fixtures, goto_assertions);
    println!("{:<20} {:>10} {:>12}", "Completion", completion_fixtures, completion_assertions);
    println!("{}", "-".repeat(44));
    let total_f = hover_fixtures + goto_fixtures + completion_fixtures;
    let total_a = hover_assertions + goto_assertions + completion_assertions;
    println!("{:<20} {:>10} {:>12}", "TOTAL", total_f, total_a);

    if let Some(rates) = observed_rates {
        println!("\nLast Run — UX Metrics");
        println!("{}", "=".repeat(60));
        if let Some(rate) = rates.workflow_pass_rate {
            println!("  workflow_pass_rate:          {:.1}%", rate * 100.0);
        }
        if let Some(rate) = rates.hover_correctness_rate {
            println!("  hover_correctness_rate:      {:.1}%", rate * 100.0);
        }
        if let Some(rate) = rates.goto_definition_exact_hit_rate {
            println!("  goto_definition_exact_hit:   {:.1}%", rate * 100.0);
        }
        if let Some(rate) = rates.completion_top5_usefulness {
            println!("  completion_top5_usefulness:  {:.1}%", rate * 100.0);
        }
        println!("  completion_top1_relevance:   (Phase 2)");
        println!("  workflow_stability_rate:     (Phase 2)");
        println!("  p95_time_to_first_result_ms: (Phase 2)");
    } else {
        println!("\n(No last-run receipt found — run the integration tests first)");
        println!(
            "  RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
            --test editor_intelligence_scorecard -- --nocapture"
        );
    }
    println!();
}

// ---------------------------------------------------------------------------
// Phase 2: Receipt-based scorecard aggregation
// ---------------------------------------------------------------------------

/// Minimum number of receipts per workflow before stability can be computed.
const MIN_STABILITY_RECEIPTS: usize = 2;

/// Metric provenance metadata conforming to the schema's `metricProvenance`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[allow(dead_code)] // constructed via Deserialize in scorecard consumers
pub struct MetricProvenance {
    pub kind: String,
    pub basis: Vec<String>,
    pub coverage: String,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumptions: Option<Vec<String>>,
}

/// A rate metric (0.0–1.0) with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RateMetric {
    pub value: f64,
    pub kind: String,
    pub basis: Vec<String>,
    pub coverage: String,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumptions: Option<Vec<String>>,
}

/// A latency metric (ms) with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LatencyMetric {
    pub value: f64,
    pub kind: String,
    pub basis: Vec<String>,
    pub coverage: String,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumptions: Option<Vec<String>>,
}

/// Top-line metrics for the measured scorecard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TopLineMetrics {
    pub workflow_pass_rate: RateMetric,
    pub workflow_stability_rate: RateMetric,
    pub p95_time_to_first_useful_result_ms: LatencyMetric,
}

/// Component-level metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentMetrics {
    pub cross_file_definition_success_rate: RateMetric,
    pub module_resolution_workflow_success_rate: RateMetric,
    pub multi_root_workspace_navigation_success_rate: RateMetric,
}

/// Per-workflow result row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkflowResult {
    pub id: String,
    pub scenario: String,
    pub subsystem_owner: String,
    pub pass_rate: RateMetric,
    pub stability_rate: RateMetric,
    pub p95_time_to_first_useful_result_ms: LatencyMetric,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_metrics: Option<BTreeMap<String, serde_json::Value>>,
    /// Maximum quarantine age in days across quarantined tests in this workflow.
    /// Present only when the workflow has quarantined scenarios.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_age_days: Option<i64>,
}

/// Provenance metadata for the scorecard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScorecardProvenance {
    pub fixture_matrix: String,
    pub harness: String,
    pub tiers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

/// The measured scorecard conforming to `.ci/schemas/editor-ux.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MeasuredEditorUxScorecard {
    pub schema_version: u32,
    pub measured_at: String,
    pub subsystem: String,
    pub top_line: TopLineMetrics,
    pub components: ComponentMetrics,
    pub workflows: Vec<WorkflowResult>,
    pub provenance: ScorecardProvenance,
}

/// Workflow entry from the fixture matrix JSON.
#[derive(Debug, Clone, Deserialize)]
struct FixtureMatrixWorkflow {
    id: String,
    scenario_file: String,
    subsystem_owner: String,
    #[allow(dead_code)] // used for filtering in future CI tier work
    ci_tier: String,
    #[serde(default)]
    measures: Vec<String>,
}

/// Top-level fixture matrix JSON structure.
#[derive(Debug, Clone, Deserialize)]
struct FixtureMatrix {
    #[allow(dead_code)]
    schema_version: u32,
    workflows: Vec<FixtureMatrixWorkflow>,
}

/// Flake ledger entry.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // fields deserialized for validation and test assertions
struct FlakeEntry {
    test: String,
    state: String,
    /// ISO-8601 date when the flake was first observed (e.g. `"2026-04-30"`).
    #[serde(default)]
    first_seen: Option<String>,
    /// Subsystem owner for triage routing.
    #[serde(default)]
    subsystem: Option<String>,
    /// GitHub handle of the responsible party. Required for active entries.
    #[serde(default)]
    owner: Option<String>,
    /// GitHub issue number tracking the flake. Required for active entries.
    #[serde(default)]
    issue: Option<u64>,
    /// How the quarantine affects CI gating.
    #[serde(default)]
    quarantine_effect: Option<String>,
}

/// Summary section of the flake ledger.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // fields deserialized for validation in tests
struct FlakeSummary {
    total: usize,
    active: usize,
    resolved: usize,
    #[serde(default)]
    by_subsystem: BTreeMap<String, usize>,
}

/// Top-level flake ledger JSON structure.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // summary field deserialized for validation in tests
struct FlakeLedger {
    entries: Vec<FlakeEntry>,
    #[serde(default)]
    summary: Option<FlakeSummary>,
}

/// Validate that the flake ledger summary counts are consistent with entries.
///
/// Returns `Ok(())` if consistent, or `Err` with a description of the mismatch.
#[cfg(test)]
fn validate_flake_ledger_summary(ledger: &FlakeLedger) -> Result<()> {
    let summary = ledger
        .summary
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("flake ledger missing summary section"))?;

    let entry_count = ledger.entries.len();
    let active_count = ledger.entries.iter().filter(|e| e.state == "active").count();
    let resolved_count = ledger.entries.iter().filter(|e| e.state == "resolved").count();

    if summary.total != entry_count {
        color_eyre::eyre::bail!(
            "summary.total ({}) != entries.len() ({})",
            summary.total,
            entry_count
        );
    }
    if summary.active != active_count {
        color_eyre::eyre::bail!(
            "summary.active ({}) != active entry count ({})",
            summary.active,
            active_count
        );
    }
    if summary.resolved != resolved_count {
        color_eyre::eyre::bail!(
            "summary.resolved ({}) != resolved entry count ({})",
            summary.resolved,
            resolved_count
        );
    }

    // Verify by_subsystem counts match actual entries.
    let mut actual_by_subsystem: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &ledger.entries {
        if let Some(ref sub) = entry.subsystem {
            *actual_by_subsystem.entry(sub.clone()).or_default() += 1;
        }
    }
    if summary.by_subsystem != actual_by_subsystem {
        color_eyre::eyre::bail!(
            "summary.by_subsystem ({:?}) != actual subsystem counts ({:?})",
            summary.by_subsystem,
            actual_by_subsystem
        );
    }

    Ok(())
}

/// Check whether a quarantine entry blocks the PR gate.
///
/// `non_blocking_pr` and `advisory` entries do not block PR merges.
/// `release_blocking` entries do not block PR merges either — they only
/// block release gates.
#[cfg(test)]
fn quarantine_blocks_pr(entry: &FlakeEntry) -> bool {
    // No quarantine_effect value blocks PR — quarantined entries are
    // non-blocking for PR by definition. Only `release_blocking` blocks
    // the release gate, not the PR gate.
    let _ = entry;
    false
}

/// Check whether a quarantine entry blocks the release gate.
///
/// Only `release_blocking` entries block the release gate.
#[cfg(test)]
fn quarantine_blocks_release(entry: &FlakeEntry) -> bool {
    entry.quarantine_effect.as_deref() == Some("release_blocking")
}

/// Intermediate per-workflow aggregation bucket.
#[derive(Debug, Default)]
struct WorkflowBucket {
    pass_count: usize,
    fail_count: usize,
    quarantined_count: usize,
    skipped_count: usize,
    /// Timing values from passing scenarios with non-null timing.
    pass_timings: Vec<f64>,
    /// All receipts for this workflow (for stability computation).
    results: Vec<UxScenarioResult>,
    /// Test names that were reclassified as quarantined (for age computation).
    quarantined_test_names: Vec<String>,
}

/// Build a default rate metric with measured provenance.
pub(crate) fn measured_rate(value: f64, sample_count: usize) -> RateMetric {
    RateMetric {
        value,
        kind: "measured".to_owned(),
        basis: vec![format!("{sample_count} receipts")],
        coverage: "receipts_included".to_owned(),
        confidence: if sample_count >= 5 { "high" } else { "medium" }.to_owned(),
        method: None,
        assumptions: None,
    }
}

/// Build a default latency metric with measured provenance.
pub(crate) fn measured_latency(value: f64, sample_count: usize) -> LatencyMetric {
    LatencyMetric {
        value,
        kind: "measured".to_owned(),
        basis: vec![format!("{sample_count} timing samples")],
        coverage: "receipts_included".to_owned(),
        confidence: if sample_count >= 5 { "high" } else { "medium" }.to_owned(),
        method: Some("p95".to_owned()),
        assumptions: None,
    }
}

/// Build an insufficient-data rate metric.
pub(crate) fn insufficient_rate(reason: &str) -> RateMetric {
    RateMetric {
        value: 0.0,
        kind: "measured".to_owned(),
        basis: vec![reason.to_owned()],
        coverage: "receipts_included".to_owned(),
        confidence: "low".to_owned(),
        method: None,
        assumptions: Some(vec!["insufficient data".to_owned()]),
    }
}

/// Build an insufficient-data latency metric.
pub(crate) fn insufficient_latency(reason: &str) -> LatencyMetric {
    LatencyMetric {
        value: 0.0,
        kind: "measured".to_owned(),
        basis: vec![reason.to_owned()],
        coverage: "receipts_included".to_owned(),
        confidence: "low".to_owned(),
        method: Some("p95".to_owned()),
        assumptions: Some(vec!["insufficient data".to_owned()]),
    }
}

/// Compute the p95 value from a sorted slice of f64 values.
/// Returns `None` if the slice is empty.
fn compute_p95(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((values.len() as f64 * 0.95).ceil() as usize).min(values.len()) - 1;
    Some(values[idx])
}

/// Compute workflow stability rate: fraction of results that are `Pass`
/// over all non-skipped results. Quarantined counts as unstable (not pass).
fn compute_stability(results: &[UxScenarioResult]) -> MetricState<f64> {
    let non_skipped: Vec<_> =
        results.iter().filter(|r| !matches!(**r, UxScenarioResult::Skipped)).collect();
    if non_skipped.len() < MIN_STABILITY_RECEIPTS {
        return MetricState::InsufficientData {
            reason: format!(
                "only {} receipts, need at least {MIN_STABILITY_RECEIPTS}",
                non_skipped.len()
            ),
        };
    }
    let pass_count = non_skipped.iter().filter(|r| matches!(***r, UxScenarioResult::Pass)).count();
    MetricState::Measured {
        value: pass_count as f64 / non_skipped.len() as f64,
        sample_count: non_skipped.len(),
    }
}

/// Load the set of quarantined test names from the flake ledger.
///
/// Returns a map from test name to the optional `first_seen` date string.
fn load_quarantined_tests(flake_ledger: Option<&Path>) -> BTreeMap<String, Option<String>> {
    let mut quarantined = BTreeMap::new();
    let Some(path) = flake_ledger else {
        return quarantined;
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return quarantined;
    };
    let Ok(ledger) = serde_json::from_str::<FlakeLedger>(&raw) else {
        return quarantined;
    };
    for entry in &ledger.entries {
        if entry.state == "active" {
            quarantined.insert(entry.test.clone(), entry.first_seen.clone());
        }
    }
    quarantined
}

/// Compute the maximum quarantine age in days for a set of quarantined test
/// names, using the `first_seen` dates from the flake ledger.
///
/// Returns `None` if no quarantined tests have a parseable `first_seen` date.
fn compute_quarantine_age(
    quarantined_test_names: &[String],
    quarantined_tests: &BTreeMap<String, Option<String>>,
) -> Option<i64> {
    let today = Utc::now().date_naive();
    let mut max_age: Option<i64> = None;
    for test_name in quarantined_test_names {
        if let Some(Some(first_seen_str)) = quarantined_tests.get(test_name)
            && let Ok(first_seen) = chrono::NaiveDate::parse_from_str(first_seen_str, "%Y-%m-%d")
        {
            let age = (today - first_seen).num_days();
            max_age = Some(max_age.map_or(age, |current| current.max(age)));
        }
    }
    max_age
}

/// Read all `UxScenarioRunReceipt` JSON files from a directory.
fn read_receipts(receipts_dir: &Path) -> Result<Vec<UxScenarioRunReceipt>> {
    let mut receipts = Vec::new();
    if !receipts_dir.exists() {
        return Ok(receipts);
    }
    let entries = fs::read_dir(receipts_dir)
        .with_context(|| format!("reading receipts directory: {}", receipts_dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| "reading directory entry")?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading receipt: {}", path.display()))?;
        match serde_json::from_str::<UxScenarioRunReceipt>(&raw) {
            Ok(receipt) => receipts.push(receipt),
            Err(_) => {
                // Skip non-receipt JSON files silently (e.g. schema files).
                continue;
            }
        }
    }
    Ok(receipts)
}

/// Aggregate `UxScenarioRunReceipt` files into a measured scorecard.
///
/// - `receipts_dir`: directory containing receipt JSON files
///   (default: `target/receipts/editor-ux/`)
/// - `fixture_matrix`: path to `editor_ux_fixture_matrix.json`
/// - `flake_ledger`: optional path to `.ci/ux-flakes.json`
pub fn aggregate_from_receipts(
    receipts_dir: &Path,
    fixture_matrix: &Path,
    flake_ledger: Option<&Path>,
) -> Result<MeasuredEditorUxScorecard> {
    // Load fixture matrix for workflow metadata.
    let matrix_raw = fs::read_to_string(fixture_matrix)
        .with_context(|| format!("reading fixture matrix: {}", fixture_matrix.display()))?;
    let matrix: FixtureMatrix = serde_json::from_str(&matrix_raw)
        .with_context(|| format!("parsing fixture matrix: {}", fixture_matrix.display()))?;

    // Build workflow metadata lookup (used for future per-workflow enrichment).
    let _workflow_meta: BTreeMap<String, &FixtureMatrixWorkflow> =
        matrix.workflows.iter().map(|w| (w.id.clone(), w)).collect();

    // Load quarantined test names.
    let quarantined_tests = load_quarantined_tests(flake_ledger);

    // Read all receipts.
    let receipts = read_receipts(receipts_dir)?;

    // Group receipts by workflow_id.
    let mut buckets: BTreeMap<String, WorkflowBucket> = BTreeMap::new();
    for receipt in &receipts {
        let bucket = buckets.entry(receipt.workflow_id.clone()).or_default();

        // Check if this test is quarantined — override result to Quarantined
        // if the test is in the flake ledger and the result is Fail.
        let effective_result = if receipt.result == UxScenarioResult::Fail
            && quarantined_tests.contains_key(&receipt.test_name)
        {
            UxScenarioResult::Quarantined
        } else {
            receipt.result
        };

        match effective_result {
            UxScenarioResult::Pass => {
                bucket.pass_count += 1;
                // Only passing scenarios with non-null timing contribute to p95.
                if let Some(timing) = receipt.time_to_first_useful_result_ms {
                    bucket.pass_timings.push(timing);
                }
            }
            UxScenarioResult::Fail => bucket.fail_count += 1,
            UxScenarioResult::Quarantined => {
                bucket.quarantined_count += 1;
                bucket.quarantined_test_names.push(receipt.test_name.clone());
            }
            UxScenarioResult::Skipped | _ => bucket.skipped_count += 1,
        }
        bucket.results.push(effective_result);
    }

    // Collect all CI tiers seen.
    let mut tiers_seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for receipt in &receipts {
        let tier_str = serde_json::to_value(receipt.ci_tier)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_owned());
        tiers_seen.insert(tier_str);
    }

    // Build per-workflow rows.
    let mut workflow_rows: Vec<WorkflowResult> = Vec::new();
    // Accumulators for top-line metrics (excluding InsufficientData workflows).
    let mut top_pass_count: usize = 0;
    let mut top_fail_count: usize = 0;
    let mut top_stability_sum: f64 = 0.0;
    let mut top_stability_count: usize = 0;
    let mut all_pass_timings: Vec<f64> = Vec::new();

    // Component metric accumulators.
    let mut cross_file_pass: usize = 0;
    let mut cross_file_total: usize = 0;
    let mut module_resolution_pass: usize = 0;
    let mut module_resolution_total: usize = 0;
    let mut multi_root_pass: usize = 0;
    let mut multi_root_total: usize = 0;

    // Iterate over all workflows in the fixture matrix to ensure every
    // workflow appears in the output, even those with no receipts.
    for wf in &matrix.workflows {
        let bucket = buckets.get(&wf.id);

        let (pass_rate, stability_rate, p95_timing) = match bucket {
            Some(b) if b.pass_count + b.fail_count > 0 => {
                // Pass rate: pass / (pass + fail), excluding quarantined/skipped.
                let eligible = b.pass_count + b.fail_count;
                let pr = b.pass_count as f64 / eligible as f64;

                // Stability rate.
                let sr = compute_stability(&b.results);

                // p95 timing from passing scenarios only.
                let mut timings = b.pass_timings.clone();
                let p95 = compute_p95(&mut timings);

                // Contribute to top-line accumulators.
                top_pass_count += b.pass_count;
                top_fail_count += b.fail_count;
                all_pass_timings.extend_from_slice(&b.pass_timings);

                let stability_metric = match &sr {
                    MetricState::Measured { value, sample_count } => {
                        top_stability_sum += value;
                        top_stability_count += 1;
                        measured_rate(*value, *sample_count)
                    }
                    _ => {
                        let reason_str = match &sr {
                            MetricState::InsufficientData { reason } => reason.clone(),
                            _ => "unknown metric state".to_owned(),
                        };
                        insufficient_rate(&reason_str)
                    }
                };

                let p95_metric = match p95 {
                    Some(v) => measured_latency(v, timings.len()),
                    None => insufficient_latency("no passing scenarios with timing data"),
                };

                (measured_rate(pr, eligible), stability_metric, p95_metric)
            }
            Some(b) if b.quarantined_count > 0 || b.skipped_count > 0 => {
                // All receipts are quarantined or skipped — no pass/fail data.
                let sr = compute_stability(&b.results);
                let stability_metric = match &sr {
                    MetricState::Measured { value, sample_count } => {
                        top_stability_sum += value;
                        top_stability_count += 1;
                        measured_rate(*value, *sample_count)
                    }
                    _ => {
                        let reason_str = match &sr {
                            MetricState::InsufficientData { reason } => reason.clone(),
                            _ => "unknown metric state".to_owned(),
                        };
                        insufficient_rate(&reason_str)
                    }
                };
                (
                    insufficient_rate("no pass/fail receipts"),
                    stability_metric,
                    insufficient_latency("no passing scenarios with timing data"),
                )
            }
            _ => {
                // No receipts at all — InsufficientData.
                (
                    insufficient_rate("no receipts"),
                    insufficient_rate("no receipts"),
                    insufficient_latency("no receipts"),
                )
            }
        };

        // Component metric contributions based on fixture matrix `measures`.
        if let Some(b) = bucket {
            let wf_pass = b.pass_count;
            let wf_eligible = b.pass_count + b.fail_count;
            if wf_eligible > 0 {
                for measure in &wf.measures {
                    match measure.as_str() {
                        "cross_file_definition_success_rate" => {
                            cross_file_pass += wf_pass;
                            cross_file_total += wf_eligible;
                        }
                        "module_resolution_workflow_success_rate" => {
                            module_resolution_pass += wf_pass;
                            module_resolution_total += wf_eligible;
                        }
                        "multi_root_workspace_navigation_success_rate" => {
                            multi_root_pass += wf_pass;
                            multi_root_total += wf_eligible;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Compute quarantine age for this workflow if it has quarantined tests.
        let quarantine_age_days = bucket.and_then(|b| {
            if b.quarantined_count > 0 {
                compute_quarantine_age(&b.quarantined_test_names, &quarantined_tests)
            } else {
                None
            }
        });

        workflow_rows.push(WorkflowResult {
            id: wf.id.clone(),
            scenario: wf.scenario_file.clone(),
            subsystem_owner: wf.subsystem_owner.clone(),
            pass_rate,
            stability_rate,
            p95_time_to_first_useful_result_ms: p95_timing,
            component_metrics: None,
            quarantine_age_days,
        });
    }

    // Top-line metrics.
    let top_eligible = top_pass_count + top_fail_count;
    let top_pass_rate = if top_eligible > 0 {
        measured_rate(top_pass_count as f64 / top_eligible as f64, top_eligible)
    } else {
        insufficient_rate("no eligible receipts")
    };

    let top_stability = if top_stability_count > 0 {
        measured_rate(top_stability_sum / top_stability_count as f64, top_stability_count)
    } else {
        insufficient_rate("no workflows with sufficient data")
    };

    let top_p95 = {
        let mut timings = all_pass_timings;
        match compute_p95(&mut timings) {
            Some(v) => measured_latency(v, timings.len()),
            None => insufficient_latency("no passing scenarios with timing data"),
        }
    };

    // Component metrics.
    let cross_file_rate = if cross_file_total > 0 {
        measured_rate(cross_file_pass as f64 / cross_file_total as f64, cross_file_total)
    } else {
        insufficient_rate("no workflows measuring cross_file_definition_success_rate")
    };

    let module_resolution_rate = if module_resolution_total > 0 {
        measured_rate(
            module_resolution_pass as f64 / module_resolution_total as f64,
            module_resolution_total,
        )
    } else {
        insufficient_rate("no workflows measuring module_resolution_workflow_success_rate")
    };

    let multi_root_rate = if multi_root_total > 0 {
        measured_rate(multi_root_pass as f64 / multi_root_total as f64, multi_root_total)
    } else {
        insufficient_rate("no workflows measuring multi_root_workspace_navigation_success_rate")
    };

    Ok(MeasuredEditorUxScorecard {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "editor_ux".to_owned(),
        top_line: TopLineMetrics {
            workflow_pass_rate: top_pass_rate,
            workflow_stability_rate: top_stability,
            p95_time_to_first_useful_result_ms: top_p95,
        },
        components: ComponentMetrics {
            cross_file_definition_success_rate: cross_file_rate,
            module_resolution_workflow_success_rate: module_resolution_rate,
            multi_root_workspace_navigation_success_rate: multi_root_rate,
        },
        workflows: workflow_rows,
        provenance: ScorecardProvenance {
            fixture_matrix: fixture_matrix.display().to_string(),
            harness: "crates/perl-lsp-ux-tests".to_owned(),
            tiers: tiers_seen.into_iter().collect(),
            notes: None,
        },
    })
}

// ---------------------------------------------------------------------------
// Receipt-based scorecard → floor metrics for ratchet checking
// ---------------------------------------------------------------------------

/// Helper: extract the numeric value from a `RateMetric`, returning `None`
/// when the metric has insufficient data (confidence == "low").
#[cfg(test)]
fn rate_metric_value(metric: &RateMetric) -> Option<f64> {
    if metric.confidence == "low" {
        return None;
    }
    Some(metric.value)
}

/// Helper: extract the numeric value from a `LatencyMetric`, returning `None`
/// when the metric has insufficient data (confidence == "low").
#[cfg(test)]
fn latency_metric_value(metric: &LatencyMetric) -> Option<f64> {
    if metric.confidence == "low" {
        return None;
    }
    Some(metric.value)
}

/// Convert a `MeasuredEditorUxScorecard` into a `BTreeMap<String, Option<f64>>`
/// keyed by the same metric names used in
/// `.ci/metrics/baselines/editor_ux.json`.
///
/// Rate metrics (0.0–1.0) are converted to percentages (0.0–100.0) to match
/// the baseline convention.  Latency metrics are passed through as-is (ms).
///
/// Metrics with insufficient data map to `None` and are silently skipped by
/// `check_floor_metrics()`.
#[cfg(test)]
pub fn scorecard_to_floor_metrics(
    scorecard: &MeasuredEditorUxScorecard,
) -> BTreeMap<String, Option<f64>> {
    let mut metrics = BTreeMap::new();

    // -- Component-level correctness rates (rate → pct) ---------------------
    let rate_to_pct = |v: Option<f64>| v.map(|r| r * 100.0);

    metrics.insert(
        "cross_file_success_pct".to_owned(),
        rate_to_pct(rate_metric_value(&scorecard.components.cross_file_definition_success_rate)),
    );

    // -- Top-line latency ---------------------------------------------------
    // The baseline has per-request-class latency keys.  The receipt-based
    // scorecard only has a single top-line p95.  We expose it under a
    // dedicated key so the baseline can opt in to checking it.
    metrics.insert(
        "p95_time_to_first_useful_result_ms".to_owned(),
        latency_metric_value(&scorecard.top_line.p95_time_to_first_useful_result_ms),
    );

    // -- Top-line rates (rate → pct) ----------------------------------------
    metrics.insert(
        "workflow_pass_rate_pct".to_owned(),
        rate_to_pct(rate_metric_value(&scorecard.top_line.workflow_pass_rate)),
    );
    metrics.insert(
        "workflow_stability_rate_pct".to_owned(),
        rate_to_pct(rate_metric_value(&scorecard.top_line.workflow_stability_rate)),
    );

    // -- Per-workflow drill-down: extract per-component pass rates -----------
    // Walk workflows and accumulate pass/total by component to produce
    // component-level correctness percentages that align with baseline keys.
    let mut component_pass: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for wf in &scorecard.workflows {
        // Skip insufficient-data workflows.
        if wf.pass_rate.confidence == "low" {
            continue;
        }
        // Derive component from the workflow id.  The fixture matrix
        // `measures` field is not available here, so we use the workflow id
        // as a heuristic to map to baseline metric names.
        let id = &wf.id;
        let component_key = if id.contains("hover") {
            Some("hover_correctness_pct")
        } else if id.contains("completion") {
            Some("completion_top5_pct")
        } else if id.contains("goto_definition") || id.contains("definition") {
            Some("definition_exact_hit_pct")
        } else if id.contains("diagnostics") || id.contains("strict_diagnostics") {
            Some("diagnostics_correct_pct")
        } else if id.contains("rename") {
            Some("rename_success_pct")
        } else if id.contains("symbol") {
            Some("symbol_correctness_pct")
        } else {
            None
        };

        if let Some(key) = component_key {
            // Reconstruct pass/total from rate and basis count.
            let basis_count: usize = wf
                .pass_rate
                .basis
                .first()
                .and_then(|s| s.split_whitespace().next())
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
            if basis_count > 0 {
                let pass = (wf.pass_rate.value * basis_count as f64).round() as usize;
                let entry = component_pass.entry(key.to_owned()).or_insert((0, 0));
                entry.0 += pass;
                entry.1 += basis_count;
            }
        }
    }

    for (key, (pass, total)) in &component_pass {
        if *total > 0 {
            metrics.insert(key.clone(), Some(*pass as f64 / *total as f64 * 100.0));
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_run_metrics_zero_total_returns_none() {
        let m = LastRunMetrics {
            hover_passed: 0,
            hover_total: 0,
            goto_passed: 0,
            goto_total: 0,
            completion_passed: 0,
            completion_total: 0,
        };
        // Zero total must not panic and must return None (not-yet-instrumented)
        assert!(m.hover_rate().is_none());
        assert!(m.goto_rate().is_none());
        assert!(m.completion_rate().is_none());
        assert!(m.workflow_pass_rate().is_none());
    }

    #[test]
    fn test_last_run_metrics_rates_partial() {
        let m = LastRunMetrics {
            hover_passed: 8,
            hover_total: 10,
            goto_passed: 5,
            goto_total: 5,
            completion_passed: 3,
            completion_total: 4,
        };
        assert!((m.hover_rate().unwrap() - 0.8).abs() < 0.001);
        assert!((m.goto_rate().unwrap() - 1.0).abs() < 0.001);
        assert!((m.completion_rate().unwrap() - 0.75).abs() < 0.001);
        // workflow_pass_rate = (8+5+3)/(10+5+4) = 16/19
        let expected = 16.0_f64 / 19.0;
        assert!((m.workflow_pass_rate().unwrap() - expected).abs() < 0.001);
    }

    #[test]
    fn test_editor_ux_metrics_schema_serializes() {
        let output = EditorUxMetrics {
            schema_version: 1,
            measured_at: "2026-04-11T00:00:00Z".to_string(),
            subsystem: "editor_ux",
            last_run: Some(LastRunMetrics {
                hover_passed: 8,
                hover_total: 10,
                goto_passed: 5,
                goto_total: 5,
                completion_passed: 3,
                completion_total: 4,
            }),
            metrics: UxMetrics {
                workflow_pass_rate: Some(0.91),
                workflow_stability_rate: None,
                p95_time_to_first_useful_result_ms: None,
                hover_correctness_rate: Some(0.89),
                completion_top1_relevance: None,
                completion_top5_relevance: Some(0.86),
                completion_top5_usefulness: Some(0.86),
                completion_empty_when_should_not_be_empty_rate: None,
                goto_definition_exact_hit_rate: Some(0.94),
                rename_success_rate: None,
                settled_diagnostics_correctness_after_edit: None,
                module_resolution_workflow_success: None,
                multi_root_workspace_navigation_success: None,
                dap_happy_path_success_rate: None,
            },
        };
        let json = serde_json::to_string_pretty(&output).expect("serialization must succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("must parse back to JSON");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["subsystem"], "editor_ux");
        assert_eq!(parsed["last_run"]["hover_passed"], 8);
        assert!((parsed["metrics"]["workflow_pass_rate"].as_f64().unwrap() - 0.91).abs() < 0.001);
        assert!(parsed["metrics"]["rename_success_rate"].is_null());
        // Verify new relevance fields serialize correctly
        assert!(
            parsed["metrics"]["completion_top1_relevance"].is_null(),
            "completion_top1_relevance should be null (Phase 2)"
        );
        assert!(
            (parsed["metrics"]["completion_top5_relevance"].as_f64().unwrap() - 0.86).abs() < 0.001,
            "completion_top5_relevance should serialize to 0.86"
        );
        // Backward-compat alias should also be present
        assert!(
            (parsed["metrics"]["completion_top5_usefulness"].as_f64().unwrap() - 0.86).abs()
                < 0.001,
            "completion_top5_usefulness alias should still serialize"
        );
    }

    #[test]
    fn test_load_last_run_from_current_schema() {
        let temp = tempfile::NamedTempFile::new().expect("temp file should be created");
        let receipt = serde_json::json!({
            "schema_version": 1,
            "measured_at": "2026-04-11T00:00:00Z",
            "subsystem": "editor_ux",
            "last_run": {
                "hover_passed": 2,
                "hover_total": 3,
                "goto_passed": 1,
                "goto_total": 2,
                "completion_passed": 4,
                "completion_total": 5
            },
            "metrics": {
                "workflow_pass_rate": 0.7
            }
        });
        fs::write(temp.path(), serde_json::to_string_pretty(&receipt).expect("serialize JSON"))
            .expect("write receipt");

        let loaded = load_last_run(temp.path()).expect("last_run should be parsed");
        assert_eq!(loaded.hover_passed, 2);
        assert_eq!(loaded.hover_total, 3);
        assert_eq!(loaded.goto_passed, 1);
        assert_eq!(loaded.goto_total, 2);
        assert_eq!(loaded.completion_passed, 4);
        assert_eq!(loaded.completion_total, 5);
    }

    #[test]
    fn test_load_observed_rates_reads_metrics_schema() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        let receipt = serde_json::json!({
            "schema_version": 1,
            "subsystem": "editor_ux",
            "metrics": {
                "workflow_pass_rate": 0.91,
                "hover_correctness_rate": 0.89,
                "goto_definition_exact_hit_rate": 0.94,
                "completion_top5_usefulness": 0.86
            }
        });
        fs::write(temp.path(), serde_json::to_string_pretty(&receipt).expect("serialize receipt"))
            .expect("write receipt");

        let observed = load_observed_rates(temp.path()).expect("observed rates");
        assert!((observed.workflow_pass_rate.expect("workflow rate") - 0.91).abs() < 0.001);
        assert!((observed.hover_correctness_rate.expect("hover rate") - 0.89).abs() < 0.001);
        assert!((observed.goto_definition_exact_hit_rate.expect("goto rate") - 0.94).abs() < 0.001);
        assert!(
            (observed.completion_top5_usefulness.expect("completion rate") - 0.86).abs() < 0.001
        );
    }

    #[test]
    fn test_load_observed_rates_prefers_last_run_when_present() {
        let temp = tempfile::NamedTempFile::new().expect("temp file");
        let receipt = serde_json::json!({
            "last_run": {
                "hover_passed": 8,
                "hover_total": 10,
                "goto_passed": 6,
                "goto_total": 8,
                "completion_passed": 9,
                "completion_total": 12
            },
            "metrics": {
                "workflow_pass_rate": 0.0,
                "hover_correctness_rate": 0.0,
                "goto_definition_exact_hit_rate": 0.0,
                "completion_top5_usefulness": 0.0
            }
        });
        fs::write(temp.path(), serde_json::to_string_pretty(&receipt).expect("serialize receipt"))
            .expect("write receipt");

        let observed = load_observed_rates(temp.path()).expect("observed rates");
        assert!((observed.hover_correctness_rate.expect("hover rate") - 0.8).abs() < 0.001);
        assert!((observed.goto_definition_exact_hit_rate.expect("goto rate") - 0.75).abs() < 0.001);
        assert!(
            (observed.completion_top5_usefulness.expect("completion rate") - 0.75).abs() < 0.001
        );
        // (8 + 6 + 9) / (10 + 8 + 12)
        assert!(
            (observed.workflow_pass_rate.expect("workflow rate") - (23.0 / 30.0)).abs() < 0.001
        );
    }

    // ── Phase 2: Receipt-based aggregation tests ─────────────────────────

    /// Helper: create a minimal fixture matrix JSON with the given workflows.
    fn write_fixture_matrix(
        dir: &Path,
        workflows: &[(&str, &str, &str, &[&str])],
    ) -> std::path::PathBuf {
        let wfs: Vec<serde_json::Value> = workflows
            .iter()
            .map(|(id, scenario, owner, measures)| {
                serde_json::json!({
                    "id": id,
                    "scenario_file": scenario,
                    "subsystem_owner": owner,
                    "ci_tier": "pr",
                    "measures": measures,
                })
            })
            .collect();
        let matrix = serde_json::json!({
            "schema_version": 1,
            "workflows": wfs,
        });
        let path = dir.join("fixture_matrix.json");
        fs::write(&path, serde_json::to_string_pretty(&matrix).unwrap_or_default())
            .unwrap_or_default();
        path
    }

    /// Helper: create a receipt JSON file in the given directory.
    fn write_receipt_file(
        dir: &Path,
        workflow_id: &str,
        test_name: &str,
        result: &str,
        timing_ms: Option<f64>,
    ) {
        let receipt = serde_json::json!({
            "kind": "ux_scenario_run",
            "schema_version": 1,
            "measured_at": "2026-06-01T00:00:00Z",
            "run_identity": {
                "sha": "abcdef12",
                "branch": "main",
            },
            "workflow_id": workflow_id,
            "scenario_file": format!("{workflow_id}.rs"),
            "test_name": test_name,
            "ci_tier": "pr",
            "result": result,
            "duration_ms": 100.0,
            "time_to_first_useful_result_ms": timing_ms,
            "assertions": {
                "passed": 1,
                "failed": 0,
                "basis": "instrumented",
            },
            "canonical_repro": "cargo test ...",
            "friendly_repro": "just ux-tests ...",
        });
        let filename = format!("{workflow_id}-{test_name}-abcdef12.json");
        let path = dir.join(filename);
        fs::write(&path, serde_json::to_string_pretty(&receipt).unwrap_or_default())
            .unwrap_or_default();
    }

    #[test]
    fn test_compute_p95_empty_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let mut values: Vec<f64> = vec![];
        assert!(compute_p95(&mut values).is_none());
        Ok(())
    }

    #[test]
    fn test_compute_p95_single_value() -> Result<(), Box<dyn std::error::Error>> {
        let mut values = vec![42.0];
        let p95 = compute_p95(&mut values);
        assert!((p95.unwrap_or(0.0) - 42.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_compute_p95_multiple_values() -> Result<(), Box<dyn std::error::Error>> {
        // 20 values: 1..=20. p95 index = ceil(20 * 0.95) - 1 = 19 - 1 = 18 → value 19.
        let mut values: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let p95 = compute_p95(&mut values);
        assert!((p95.unwrap_or(0.0) - 19.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_compute_stability_insufficient_data() -> Result<(), Box<dyn std::error::Error>> {
        // Only 1 receipt — below MIN_STABILITY_RECEIPTS (2).
        let results = vec![UxScenarioResult::Pass];
        let state = compute_stability(&results);
        assert!(
            matches!(state, MetricState::InsufficientData { .. }),
            "expected InsufficientData, got {state:?}"
        );
        Ok(())
    }

    #[test]
    fn test_compute_stability_all_pass() -> Result<(), Box<dyn std::error::Error>> {
        let results = vec![UxScenarioResult::Pass, UxScenarioResult::Pass, UxScenarioResult::Pass];
        let state = compute_stability(&results);
        match state {
            MetricState::Measured { value, sample_count } => {
                assert!((value - 1.0).abs() < 0.001);
                assert_eq!(sample_count, 3);
            }
            _ => return Err("expected Measured".into()),
        }
        Ok(())
    }

    #[test]
    fn test_compute_stability_quarantined_counts_as_unstable()
    -> Result<(), Box<dyn std::error::Error>> {
        let results =
            vec![UxScenarioResult::Pass, UxScenarioResult::Quarantined, UxScenarioResult::Pass];
        let state = compute_stability(&results);
        match state {
            MetricState::Measured { value, sample_count } => {
                // 2 pass out of 3 non-skipped = 0.667
                assert!((value - 2.0 / 3.0).abs() < 0.001);
                assert_eq!(sample_count, 3);
            }
            _ => return Err("expected Measured".into()),
        }
        Ok(())
    }

    #[test]
    fn test_compute_stability_skipped_excluded() -> Result<(), Box<dyn std::error::Error>> {
        let results =
            vec![UxScenarioResult::Pass, UxScenarioResult::Skipped, UxScenarioResult::Pass];
        let state = compute_stability(&results);
        match state {
            MetricState::Measured { value, sample_count } => {
                // 2 pass out of 2 non-skipped = 1.0
                assert!((value - 1.0).abs() < 0.001);
                assert_eq!(sample_count, 2);
            }
            _ => return Err("expected Measured".into()),
        }
        Ok(())
    }

    #[test]
    fn test_aggregate_from_receipts_basic() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        // Create fixture matrix with one workflow.
        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Write 3 passing receipts with timing.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", Some(10.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", Some(20.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_c", "pass", Some(30.0));

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        assert_eq!(scorecard.schema_version, 1);
        assert_eq!(scorecard.subsystem, "editor_ux");

        // Top-line pass rate: 3/3 = 1.0
        assert!((scorecard.top_line.workflow_pass_rate.value - 1.0).abs() < 0.001);

        // Stability: 3 pass out of 3 = 1.0
        assert!((scorecard.top_line.workflow_stability_rate.value - 1.0).abs() < 0.001);

        // p95 timing: p95 of [10, 20, 30] = 30.0
        assert!(scorecard.top_line.p95_time_to_first_useful_result_ms.value > 0.0);

        // One workflow row.
        assert_eq!(scorecard.workflows.len(), 1);
        assert_eq!(scorecard.workflows[0].id, "wf_01");
        assert_eq!(scorecard.workflows[0].subsystem_owner, "editor_intelligence");

        // Provenance.
        assert_eq!(scorecard.provenance.harness, "crates/perl-lsp-ux-tests");

        Ok(())
    }

    #[test]
    fn test_aggregate_pass_rate_excludes_quarantined_and_skipped()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // 2 pass, 1 fail, 1 quarantined, 1 skipped.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_c", "fail", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_d", "quarantined", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_e", "skipped", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // Pass rate: 2 / (2 + 1) = 0.667 (quarantined and skipped excluded).
        assert!(
            (scorecard.top_line.workflow_pass_rate.value - 2.0 / 3.0).abs() < 0.001,
            "expected ~0.667, got {}",
            scorecard.top_line.workflow_pass_rate.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_timing_only_from_passing_scenarios() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Pass with timing, fail with timing (should be excluded).
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", Some(15.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", Some(25.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_c", "fail", Some(999.0));

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // p95 should only include [15, 25], not 999.
        assert!(
            scorecard.top_line.p95_time_to_first_useful_result_ms.value < 100.0,
            "p95 should exclude failing scenario timing, got {}",
            scorecard.top_line.p95_time_to_first_useful_result_ms.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_empty_receipts_dir() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // No receipts → insufficient data.
        assert_eq!(scorecard.top_line.workflow_pass_rate.confidence, "low");
        assert_eq!(scorecard.workflows.len(), 1);
        assert_eq!(scorecard.workflows[0].pass_rate.confidence, "low");

        Ok(())
    }

    #[test]
    fn test_aggregate_nonexistent_receipts_dir() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("nonexistent");

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // No receipts → insufficient data.
        assert_eq!(scorecard.top_line.workflow_pass_rate.confidence, "low");

        Ok(())
    }

    #[test]
    fn test_aggregate_component_metrics() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[
                (
                    "wf_def",
                    "ux_scenario_10.rs",
                    "editor_intelligence",
                    &["cross_file_definition_success_rate"],
                ),
                (
                    "wf_mod",
                    "ux_scenario_14.rs",
                    "module_resolution",
                    &["module_resolution_workflow_success_rate"],
                ),
            ],
        );

        // wf_def: 2 pass, 1 fail → cross_file = 2/3
        write_receipt_file(&receipts_dir, "wf_def", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_def", "test_b", "pass", None);
        write_receipt_file(&receipts_dir, "wf_def", "test_c", "fail", None);

        // wf_mod: 1 pass, 1 fail → module_resolution = 1/2
        write_receipt_file(&receipts_dir, "wf_mod", "test_x", "pass", None);
        write_receipt_file(&receipts_dir, "wf_mod", "test_y", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        assert!(
            (scorecard.components.cross_file_definition_success_rate.value - 2.0 / 3.0).abs()
                < 0.001
        );
        assert!(
            (scorecard.components.module_resolution_workflow_success_rate.value - 0.5).abs()
                < 0.001
        );
        // multi_root has no workflows → insufficient data.
        assert_eq!(
            scorecard.components.multi_root_workspace_navigation_success_rate.confidence,
            "low"
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_scorecard_serializes_to_valid_json() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", Some(10.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", Some(20.0));

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;
        let json = serde_json::to_string_pretty(&scorecard)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;

        // Verify schema-required fields.
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["subsystem"], "editor_ux");
        assert!(parsed["measured_at"].is_string());
        assert!(parsed["top_line"]["workflow_pass_rate"]["value"].is_number());
        assert!(parsed["top_line"]["workflow_stability_rate"]["value"].is_number());
        assert!(parsed["top_line"]["p95_time_to_first_useful_result_ms"]["value"].is_number());
        assert!(parsed["components"]["cross_file_definition_success_rate"]["value"].is_number());
        assert!(parsed["workflows"].is_array());
        assert!(parsed["provenance"]["fixture_matrix"].is_string());
        assert!(parsed["provenance"]["harness"].is_string());
        assert!(parsed["provenance"]["tiers"].is_array());

        Ok(())
    }

    #[test]
    fn test_aggregate_single_receipt_pass_rate_valid_stability_insufficient()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Single passing receipt.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", Some(42.0));

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // Pass rate: 1/1 = 1.0 — valid even with a single receipt.
        assert!(
            (scorecard.workflows[0].pass_rate.value - 1.0).abs() < 0.001,
            "single pass receipt should yield 1.0 pass rate, got {}",
            scorecard.workflows[0].pass_rate.value
        );

        // Stability: only 1 non-skipped receipt, below MIN_STABILITY_RECEIPTS (2)
        // → InsufficientData, represented as confidence "low" with assumptions.
        assert_eq!(
            scorecard.workflows[0].stability_rate.confidence, "low",
            "single receipt should produce low-confidence stability"
        );
        assert!(
            scorecard.workflows[0]
                .stability_rate
                .assumptions
                .as_ref()
                .is_some_and(|a| a.iter().any(|s| s.contains("insufficient"))),
            "stability assumptions should mention insufficient data"
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_all_quarantined_receipts_pass_rate_insufficient()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // All receipts are quarantined — no pass/fail data.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "quarantined", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "quarantined", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_c", "quarantined", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // Pass rate: no pass/fail receipts → InsufficientData.
        assert_eq!(
            scorecard.workflows[0].pass_rate.confidence, "low",
            "all-quarantined should produce low-confidence pass rate"
        );
        assert!(
            scorecard.workflows[0]
                .pass_rate
                .assumptions
                .as_ref()
                .is_some_and(|a| a.iter().any(|s| s.contains("insufficient"))),
            "pass rate assumptions should mention insufficient data"
        );

        // Top-line pass rate should also be insufficient (no eligible receipts).
        assert_eq!(
            scorecard.top_line.workflow_pass_rate.confidence, "low",
            "top-line pass rate should be low confidence when all quarantined"
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_mixed_results_per_workflow_pass_rate()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // 3 pass, 2 fail, 1 quarantined, 1 skipped.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_c", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_d", "fail", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_e", "fail", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_f", "quarantined", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_g", "skipped", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // Per-workflow pass rate: 3 / (3 + 2) = 0.6 (quarantined and skipped excluded).
        assert!(
            (scorecard.workflows[0].pass_rate.value - 0.6).abs() < 0.001,
            "expected per-workflow pass rate 0.6, got {}",
            scorecard.workflows[0].pass_rate.value
        );

        // Top-line pass rate should match (single workflow).
        assert!(
            (scorecard.top_line.workflow_pass_rate.value - 0.6).abs() < 0.001,
            "expected top-line pass rate 0.6, got {}",
            scorecard.top_line.workflow_pass_rate.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_timing_excludes_null_timing_from_passing()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Two passing: one with timing, one without (null timing).
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", Some(50.0));
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // p95 should only include [50.0] — the null-timing pass should not contribute.
        assert!(
            (scorecard.top_line.p95_time_to_first_useful_result_ms.value - 50.0).abs() < 0.001,
            "p95 should be 50.0 (only non-null timing from passing), got {}",
            scorecard.top_line.p95_time_to_first_useful_result_ms.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_zero_receipt_workflow_is_insufficient_not_zero()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // No receipts written — directory is empty.
        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        // Workflow pass rate should be InsufficientData (value 0.0 with assumptions).
        let wf = &scorecard.workflows[0];
        assert_eq!(wf.pass_rate.confidence, "low");
        assert!(
            wf.pass_rate
                .assumptions
                .as_ref()
                .is_some_and(|a| a.iter().any(|s| s.contains("insufficient"))),
            "zero-receipt workflow pass rate should have insufficient data assumption, got {:?}",
            wf.pass_rate.assumptions
        );

        // Stability should also be InsufficientData.
        assert_eq!(wf.stability_rate.confidence, "low");
        assert!(
            wf.stability_rate
                .assumptions
                .as_ref()
                .is_some_and(|a| a.iter().any(|s| s.contains("insufficient"))),
            "zero-receipt workflow stability should have insufficient data assumption, got {:?}",
            wf.stability_rate.assumptions
        );

        // Latency should also be InsufficientData.
        assert_eq!(wf.p95_time_to_first_useful_result_ms.confidence, "low");
        assert!(
            wf.p95_time_to_first_useful_result_ms
                .assumptions
                .as_ref()
                .is_some_and(|a| a.iter().any(|s| s.contains("insufficient"))),
            "zero-receipt workflow latency should have insufficient data assumption, got {:?}",
            wf.p95_time_to_first_useful_result_ms.assumptions
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_with_flake_ledger_quarantines_failures()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Write a flake ledger with one quarantined test.
        let flake_path = tmp.path().join("ux-flakes.json");
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "quarantined_test",
                    "state": "active",
                }
            ],
            "summary": { "total": 1, "active": 1, "resolved": 0 }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // Write receipts: one pass, one fail that should be quarantined.
        write_receipt_file(&receipts_dir, "wf_01", "normal_test", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "quarantined_test", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;

        // Pass rate should be 1/1 = 1.0 (quarantined excluded from pass/fail).
        assert!(
            (scorecard.top_line.workflow_pass_rate.value - 1.0).abs() < 0.001,
            "quarantined failure should not count against pass rate, got {}",
            scorecard.top_line.workflow_pass_rate.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_quarantine_age_computed_from_first_seen()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Write a flake ledger with first_seen 10 days ago.
        let flake_path = tmp.path().join("ux-flakes.json");
        let first_seen =
            (chrono::Utc::now() - chrono::Duration::days(10)).format("%Y-%m-%d").to_string();
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "flaky_test",
                    "state": "active",
                    "first_seen": first_seen,
                }
            ],
            "summary": { "total": 1, "active": 1, "resolved": 0 }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // Write a failing receipt that matches the quarantined test.
        write_receipt_file(&receipts_dir, "wf_01", "normal_test", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_test", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;

        // Quarantine age should be approximately 10 days.
        let wf = &scorecard.workflows[0];
        let age = wf.quarantine_age_days;
        assert!(age.is_some(), "quarantine_age_days should be present for quarantined workflow");
        let age_val = age.unwrap_or(0);
        assert!((9..=11).contains(&age_val), "quarantine age should be ~10 days, got {age_val}");

        Ok(())
    }

    #[test]
    fn test_aggregate_quarantine_age_none_for_non_quarantined()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // All passing — no quarantine.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, None)?;

        let wf = &scorecard.workflows[0];
        assert!(
            wf.quarantine_age_days.is_none(),
            "quarantine_age_days should be None for non-quarantined workflow, got {:?}",
            wf.quarantine_age_days
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_quarantine_age_uses_max_across_tests()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Two quarantined tests with different first_seen dates.
        let flake_path = tmp.path().join("ux-flakes.json");
        let recent =
            (chrono::Utc::now() - chrono::Duration::days(5)).format("%Y-%m-%d").to_string();
        let older =
            (chrono::Utc::now() - chrono::Duration::days(20)).format("%Y-%m-%d").to_string();
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "flaky_recent",
                    "state": "active",
                    "first_seen": recent,
                },
                {
                    "test": "flaky_older",
                    "state": "active",
                    "first_seen": older,
                }
            ],
            "summary": { "total": 2, "active": 2, "resolved": 0 }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // Both fail and get quarantined.
        write_receipt_file(&receipts_dir, "wf_01", "normal_test", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_recent", "fail", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_older", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;

        // Should use the max age (20 days, not 5).
        let wf = &scorecard.workflows[0];
        let age = wf.quarantine_age_days;
        assert!(age.is_some(), "quarantine_age_days should be present");
        let age_val = age.unwrap_or(0);
        assert!(
            (19..=21).contains(&age_val),
            "quarantine age should be ~20 days (max), got {age_val}"
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_flake_ledger_quarantined_counts_against_stability()
    -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Write a flake ledger with one quarantined test.
        let flake_path = tmp.path().join("ux-flakes.json");
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "flaky_test",
                    "state": "active",
                    "first_seen": "2026-01-01",
                }
            ],
            "summary": { "total": 1, "active": 1, "resolved": 0 }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // 2 pass + 1 fail (quarantined via ledger) = stability 2/3.
        write_receipt_file(&receipts_dir, "wf_01", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "test_b", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_test", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;

        // Pass rate: 2/2 = 1.0 (quarantined excluded from pass/fail denominator).
        assert!(
            (scorecard.workflows[0].pass_rate.value - 1.0).abs() < 0.001,
            "pass rate should be 1.0 (quarantined excluded), got {}",
            scorecard.workflows[0].pass_rate.value
        );

        // Stability: 2 pass out of 3 non-skipped (quarantined counts as unstable) = 0.667.
        assert!(
            (scorecard.workflows[0].stability_rate.value - 2.0 / 3.0).abs() < 0.001,
            "stability should be ~0.667 (quarantined counts as unstable), got {}",
            scorecard.workflows[0].stability_rate.value
        );

        Ok(())
    }

    #[test]
    fn test_aggregate_quarantine_age_serializes_in_json() -> Result<(), Box<dyn std::error::Error>>
    {
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[
                ("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[]),
                ("wf_02", "ux_scenario_02.rs", "editor_intelligence", &[]),
            ],
        );

        // Quarantine one test in wf_01.
        let flake_path = tmp.path().join("ux-flakes.json");
        let first_seen =
            (chrono::Utc::now() - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "flaky_test",
                    "state": "active",
                    "first_seen": first_seen,
                }
            ],
            "summary": { "total": 1, "active": 1, "resolved": 0 }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // wf_01: one pass, one quarantined fail.
        write_receipt_file(&receipts_dir, "wf_01", "normal_test", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_test", "fail", None);
        // wf_02: all pass, no quarantine.
        write_receipt_file(&receipts_dir, "wf_02", "test_a", "pass", None);
        write_receipt_file(&receipts_dir, "wf_02", "test_b", "pass", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;
        let json = serde_json::to_string_pretty(&scorecard)?;
        let parsed: serde_json::Value = serde_json::from_str(&json)?;

        // wf_01 should have quarantine_age_days.
        let wf_01 = &parsed["workflows"][0];
        assert!(
            wf_01["quarantine_age_days"].is_number(),
            "wf_01 should have quarantine_age_days, got {:?}",
            wf_01["quarantine_age_days"]
        );

        // wf_02 should NOT have quarantine_age_days (skip_serializing_if = None).
        let wf_02 = &parsed["workflows"][1];
        assert!(
            wf_02.get("quarantine_age_days").is_none(),
            "wf_02 should not have quarantine_age_days, got {:?}",
            wf_02.get("quarantine_age_days")
        );

        Ok(())
    }

    // ── Flake ledger tests (Task 11.3) ───────────────────────────────────

    #[test]
    fn test_flake_ledger_summary_consistency_with_committed_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = crate::utils::project_root()?;
        let flake_path = root.join(".ci").join("ux-flakes.json");
        let raw = fs::read_to_string(&flake_path)?;
        let ledger: FlakeLedger = serde_json::from_str(&raw)?;

        validate_flake_ledger_summary(&ledger)?;

        // Additionally verify the counts are non-negative and consistent.
        let summary = ledger.summary.as_ref().ok_or("missing summary")?;
        assert_eq!(
            summary.total,
            summary.active + summary.resolved,
            "total ({}) should equal active ({}) + resolved ({})",
            summary.total,
            summary.active,
            summary.resolved
        );

        // Verify by_subsystem values sum to total.
        let subsystem_sum: usize = summary.by_subsystem.values().sum();
        assert_eq!(
            subsystem_sum, summary.total,
            "by_subsystem sum ({subsystem_sum}) should equal total ({})",
            summary.total
        );

        Ok(())
    }

    #[test]
    fn test_flake_ledger_summary_consistency_synthetic() -> Result<(), Box<dyn std::error::Error>> {
        // Construct a ledger with mixed active/resolved entries and verify
        // the validation function catches correct and incorrect summaries.
        let ledger_json = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "test_a",
                    "state": "active",
                    "subsystem": "editor_ux",
                    "first_seen": "2026-05-01",
                    "owner": "@dev",
                    "issue": 100,
                    "quarantine_effect": "non_blocking_pr",
                },
                {
                    "test": "test_b",
                    "state": "resolved",
                    "subsystem": "editor_ux",
                    "first_seen": "2026-04-01",
                },
                {
                    "test": "test_c",
                    "state": "active",
                    "subsystem": "module_resolution",
                    "first_seen": "2026-05-10",
                    "owner": "@dev2",
                    "issue": 200,
                    "quarantine_effect": "release_blocking",
                },
            ],
            "summary": {
                "total": 3,
                "active": 2,
                "resolved": 1,
                "by_subsystem": {
                    "editor_ux": 2,
                    "module_resolution": 1,
                }
            }
        });
        let ledger: FlakeLedger = serde_json::from_value(ledger_json)?;
        validate_flake_ledger_summary(&ledger)?;

        Ok(())
    }

    #[test]
    fn test_flake_ledger_summary_mismatch_detected() -> Result<(), Box<dyn std::error::Error>> {
        // Summary says total=5 but only 2 entries — should fail validation.
        let ledger_json = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "test_a",
                    "state": "active",
                    "subsystem": "editor_ux",
                    "first_seen": "2026-05-01",
                },
                {
                    "test": "test_b",
                    "state": "active",
                    "subsystem": "editor_ux",
                    "first_seen": "2026-05-02",
                },
            ],
            "summary": {
                "total": 5,
                "active": 2,
                "resolved": 0,
                "by_subsystem": { "editor_ux": 2 }
            }
        });
        let ledger: FlakeLedger = serde_json::from_value(ledger_json)?;
        let result = validate_flake_ledger_summary(&ledger);
        assert!(result.is_err(), "mismatched total should fail validation");

        Ok(())
    }

    #[test]
    fn test_flake_ledger_schema_requires_owner_and_issue_for_active()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = crate::utils::project_root()?;
        let schema_path = root.join(".ci").join("schemas").join("ux-flakes.schema.json");
        let schema_raw = fs::read_to_string(&schema_path)?;
        let schema_value: serde_json::Value = serde_json::from_str(&schema_raw)?;

        // Verify the schema has an if/then conditional requiring owner, issue,
        // and failure_class for active entries.
        let entry_def = &schema_value["$defs"]["flakeEntry"];
        let if_clause = &entry_def["if"];
        let then_clause = &entry_def["then"];

        // The if clause should check for state == "active".
        assert_eq!(
            if_clause["properties"]["state"]["const"], "active",
            "schema if-clause should check for state=active"
        );

        // The then clause should require owner, issue, and failure_class.
        let then_required =
            then_clause["required"].as_array().ok_or("then.required should be an array")?;
        let required_fields: Vec<&str> = then_required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            required_fields.contains(&"owner"),
            "schema should require 'owner' for active entries, got {required_fields:?}"
        );
        assert!(
            required_fields.contains(&"issue"),
            "schema should require 'issue' for active entries, got {required_fields:?}"
        );
        assert!(
            required_fields.contains(&"failure_class"),
            "schema should require 'failure_class' for active entries, got {required_fields:?}"
        );

        // The then clause should narrow owner to non-nullable string.
        let then_owner_type = &then_clause["properties"]["owner"]["type"];
        assert_eq!(
            then_owner_type, "string",
            "active entry owner should be narrowed to string (non-nullable)"
        );

        // The then clause should narrow issue to non-nullable integer.
        let then_issue_type = &then_clause["properties"]["issue"]["type"];
        assert_eq!(
            then_issue_type, "integer",
            "active entry issue should be narrowed to integer (non-nullable)"
        );

        Ok(())
    }

    #[test]
    fn test_flake_ledger_committed_active_entries_have_owner_and_issue()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = crate::utils::project_root()?;
        let flake_path = root.join(".ci").join("ux-flakes.json");
        let raw = fs::read_to_string(&flake_path)?;
        let ledger: FlakeLedger = serde_json::from_str(&raw)?;

        for entry in &ledger.entries {
            if entry.state == "active" {
                assert!(entry.owner.is_some(), "active entry '{}' must have an owner", entry.test);
                assert!(
                    entry.issue.is_some(),
                    "active entry '{}' must have an issue number",
                    entry.test
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_quarantine_effect_non_blocking_pr_does_not_block_pr()
    -> Result<(), Box<dyn std::error::Error>> {
        let entry = FlakeEntry {
            test: "flaky_test".to_owned(),
            state: "active".to_owned(),
            first_seen: Some("2026-05-01".to_owned()),
            subsystem: Some("editor_ux".to_owned()),
            owner: Some("@dev".to_owned()),
            issue: Some(7570),
            quarantine_effect: Some("non_blocking_pr".to_owned()),
        };

        assert!(!quarantine_blocks_pr(&entry), "non_blocking_pr should not block PR gate");
        assert!(
            !quarantine_blocks_release(&entry),
            "non_blocking_pr should not block release gate"
        );

        Ok(())
    }

    #[test]
    fn test_quarantine_effect_release_blocking_blocks_release()
    -> Result<(), Box<dyn std::error::Error>> {
        let entry = FlakeEntry {
            test: "critical_flake".to_owned(),
            state: "active".to_owned(),
            first_seen: Some("2026-05-01".to_owned()),
            subsystem: Some("editor_ux".to_owned()),
            owner: Some("@dev".to_owned()),
            issue: Some(8000),
            quarantine_effect: Some("release_blocking".to_owned()),
        };

        assert!(
            !quarantine_blocks_pr(&entry),
            "release_blocking should not block PR gate (quarantine is PR-non-blocking)"
        );
        assert!(quarantine_blocks_release(&entry), "release_blocking should block release gate");

        Ok(())
    }

    #[test]
    fn test_quarantine_effect_advisory_blocks_neither_gate()
    -> Result<(), Box<dyn std::error::Error>> {
        let entry = FlakeEntry {
            test: "advisory_flake".to_owned(),
            state: "active".to_owned(),
            first_seen: Some("2026-05-01".to_owned()),
            subsystem: Some("editor_ux".to_owned()),
            owner: Some("@dev".to_owned()),
            issue: Some(9000),
            quarantine_effect: Some("advisory".to_owned()),
        };

        assert!(!quarantine_blocks_pr(&entry), "advisory should not block PR gate");
        assert!(!quarantine_blocks_release(&entry), "advisory should not block release gate");

        Ok(())
    }

    #[test]
    fn test_quarantine_effect_non_blocking_pr_aggregator_excludes_from_pass_rate()
    -> Result<(), Box<dyn std::error::Error>> {
        // Verify that a non_blocking_pr quarantined test that fails is
        // reclassified as quarantined and excluded from pass rate.
        let tmp = tempfile::tempdir()?;
        let receipts_dir = tmp.path().join("receipts");
        fs::create_dir_all(&receipts_dir)?;

        let matrix_path = write_fixture_matrix(
            tmp.path(),
            &[("wf_01", "ux_scenario_01.rs", "editor_intelligence", &[])],
        );

        // Write a flake ledger with a non_blocking_pr entry.
        let flake_path = tmp.path().join("ux-flakes.json");
        let flake_ledger = serde_json::json!({
            "schema_version": 1,
            "entries": [
                {
                    "test": "flaky_test",
                    "state": "active",
                    "first_seen": "2026-05-01",
                    "quarantine_effect": "non_blocking_pr",
                }
            ],
            "summary": { "total": 1, "active": 1, "resolved": 0, "by_subsystem": {} }
        });
        fs::write(&flake_path, serde_json::to_string_pretty(&flake_ledger)?)?;

        // One pass, one fail that matches the quarantined test.
        write_receipt_file(&receipts_dir, "wf_01", "normal_test", "pass", None);
        write_receipt_file(&receipts_dir, "wf_01", "flaky_test", "fail", None);

        let scorecard = aggregate_from_receipts(&receipts_dir, &matrix_path, Some(&flake_path))?;

        // Pass rate: 1/1 = 1.0 — the quarantined failure is excluded.
        assert!(
            (scorecard.top_line.workflow_pass_rate.value - 1.0).abs() < 0.001,
            "non_blocking_pr quarantined failure should not affect pass rate, got {}",
            scorecard.top_line.workflow_pass_rate.value
        );

        // Stability: quarantined counts as unstable → 1 pass out of 2 = 0.5.
        assert!(
            (scorecard.workflows[0].stability_rate.value - 0.5).abs() < 0.001,
            "quarantined failure should count against stability, got {}",
            scorecard.workflows[0].stability_rate.value
        );

        Ok(())
    }

    // ── scorecard_to_floor_metrics tests (Task 13.1) ─────────────────────

    /// Helper: build a minimal `MeasuredEditorUxScorecard` for floor-metric
    /// extraction tests.
    fn make_scorecard_for_ratchet(
        pass_rate: f64,
        stability_rate: f64,
        p95_latency: f64,
        cross_file_rate: f64,
    ) -> MeasuredEditorUxScorecard {
        MeasuredEditorUxScorecard {
            schema_version: 1,
            measured_at: "2026-06-01T00:00:00Z".to_owned(),
            subsystem: "editor_ux".to_owned(),
            top_line: TopLineMetrics {
                workflow_pass_rate: measured_rate(pass_rate, 10),
                workflow_stability_rate: measured_rate(stability_rate, 10),
                p95_time_to_first_useful_result_ms: measured_latency(p95_latency, 10),
            },
            components: ComponentMetrics {
                cross_file_definition_success_rate: measured_rate(cross_file_rate, 5),
                module_resolution_workflow_success_rate: insufficient_rate("no data"),
                multi_root_workspace_navigation_success_rate: insufficient_rate("no data"),
            },
            workflows: vec![],
            provenance: ScorecardProvenance {
                fixture_matrix: "test".to_owned(),
                harness: "test".to_owned(),
                tiers: vec!["pr".to_owned()],
                notes: None,
            },
        }
    }

    #[test]
    fn test_scorecard_to_floor_metrics_extracts_top_line_rates()
    -> Result<(), Box<dyn std::error::Error>> {
        let sc = make_scorecard_for_ratchet(0.95, 0.85, 42.0, 0.90);
        let floor = scorecard_to_floor_metrics(&sc);

        // workflow_pass_rate_pct: 0.95 → 95.0
        let wpr = floor.get("workflow_pass_rate_pct").and_then(|v| *v);
        assert!(
            (wpr.ok_or("missing workflow_pass_rate_pct")? - 95.0).abs() < 0.01,
            "expected 95.0, got {wpr:?}"
        );

        // workflow_stability_rate_pct: 0.85 → 85.0
        let wsr = floor.get("workflow_stability_rate_pct").and_then(|v| *v);
        assert!(
            (wsr.ok_or("missing workflow_stability_rate_pct")? - 85.0).abs() < 0.01,
            "expected 85.0, got {wsr:?}"
        );

        Ok(())
    }

    #[test]
    fn test_scorecard_to_floor_metrics_extracts_latency() -> Result<(), Box<dyn std::error::Error>>
    {
        let sc = make_scorecard_for_ratchet(1.0, 1.0, 42.5, 1.0);
        let floor = scorecard_to_floor_metrics(&sc);

        let p95 = floor.get("p95_time_to_first_useful_result_ms").and_then(|v| *v);
        assert!((p95.ok_or("missing p95")? - 42.5).abs() < 0.01, "expected 42.5, got {p95:?}");

        Ok(())
    }

    #[test]
    fn test_scorecard_to_floor_metrics_extracts_component_rates()
    -> Result<(), Box<dyn std::error::Error>> {
        let sc = make_scorecard_for_ratchet(1.0, 1.0, 10.0, 0.80);
        let floor = scorecard_to_floor_metrics(&sc);

        // cross_file_success_pct: 0.80 → 80.0
        let cf = floor.get("cross_file_success_pct").and_then(|v| *v);
        assert!(
            (cf.ok_or("missing cross_file_success_pct")? - 80.0).abs() < 0.01,
            "expected 80.0, got {cf:?}"
        );

        Ok(())
    }

    #[test]
    fn test_scorecard_to_floor_metrics_insufficient_data_maps_to_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // Build a scorecard where all metrics are insufficient data.
        let sc = MeasuredEditorUxScorecard {
            schema_version: 1,
            measured_at: "2026-06-01T00:00:00Z".to_owned(),
            subsystem: "editor_ux".to_owned(),
            top_line: TopLineMetrics {
                workflow_pass_rate: insufficient_rate("no data"),
                workflow_stability_rate: insufficient_rate("no data"),
                p95_time_to_first_useful_result_ms: insufficient_latency("no data"),
            },
            components: ComponentMetrics {
                cross_file_definition_success_rate: insufficient_rate("no data"),
                module_resolution_workflow_success_rate: insufficient_rate("no data"),
                multi_root_workspace_navigation_success_rate: insufficient_rate("no data"),
            },
            workflows: vec![],
            provenance: ScorecardProvenance {
                fixture_matrix: "test".to_owned(),
                harness: "test".to_owned(),
                tiers: vec![],
                notes: None,
            },
        };

        let floor = scorecard_to_floor_metrics(&sc);

        // All values should be None (insufficient data).
        assert_eq!(
            floor.get("workflow_pass_rate_pct").and_then(|v| *v),
            None,
            "insufficient workflow_pass_rate should map to None"
        );
        assert_eq!(
            floor.get("workflow_stability_rate_pct").and_then(|v| *v),
            None,
            "insufficient workflow_stability_rate should map to None"
        );
        assert_eq!(
            floor.get("p95_time_to_first_useful_result_ms").and_then(|v| *v),
            None,
            "insufficient p95 latency should map to None"
        );
        assert_eq!(
            floor.get("cross_file_success_pct").and_then(|v| *v),
            None,
            "insufficient cross_file rate should map to None"
        );

        Ok(())
    }

    #[test]
    fn test_scorecard_to_floor_metrics_per_workflow_component_drill_down()
    -> Result<(), Box<dyn std::error::Error>> {
        // Build a scorecard with workflow rows that map to component keys.
        let mut sc = make_scorecard_for_ratchet(1.0, 1.0, 10.0, 1.0);
        sc.workflows = vec![
            WorkflowResult {
                id: "hover_core".to_owned(),
                scenario: "ux_scenario_11_hover.rs".to_owned(),
                subsystem_owner: "editor_intelligence".to_owned(),
                pass_rate: measured_rate(0.80, 5),
                stability_rate: measured_rate(0.80, 5),
                p95_time_to_first_useful_result_ms: measured_latency(10.0, 5),
                component_metrics: None,
                quarantine_age_days: None,
            },
            WorkflowResult {
                id: "goto_definition_core".to_owned(),
                scenario: "ux_scenario_10_goto_definition.rs".to_owned(),
                subsystem_owner: "editor_intelligence".to_owned(),
                pass_rate: measured_rate(1.0, 4),
                stability_rate: measured_rate(1.0, 4),
                p95_time_to_first_useful_result_ms: measured_latency(15.0, 4),
                component_metrics: None,
                quarantine_age_days: None,
            },
        ];

        let floor = scorecard_to_floor_metrics(&sc);

        // hover_correctness_pct: 0.80 * 5 = 4 pass out of 5 → 80.0%
        let hover = floor.get("hover_correctness_pct").and_then(|v| *v);
        assert!(
            (hover.ok_or("missing hover_correctness_pct")? - 80.0).abs() < 0.01,
            "expected 80.0, got {hover:?}"
        );

        // definition_exact_hit_pct: 1.0 * 4 = 4 pass out of 4 → 100.0%
        let def = floor.get("definition_exact_hit_pct").and_then(|v| *v);
        assert!(
            (def.ok_or("missing definition_exact_hit_pct")? - 100.0).abs() < 0.01,
            "expected 100.0, got {def:?}"
        );

        Ok(())
    }

    #[test]
    fn test_scorecard_to_floor_metrics_skips_low_confidence_workflows()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = make_scorecard_for_ratchet(1.0, 1.0, 10.0, 1.0);
        sc.workflows = vec![WorkflowResult {
            id: "hover_core".to_owned(),
            scenario: "ux_scenario_11_hover.rs".to_owned(),
            subsystem_owner: "editor_intelligence".to_owned(),
            pass_rate: insufficient_rate("no receipts"),
            stability_rate: insufficient_rate("no receipts"),
            p95_time_to_first_useful_result_ms: insufficient_latency("no receipts"),
            component_metrics: None,
            quarantine_age_days: None,
        }];

        let floor = scorecard_to_floor_metrics(&sc);

        // hover_correctness_pct should not be present (low confidence workflow skipped).
        assert!(
            !floor.contains_key("hover_correctness_pct")
                || floor.get("hover_correctness_pct").and_then(|v| *v).is_none(),
            "low-confidence workflow should not contribute to component metrics"
        );

        Ok(())
    }
}

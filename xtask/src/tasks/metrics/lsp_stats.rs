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
use serde::{Deserialize, Serialize};
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

/// Run `cargo xtask metrics lsp-stats`.
///
/// Pass `json = true` to write `.ci/metrics/editor_ux.json`.
pub fn run_with_json(json: bool) -> Result<()> {
    let root = project_root()?;
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

fn write_json_receipt(path: &Path, output: &EditorUxMetrics) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(output)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
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
}

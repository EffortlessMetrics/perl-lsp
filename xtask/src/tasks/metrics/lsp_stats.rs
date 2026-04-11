//! LSP editor-intelligence scorecard subcommand.
//!
//! Reports fixture inventory and (optionally) reads the most-recently
//! produced `editor_intelligence.json` receipt.  The actual correctness
//! measurements are produced by the integration-test suite in
//! `crates/perl-lsp/tests/editor_intelligence_scorecard.rs`; this command
//! surfaces those numbers in a human-readable table.
//!
//! ## Usage
//!
//! ```bash
//! # Print fixture inventory
//! cargo xtask metrics lsp-stats
//!
//! # Write receipt to .ci/metrics/editor_intelligence.json
//! cargo xtask metrics lsp-stats --json
//! ```

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
// Output schema for .ci/metrics/editor_intelligence.json
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct EditorIntelligenceMetrics {
    pub schema_version: u32,
    pub measured_at: String,
    pub subsystem: &'static str,
    pub fixture_counts: FixtureCounts,
    /// Pass-rate data loaded from the previous test run receipt, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run: Option<LastRunMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureCounts {
    pub hover: usize,
    pub goto: usize,
    pub completion: usize,
    pub total_assertions: AssertionCounts,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssertionCounts {
    pub hover: usize,
    pub goto: usize,
    pub completion: usize,
}

/// Pass-rate data written by the integration test suite.
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
    fn hover_rate(&self) -> f64 {
        if self.hover_total == 0 {
            100.0
        } else {
            self.hover_passed as f64 / self.hover_total as f64 * 100.0
        }
    }
    fn goto_rate(&self) -> f64 {
        if self.goto_total == 0 {
            100.0
        } else {
            self.goto_passed as f64 / self.goto_total as f64 * 100.0
        }
    }
    fn completion_rate(&self) -> f64 {
        if self.completion_total == 0 {
            100.0
        } else {
            self.completion_passed as f64 / self.completion_total as f64 * 100.0
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run `cargo xtask metrics lsp-stats`.
///
/// Pass `json = true` to write `.ci/metrics/editor_intelligence.json`.
pub fn run_with_json(json: bool) -> Result<()> {
    let root = project_root()?;
    let gold_root = root.join("test_corpus").join("gold");

    // Count fixtures and assertions
    let hover_fixtures = load_hover_gold_fixtures(&gold_root).unwrap_or_default();
    let goto_fixtures = load_goto_gold_fixtures(&gold_root).unwrap_or_default();
    let completion_fixtures = load_completion_gold_fixtures(&gold_root).unwrap_or_default();

    let hover_assertions: usize = hover_fixtures.iter().map(|f| f.hover_assertions.len()).sum();
    let goto_assertions: usize = goto_fixtures.iter().map(|f| f.goto_assertions.len()).sum();
    let completion_assertions: usize =
        completion_fixtures.iter().map(|f| f.completion_assertions.len()).sum();

    let counts = FixtureCounts {
        hover: hover_fixtures.len(),
        goto: goto_fixtures.len(),
        completion: completion_fixtures.len(),
        total_assertions: AssertionCounts {
            hover: hover_assertions,
            goto: goto_assertions,
            completion: completion_assertions,
        },
    };

    // Try to load the last run receipt
    let metrics_path = root.join(".ci").join("metrics").join("editor_intelligence.json");
    let last_run = load_last_run(&metrics_path);

    print_table(&counts, last_run.as_ref());

    if json {
        let output = EditorIntelligenceMetrics {
            schema_version: 1,
            measured_at: Utc::now().to_rfc3339(),
            subsystem: "editor_intelligence",
            fixture_counts: counts,
            last_run,
        };
        write_json_receipt(&metrics_path, &output)
            .with_context(|| format!("writing receipt to {}", metrics_path.display()))?;
        println!("\nWrote receipt: {}", metrics_path.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_last_run(path: &Path) -> Option<LastRunMetrics> {
    let raw = fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let last = doc.get("last_run")?;
    serde_json::from_value(last.clone()).ok()
}

fn write_json_receipt(path: &Path, output: &EditorIntelligenceMetrics) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(output)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn print_table(counts: &FixtureCounts, last_run: Option<&LastRunMetrics>) {
    println!("\nEditor Intelligence Scorecard");
    println!("{}", "=".repeat(60));
    println!("{:<20} {:>10} {:>12}", "Kind", "Fixtures", "Assertions");
    println!("{}", "-".repeat(44));
    println!("{:<20} {:>10} {:>12}", "Hover", counts.hover, counts.total_assertions.hover);
    println!("{:<20} {:>10} {:>12}", "Goto-Definition", counts.goto, counts.total_assertions.goto);
    println!(
        "{:<20} {:>10} {:>12}",
        "Completion", counts.completion, counts.total_assertions.completion
    );
    println!("{}", "-".repeat(44));
    let total_fixtures = counts.hover + counts.goto + counts.completion;
    let total_assertions = counts.total_assertions.hover
        + counts.total_assertions.goto
        + counts.total_assertions.completion;
    println!("{:<20} {:>10} {:>12}", "TOTAL", total_fixtures, total_assertions);

    if let Some(run) = last_run {
        println!("\nLast Run Results");
        println!("{}", "=".repeat(60));
        println!("{:<20} {:>8} {:>8} {:>8}", "Kind", "Passed", "Total", "Rate");
        println!("{}", "-".repeat(48));
        println!(
            "{:<20} {:>8} {:>8} {:>7.1}%",
            "Hover",
            run.hover_passed,
            run.hover_total,
            run.hover_rate()
        );
        println!(
            "{:<20} {:>8} {:>8} {:>7.1}%",
            "Goto-Definition",
            run.goto_passed,
            run.goto_total,
            run.goto_rate()
        );
        println!(
            "{:<20} {:>8} {:>8} {:>7.1}%",
            "Completion",
            run.completion_passed,
            run.completion_total,
            run.completion_rate()
        );
        let total_passed = run.hover_passed + run.goto_passed + run.completion_passed;
        let total_total = run.hover_total + run.goto_total + run.completion_total;
        let total_rate =
            if total_total == 0 { 100.0 } else { total_passed as f64 / total_total as f64 * 100.0 };
        println!("{}", "-".repeat(48));
        println!("{:<20} {:>8} {:>8} {:>7.1}%", "TOTAL", total_passed, total_total, total_rate);
    } else {
        println!("\n(No last-run receipt found — run the integration tests first)");
        println!(
            "  RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard -- --nocapture"
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_run_metrics_rates_zero_total() {
        let m = LastRunMetrics {
            hover_passed: 0,
            hover_total: 0,
            goto_passed: 0,
            goto_total: 0,
            completion_passed: 0,
            completion_total: 0,
        };
        // Zero total must not panic and must report 100%
        assert!((m.hover_rate() - 100.0).abs() < 0.001);
        assert!((m.goto_rate() - 100.0).abs() < 0.001);
        assert!((m.completion_rate() - 100.0).abs() < 0.001);
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
        assert!((m.hover_rate() - 80.0).abs() < 0.001);
        assert!((m.goto_rate() - 100.0).abs() < 0.001);
        assert!((m.completion_rate() - 75.0).abs() < 0.001);
    }
}

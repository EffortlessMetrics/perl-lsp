//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::Result;
use regex::Regex;
use serde::Deserialize;

use super::replace_block;
use super::token::TokenHealthMetrics;

mod accuracy;
mod failure;
mod render;

use accuracy::{ParserAccuracyArtifactSummary, read_parser_accuracy_artifact};

pub(super) fn generate_parser_status(metrics: &ParserMetrics, original: &str) -> Result<String> {
    render::generate_parser_status(metrics, original)
}

fn parser_marker_bounds(marker_name: &str) -> (String, String) {
    (format!("<!-- BEGIN: {marker_name} -->"), format!("<!-- END: {marker_name} -->"))
}

fn replace_parser_status_block(text: &str, marker_name: &str, new_content: &str) -> Result<String> {
    let (begin_marker, end_marker) = parser_marker_bounds(marker_name);
    replace_block(text, &begin_marker, &end_marker, new_content)
}

// ---------------------------------------------------------------------------
// Parser metrics struct
// ---------------------------------------------------------------------------

pub(super) struct ParserMetrics {
    pub syntax_sections: usize,
    pub system_receipt: Option<ParserSweepReceipt>,
    pub cpan_receipt: Option<ParserSweepReceipt>,
    pub project_corpus: Option<super::super::corpus_audit::StatusSummary>,
    /// Receipt from `just common-corpus-check` — the strict-clean pinned-module gate.
    pub common_corpus_receipt: Option<ParserSweepReceipt>,
    /// Number of pinned modules in `.ci/common-corpus-manifest.txt`.
    pub common_corpus_pinned: usize,
    pub performance_scorecard: Option<ParserPerformanceScorecard>,
    parser_accuracy: Option<ParserAccuracyArtifactSummary>,
    pub token_metrics: TokenHealthMetrics,
}

#[derive(Debug, Clone)]
pub(super) struct ParserSweepReceipt {
    report: super::super::parser_corpus_sweep::SweepReport,
    has_recovery_shape: bool,
}

impl ParserSweepReceipt {
    #[cfg(test)]
    fn with_recovery_shape(report: super::super::parser_corpus_sweep::SweepReport) -> Self {
        Self { report, has_recovery_shape: true }
    }

    #[cfg(test)]
    fn without_recovery_shape(report: super::super::parser_corpus_sweep::SweepReport) -> Self {
        Self { report, has_recovery_shape: false }
    }
}

impl Deref for ParserSweepReceipt {
    type Target = super::super::parser_corpus_sweep::SweepReport;

    fn deref(&self) -> &Self::Target {
        &self.report
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParserPerformanceScorecard {
    generated_at_epoch_s: u64,
    metrics: std::collections::BTreeMap<String, ParserPerfMetric>,
}

#[derive(Debug, Clone, Deserialize)]
struct ParserPerfMetric {
    iterations: usize,
    median_ns: u128,
    p95_ns: u128,
    mean_ns: u128,
}

pub(super) fn collect_parser_metrics(root: &Path) -> ParserMetrics {
    let common_corpus_receipt =
        read_sweep_report(&root.join("target/receipts/common-corpus-sweep.json"));
    let common_corpus_pinned = count_common_corpus_pinned(root);
    ParserMetrics {
        syntax_sections: count_corpus_sections(root),
        system_receipt: read_sweep_report(&root.join(".ci/parser-corpus-baseline.json")),
        cpan_receipt: read_sweep_report(&root.join(".ci/cpan-corpus-baseline.json")),
        project_corpus: super::super::corpus_audit::compute_status_summary(
            root,
            Duration::from_secs(5),
        )
        .ok(),
        common_corpus_receipt,
        common_corpus_pinned,
        performance_scorecard: read_parser_performance_scorecard(root),
        parser_accuracy: read_parser_accuracy_artifact(root),
        token_metrics: super::token::collect_token_health_metrics(root),
    }
}

/// Count the non-comment, non-blank lines in `.ci/common-corpus-manifest.txt`.
pub(super) fn count_common_corpus_pinned(root: &Path) -> usize {
    let path = root.join(".ci/common-corpus-manifest.txt");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    raw.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#')).count()
}

pub(super) fn read_sweep_report(path: &Path) -> Option<ParserSweepReceipt> {
    let raw = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let has_recovery_shape = value.get("files_with_structured_recovery_only").is_some()
        && value.get("files_with_error_nodes").is_some()
        && value.get("files_with_catastrophic_parse_failure").is_some()
        && value.get("total_dirty_files").is_some();
    let report = serde_json::from_value(value).ok()?;
    Some(ParserSweepReceipt { report, has_recovery_shape })
}

fn read_parser_performance_scorecard(root: &Path) -> Option<ParserPerformanceScorecard> {
    let path = root.join("docs/project/status/parser_performance_scorecard.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub(super) fn count_corpus_sections(root: &Path) -> usize {
    let corpus_dir = root.join("tree-sitter-perl/test/corpus");
    let marker = Regex::new(r"^=+\s*$").ok();
    let mut total: usize = 0;

    let walker =
        walkdir::WalkDir::new(&corpus_dir).into_iter().filter_map(|e| e.ok()).filter(|e| {
            e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "txt")
        });

    for entry in walker {
        if let Ok(content) = fs::read_to_string(entry.path())
            && let Some(ref re) = marker
        {
            total += content.lines().filter(|line| re.is_match(line)).count();
        }
    }
    total
}

fn format_clean_rate(clean_files: usize, total_files: usize) -> String {
    let clean_pct = 100.0 * clean_files as f64 / total_files.max(1) as f64;
    format!("{clean_pct:.1}% clean (`{clean_files}/{total_files}`)")
}

fn format_salvage_rate(salvage_rate: Option<f64>) -> String {
    match salvage_rate {
        Some(rate) => format!("{:.1}% salvage", rate * 100.0),
        None => "insufficient_data salvage".to_string(),
    }
}

fn format_recovery_shape_note(receipt: &ParserSweepReceipt) -> String {
    if receipt.has_recovery_shape {
        format!(
            "`{}` unreadable, `{}` recovery-only, `{}` ERROR-node files, `{}` catastrophic",
            receipt.files_unreadable,
            receipt.files_with_structured_recovery_only,
            receipt.files_with_error_nodes,
            receipt.files_with_catastrophic_parse_failure,
        )
    } else {
        format!(
            "`{}` unreadable, `insufficient_data` recovery-only, `insufficient_data` ERROR-node files, `insufficient_data` catastrophic",
            receipt.files_unreadable,
        )
    }
}

fn short_day(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}

fn format_failure_receipt_note(receipt: &ParserSweepReceipt) -> String {
    format!(
        "Receipt snapshot: profile `{}`, commit `{}`, generated `{}`, Perl `{}`, `{}` resolved roots. Raw bucket counts are point-in-time compatibility data; before starting a parser-fix lane from a bucket, rerun `cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt` on Linux or add a focused fixture when system roots are unavailable.",
        receipt.corpus_profile,
        receipt.commit,
        short_day(&receipt.timestamp),
        receipt.perl_version,
        receipt.resolved_roots_count,
    )
}

fn ns_to_ms(ns: u128) -> f64 {
    ns as f64 / 1_000_000.0
}

fn format_perf_metric_row(name: &str, metric: Option<&ParserPerfMetric>) -> String {
    metric.map_or_else(
        || format!("| **{name}** | UNVERIFIED | benchmark receipt missing | `docs/project/status/parser_performance_scorecard.json` |"),
        |m| {
            format!(
                "| **{name}** | p50 {:.3} ms / p95 {:.3} ms | mean {:.3} ms over {} samples | `docs/project/status/parser_performance_scorecard.json` |",
                ns_to_ms(m.median_ns),
                ns_to_ms(m.p95_ns),
                ns_to_ms(m.mean_ns),
                m.iterations,
            )
        },
    )
}

fn format_nodekind_gap_note(summary: &super::super::corpus_audit::StatusSummary) -> String {
    match (
        summary.nodekind_actionable_never_seen,
        summary.nodekind_allowlisted_never_seen,
        summary.nodekind_never_seen,
    ) {
        (0, 0, 0) => "0 never-seen node kinds".to_string(),
        (0, allowlisted, _) => {
            format!("0 actionable never-seen; {allowlisted} recovery-only allowlisted")
        }
        (actionable, 0, total) => {
            format!("{actionable} actionable never-seen; {total} total never-seen")
        }
        (actionable, allowlisted, total) => {
            format!(
                "{actionable} actionable never-seen; {allowlisted} recovery-only allowlisted; {total} total never-seen"
            )
        }
    }
}

#[cfg(test)]
mod tests;

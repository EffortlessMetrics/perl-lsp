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

use accuracy::{
    ParserAccuracyArtifactSummary, parser_accuracy_rows, read_parser_accuracy_artifact,
};
use failure::{build_failure_bucket_details, build_failure_worklist};

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

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

pub(super) fn generate_parser_status(metrics: &ParserMetrics, original: &str) -> Result<String> {
    let system_row = metrics.system_receipt.as_ref().map_or_else(
        || {
            "| **Ubuntu system Perl** | UNVERIFIED | baseline receipt unavailable | `.ci/parser-corpus-baseline.json` |".to_string()
        },
        |report| {
            format!(
                "| **Ubuntu system Perl** | {} / {} | Compatibility baseline; Perl `{}`, {}, baseline `{}` | `.ci/parser-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                format_salvage_rate(report.recovery_salvage_rate),
                report.perl_version,
                format_recovery_shape_note(report),
                short_day(&report.timestamp),
            )
        },
    );

    let cpan_row = metrics.cpan_receipt.as_ref().map_or_else(
        || {
            "| **CPAN top 1000** | UNVERIFIED | baseline receipt unavailable | `.ci/cpan-corpus-baseline.json` |".to_string()
        },
        |report| {
            format!(
                "| **CPAN top 1000** | {} / {} | Ecosystem breadth baseline; {}, cached downloads in `target/cpan-corpus/.cpanm`, baseline `{}` | `.ci/cpan-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                format_salvage_rate(report.recovery_salvage_rate),
                format_recovery_shape_note(report),
                short_day(&report.timestamp),
            )
        },
    );

    let project_row = metrics.project_corpus.as_ref().map_or_else(
        || {
            "| **Project corpus** | UNVERIFIED | live repo scan unavailable | `test_corpus/` + `crates/perl-corpus/src/gen` |".to_string()
        },
        |summary| {
            format!(
                "| **Project corpus** | {} | Deterministic regression baseline; `{}` `test_corpus/` + `{}` `perl-corpus` files, `{}` errors, `{}` timeouts, `{}` panics, `{}/{}` NodeKinds, `{}/{}` GA features | `test_corpus/` + `crates/perl-corpus/src/gen` |",
                format_clean_rate(summary.ok_files, summary.total_files),
                summary.test_corpus_files,
                summary.perl_corpus_files,
                summary.error_files,
                summary.timeout_files,
                summary.panic_files,
                summary.nodekind_covered,
                summary.nodekind_total,
                summary.ga_covered,
                summary.ga_total,
            )
        },
    );

    let nodekind_row = metrics.project_corpus.as_ref().map_or_else(
        || {
            "| **Node-kind coverage** | UNVERIFIED | live repo scan unavailable | `corpus_audit` |"
                .to_string()
        },
        |summary| {
            let pct = if summary.nodekind_total == 0 {
                0.0
            } else {
                100.0 * summary.nodekind_covered as f64 / summary.nodekind_total as f64
            };
            let gap_note = format_nodekind_gap_note(summary);
            format!(
                "| **Node-kind coverage** | {}/{} ({:.1}%) | {} | `corpus_audit` |",
                summary.nodekind_covered, summary.nodekind_total, pct, gap_note,
            )
        },
    );

    let reliability_row = {
        let sys_unread = metrics
            .system_receipt
            .as_ref()
            .map_or_else(|| "?".to_string(), |r| r.files_unreadable.to_string());
        let cpan_unread = metrics
            .cpan_receipt
            .as_ref()
            .map_or_else(|| "?".to_string(), |r| r.files_unreadable.to_string());
        let proj_detail = metrics.project_corpus.as_ref().map_or_else(
            || "Project: UNVERIFIED".to_string(),
            |s| format!("Project: {} timeout, {} panic, 0 unread", s.timeout_files, s.panic_files),
        );
        format!(
            "| **Reliability** | Ubuntu: {} unread / CPAN: {} unread / {} | -- | `.ci/*-baseline.json` |",
            sys_unread, cpan_unread, proj_detail,
        )
    };

    let pinned = metrics.common_corpus_pinned;
    let strict_clean_row = metrics.common_corpus_receipt.as_ref().map_or_else(
        || {
            format!(
                "| **Strict-clean subset** | insufficient_data | {pinned} pinned modules; run `just common-corpus-check` to generate receipt | `.ci/common-corpus-manifest.txt` |"
            )
        },
        |receipt| {
            let pass = receipt.clean_files;
            let total = receipt.total_files;
            let pct = if total == 0 { 100.0 } else { 100.0 * pass as f64 / total as f64 };
            format!(
                "| **Strict-clean subset** | {pass}/{total} ({pct:.0}%) | {pinned} pinned modules, zero-error gate | `.ci/common-corpus-manifest.txt` |",
            )
        },
    );

    let perf_table = metrics.performance_scorecard.as_ref().map_or_else(
        || {
            [
                format_perf_metric_row("cold parse", None),
                format_perf_metric_row("warm reparse", None),
                format_perf_metric_row("incremental small edit", None),
                format_perf_metric_row("incremental multiple edits", None),
                format_perf_metric_row("lexer-only", None),
                format_perf_metric_row("scope analysis", None),
            ]
            .join("\n")
        },
        |scorecard| {
            [
                format_perf_metric_row("cold parse", scorecard.metrics.get("cold_parse")),
                format_perf_metric_row("warm reparse", scorecard.metrics.get("warm_reparse")),
                format_perf_metric_row(
                    "incremental small edit",
                    scorecard.metrics.get("incremental_small_edit"),
                ),
                format_perf_metric_row(
                    "incremental multiple edits",
                    scorecard.metrics.get("incremental_multiple_edits"),
                ),
                format_perf_metric_row("lexer-only", scorecard.metrics.get("lexer_only")),
                format_perf_metric_row("scope analysis", scorecard.metrics.get("scope_analysis")),
            ]
            .join("\n")
        },
    );

    let perf_receipt_note = metrics.performance_scorecard.as_ref().map_or_else(
        || "UNVERIFIED (run parser benches to regenerate receipt)".to_string(),
        |scorecard| format!("epoch {} (UTC seconds)", scorecard.generated_at_epoch_s),
    );

    let parser_accuracy_summary = parser_accuracy_rows(metrics.parser_accuracy.as_ref());

    let token = &metrics.token_metrics;
    let token_table = format!(
        "| **TokenKind variants** | {} | enum size in `perl-token` | `crates/perl-token/src/kind.rs` |\n\
         | **Token metadata coverage** | {}/{} ({}) | `display_name()` mappings for all variants | `crates/perl-token/src/kind.rs` + `.ci/metrics/baselines/token.json` |\n\
         | **Category partition** | {} | keywords/operators/delimiters/literals/identifiers/special | `crates/perl-token/src/kind.rs` |\n\
         | **Display-name coverage** | {}/{} | user-facing token labels present | `crates/perl-token/src/kind.rs` |\n\
         | **Lexer/parser conformance** | {} | integration through shared token crate | `crates/perl-lexer/Cargo.toml` + `crates/perl-parser-core/Cargo.toml` |\n\
         | **Token perf (p50/p95)** | {} | key token operations benchmark health | `docs/project/status/token_performance_scorecard.json` |\n\
         | **Runtime dependencies** | {} | non-dev deps in `perl-token` | `crates/perl-token/Cargo.toml` |",
        token.variant_count,
        token.metadata_coverage_count,
        token.variant_count,
        token.metadata_status,
        token.category_partition_status,
        token.display_name_coverage_count,
        token.variant_count,
        token.lexer_parser_conformance_status,
        token.performance_row,
        token.runtime_dependency_count,
    );

    let tracking_table = [system_row, cpan_row, project_row].join("\n");

    let failure_worklist = metrics.system_receipt.as_ref().map_or_else(
        || {
            "| insufficient_data (no receipt — run `just corpus-sweep-check` to generate) | insufficient_data |"
                .to_string()
        },
        |receipt| build_failure_worklist(receipt),
    );
    let failure_receipt_note = metrics.system_receipt.as_ref().map_or_else(
        || {
            "Receipt snapshot unavailable. Raw bucket counts are `insufficient_data` until `just corpus-sweep-check` refreshes `.ci/parser-corpus-baseline.json`."
                .to_string()
        },
        format_failure_receipt_note,
    );
    let failure_bucket_details = metrics.system_receipt.as_ref().map_or_else(
        || {
            "| insufficient_data (no receipt — run `just corpus-sweep-check` to generate) | insufficient_data | insufficient_data |"
                .to_string()
        },
        |receipt| build_failure_bucket_details(receipt),
    );

    let parser_coverage_bullets = format!(
        "- **Three-baseline model**: compatibility is tracked with `just corpus-sweep-check` against Ubuntu system Perl, ecosystem breadth with `just cpan-corpus-check` against the cached CPAN top-1000 install, and deterministic regression coverage with `just parser-audit` against the repo-owned corpus.\n\
         - **Strict promise lists**: `just common-corpus-check` and the CPAN known-clean manifest inside `just cpan-corpus-check` pin subsets that must remain clean on top of the broader baseline receipts.\n\
         - **Fixture bank**: `tree-sitter-perl/test/corpus` contributes ~{} focused syntax sections for targeted parser cases.\n\
         - **CPAN install hygiene**: `cargo xtask cpan-corpus install` reuses `target/cpan-corpus/.cpanm`; pass `--reset` only for a cold rebuild.
\
         - **Parser performance receipt**: `{}` from `docs/project/status/parser_performance_scorecard.json`; generated by `cargo bench -p perl-parser --bench incremental_benchmark` + `cargo bench -p perl-parser --bench parser_benchmark`.",
        metrics.syntax_sections,
        perf_receipt_note,
    );

    let mut text = original.to_string();
    text = replace_parser_status_block(&text, "PARSER_TRACKING_TABLE", &tracking_table)?;
    text = replace_parser_status_block(&text, "PARSER_PERFORMANCE_TABLE", &perf_table)?;
    text = replace_parser_status_block(&text, "PARSER_METRICS_BULLETS", &parser_coverage_bullets)?;
    text = replace_parser_status_block(&text, "TOKEN_HEALTH_TABLE", &token_table)?;
    text = replace_parser_status_block(&text, "PARSER_NODEKIND_ROW", &nodekind_row)?;
    text = replace_parser_status_block(&text, "PARSER_RELIABILITY_ROW", &reliability_row)?;
    text = replace_parser_status_block(&text, "PARSER_STRICT_CLEAN_ROW", &strict_clean_row)?;
    text = replace_parser_status_block(&text, "PARSER_ACCURACY_SUMMARY", &parser_accuracy_summary)?;
    text = replace_parser_status_block(&text, "PARSER_FAILURE_WORKLIST", &failure_worklist)?;
    text =
        replace_parser_status_block(&text, "PARSER_FAILURE_RECEIPT_NOTE", &failure_receipt_note)?;
    text = replace_parser_status_block(&text, "PARSER_FAILURE_BUCKETS", &failure_bucket_details)?;
    Ok(text)
}

#[cfg(test)]
mod tests;

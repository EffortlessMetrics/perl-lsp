//! Parser status markdown rendering.

use color_eyre::eyre::Result;

use super::accuracy::parser_accuracy_rows;
use super::failure::{build_failure_bucket_details, build_failure_worklist};
use super::{
    ParserMetrics, format_clean_rate, format_failure_receipt_note, format_nodekind_gap_note,
    format_perf_metric_row, format_recovery_shape_note, format_salvage_rate,
    replace_parser_status_block, short_day,
};

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

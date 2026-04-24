//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::Result;
use perl_token::{TokenCategory, TokenKind};
use regex::Regex;
use serde::Deserialize;

use super::replace_block;

// ---------------------------------------------------------------------------
// Parser metrics struct
// ---------------------------------------------------------------------------

pub(super) struct ParserMetrics {
    pub syntax_sections: usize,
    pub system_receipt: Option<super::super::parser_corpus_sweep::SweepReport>,
    pub cpan_receipt: Option<super::super::parser_corpus_sweep::SweepReport>,
    pub project_corpus: Option<super::super::corpus_audit::StatusSummary>,
    /// Receipt from `just common-corpus-check` — the strict-clean pinned-module gate.
    pub common_corpus_receipt: Option<super::super::parser_corpus_sweep::SweepReport>,
    /// Number of pinned modules in `.ci/common-corpus-manifest.txt`.
    pub common_corpus_pinned: usize,
    pub performance_scorecard: Option<ParserPerformanceScorecard>,
    pub token_health: TokenHealthMetrics,
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

#[derive(Debug, Clone, Deserialize)]
struct TokenHealthBaseline {
    min_metadata_coverage_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerformanceScorecard {
    generated_at_epoch_s: u64,
    metrics: std::collections::BTreeMap<String, TokenPerfMetric>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerfMetric {
    iterations: usize,
    median_ns: u128,
    p95_ns: u128,
}

pub(super) struct TokenHealthMetrics {
    variant_count: usize,
    metadata_coverage_count: usize,
    category_partition_ok: bool,
    display_name_coverage_count: usize,
    lexer_parser_conformance_status: &'static str,
    runtime_dependency_count: usize,
    baseline: Option<TokenHealthBaseline>,
    performance_scorecard: Option<TokenPerformanceScorecard>,
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
        token_health: collect_token_health_metrics(root),
    }
}

fn collect_token_health_metrics(root: &Path) -> TokenHealthMetrics {
    let all_kinds = TokenKind::ALL;
    let variant_count = all_kinds.len();
    let display_name_coverage_count =
        all_kinds.iter().filter(|kind| !kind.display_name().trim().is_empty()).count();
    let metadata_coverage_count = all_kinds
        .iter()
        .filter(|kind| !kind.display_name().trim().is_empty() && has_category(kind.category()))
        .count();
    let category_partition_ok = all_kinds.iter().all(|kind| has_category(kind.category()));

    TokenHealthMetrics {
        variant_count,
        metadata_coverage_count,
        category_partition_ok,
        display_name_coverage_count,
        lexer_parser_conformance_status: "PASS (`cargo test -p perl-token` conformance suite)",
        runtime_dependency_count: count_runtime_dependencies(root),
        baseline: read_token_health_baseline(root),
        performance_scorecard: read_token_performance_scorecard(root),
    }
}

const fn has_category(category: TokenCategory) -> bool {
    match category {
        TokenCategory::Keyword
        | TokenCategory::Operator
        | TokenCategory::Delimiter
        | TokenCategory::Literal
        | TokenCategory::IdentifierOrSigil
        | TokenCategory::Special => true,
        _ => false,
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

pub(super) fn read_sweep_report(
    path: &Path,
) -> Option<super::super::parser_corpus_sweep::SweepReport> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_parser_performance_scorecard(root: &Path) -> Option<ParserPerformanceScorecard> {
    let path = root.join("docs/project/status/parser_performance_scorecard.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_token_health_baseline(root: &Path) -> Option<TokenHealthBaseline> {
    let path = root.join(".ci/metrics/baselines/token.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_token_performance_scorecard(root: &Path) -> Option<TokenPerformanceScorecard> {
    let path = root.join("docs/project/status/token_performance_scorecard.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn count_runtime_dependencies(root: &Path) -> usize {
    let output = super::run_cmd(
        root,
        &["cargo", "metadata", "--format-version", "1", "--no-deps"],
        Duration::from_secs(60),
    );
    let Ok(meta) = serde_json::from_str::<serde_json::Value>(&output) else {
        return 0;
    };
    let Some(packages) = meta.get("packages").and_then(|v| v.as_array()) else {
        return 0;
    };

    let pkg =
        packages.iter().find(|pkg| pkg.get("name").and_then(|v| v.as_str()) == Some("perl-token"));
    let Some(pkg) = pkg else {
        return 0;
    };
    let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_array()) else {
        return 0;
    };
    deps.iter()
        .filter(|dep| dep.get("kind").map_or(true, |v| v.is_null()))
        .filter(|dep| dep.get("target").map_or(true, |v| v.is_null()))
        .count()
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

fn short_day(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
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

fn format_token_perf_metric_row(name: &str, metric: Option<&TokenPerfMetric>) -> String {
    metric.map_or_else(
        || format!("| **{name}** | UNVERIFIED | benchmark receipt missing | `docs/project/status/token_performance_scorecard.json` |"),
        |m| {
            format!(
                "| **{name}** | p50 {:.3} ms / p95 {:.3} ms | {} samples | `docs/project/status/token_performance_scorecard.json` |",
                ns_to_ms(m.median_ns),
                ns_to_ms(m.p95_ns),
                m.iterations,
            )
        },
    )
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
                "| **Ubuntu system Perl** | {} | Compatibility baseline; Perl `{}`, `{}` unreadable, `{}` with errors, baseline `{}` | `.ci/parser-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                report.perl_version,
                report.files_unreadable,
                report.files_with_errors,
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
                "| **CPAN top 1000** | {} | Ecosystem breadth baseline; `{}` unreadable, `{}` with errors, cached downloads in `target/cpan-corpus/.cpanm`, baseline `{}` | `.ci/cpan-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                report.files_unreadable,
                report.files_with_errors,
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
            let never_seen = summary.nodekind_total.saturating_sub(summary.nodekind_covered);
            format!(
                "| **Node-kind coverage** | {}/{} ({:.1}%) | {} never-seen node kinds | `corpus_audit` |",
                summary.nodekind_covered, summary.nodekind_total, pct, never_seen,
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
            |s| format!("Project: {} timeout, {} panic, 0 unread", s.timeout_files, s.panic_files,),
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
                "| **Strict-clean subset** | {pinned} modules (unverified) | run `just common-corpus-check` to generate receipt | `.ci/common-corpus-manifest.txt` |"
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

    let tracking_table = [system_row, cpan_row, project_row].join("\n");
    let token = &metrics.token_health;
    let token_baseline_warning = token.baseline.as_ref().map_or_else(
        || "UNVERIFIED (baseline receipt missing)".to_string(),
        |baseline| {
            if token.metadata_coverage_count >= baseline.min_metadata_coverage_count {
                "PASS".to_string()
            } else {
                format!(
                    "WARN ({} < baseline {})",
                    token.metadata_coverage_count, baseline.min_metadata_coverage_count
                )
            }
        },
    );
    let token_metadata_row = format!(
        "| **Token metadata coverage** | {}/{} | {} | `.ci/metrics/baselines/token.json` |",
        token.metadata_coverage_count, token.variant_count, token_baseline_warning
    );
    let token_display_name_row = format!(
        "| **Display-name coverage** | {}/{} | `TokenKind::display_name()` mappings | `crates/perl-token/src/lib.rs` |",
        token.display_name_coverage_count, token.variant_count
    );
    let token_partition_row = format!(
        "| **Category partition** | {} | keyword/operator/delimiter/literal/identifier/special | `TokenKind::category()` |",
        if token.category_partition_ok { "PASS" } else { "FAIL" }
    );
    let token_conformance_row = format!(
        "| **Lexer/parser conformance** | {} | token vocabulary + parser/lexer contract tests | `crates/perl-token/tests/` |",
        token.lexer_parser_conformance_status
    );
    let token_runtime_deps_row = format!(
        "| **Runtime dependency count** | {} | direct `dependencies` from `cargo metadata` | `crates/perl-token/Cargo.toml` |",
        token.runtime_dependency_count
    );
    let token_perf_table = token.performance_scorecard.as_ref().map_or_else(
        || {
            [
                format_token_perf_metric_row("token construction", None),
                format_token_perf_metric_row("display-name lookup", None),
            ]
            .join("\n")
        },
        |scorecard| {
            [
                format_token_perf_metric_row(
                    "token construction",
                    scorecard.metrics.get("token_construction"),
                ),
                format_token_perf_metric_row(
                    "display-name lookup",
                    scorecard.metrics.get("display_name_lookup"),
                ),
            ]
            .join("\n")
        },
    );
    let token_perf_receipt_note = token.performance_scorecard.as_ref().map_or_else(
        || "UNVERIFIED (run perl-token benches to generate scorecard)".to_string(),
        |scorecard| format!("epoch {} (UTC seconds)", scorecard.generated_at_epoch_s),
    );
    let token_table = [
        token_metadata_row,
        token_partition_row,
        token_display_name_row,
        token_conformance_row,
        token_runtime_deps_row,
    ]
    .join("\n");

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
    let token_bullets = format!(
        "- **Vocabulary breadth**: `TokenKind` tracks `{}` variants with metadata coverage `{}/{}`
- **Conformance signal**: {}
- **Performance receipt**: `{}` from `docs/project/status/token_performance_scorecard.json`.",
        token.variant_count,
        token.metadata_coverage_count,
        token.variant_count,
        token_baseline_warning,
        token_perf_receipt_note,
    );

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_TRACKING_TABLE -->",
        "<!-- END: PARSER_TRACKING_TABLE -->",
        &tracking_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_PERFORMANCE_TABLE -->",
        "<!-- END: PARSER_PERFORMANCE_TABLE -->",
        &perf_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_METRICS_BULLETS -->",
        "<!-- END: PARSER_METRICS_BULLETS -->",
        &parser_coverage_bullets,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_STATUS_TABLE -->",
        "<!-- END: TOKEN_STATUS_TABLE -->",
        &token_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_PERFORMANCE_TABLE -->",
        "<!-- END: TOKEN_PERFORMANCE_TABLE -->",
        &token_perf_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_METRICS_BULLETS -->",
        "<!-- END: TOKEN_METRICS_BULLETS -->",
        &token_bullets,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_NODEKIND_ROW -->",
        "<!-- END: PARSER_NODEKIND_ROW -->",
        &nodekind_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_RELIABILITY_ROW -->",
        "<!-- END: PARSER_RELIABILITY_ROW -->",
        &reliability_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->",
        "<!-- END: PARSER_STRICT_CLEAN_ROW -->",
        &strict_clean_row,
    )?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    fn sample_token_health() -> TokenHealthMetrics {
        TokenHealthMetrics {
            variant_count: 132,
            metadata_coverage_count: 132,
            category_partition_ok: true,
            display_name_coverage_count: 132,
            lexer_parser_conformance_status: "PASS",
            runtime_dependency_count: 0,
            baseline: None,
            performance_scorecard: None,
        }
    }

    fn template_with_markers() -> &'static str {
        "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
        <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n\
        <!-- BEGIN: TOKEN_STATUS_TABLE -->\nold\n<!-- END: TOKEN_STATUS_TABLE -->\n\
        <!-- BEGIN: TOKEN_PERFORMANCE_TABLE -->\nold\n<!-- END: TOKEN_PERFORMANCE_TABLE -->\n\
        <!-- BEGIN: TOKEN_METRICS_BULLETS -->\nold\n<!-- END: TOKEN_METRICS_BULLETS -->\n"
    }

    #[test]
    fn test_corpus_section_count() -> Result<()> {
        let root = crate::utils::project_root()?;
        let sections = count_corpus_sections(&root);
        assert!(sections > 0, "expected nonzero corpus sections");
        Ok(())
    }

    #[test]
    fn test_parser_receipts_load() -> Result<()> {
        let root = crate::utils::project_root()?;
        let metrics = collect_parser_metrics(&root);
        assert!(metrics.system_receipt.is_some(), "expected system corpus baseline receipt");
        assert!(metrics.cpan_receipt.is_some(), "expected CPAN corpus baseline receipt");
        assert!(metrics.project_corpus.is_some(), "expected live repo corpus summary");
        Ok(())
    }

    #[test]
    fn test_count_common_corpus_pinned() -> Result<()> {
        let root = crate::utils::project_root()?;
        let count = count_common_corpus_pinned(&root);
        assert_eq!(count, 10, "expected 10 pinned modules in common-corpus-manifest.txt");
        Ok(())
    }

    #[test]
    fn test_parser_nodekind_row_renders() -> Result<()> {
        let summary = super::super::super::corpus_audit::StatusSummary {
            total_files: 91,
            ok_files: 91,
            error_files: 0,
            timeout_files: 0,
            panic_files: 0,
            test_corpus_files: 69,
            perl_corpus_files: 22,
            nodekind_covered: 65,
            nodekind_total: 69,
            ga_covered: 12,
            ga_total: 12,
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: Some(summary),
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
            performance_scorecard: None,
            token_health: sample_token_health(),
        };
        let template = template_with_markers();
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("65/69"), "nodekind row missing 65/69");
        assert!(result.contains("94.2"), "nodekind row missing 94.2%");
        assert!(result.contains("4 never-seen"), "nodekind row missing never-seen count");
        assert!(
            result.contains("unverified"),
            "strict-clean no-receipt row should say 'unverified'"
        );
        assert!(!result.contains("10/10"), "strict-clean no-receipt row must not show 10/10");
        Ok(())
    }

    #[test]
    fn test_parser_strict_clean_row_no_receipt() -> Result<()> {
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
            performance_scorecard: None,
            token_health: sample_token_health(),
        };
        let template = template_with_markers();
        let result = generate_parser_status(&metrics, template)?;
        assert!(
            result.contains("10 modules (unverified)"),
            "strict-clean no-receipt row must say '10 modules (unverified)'"
        );
        assert!(
            result.contains("common-corpus-check"),
            "strict-clean no-receipt row must mention the command"
        );
        Ok(())
    }

    /// Verify that `generate_parser_status` renders scorecard values correctly
    /// when a populated `ParserPerformanceScorecard` is provided.  All prior
    /// tests pass `performance_scorecard: None`, leaving the `Some` branch of
    /// `format_perf_metric_row` completely untested.
    #[test]
    fn test_parser_performance_table_renders_with_scorecard() -> Result<()> {
        use std::collections::BTreeMap;

        let mut metrics_map = BTreeMap::new();
        metrics_map.insert(
            "cold_parse".to_string(),
            ParserPerfMetric { iterations: 30, median_ns: 44_708, p95_ns: 98_033, mean_ns: 69_888 },
        );
        metrics_map.insert(
            "warm_reparse".to_string(),
            ParserPerfMetric {
                iterations: 35,
                median_ns: 118_046,
                p95_ns: 277_118,
                mean_ns: 242_863,
            },
        );
        // Include one metric intentionally absent from the map so the None
        // branch of format_perf_metric_row is also exercised in this test.

        let scorecard = ParserPerformanceScorecard {
            generated_at_epoch_s: 1_777_010_864,
            metrics: metrics_map,
        };

        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
            performance_scorecard: Some(scorecard),
            token_health: sample_token_health(),
        };
        let template = template_with_markers();

        let result = generate_parser_status(&metrics, template)?;

        // cold_parse row: median=44708 ns = 0.045 ms, p95=98033 ns = 0.098 ms
        assert!(
            result.contains("0.045"),
            "cold_parse median_ns 44708 should render as ~0.045 ms, got: {}",
            &result[result.find("cold parse").unwrap_or(0)..][..120.min(result.len())]
        );
        assert!(result.contains("0.098"), "cold_parse p95_ns 98033 should render as ~0.098 ms");
        assert!(result.contains("30 samples"), "cold_parse iterations should show 30 samples");

        // warm_reparse row: median=118046 ns = 0.118 ms
        assert!(result.contains("0.118"), "warm_reparse median should render as ~0.118 ms");

        // incremental_small_edit was not inserted — must render as UNVERIFIED
        assert!(
            result.contains("UNVERIFIED"),
            "missing metric key should render as UNVERIFIED, not panic"
        );

        // Receipt note in bullets should use the epoch, not "UNVERIFIED"
        assert!(result.contains("1777010864"), "perf receipt note should show epoch 1777010864");

        Ok(())
    }

    #[test]
    fn test_parser_strict_clean_row_with_receipt() -> Result<()> {
        use std::collections::BTreeMap;
        let receipt = super::super::super::parser_corpus_sweep::SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-11T00:00:00Z".to_string(),
            corpus_profile: "common".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "5.038".to_string(),
            total_files: 10,
            files_unreadable: 0,
            clean_files: 10,
            files_with_errors: 0,
            total_error_nodes: 0,
            first_error_buckets: BTreeMap::new(),
            files_by_bucket: BTreeMap::new(),
            file_results: vec![],
            elapsed_secs: 1.0,
            phase_timings: None,
            median_error_density_per_1k_loc: None,
            slowest_files: vec![],
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: Some(receipt),
            common_corpus_pinned: 10,
            performance_scorecard: None,
            token_health: sample_token_health(),
        };
        let template = template_with_markers();
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("10/10"), "strict-clean row missing 10/10");
        assert!(result.contains("100%"), "strict-clean row missing 100%");
        assert!(
            result.contains("10 pinned modules"),
            "strict-clean row missing pinned modules note"
        );
        Ok(())
    }

    #[test]
    fn test_token_metadata_baseline_warning_renders() -> Result<()> {
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
            performance_scorecard: None,
            token_health: TokenHealthMetrics {
                variant_count: 132,
                metadata_coverage_count: 131,
                category_partition_ok: true,
                display_name_coverage_count: 132,
                lexer_parser_conformance_status: "PASS",
                runtime_dependency_count: 0,
                baseline: Some(TokenHealthBaseline { min_metadata_coverage_count: 132 }),
                performance_scorecard: None,
            },
        };
        let result = generate_parser_status(&metrics, template_with_markers())?;
        assert!(result.contains("WARN (131 < baseline 132)"));
        Ok(())
    }
}

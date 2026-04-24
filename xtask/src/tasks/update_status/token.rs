//! Token subsystem status generator.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::Result;
use serde::Deserialize;

use super::{replace_block, run_cmd};

pub(super) struct TokenMetrics {
    token_kind_variants: usize,
    metadata_coverage_count: usize,
    display_name_coverage_count: usize,
    metadata_drop_warning: Option<String>,
    category_partition_status: String,
    lexer_parser_conformance_status: &'static str,
    runtime_dependency_count: usize,
    benchmark_scorecard: Option<TokenPerformanceScorecard>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenBaseline {
    floor_metrics: TokenBaselineFloor,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenBaselineFloor {
    metadata_coverage_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerformanceScorecard {
    generated_at_epoch_s: u64,
    metrics: BTreeMap<String, TokenPerfMetric>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerfMetric {
    iterations: usize,
    median_ns: u128,
    p95_ns: u128,
}

pub(super) fn collect_token_metrics(root: &Path) -> TokenMetrics {
    let token_kind_variants = count_token_kind_variants(root);
    let metadata_coverage_count = count_display_name_match_arms(root);
    let display_name_coverage_count = metadata_coverage_count;

    let metadata_drop_warning = read_token_baseline(root).and_then(|baseline| {
        let coverage = metadata_coverage_count as f64 / token_kind_variants.max(1) as f64;
        if coverage + f64::EPSILON < baseline.floor_metrics.metadata_coverage_pct {
            Some(format!(
                "metadata coverage dropped below baseline ({coverage:.1}% < {:.1}%)",
                baseline.floor_metrics.metadata_coverage_pct * 100.0,
            ))
        } else {
            None
        }
    });

    let partition_test_pass = run_named_test(
        root,
        &[
            "cargo",
            "test",
            "-p",
            "perl-token",
            "every_variant_has_exactly_one_category",
            "--",
            "--exact",
        ],
    );
    let test_catalog_count = count_test_catalog_kinds(root);
    let category_partition_status = if partition_test_pass
        && test_catalog_count == token_kind_variants
        && token_kind_variants > 0
    {
        "PASS".to_string()
    } else {
        format!("WARN (catalog {test_catalog_count}/{token_kind_variants})")
    };

    let lexer_parser_conformance_status = if run_cmd(
        root,
        &["cargo", "check", "-p", "perl-lexer", "-p", "perl-parser-core"],
        Duration::from_secs(240),
    )
    .contains("Finished")
    {
        "PASS"
    } else {
        "WARN"
    };

    TokenMetrics {
        token_kind_variants,
        metadata_coverage_count,
        display_name_coverage_count,
        metadata_drop_warning,
        category_partition_status,
        lexer_parser_conformance_status,
        runtime_dependency_count: count_runtime_dependencies(root),
        benchmark_scorecard: read_token_performance_scorecard(root),
    }
}

fn run_named_test(root: &Path, args: &[&str]) -> bool {
    let output = run_cmd(root, args, Duration::from_secs(180));
    output.contains("test result: ok")
}

fn count_token_kind_variants(root: &Path) -> usize {
    let path = root.join("crates/perl-token/src/lib.rs");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Some(enum_start) = raw.find("pub enum TokenKind {") else {
        return 0;
    };
    let Some(enum_end) = raw.find("impl TokenKind {") else {
        return 0;
    };
    raw[enum_start..enum_end]
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.ends_with(',') && line.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
        })
        .count()
}

fn count_display_name_match_arms(root: &Path) -> usize {
    let path = root.join("crates/perl-token/src/lib.rs");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Some(start) = raw.find("pub fn display_name") else {
        return 0;
    };
    raw[start..].lines().filter(|line| line.contains("TokenKind::") && line.contains("=>")).count()
}

fn count_test_catalog_kinds(root: &Path) -> usize {
    let path = root.join("crates/perl-token/tests/token_kinds_and_display.rs");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Some(start) = raw.find("fn all_kinds() -> Vec<TokenKind> {") else {
        return 0;
    };
    let Some(end_rel) = raw[start..].find("]\n}") else {
        return 0;
    };
    raw[start..start + end_rel].lines().filter(|line| line.contains("TokenKind::")).count()
}

fn count_runtime_dependencies(root: &Path) -> usize {
    let path = root.join("crates/perl-token/Cargo.toml");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Ok(value) = raw.parse::<toml::Value>() else {
        return 0;
    };
    value.get("dependencies").and_then(toml::Value::as_table).map_or(0, |deps| deps.len())
}

fn read_token_baseline(root: &Path) -> Option<TokenBaseline> {
    let path = root.join(".ci/metrics/baselines/token.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_token_performance_scorecard(root: &Path) -> Option<TokenPerformanceScorecard> {
    let path = root.join("docs/project/status/token_performance_scorecard.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn format_perf_row(name: &str, metric: Option<&TokenPerfMetric>) -> String {
    metric.map_or_else(
        || {
            format!(
                "| **{name}** | UNVERIFIED | scorecard missing metric key | `docs/project/status/token_performance_scorecard.json` |"
            )
        },
        |m| {
            format!(
                "| **{name}** | p50 {:.3} µs / p95 {:.3} µs | {} samples | `docs/project/status/token_performance_scorecard.json` |",
                m.median_ns as f64 / 1_000.0,
                m.p95_ns as f64 / 1_000.0,
                m.iterations,
            )
        },
    )
}

pub(super) fn generate_token_status(metrics: &TokenMetrics, original: &str) -> Result<String> {
    let metadata_status = if metrics.metadata_drop_warning.is_some() { "WARN" } else { "PASS" };

    let summary_rows = format!(
        "| **TokenKind variants** | {} | `TokenKind` enum cardinality | `crates/perl-token/src/lib.rs` |\n\
         | **Token metadata coverage** | {}/{} ({:.1}%) | display-name metadata mapped for every variant | `crates/perl-token/src/lib.rs` |\n\
         | **Category partition** | {} | every variant maps to exactly one category | `crates/perl-token/tests/token_kinds_and_display.rs` |\n\
         | **Display-name coverage** | {}/{} ({:.1}%) | `TokenKind::display_name` match coverage | `crates/perl-token/src/lib.rs` |\n\
         | **Lexer/parser conformance** | {} | `cargo check -p perl-lexer -p perl-parser-core` | `crates/perl-lexer`, `crates/perl-parser-core` |\n\
         | **Runtime dependencies** | {} | non-dev `[dependencies]` in `perl-token` | `crates/perl-token/Cargo.toml` |",
        metrics.token_kind_variants,
        metrics.metadata_coverage_count,
        metrics.token_kind_variants,
        100.0 * metrics.metadata_coverage_count as f64 / metrics.token_kind_variants.max(1) as f64,
        metrics.category_partition_status,
        metrics.display_name_coverage_count,
        metrics.token_kind_variants,
        100.0 * metrics.display_name_coverage_count as f64
            / metrics.token_kind_variants.max(1) as f64,
        metrics.lexer_parser_conformance_status,
        metrics.runtime_dependency_count,
    );

    let perf_rows = metrics.benchmark_scorecard.as_ref().map_or_else(
        || {
            [
                format_perf_row("display_name()", None),
                format_perf_row("Token::new", None),
                format_perf_row("Token::len", None),
            ]
            .join("\n")
        },
        |scorecard| {
            [
                format_perf_row("display_name()", scorecard.metrics.get("display_name")),
                format_perf_row("Token::new", scorecard.metrics.get("token_new")),
                format_perf_row("Token::len", scorecard.metrics.get("token_len")),
            ]
            .join("\n")
        },
    );

    let warning_line = metrics.metadata_drop_warning.as_ref().map_or_else(
        || "- **Metadata ratchet**: PASS (coverage meets or exceeds baseline).".to_string(),
        |msg| format!("- **Metadata ratchet**: WARN — {msg}."),
    );

    let perf_receipt_line = metrics.benchmark_scorecard.as_ref().map_or_else(
        || {
            "- **Token perf receipt**: UNVERIFIED (add `docs/project/status/token_performance_scorecard.json` to publish p50/p95).".to_string()
        },
        |scorecard| {
            format!(
                "- **Token perf receipt**: epoch {} (UTC seconds).",
                scorecard.generated_at_epoch_s
            )
        },
    );

    let bullets = format!(
        "{warning_line}\n\
         - **Conformance gate**: {}.\n\
         {}",
        metrics.lexer_parser_conformance_status, perf_receipt_line,
    );

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_SUMMARY_ROWS -->",
        "<!-- END: TOKEN_SUMMARY_ROWS -->",
        &summary_rows,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_PERFORMANCE_ROWS -->",
        "<!-- END: TOKEN_PERFORMANCE_ROWS -->",
        &perf_rows,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_METRICS_BULLETS -->",
        "<!-- END: TOKEN_METRICS_BULLETS -->",
        &bullets,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_METADATA_STATUS -->",
        "<!-- END: TOKEN_METADATA_STATUS -->",
        &format!(
            "| **Metadata ratchet status** | {metadata_status} | baseline in `.ci/metrics/baselines/token.json` |"
        ),
    )?;
    Ok(text)
}

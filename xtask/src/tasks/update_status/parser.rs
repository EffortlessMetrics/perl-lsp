//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use color_eyre::eyre::Result;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

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

#[derive(Debug, Clone)]
pub(super) struct TokenHealthMetrics {
    pub variant_count: usize,
    pub metadata_coverage_count: usize,
    pub display_name_coverage_count: usize,
    pub category_partition_status: String,
    pub lexer_parser_conformance_status: String,
    pub runtime_dependency_count: usize,
    pub metadata_coverage_status: String,
    pub benchmark_rows: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TokenHealthBaseline {
    #[serde(default)]
    minimum_metadata_coverage: Option<usize>,
    #[serde(default)]
    minimum_display_name_coverage: Option<usize>,
    #[serde(default)]
    expected_variant_count: Option<usize>,
    #[serde(default)]
    max_runtime_dependencies: Option<usize>,
    #[serde(default)]
    minimum_parser_lexer_references: Option<usize>,
}

static TOKEN_ENUM_VARIANT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*([A-Z][A-Za-z0-9_]*)\s*,\s*$").expect("token variant regex is valid")
});

static TOKEN_MATCH_ARM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*TokenKind::([A-Z][A-Za-z0-9_]*)\s*=>").expect("token match arm regex is valid")
});

static TOKEN_KIND_USAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"TokenKind::([A-Z][A-Za-z0-9_]*)").expect("token usage regex is valid")
});

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
        token_health: collect_token_health(root),
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

fn collect_token_health(root: &Path) -> TokenHealthMetrics {
    let token_lib_path = root.join("crates/perl-token/src/lib.rs");
    let token_toml_path = root.join("crates/perl-token/Cargo.toml");
    let baseline_path = root.join(".ci/metrics/baselines/token.json");

    let variant_names = parse_token_variants(&token_lib_path);
    let variant_count = variant_names.len();
    let display_names = parse_token_display_names(&token_lib_path);
    let display_name_coverage_count = display_names.len();

    let metadata_coverage_count = display_name_coverage_count;
    let categories = count_token_categories(&token_lib_path);
    let category_partition_status = {
        let categorized_total: usize = categories.values().sum();
        if categorized_total == variant_count && !categories.is_empty() {
            let breakdown = [
                ("keywords", categories.get("keywords").copied().unwrap_or(0)),
                ("operators", categories.get("operators").copied().unwrap_or(0)),
                ("delimiters", categories.get("delimiters").copied().unwrap_or(0)),
                ("literals", categories.get("literals").copied().unwrap_or(0)),
                ("identifiers", categories.get("identifiers").copied().unwrap_or(0)),
                ("special", categories.get("special").copied().unwrap_or(0)),
            ]
            .into_iter()
            .map(|(name, count)| format!("{name}:{count}"))
            .collect::<Vec<_>>()
            .join(", ");
            format!("PASS ({breakdown})")
        } else {
            format!("WARN ({categorized_total}/{variant_count} categorized)")
        }
    };

    let runtime_dependency_count = count_runtime_dependencies(&token_toml_path);
    let baseline = read_token_health_baseline(&baseline_path);

    let metadata_coverage_status = evaluate_metadata_coverage_status(
        variant_count,
        metadata_coverage_count,
        display_name_coverage_count,
        runtime_dependency_count,
        baseline.as_ref(),
    );

    let (lexer_refs, parser_refs, unknown_refs) =
        collect_lexer_parser_token_refs(root, &variant_names);
    let combined_refs = lexer_refs + parser_refs;
    let lexer_parser_conformance_status = if !unknown_refs.is_empty() {
        format!("FAIL (unknown TokenKind references: {})", unknown_refs.join(", "))
    } else if lexer_refs == 0 || parser_refs == 0 {
        format!(
            "WARN (lexer refs: {lexer_refs}, parser refs: {parser_refs}; expected both sides to reference TokenKind)"
        )
    } else if combined_refs == 0 {
        "UNVERIFIED (no lexer/parser references found)".to_string()
    } else if baseline
        .as_ref()
        .and_then(|b| b.minimum_parser_lexer_references)
        .is_some_and(|min| combined_refs < min)
    {
        let min = baseline.as_ref().and_then(|b| b.minimum_parser_lexer_references).unwrap_or(0);
        format!("WARN ({combined_refs} references; baseline floor {min})")
    } else {
        format!("PASS (lexer refs: {lexer_refs}, parser refs: {parser_refs})")
    };

    let benchmark_rows = read_token_benchmark_rows(&baseline_path);

    TokenHealthMetrics {
        variant_count,
        metadata_coverage_count,
        display_name_coverage_count,
        category_partition_status,
        lexer_parser_conformance_status,
        runtime_dependency_count,
        metadata_coverage_status,
        benchmark_rows,
    }
}

fn parse_token_variants(path: &Path) -> std::collections::BTreeSet<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return std::collections::BTreeSet::new();
    };

    let mut inside_enum = false;
    let mut variants = std::collections::BTreeSet::new();
    for line in raw.lines() {
        if line.contains("pub enum TokenKind") {
            inside_enum = true;
            continue;
        }
        if inside_enum && line.trim() == "}" {
            break;
        }
        if inside_enum && let Some(caps) = TOKEN_ENUM_VARIANT_RE.captures(line) {
            variants.insert(caps[1].to_string());
        }
    }
    variants
}

fn parse_token_display_names(path: &Path) -> std::collections::BTreeSet<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return std::collections::BTreeSet::new();
    };

    let mut inside_display = false;
    let mut names = std::collections::BTreeSet::new();
    for line in raw.lines() {
        if line.contains("pub fn display_name(self)") {
            inside_display = true;
            continue;
        }
        if inside_display && line.trim() == "}" {
            continue;
        }
        if inside_display && let Some(caps) = TOKEN_MATCH_ARM_RE.captures(line) {
            names.insert(caps[1].to_string());
        }
    }
    names
}

fn count_token_categories(path: &Path) -> std::collections::BTreeMap<String, usize> {
    let Ok(raw) = fs::read_to_string(path) else {
        return std::collections::BTreeMap::new();
    };
    let mut current: Option<&str> = None;
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.contains("===== Keywords") {
            current = Some("keywords");
        } else if trimmed.contains("===== Operators") {
            current = Some("operators");
        } else if trimmed.contains("===== Delimiters") {
            current = Some("delimiters");
        } else if trimmed.contains("===== Literals") {
            current = Some("literals");
        } else if trimmed.contains("===== Identifiers and Variables") {
            current = Some("identifiers");
        } else if trimmed.contains("===== Special") {
            current = Some("special");
        } else if let Some(category) = current
            && TOKEN_ENUM_VARIANT_RE.is_match(trimmed)
        {
            *counts.entry(category.to_string()).or_default() += 1;
        }
    }
    counts
}

fn count_runtime_dependencies(path: &Path) -> usize {
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Ok(value) = raw.parse::<toml::Table>() else {
        return 0;
    };
    value.get("dependencies").and_then(|v| v.as_table()).map_or(0, |t| t.len())
}

fn read_token_health_baseline(path: &Path) -> Option<TokenHealthBaseline> {
    let raw = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&raw).ok()?;
    serde_json::from_value(json.get("health_floor")?.clone()).ok()
}

fn evaluate_metadata_coverage_status(
    variant_count: usize,
    metadata_coverage_count: usize,
    display_name_coverage_count: usize,
    runtime_dependency_count: usize,
    baseline: Option<&TokenHealthBaseline>,
) -> String {
    let mut warnings = Vec::new();

    if metadata_coverage_count < variant_count {
        warnings
            .push(format!("metadata coverage dropped ({metadata_coverage_count}/{variant_count})"));
    }
    if display_name_coverage_count < variant_count {
        warnings.push(format!(
            "display-name coverage dropped ({display_name_coverage_count}/{variant_count})"
        ));
    }

    if let Some(base) = baseline {
        if let Some(expected_variant_count) = base.expected_variant_count
            && variant_count < expected_variant_count
        {
            warnings.push(format!(
                "variant count dropped ({variant_count} < baseline {expected_variant_count})"
            ));
        }
        if let Some(min_meta) = base.minimum_metadata_coverage
            && metadata_coverage_count < min_meta
        {
            warnings.push(format!(
                "metadata coverage below baseline ({metadata_coverage_count} < {min_meta})"
            ));
        }
        if let Some(min_display) = base.minimum_display_name_coverage
            && display_name_coverage_count < min_display
        {
            warnings.push(format!(
                "display-name coverage below baseline ({display_name_coverage_count} < {min_display})"
            ));
        }
        if let Some(max_runtime) = base.max_runtime_dependencies
            && runtime_dependency_count > max_runtime
        {
            warnings.push(format!(
                "runtime deps increased ({runtime_dependency_count} > baseline {max_runtime})"
            ));
        }
    }

    if warnings.is_empty() { "PASS".to_string() } else { format!("WARN ({})", warnings.join("; ")) }
}

fn collect_lexer_parser_token_refs(
    root: &Path,
    variants: &std::collections::BTreeSet<String>,
) -> (usize, usize, Vec<String>) {
    let lexer_refs = collect_token_refs_in_tree(root.join("crates/perl-lexer/src"), variants);
    let parser_refs =
        collect_token_refs_in_tree(root.join("crates/perl-parser-core/src"), variants);
    let mut unknown = std::collections::BTreeSet::new();
    for name in lexer_refs.1.into_iter().chain(parser_refs.1) {
        unknown.insert(name);
    }
    (lexer_refs.0, parser_refs.0, unknown.into_iter().collect())
}

fn collect_token_refs_in_tree(
    root: PathBuf,
    variants: &std::collections::BTreeSet<String>,
) -> (usize, Vec<String>) {
    if !root.exists() {
        return (0, Vec::new());
    }
    let mut refs = std::collections::BTreeSet::new();
    let mut unknown = std::collections::BTreeSet::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || !entry.path().extension().is_some_and(|ext| ext == "rs")
        {
            continue;
        }
        let Ok(raw) = fs::read_to_string(entry.path()) else {
            continue;
        };
        for caps in TOKEN_KIND_USAGE_RE.captures_iter(&raw) {
            let name = caps[1].to_string();
            if variants.contains(&name) {
                refs.insert(name);
            } else {
                unknown.insert(name);
            }
        }
    }

    (refs.len(), unknown.into_iter().collect())
}

fn read_token_benchmark_rows(path: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return vec![
            "| `display_name` lookup | UNVERIFIED | scorecard missing | `.ci/metrics/baselines/token.json` |".to_string(),
            "| `Token::new` construction | UNVERIFIED | scorecard missing | `.ci/metrics/baselines/token.json` |".to_string(),
            "| span ops (`len`/`is_empty`) | UNVERIFIED | scorecard missing | `.ci/metrics/baselines/token.json` |".to_string(),
        ];
    };
    let Ok(json) = serde_json::from_str::<Value>(&raw) else {
        return vec![
            "| `display_name` lookup | UNVERIFIED | invalid scorecard JSON | `.ci/metrics/baselines/token.json` |".to_string(),
            "| `Token::new` construction | UNVERIFIED | invalid scorecard JSON | `.ci/metrics/baselines/token.json` |".to_string(),
            "| span ops (`len`/`is_empty`) | UNVERIFIED | invalid scorecard JSON | `.ci/metrics/baselines/token.json` |".to_string(),
        ];
    };
    let Some(bench) = json.get("benchmarks").and_then(|v| v.as_object()) else {
        return vec![
            "| `display_name` lookup | UNVERIFIED | no benchmark section in baseline | `.ci/metrics/baselines/token.json` |".to_string(),
            "| `Token::new` construction | UNVERIFIED | no benchmark section in baseline | `.ci/metrics/baselines/token.json` |".to_string(),
            "| span ops (`len`/`is_empty`) | UNVERIFIED | no benchmark section in baseline | `.ci/metrics/baselines/token.json` |".to_string(),
        ];
    };

    let key_map = [
        ("display_name_lookup_ns", "`display_name` lookup"),
        ("token_new_ns", "`Token::new` construction"),
        ("span_ops_ns", "span ops (`len`/`is_empty`)"),
    ];

    key_map
        .into_iter()
        .map(|(key, label)| {
            let Some(obj) = bench.get(key).and_then(|v| v.as_object()) else {
                return format!(
                    "| {label} | UNVERIFIED | `{key}` missing from scorecard | `.ci/metrics/baselines/token.json` |"
                );
            };
            let median = obj.get("median_ns").and_then(|v| v.as_u64());
            let p95 = obj.get("p95_ns").and_then(|v| v.as_u64());
            match (median, p95) {
                (Some(median), Some(p95)) => format!(
                    "| {label} | median {median} ns / p95 {p95} ns | key op latency | `.ci/metrics/baselines/token.json` |"
                ),
                _ => format!(
                    "| {label} | UNVERIFIED | `{key}` missing median_ns/p95_ns | `.ci/metrics/baselines/token.json` |"
                ),
            }
        })
        .collect()
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
    let token_rows = [
        format!(
            "| **TokenKind variants** | {} | shared token vocabulary size | `crates/perl-token/src/lib.rs` |",
            metrics.token_health.variant_count
        ),
        format!(
            "| **Metadata coverage** | {}/{} | {} | `crates/perl-token/src/lib.rs` + `.ci/metrics/baselines/token.json` |",
            metrics.token_health.metadata_coverage_count,
            metrics.token_health.variant_count,
            metrics.token_health.metadata_coverage_status
        ),
        format!(
            "| **Category partition** | {} | category buckets must partition the enum | `crates/perl-token/src/lib.rs` |",
            metrics.token_health.category_partition_status
        ),
        format!(
            "| **Display-name coverage** | {}/{} | user-facing parser diagnostics text | `TokenKind::display_name` |",
            metrics.token_health.display_name_coverage_count,
            metrics.token_health.variant_count
        ),
        format!(
            "| **Lexer/parser conformance** | {} | checks TokenKind references in `perl-lexer` + `perl-parser-core` | source scan |",
            metrics.token_health.lexer_parser_conformance_status
        ),
        format!(
            "| **Runtime dependencies** | {} | `perl-token` runtime dependency surface | `crates/perl-token/Cargo.toml` |",
            metrics.token_health.runtime_dependency_count
        ),
    ]
    .join("\n");
    let token_bench_rows = metrics.token_health.benchmark_rows.join("\n");

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
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_HEALTH_TABLE -->",
        "<!-- END: TOKEN_HEALTH_TABLE -->",
        &token_rows,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TOKEN_BENCHMARK_TABLE -->",
        "<!-- END: TOKEN_BENCHMARK_TABLE -->",
        &token_bench_rows,
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
            display_name_coverage_count: 132,
            category_partition_status: "PASS".to_string(),
            lexer_parser_conformance_status: "PASS".to_string(),
            runtime_dependency_count: 0,
            metadata_coverage_status: "PASS".to_string(),
            benchmark_rows: vec![
                "| `display_name` lookup | median 91 ns / p95 140 ns | key op latency | `.ci/metrics/baselines/token.json` |".to_string(),
            ],
        }
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
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n                        <!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->\n\
                        <!-- BEGIN: TOKEN_BENCHMARK_TABLE -->\nold\n<!-- END: TOKEN_BENCHMARK_TABLE -->\n";
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
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n                        <!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->\n\
                        <!-- BEGIN: TOKEN_BENCHMARK_TABLE -->\nold\n<!-- END: TOKEN_BENCHMARK_TABLE -->\n";
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

        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n                        <!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->\n\
                        <!-- BEGIN: TOKEN_BENCHMARK_TABLE -->\nold\n<!-- END: TOKEN_BENCHMARK_TABLE -->\n";

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
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n                        <!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->\n\
                        <!-- BEGIN: TOKEN_BENCHMARK_TABLE -->\nold\n<!-- END: TOKEN_BENCHMARK_TABLE -->\n";
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
    fn test_metadata_coverage_warns_on_drop() {
        let status = evaluate_metadata_coverage_status(132, 130, 130, 0, None);
        assert!(status.starts_with("WARN"), "expected WARN status, got {status}");
        assert!(status.contains("metadata coverage dropped"));
    }

    #[test]
    fn test_token_benchmark_rows_from_baseline_json() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let baseline = dir.path().join("token.json");
        fs::write(
            &baseline,
            r#"{
  "benchmarks": {
    "display_name_lookup_ns": { "median_ns": 91, "p95_ns": 140 },
    "token_new_ns": { "median_ns": 37, "p95_ns": 65 },
    "span_ops_ns": { "median_ns": 12, "p95_ns": 19 }
  }
}"#,
        )?;
        let rows = read_token_benchmark_rows(&baseline);
        assert!(rows.iter().any(|row| row.contains("median 91 ns / p95 140 ns")));
        assert!(rows.iter().all(|row| !row.contains("UNVERIFIED")));
        Ok(())
    }
}

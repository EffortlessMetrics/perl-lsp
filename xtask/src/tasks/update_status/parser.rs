//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::fs;
use std::path::Path;
use std::time::Duration;
use std::{cmp::Reverse, collections::BTreeMap};

use color_eyre::eyre::Result;
use regex::Regex;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FailureCluster {
    TransliterationQuote,
    DeclarationPackage,
    HeredocDelimiter,
    RecoveryOnly,
    EncodingMultibyte,
    Other,
}

impl FailureCluster {
    fn label(self) -> &'static str {
        match self {
            Self::TransliterationQuote => "transliteration / quote parsing",
            Self::DeclarationPackage => "declaration / package parsing",
            Self::HeredocDelimiter => "heredoc / delimiter handling",
            Self::RecoveryOnly => "recovery-only failures",
            Self::EncodingMultibyte => "encoding / multibyte failures",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
struct ClusterStats {
    count: usize,
    examples: Vec<(String, String)>,
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

fn classify_bucket(bucket: &str) -> FailureCluster {
    let name = bucket.to_ascii_lowercase();
    if name.contains("encoding")
        || name.contains("unicode")
        || name.contains("utf")
        || name.contains("multibyte")
        || name.contains("wide_char")
    {
        return FailureCluster::EncodingMultibyte;
    }
    if name.contains("heredoc")
        || name.contains("delimiter")
        || name.contains("unclosed_substitution")
    {
        return FailureCluster::HeredocDelimiter;
    }
    if name.contains("translit")
        || name.contains("transliteration")
        || name.contains("quote")
        || name.contains("substitution")
    {
        return FailureCluster::TransliterationQuote;
    }
    if name.contains("package")
        || name.contains("signature")
        || name.contains("module")
        || name.contains("subroutine")
        || name.contains("expected_identifier")
    {
        return FailureCluster::DeclarationPackage;
    }
    if name.starts_with("expected_")
        || name.starts_with("unexpected_")
        || name.starts_with("unclosed_")
        || name == "catastrophic_parse_failure"
    {
        return FailureCluster::RecoveryOnly;
    }
    FailureCluster::Other
}

fn collect_failure_clusters(metrics: &ParserMetrics) -> BTreeMap<FailureCluster, ClusterStats> {
    let mut by_cluster: BTreeMap<FailureCluster, ClusterStats> = BTreeMap::new();

    for report in [&metrics.system_receipt, &metrics.cpan_receipt].into_iter().flatten() {
        for (bucket, count) in &report.first_error_buckets {
            let cluster = classify_bucket(bucket);
            let stats = by_cluster
                .entry(cluster)
                .or_insert_with(|| ClusterStats { count: 0, examples: Vec::new() });
            stats.count += count;

            if let Some(paths) = report.files_by_bucket.get(bucket) {
                for path in paths.iter().take(3) {
                    stats.examples.push((bucket.clone(), path.clone()));
                }
            }
        }
    }

    for stats in by_cluster.values_mut() {
        stats.examples.sort();
        stats.examples.dedup();
        if stats.examples.len() > 3 {
            stats.examples.truncate(3);
        }
    }

    by_cluster
}

fn render_failure_worklist(metrics: &ParserMetrics) -> String {
    let clusters = collect_failure_clusters(metrics);
    if clusters.is_empty() {
        return "- No parser corpus failures were available from baseline receipts.".to_string();
    }

    let mut lines = vec![
        "| Cluster | Files | Representative examples |".to_string(),
        "| --- | ---: | --- |".to_string(),
    ];

    let mut rows: Vec<(FailureCluster, ClusterStats)> = clusters.into_iter().collect();
    rows.sort_by_key(|(cluster, stats)| (Reverse(stats.count), *cluster));

    for (cluster, stats) in rows {
        let examples = if stats.examples.is_empty() {
            "n/a".to_string()
        } else {
            stats
                .examples
                .iter()
                .map(|(bucket, path)| format!("`{bucket}` → `{path}`"))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        lines.push(format!("| {} | {} | {} |", cluster.label(), stats.count, examples));
    }

    lines.join("\n")
}

fn render_never_seen_nodekinds(metrics: &ParserMetrics) -> String {
    let Some(summary) = metrics.project_corpus.as_ref() else {
        return "- UNVERIFIED: live repo scan unavailable.".to_string();
    };
    if summary.nodekind_never_seen.is_empty() {
        return "- None: all declared NodeKinds are represented in the project corpus.".to_string();
    }

    let list = summary
        .nodekind_never_seen
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("- Missing NodeKinds ({}): {list}", summary.nodekind_never_seen.len())
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

    let tracking_table = [system_row, cpan_row, project_row].join("\n");

    let parser_coverage_bullets = format!(
        "- **Three-baseline model**: compatibility is tracked with `just corpus-sweep-check` against Ubuntu system Perl, ecosystem breadth with `just cpan-corpus-check` against the cached CPAN top-1000 install, and deterministic regression coverage with `just parser-audit` against the repo-owned corpus.\n\
         - **Strict promise lists**: `just common-corpus-check` and the CPAN known-clean manifest inside `just cpan-corpus-check` pin subsets that must remain clean on top of the broader baseline receipts.\n\
         - **Fixture bank**: `tree-sitter-perl/test/corpus` contributes ~{} focused syntax sections for targeted parser cases.\n\
         - **CPAN install hygiene**: `cargo xtask cpan-corpus install` reuses `target/cpan-corpus/.cpanm`; pass `--reset` only for a cold rebuild.",
        metrics.syntax_sections,
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
        "<!-- BEGIN: PARSER_FAILURE_WORKLIST -->",
        "<!-- END: PARSER_FAILURE_WORKLIST -->",
        &render_failure_worklist(metrics),
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->",
        "<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->",
        &render_never_seen_nodekinds(metrics),
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
            nodekind_never_seen: vec![
                "DereferenceExpression".to_string(),
                "FormatStatement".to_string(),
                "GivenStatement".to_string(),
                "WhenStatement".to_string(),
            ],
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
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
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
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
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
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
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
    fn test_failure_worklist_and_never_seen_section() -> Result<()> {
        use std::collections::BTreeMap;

        let summary = super::super::super::corpus_audit::StatusSummary {
            total_files: 91,
            ok_files: 90,
            error_files: 1,
            timeout_files: 0,
            panic_files: 0,
            test_corpus_files: 69,
            perl_corpus_files: 22,
            nodekind_covered: 65,
            nodekind_total: 69,
            nodekind_never_seen: vec!["GivenStatement".to_string(), "WhenStatement".to_string()],
            ga_covered: 12,
            ga_total: 12,
        };
        let receipt = super::super::super::parser_corpus_sweep::SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-11T00:00:00Z".to_string(),
            corpus_profile: "cpan".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "5.038".to_string(),
            total_files: 10,
            files_unreadable: 0,
            clean_files: 8,
            files_with_errors: 2,
            total_error_nodes: 2,
            first_error_buckets: BTreeMap::from([
                ("unexpected_token_package".to_string(), 1),
                ("unclosed_substitution_delimiter".to_string(), 1),
            ]),
            files_by_bucket: BTreeMap::from([
                (
                    "unexpected_token_package".to_string(),
                    vec!["target/cpan-corpus/lib/perl5/Foo/Bar.pm".to_string()],
                ),
                (
                    "unclosed_substitution_delimiter".to_string(),
                    vec!["target/cpan-corpus/lib/perl5/Baz.pm".to_string()],
                ),
            ]),
            file_results: vec![],
            elapsed_secs: 1.0,
            phase_timings: None,
            median_error_density_per_1k_loc: None,
            slowest_files: vec![],
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: Some(receipt),
            project_corpus: Some(summary),
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("declaration / package parsing"));
        assert!(result.contains("heredoc / delimiter handling"));
        assert!(result.contains("GivenStatement"));
        assert!(result.contains("WhenStatement"));
        Ok(())
    }
}

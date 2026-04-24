//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::Result;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

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
enum FailureCategory {
    TransliterationQuote,
    DeclarationPackage,
    HeredocDelimiter,
    RecoveryOnly,
    EncodingMultibyte,
    Other,
}

impl FailureCategory {
    fn label(self) -> &'static str {
        match self {
            FailureCategory::TransliterationQuote => "transliteration / quote parsing",
            FailureCategory::DeclarationPackage => "declaration / package parsing",
            FailureCategory::HeredocDelimiter => "heredoc / delimiter handling",
            FailureCategory::RecoveryOnly => "recovery-only failures",
            FailureCategory::EncodingMultibyte => "encoding / multibyte failures",
            FailureCategory::Other => "other",
        }
    }
}

#[derive(Default)]
struct FailureCategorySummary {
    files: usize,
    buckets: BTreeMap<String, usize>,
    examples: BTreeSet<String>,
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

fn categorize_bucket(bucket: &str) -> FailureCategory {
    let lower = bucket.to_ascii_lowercase();
    if lower.contains("substitution")
        || lower.contains("modifier")
        || lower.contains("quote")
        || lower.contains("transliteration")
    {
        return FailureCategory::TransliterationQuote;
    }
    if lower.contains("signature")
        || lower.contains("module_name")
        || lower.contains("package")
        || lower.contains("check must be followed by a block")
    {
        return FailureCategory::DeclarationPackage;
    }
    if lower.starts_with("unclosed_") || lower.contains("delimiter") || lower.contains("heredoc") {
        return FailureCategory::HeredocDelimiter;
    }
    if lower.contains("encoding")
        || lower.contains("unicode")
        || lower.contains("multibyte")
        || lower.contains("utf")
        || lower.contains("wide character")
    {
        return FailureCategory::EncodingMultibyte;
    }
    if lower.starts_with("expected_")
        || lower.starts_with("unexpected_")
        || lower.contains("incomplete arrow expression")
    {
        return FailureCategory::RecoveryOnly;
    }
    FailureCategory::Other
}

fn summarize_failure_categories(
    reports: &[&super::super::parser_corpus_sweep::SweepReport],
) -> Vec<(FailureCategory, FailureCategorySummary)> {
    let mut by_category: BTreeMap<FailureCategory, FailureCategorySummary> = BTreeMap::new();

    for report in reports {
        for (bucket, count) in &report.first_error_buckets {
            let category = categorize_bucket(bucket);
            let category_entry = by_category.entry(category).or_default();
            category_entry.files += *count;
            *category_entry.buckets.entry(bucket.clone()).or_default() += *count;
            if let Some(paths) = report.files_by_bucket.get(bucket) {
                for path in paths {
                    category_entry.examples.insert(path.clone());
                }
            }
        }
    }

    let ordered = [
        FailureCategory::TransliterationQuote,
        FailureCategory::DeclarationPackage,
        FailureCategory::HeredocDelimiter,
        FailureCategory::RecoveryOnly,
        FailureCategory::EncodingMultibyte,
        FailureCategory::Other,
    ];

    ordered
        .iter()
        .map(|category| (*category, by_category.remove(category).unwrap_or_default()))
        .collect()
}

fn format_failure_worklist(
    metrics: &ParserMetrics,
    nodekind_never_seen: Option<&[String]>,
) -> String {
    let mut report_refs = Vec::new();
    if let Some(report) = metrics.system_receipt.as_ref() {
        report_refs.push(report);
    }
    if let Some(report) = metrics.cpan_receipt.as_ref() {
        report_refs.push(report);
    }

    if report_refs.is_empty() {
        return "- **Parser failure clusters**: unavailable (sweep receipts missing).\n\
                - **Never-seen node kinds**: unavailable (project corpus summary missing)."
            .to_string();
    }

    let category_rows = summarize_failure_categories(&report_refs);
    let mut lines = vec![
        "- **Parser failure clusters** (first-error buckets across Ubuntu + CPAN baselines):"
            .to_string(),
        "  | Category | File count | Top buckets | Representative files |".to_string(),
        "  | --- | ---: | --- | --- |".to_string(),
    ];

    for (category, summary) in category_rows {
        let top_buckets = if summary.buckets.is_empty() {
            "none".to_string()
        } else {
            let mut buckets: Vec<_> = summary.buckets.iter().collect();
            buckets.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            buckets
                .into_iter()
                .take(2)
                .map(|(bucket, count)| format!("{bucket} ({count})"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let examples = if summary.examples.is_empty() {
            "—".to_string()
        } else {
            summary
                .examples
                .iter()
                .take(2)
                .map(|path| format!("`{path}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        lines.push(format!(
            "  | {} | {} | {} | {} |",
            category.label(),
            summary.files,
            top_buckets,
            examples
        ));
    }

    let never_seen_line = if let Some(nodekinds) = nodekind_never_seen {
        if nodekinds.is_empty() {
            "- **Never-seen node kinds**: none.".to_string()
        } else {
            let preview = nodekinds
                .iter()
                .take(8)
                .map(|kind| format!("`{kind}`"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("- **Never-seen node kinds** ({}): {preview}.", nodekinds.len(),)
        }
    } else {
        "- **Never-seen node kinds**: unavailable (project corpus summary missing).".to_string()
    };
    lines.push(never_seen_line);
    lines.join("\n")
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

    let nodekind_never_seen = metrics
        .project_corpus
        .as_ref()
        .map(|summary| summary.never_seen_nodekinds.as_slice());
    let failure_worklist = format_failure_worklist(metrics, nodekind_never_seen);

    let parser_coverage_bullets = format!(
        "- **Three-baseline model**: compatibility is tracked with `just corpus-sweep-check` against Ubuntu system Perl, ecosystem breadth with `just cpan-corpus-check` against the cached CPAN top-1000 install, and deterministic regression coverage with `just parser-audit` against the repo-owned corpus.\n\
         - **Strict promise lists**: `just common-corpus-check` and the CPAN known-clean manifest inside `just cpan-corpus-check` pin subsets that must remain clean on top of the broader baseline receipts.\n\
         - **Fixture bank**: `tree-sitter-perl/test/corpus` contributes ~{} focused syntax sections for targeted parser cases.\n\
         - **CPAN install hygiene**: `cargo xtask cpan-corpus install` reuses `target/cpan-corpus/.cpanm`; pass `--reset` only for a cold rebuild.\n\
         {}",
        metrics.syntax_sections,
        failure_worklist,
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
            never_seen_nodekinds: vec![
                "HereDoc".to_string(),
                "MatchRegex".to_string(),
                "Transliteration".to_string(),
                "VersionLiteral".to_string(),
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
        assert!(
            result.contains("Never-seen node kinds (4): `HereDoc`, `MatchRegex`, `Transliteration`, `VersionLiteral`."),
            "metrics bullets should include explicit missing node-kind set",
        );
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
    fn test_failure_cluster_worklist_renders_categories_and_examples() {
        use std::collections::BTreeMap;
        let report = super::super::super::parser_corpus_sweep::SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-11T00:00:00Z".to_string(),
            corpus_profile: "system".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "5.038".to_string(),
            total_files: 4,
            files_unreadable: 0,
            clean_files: 0,
            files_with_errors: 4,
            total_error_nodes: 8,
            first_error_buckets: BTreeMap::from([
                ("invalid_substitution_modifier".to_string(), 2),
                ("unclosed_brace".to_string(), 1),
                ("unexpected_token_in_expr".to_string(), 1),
            ]),
            files_by_bucket: BTreeMap::from([
                (
                    "invalid_substitution_modifier".to_string(),
                    vec!["lib/Foo.pm".to_string()],
                ),
                ("unclosed_brace".to_string(), vec!["lib/Bar.pm".to_string()]),
            ]),
            file_results: vec![],
            elapsed_secs: 1.0,
            phase_timings: None,
            median_error_density_per_1k_loc: None,
            slowest_files: vec![],
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: Some(report),
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
        };
        let rendered = format_failure_worklist(&metrics, None);
        assert!(rendered.contains("transliteration / quote parsing"));
        assert!(rendered.contains("heredoc / delimiter handling"));
        assert!(rendered.contains("recovery-only failures"));
        assert!(rendered.contains("`lib/Foo.pm`"));
        assert!(rendered.contains("`lib/Bar.pm`"));
    }
}

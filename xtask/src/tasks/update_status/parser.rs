//! Parser subsystem status generator.
//!
//! Owns corpus tracking, sweep report loading, and parser.md generation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

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
    fn as_label(self) -> &'static str {
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

#[derive(Debug, Default, Clone)]
struct ClusterSummary {
    count: usize,
    representative_buckets: Vec<String>,
    representative_files: Vec<String>,
}

fn classify_failure_bucket(bucket: &str) -> FailureCluster {
    let lower = bucket.to_ascii_lowercase();
    if lower.contains("recover")
        || lower.contains("incomplete")
        || lower.contains("unexpected_token_in_expr")
    {
        return FailureCluster::RecoveryOnly;
    }
    if lower.contains("delimiter")
        || lower.contains("unclosed_")
        || lower.contains("substitution")
        || lower.contains("angle")
        || lower.contains("bracket")
        || lower.contains("paren")
        || lower.contains("brace")
    {
        return FailureCluster::HeredocDelimiter;
    }
    if lower.contains("package")
        || lower.contains("module")
        || lower.contains("import")
        || lower.contains("identifier")
        || lower.contains("signature")
        || lower.contains("variable")
        || lower.contains("check must be followed by a block")
    {
        return FailureCluster::DeclarationPackage;
    }
    if lower.contains("encoding")
        || lower.contains("utf")
        || lower.contains("unicode")
        || lower.contains("wide character")
        || lower.contains("multibyte")
    {
        return FailureCluster::EncodingMultibyte;
    }
    if lower.contains("quote")
        || lower.contains("translit")
        || lower.contains("tr/")
        || lower.contains("y/")
        || lower.contains("string")
    {
        return FailureCluster::TransliterationQuote;
    }
    FailureCluster::Other
}

fn build_failure_worklist(report: &super::super::parser_corpus_sweep::SweepReport) -> String {
    if report.first_error_buckets.is_empty() {
        return format!(
            "| {} | 0 | n/a | n/a |\n| {} | 0 | n/a | n/a |\n| {} | 0 | n/a | n/a |\n| {} | 0 | n/a | n/a |\n| {} | 0 | n/a | n/a |\n| {} | 0 | n/a | n/a |",
            FailureCluster::TransliterationQuote.as_label(),
            FailureCluster::DeclarationPackage.as_label(),
            FailureCluster::HeredocDelimiter.as_label(),
            FailureCluster::RecoveryOnly.as_label(),
            FailureCluster::EncodingMultibyte.as_label(),
            FailureCluster::Other.as_label()
        );
    }

    let mut clusters: BTreeMap<FailureCluster, ClusterSummary> = BTreeMap::new();
    for (bucket, count) in &report.first_error_buckets {
        let cluster = classify_failure_bucket(bucket);
        let summary = clusters.entry(cluster).or_default();
        summary.count += count;
        summary.representative_buckets.push(bucket.clone());
        if let Some(files) = report.files_by_bucket.get(bucket) {
            summary.representative_files.extend(files.iter().take(2).cloned());
        }
    }

    let ordered = [
        FailureCluster::TransliterationQuote,
        FailureCluster::DeclarationPackage,
        FailureCluster::HeredocDelimiter,
        FailureCluster::RecoveryOnly,
        FailureCluster::EncodingMultibyte,
        FailureCluster::Other,
    ];

    ordered
        .into_iter()
        .map(|cluster| {
            let details = clusters.get(&cluster).cloned().unwrap_or_default();
            let mut buckets = details.representative_buckets;
            buckets.sort();
            buckets.truncate(2);
            let bucket_note =
                if buckets.is_empty() { "n/a".to_string() } else { buckets.join(", ") };

            let mut files = details.representative_files;
            files.sort();
            files.dedup();
            files.truncate(2);
            let example_note =
                if files.is_empty() { "n/a".to_string() } else { files.join("<br>") };
            format!(
                "| {} | {} | {} | {} |",
                cluster.as_label(),
                details.count,
                bucket_note,
                example_note
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_failure_worklist_source(
    label: &str,
    report: &super::super::parser_corpus_sweep::SweepReport,
) -> String {
    let categories_table = build_failure_worklist(report);
    format!(
        "### {label}\n\
         - Files with errors: `{}`\n\
         - Baseline date: `{}`\n\n\
         | Category | Count | Representative buckets | Representative examples |\n\
         | --- | ---: | --- | --- |\n\
         {}\n",
        report.files_with_errors,
        short_day(&report.timestamp),
        categories_table
    )
}

fn render_never_seen_nodekinds(summary: &super::super::corpus_audit::StatusSummary) -> String {
    if summary.never_seen_nodekinds.is_empty() {
        return "- None (all node kinds observed in corpus parses).".to_string();
    }
    summary
        .never_seen_nodekinds
        .iter()
        .map(|kind| format!("- `{kind}`"))
        .collect::<Vec<_>>()
        .join("\n")
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
            let kinds_preview =
                summary.never_seen_nodekinds.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let notes = if kinds_preview.is_empty() {
                format!("{never_seen} never-seen node kinds")
            } else {
                format!("{never_seen} never-seen node kinds ({kinds_preview})")
            };
            format!(
                "| **Node-kind coverage** | {}/{} ({:.1}%) | {} | `corpus_audit` |",
                summary.nodekind_covered, summary.nodekind_total, pct, notes,
            )
        },
    );

    let never_seen_nodekinds = metrics.project_corpus.as_ref().map_or_else(
        || "- UNVERIFIED (live repo scan unavailable).".to_string(),
        render_never_seen_nodekinds,
    );

    let failure_worklist = match (&metrics.system_receipt, &metrics.cpan_receipt) {
        (None, None) => "- UNVERIFIED (baseline receipts unavailable).".to_string(),
        (system, cpan) => {
            let mut sections = Vec::new();
            if let Some(report) = system {
                sections.push(render_failure_worklist_source("Ubuntu system Perl", report));
            }
            if let Some(report) = cpan {
                sections.push(render_failure_worklist_source("CPAN top 1000", report));
            }
            sections.join("\n")
        }
    };

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
        "<!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->",
        "<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->",
        &never_seen_nodekinds,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_FAILURE_WORKLIST -->",
        "<!-- END: PARSER_FAILURE_WORKLIST -->",
        &failure_worklist,
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
                "Reserved".to_string(),
                "PackageBlock".to_string(),
                "BitwiseNot".to_string(),
                "SpecialLiteral".to_string(),
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
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("65/69"), "nodekind row missing 65/69");
        assert!(result.contains("94.2"), "nodekind row missing 94.2%");
        assert!(result.contains("4 never-seen"), "nodekind row missing never-seen count");
        assert!(
            result.contains("Reserved"),
            "never-seen section should list representative node kinds"
        );
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
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
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
    fn test_classify_failure_bucket_routing() {
        // RecoveryOnly matches take highest priority
        assert_eq!(
            classify_failure_bucket("unexpected_token_in_expr"),
            FailureCluster::RecoveryOnly,
            "catch-all expr token bucket must be RecoveryOnly"
        );
        assert_eq!(
            classify_failure_bucket("Incomplete arrow expression"),
            FailureCluster::RecoveryOnly,
            "'incomplete' substring routes to RecoveryOnly"
        );

        // HeredocDelimiter for bracket/brace/paren/substitution errors
        assert_eq!(
            classify_failure_bucket("expected_left_brace"),
            FailureCluster::HeredocDelimiter,
            "brace errors map to HeredocDelimiter cluster"
        );
        assert_eq!(
            classify_failure_bucket("unclosed_substitution_delimiter"),
            FailureCluster::HeredocDelimiter,
            "unclosed_ prefix maps to HeredocDelimiter"
        );

        // DeclarationPackage for identifier/variable/signature errors
        assert_eq!(
            classify_failure_bucket("expected_identifier"),
            FailureCluster::DeclarationPackage,
            "'identifier' routes to DeclarationPackage"
        );
        assert_eq!(
            classify_failure_bucket("expected_variable"),
            FailureCluster::DeclarationPackage,
            "'variable' routes to DeclarationPackage"
        );
        assert_eq!(
            classify_failure_bucket("CHECK must be followed by a block"),
            FailureCluster::DeclarationPackage,
            "CHECK block error routes to DeclarationPackage"
        );

        // EncodingMultibyte for utf/unicode/wide character errors
        assert_eq!(
            classify_failure_bucket("wide character in syswrite"),
            FailureCluster::EncodingMultibyte,
            "'wide character' substring routes to EncodingMultibyte"
        );

        // TransliterationQuote for quote/translit/tr/y/string errors
        assert_eq!(
            classify_failure_bucket("tr/abc/xyz/ misparse"),
            FailureCluster::TransliterationQuote,
            "'tr/' routes to TransliterationQuote"
        );
        assert_eq!(
            classify_failure_bucket("unclosed string literal"),
            FailureCluster::TransliterationQuote,
            "'string' routes to TransliterationQuote"
        );

        // Other for unrecognized errors
        assert_eq!(
            classify_failure_bucket("expected_comma"),
            FailureCluster::Other,
            "comma errors fall through to Other"
        );
        assert_eq!(
            classify_failure_bucket("expected_colon"),
            FailureCluster::Other,
            "colon errors fall through to Other"
        );
    }

    #[test]
    fn test_build_failure_worklist_with_populated_receipt() -> Result<()> {
        use std::collections::BTreeMap;

        let mut buckets = BTreeMap::new();
        buckets.insert("expected_variable".to_string(), 6usize);
        buckets.insert("expected_left_brace".to_string(), 10usize);
        buckets.insert("unexpected_token_in_expr".to_string(), 3usize);
        buckets.insert("expected_colon".to_string(), 5usize);

        let mut files_by_bucket = BTreeMap::new();
        files_by_bucket.insert(
            "expected_variable".to_string(),
            vec!["/usr/share/perl5/Foo.pm".to_string()],
        );
        files_by_bucket.insert(
            "expected_left_brace".to_string(),
            vec!["/usr/share/perl5/Bar.pm".to_string(), "/usr/share/perl5/Baz.pm".to_string()],
        );

        let report = super::super::super::parser_corpus_sweep::SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-09T00:00:00Z".to_string(),
            corpus_profile: "system".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "5.038".to_string(),
            total_files: 200,
            files_unreadable: 0,
            clean_files: 176,
            files_with_errors: 24,
            total_error_nodes: 100,
            first_error_buckets: buckets,
            files_by_bucket,
            file_results: vec![],
            elapsed_secs: 1.0,
            phase_timings: None,
            median_error_density_per_1k_loc: None,
            slowest_files: vec![],
        };

        let worklist = build_failure_worklist(&report);

        // DeclarationPackage: expected_variable (6)
        assert!(
            worklist.contains("declaration / package parsing"),
            "DeclarationPackage cluster missing from worklist"
        );
        // HeredocDelimiter: expected_left_brace (10)
        assert!(
            worklist.contains("heredoc / delimiter handling"),
            "HeredocDelimiter cluster missing from worklist"
        );
        // RecoveryOnly: unexpected_token_in_expr (3)
        assert!(
            worklist.contains("recovery-only failures"),
            "RecoveryOnly cluster missing from worklist"
        );
        // Other: expected_colon (5)
        assert!(worklist.contains("other"), "Other cluster missing from worklist");

        // Counts should appear in the output rows
        assert!(worklist.contains("| 6 |"), "DeclarationPackage count (6) not found");
        assert!(worklist.contains("| 10 |"), "HeredocDelimiter count (10) not found");
        assert!(worklist.contains("| 3 |"), "RecoveryOnly count (3) not found");
        assert!(worklist.contains("| 5 |"), "Other count (5) not found");

        // Rows are deterministic — same input always produces same output
        let worklist2 = build_failure_worklist(&report);
        assert_eq!(worklist, worklist2, "cluster worklist must be deterministic");

        Ok(())
    }

    #[test]
    fn test_build_failure_worklist_empty_buckets() {
        use std::collections::BTreeMap;
        use super::super::super::parser_corpus_sweep::SweepReport;

        let report = SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-09T00:00:00Z".to_string(),
            corpus_profile: "system".to_string(),
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
            elapsed_secs: 0.5,
            phase_timings: None,
            median_error_density_per_1k_loc: None,
            slowest_files: vec![],
        };

        let worklist = build_failure_worklist(&report);
        // All six clusters should appear with 0 counts
        assert!(worklist.contains("transliteration / quote parsing"), "TransliterationQuote row missing in empty case");
        assert!(worklist.contains("declaration / package parsing"), "DeclarationPackage row missing in empty case");
        assert!(worklist.contains("| 0 |"), "empty worklist should show 0 counts");
        // Output should have 6 rows
        let row_count = worklist.lines().count();
        assert_eq!(row_count, 6, "empty worklist must have exactly 6 rows, got {row_count}");
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
                        <!-- BEGIN: PARSER_NEVER_SEEN_NODEKINDS -->\nold\n<!-- END: PARSER_NEVER_SEEN_NODEKINDS -->\n\
                        <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
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
}

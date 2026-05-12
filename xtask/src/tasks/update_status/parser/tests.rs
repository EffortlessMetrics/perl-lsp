use super::super::token;
use super::accuracy::{
    ParserAccuracyArtifactSummary, ParserAccuracyDenominator, ParserAccuracyFamilySummary,
    ParserAccuracyMetricSummary,
};
use super::failure::{
    FailureCluster, build_failure_bucket_details, build_failure_worklist, classify_failure_bucket,
};
use super::*;
use color_eyre::eyre::Result;

const PARSER_STATUS_MARKER_NAMES: [&str; 10] = [
    "PARSER_TRACKING_TABLE",
    "PARSER_PERFORMANCE_TABLE",
    "PARSER_METRICS_BULLETS",
    "TOKEN_HEALTH_TABLE",
    "PARSER_NODEKIND_ROW",
    "PARSER_RELIABILITY_ROW",
    "PARSER_STRICT_CLEAN_ROW",
    "PARSER_ACCURACY_SUMMARY",
    "PARSER_FAILURE_WORKLIST",
    "PARSER_FAILURE_BUCKETS",
];

fn parser_status_template() -> &'static str {
    "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
         <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
         <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
         <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
         <!-- BEGIN: PARSER_ACCURACY_SUMMARY -->\nold\n<!-- END: PARSER_ACCURACY_SUMMARY -->\n\
         <!-- BEGIN: PARSER_PERFORMANCE_TABLE -->\nold\n<!-- END: PARSER_PERFORMANCE_TABLE -->\n\
         <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n\
         <!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->\n\
         <!-- BEGIN: PARSER_FAILURE_WORKLIST -->\nold\n<!-- END: PARSER_FAILURE_WORKLIST -->\n\
         <!-- BEGIN: PARSER_FAILURE_BUCKETS -->\nold\n<!-- END: PARSER_FAILURE_BUCKETS -->\n"
}

#[test]
fn test_corpus_section_count() -> Result<()> {
    let root = crate::utils::project_root()?;
    let sections = count_corpus_sections(&root);
    assert!(sections > 0, "expected nonzero corpus sections");
    Ok(())
}

#[test]
fn test_parser_status_marker_contract() -> Result<()> {
    let root = crate::utils::project_root()?;
    let target_file = "docs/project/status/parser.md";
    let parser_status = std::fs::read_to_string(root.join(target_file))?;

    for marker in PARSER_STATUS_MARKER_NAMES {
        let begin_marker = format!("<!-- BEGIN: {marker} -->");
        let end_marker = format!("<!-- END: {marker} -->");

        let begin_count = parser_status.match_indices(&begin_marker).count();
        assert_eq!(
            begin_count, 1,
            "missing or duplicate marker in {target_file}: expected BEGIN marker exactly once: `{begin_marker}`; found {begin_count}"
        );

        let end_count = parser_status.match_indices(&end_marker).count();
        assert_eq!(
            end_count, 1,
            "missing or duplicate marker in {target_file}: expected END marker exactly once: `{end_marker}`; found {end_count}"
        );
    }

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
        nodekind_never_seen: 4,
        nodekind_allowlisted_never_seen: 4,
        nodekind_actionable_never_seen: 0,
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
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let template = parser_status_template();
    let result = generate_parser_status(&metrics, template)?;
    assert!(result.contains("65/69"), "nodekind row missing 65/69");
    assert!(result.contains("94.2"), "nodekind row missing 94.2%");
    assert!(result.contains("0 actionable never-seen"), "nodekind row missing actionable count");
    assert!(result.contains("4 recovery-only allowlisted"), "nodekind row missing allowlist count");
    assert!(
        result.contains("insufficient_data"),
        "strict-clean no-receipt row should report insufficient_data"
    );
    assert!(!result.contains("10/10"), "strict-clean no-receipt row must not show 10/10");
    Ok(())
}

#[test]
fn test_nodekind_gap_note_distinguishes_actionable_and_allowlisted() {
    let mut summary = super::super::super::corpus_audit::StatusSummary {
        total_files: 91,
        ok_files: 91,
        error_files: 0,
        timeout_files: 0,
        panic_files: 0,
        test_corpus_files: 69,
        perl_corpus_files: 22,
        nodekind_covered: 65,
        nodekind_total: 69,
        nodekind_never_seen: 4,
        nodekind_allowlisted_never_seen: 4,
        nodekind_actionable_never_seen: 0,
        ga_covered: 12,
        ga_total: 12,
    };

    assert_eq!(
        format_nodekind_gap_note(&summary),
        "0 actionable never-seen; 4 recovery-only allowlisted"
    );

    summary.nodekind_never_seen = 3;
    summary.nodekind_allowlisted_never_seen = 1;
    summary.nodekind_actionable_never_seen = 2;

    assert_eq!(
        format_nodekind_gap_note(&summary),
        "2 actionable never-seen; 1 recovery-only allowlisted; 3 total never-seen"
    );
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
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let template = parser_status_template();
    let result = generate_parser_status(&metrics, template)?;
    assert!(
        result.contains("insufficient_data"),
        "strict-clean no-receipt row must report insufficient_data"
    );
    assert!(
        result.contains("common-corpus-check"),
        "strict-clean no-receipt row must mention the command"
    );
    assert!(
        result.contains("10 pinned modules"),
        "strict-clean no-receipt row must keep the pinned module denominator visible"
    );
    Ok(())
}

#[test]
fn test_parser_failure_worklist_no_receipt_reports_insufficient_data() -> Result<()> {
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let result = generate_parser_status(&metrics, parser_status_template())?;
    assert!(
        result.contains("insufficient_data (no receipt"),
        "failure worklist must not render missing receipt as a zero-count row"
    );
    assert!(
        result.contains("| insufficient_data |"),
        "failure worklist missing-receipt count must be insufficient_data"
    );
    Ok(())
}

#[test]
fn test_parser_accuracy_missing_artifact_reports_insufficient_data() -> Result<()> {
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let result = generate_parser_status(&metrics, parser_status_template())?;
    assert!(
        result.contains("| **Accuracy denominator** | insufficient_data |"),
        "missing parser accuracy artifact must report insufficient_data"
    );
    assert!(
        result.contains("cargo xtask metrics parser-accuracy --json"),
        "missing parser accuracy artifact row should include the generation command"
    );
    assert!(
        result.contains(".kiro/specs/parser-accuracy-observability"),
        "parser accuracy status should link the authoritative spec"
    );
    Ok(())
}

#[test]
fn test_parser_accuracy_artifact_renders_denominator_and_metric_rows() -> Result<()> {
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: Some(ParserAccuracyArtifactSummary {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            cadence: "pr".to_string(),
            denominator: ParserAccuracyDenominator {
                fixture_count: 2,
                fixture_family_count: 2,
                scored_line_count: 3,
                scored_symbol_count: 2,
                fully_labeled_region_count: 1,
                partial_labeled_region_count: 1,
                unknown_region_count: 1,
                negative_region_count: 1,
                dynamic_boundary_case_count: 1,
                unsupported_construct_case_count: 0,
                real_project_file_count: 0,
                generated_fixture_count: 0,
                hand_labeled_fixture_count: 2,
            },
            families: vec![
                ParserAccuracyFamilySummary {
                    family: "dynamic_require".to_string(),
                    fixture_count: 1,
                },
                ParserAccuracyFamilySummary { family: "packages".to_string(), fixture_count: 1 },
            ],
            metrics: vec![
                ParserAccuracyMetricSummary::Measured {
                    metric: "denominator_fixture_count".to_string(),
                    value: 2.0,
                    sample_count: 2,
                },
                ParserAccuracyMetricSummary::Measured {
                    metric: "line_construct_f1".to_string(),
                    value: 1.0,
                    sample_count: 6,
                },
                ParserAccuracyMetricSummary::Measured {
                    metric: "whitespace_invariance_rate".to_string(),
                    value: 0.32,
                    sample_count: 44,
                },
            ],
            failure_packets: vec![],
        }),
        token_metrics: token::token_metrics_fixture(),
    };
    let result = generate_parser_status(&metrics, parser_status_template())?;
    assert!(
        result.contains("| **Accuracy denominator** | 2 fixtures / 2 families |"),
        "parser accuracy denominator row should render fixture and family counts"
    );
    assert!(
        result.contains("3 scored lines, 2 scored symbols"),
        "parser accuracy denominator row should render labeled denominator counts"
    );
    assert!(
        result.contains("dynamic_require (1), packages (1)"),
        "parser accuracy family row should render family inventory"
    );
    assert!(
        result.contains("line_construct_f1=1.0"),
        "measured accuracy scorer rows should render their values"
    );
    assert!(
        result.contains("whitespace_invariance_rate=0.3 (trailing whitespace; n=44)"),
        "whitespace invariance summary must disclose its sampled trailing-whitespace basis"
    );
    Ok(())
}

#[test]
fn test_read_parser_accuracy_artifact_loads_target_metrics() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let metrics_dir = tmp.path().join("target").join("metrics");
    std::fs::create_dir_all(&metrics_dir)?;
    std::fs::write(
        metrics_dir.join("parser_accuracy.json"),
        r#"{
  "schema_version": 1,
  "subsystem": "parser_accuracy",
  "generated_at": "2026-05-02T15:00:00Z",
  "commit": "abc123",
  "cadence": "pr",
  "denominator": {
    "fixture_count": 2,
    "fixture_family_count": 2,
    "scored_line_count": 3,
    "scored_symbol_count": 2,
    "fully_labeled_region_count": 1,
    "partial_labeled_region_count": 1,
    "unknown_region_count": 1,
    "negative_region_count": 1,
    "dynamic_boundary_case_count": 1,
    "unsupported_construct_case_count": 0,
    "real_project_file_count": 0,
    "generated_fixture_count": 0,
    "hand_labeled_fixture_count": 2
  },
  "families": [
    {
      "family": "packages",
      "fixture_count": 1,
      "label_modes": ["full"],
      "scored_line_count": 2,
      "scored_symbol_count": 1,
      "dynamic_boundary_case_count": 0,
      "unsupported_construct_case_count": 0
    }
  ],
  "metrics": [
    {
      "state": "insufficient_data",
      "metric": "line_construct_f1",
      "reason": "line-level gold scorer is not wired yet",
      "sample_count": 0,
      "confidence": "low"
    }
  ],
  "failure_packets": [],
  "gold_drift": {},
  "metric_runtime": {}
}"#,
    )?;

    let artifact = read_parser_accuracy_artifact(tmp.path())
        .ok_or_else(|| color_eyre::eyre::eyre!("valid parser accuracy artifact should load"))?;
    assert_eq!(artifact.denominator.fixture_count, 2);
    assert_eq!(artifact.metrics.len(), 1);
    Ok(())
}

#[test]
fn test_parser_salvage_missing_reports_insufficient_data() {
    assert_eq!(format_salvage_rate(None), "insufficient_data salvage");
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
        ParserPerfMetric { iterations: 35, median_ns: 118_046, p95_ns: 277_118, mean_ns: 242_863 },
    );
    // Include one metric intentionally absent from the map so the None
    // branch of format_perf_metric_row is also exercised in this test.

    let scorecard =
        ParserPerformanceScorecard { generated_at_epoch_s: 1_777_010_864, metrics: metrics_map };

    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: Some(scorecard),
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };

    let template = parser_status_template();

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
fn parser_failure_worklist_builds_cluster_and_bucket_details_with_populated_receipt() -> Result<()>
{
    use std::collections::BTreeMap;

    let mut buckets = BTreeMap::new();
    buckets.insert("expected_variable".to_string(), 6usize);
    buckets.insert("expected_left_brace".to_string(), 10usize);
    buckets.insert("unexpected_token_in_expr".to_string(), 3usize);
    buckets.insert("expected_colon".to_string(), 5usize);

    let mut files_by_bucket = BTreeMap::new();
    files_by_bucket
        .insert("expected_variable".to_string(), vec!["/usr/share/perl5/Foo.pm".to_string()]);
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
        total_dirty_files: 24,
        files_with_structured_recovery_only: 0,
        files_with_error_nodes: 24,
        files_with_catastrophic_parse_failure: 0,
        total_error_nodes: 100,
        recovered_node_count: 0,
        first_unrecovered_error_node_buckets: std::collections::BTreeMap::new(),
        first_error_buckets: buckets,
        files_by_bucket,
        file_results: vec![],
        elapsed_secs: 1.0,
        phase_timings: None,
        median_error_density_per_1k_loc: None,
        recovery_salvage_rate: None,
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

    let bucket_details = build_failure_bucket_details(&report);
    assert!(bucket_details.contains("| declaration / package parsing | `expected_variable` | 6 |"));
    assert!(
        bucket_details.contains("| heredoc / delimiter handling | `expected_left_brace` | 10 |")
    );
    assert!(bucket_details.contains("| recovery-only failures | `unexpected_token_in_expr` | 3 |"));
    assert!(bucket_details.contains("| other | `expected_colon` | 5 |"));
    assert!(matches!(
        (
            bucket_details.find("expected_variable"),
            bucket_details.find("expected_left_brace"),
            bucket_details.find("unexpected_token_in_expr"),
            bucket_details.find("expected_colon")
        ),
        (Some(declaration), Some(heredoc), Some(recovery), Some(other))
            if declaration < heredoc && heredoc < recovery && recovery < other
    ));

    Ok(())
}

#[test]
fn parser_failure_worklist_replaces_cluster_and_bucket_status_markers() -> Result<()> {
    use std::collections::BTreeMap;

    let report = super::super::super::parser_corpus_sweep::SweepReport {
        schema_version: "1".to_string(),
        commit: "abc".to_string(),
        timestamp: "2026-04-09T00:00:00Z".to_string(),
        corpus_profile: "system".to_string(),
        corpus_roots: vec![],
        resolved_roots_count: 0,
        perl_version: "5.038".to_string(),
        total_files: 20,
        files_unreadable: 0,
        clean_files: 18,
        files_with_errors: 2,
        total_dirty_files: 2,
        files_with_structured_recovery_only: 0,
        files_with_error_nodes: 2,
        files_with_catastrophic_parse_failure: 0,
        total_error_nodes: 2,
        recovered_node_count: 0,
        first_unrecovered_error_node_buckets: BTreeMap::new(),
        first_error_buckets: BTreeMap::from([
            ("unclosed_paren_identifier".to_string(), 2usize),
            ("unexpected_token_in_expr".to_string(), 1usize),
        ]),
        files_by_bucket: BTreeMap::new(),
        file_results: vec![],
        elapsed_secs: 1.0,
        phase_timings: None,
        median_error_density_per_1k_loc: None,
        recovery_salvage_rate: None,
        slowest_files: vec![],
    };
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: Some(ParserSweepReceipt::with_recovery_shape(report)),
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };

    let result = generate_parser_status(&metrics, parser_status_template())?;

    assert!(result.contains("| heredoc / delimiter handling | 2 |"));
    assert!(result.contains("| recovery-only failures | 1 |"));
    assert!(result.contains("| heredoc / delimiter handling | `unclosed_paren_identifier` | 2 |"));
    assert!(result.contains("| recovery-only failures | `unexpected_token_in_expr` | 1 |"));
    assert!(!result.contains("\nold\n"), "all parser status markers should be replaced");
    Ok(())
}

#[test]
fn parser_failure_worklist_handles_empty_buckets() {
    use super::super::super::parser_corpus_sweep::SweepReport;
    use std::collections::BTreeMap;

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
        total_dirty_files: 0,
        files_with_structured_recovery_only: 0,
        files_with_error_nodes: 0,
        files_with_catastrophic_parse_failure: 0,
        total_error_nodes: 0,
        recovered_node_count: 0,
        first_unrecovered_error_node_buckets: BTreeMap::new(),
        first_error_buckets: BTreeMap::new(),
        files_by_bucket: BTreeMap::new(),
        file_results: vec![],
        elapsed_secs: 0.5,
        phase_timings: None,
        median_error_density_per_1k_loc: None,
        recovery_salvage_rate: None,
        slowest_files: vec![],
    };

    let worklist = build_failure_worklist(&report);
    let bucket_details = build_failure_bucket_details(&report);
    // All six clusters should appear with 0 counts
    assert!(
        worklist.contains("transliteration / quote parsing"),
        "TransliterationQuote row missing in empty case"
    );
    assert!(
        worklist.contains("declaration / package parsing"),
        "DeclarationPackage row missing in empty case"
    );
    assert!(worklist.contains("| 0 |"), "empty worklist should show 0 counts");
    // Output should have 6 rows
    let row_count = worklist.lines().count();
    assert_eq!(row_count, 6, "empty worklist must have exactly 6 rows, got {row_count}");
    assert_eq!(
        bucket_details, "| none | n/a | 0 |",
        "raw bucket detail should be explicit when no buckets are present"
    );
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
        total_dirty_files: 0,
        files_with_structured_recovery_only: 0,
        files_with_error_nodes: 0,
        files_with_catastrophic_parse_failure: 0,
        total_error_nodes: 0,
        recovered_node_count: 0,
        first_unrecovered_error_node_buckets: BTreeMap::new(),
        first_error_buckets: BTreeMap::new(),
        files_by_bucket: BTreeMap::new(),
        file_results: vec![],
        elapsed_secs: 1.0,
        phase_timings: None,
        median_error_density_per_1k_loc: None,
        recovery_salvage_rate: None,
        slowest_files: vec![],
    };
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: Some(ParserSweepReceipt::with_recovery_shape(receipt)),
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let template = parser_status_template();
    let result = generate_parser_status(&metrics, template)?;
    assert!(result.contains("10/10"), "strict-clean row missing 10/10");
    assert!(result.contains("100%"), "strict-clean row missing 100%");
    assert!(result.contains("10 pinned modules"), "strict-clean row missing pinned modules note");
    Ok(())
}

#[test]
fn test_parser_tracking_old_cpan_receipt_missing_recovery_shape_reports_insufficient_data()
-> Result<()> {
    use std::collections::BTreeMap;

    let receipt = super::super::super::parser_corpus_sweep::SweepReport {
        schema_version: "1.2.0".to_string(),
        commit: "old".to_string(),
        timestamp: "2026-04-09T00:00:00Z".to_string(),
        corpus_profile: "cpan".to_string(),
        corpus_roots: vec![],
        resolved_roots_count: 151,
        perl_version: "5.038002".to_string(),
        total_files: 9_372,
        files_unreadable: 6,
        clean_files: 8_931,
        files_with_errors: 435,
        total_dirty_files: 0,
        files_with_structured_recovery_only: 0,
        files_with_error_nodes: 0,
        files_with_catastrophic_parse_failure: 0,
        total_error_nodes: 3_015,
        recovered_node_count: 0,
        first_unrecovered_error_node_buckets: BTreeMap::new(),
        first_error_buckets: BTreeMap::from([("unexpected_token_in_expr".to_string(), 435)]),
        files_by_bucket: BTreeMap::new(),
        file_results: vec![],
        elapsed_secs: 1.0,
        phase_timings: None,
        median_error_density_per_1k_loc: None,
        recovery_salvage_rate: None,
        slowest_files: vec![],
    };
    let metrics = ParserMetrics {
        syntax_sections: 611,
        system_receipt: None,
        cpan_receipt: Some(ParserSweepReceipt::without_recovery_shape(receipt)),
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 10,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };

    let result = generate_parser_status(&metrics, parser_status_template())?;
    assert!(
        result.contains("insufficient_data salvage"),
        "old CPAN receipt must not fabricate a salvage rate"
    );
    assert!(
        result.contains("`insufficient_data` recovery-only"),
        "old CPAN receipt must mark missing recovery-only count as insufficient_data"
    );
    assert!(
        result.contains("`insufficient_data` ERROR-node files"),
        "old CPAN receipt must mark missing ERROR-node file count as insufficient_data"
    );
    assert!(
        result.contains("`insufficient_data` catastrophic"),
        "old CPAN receipt must mark missing catastrophic count as insufficient_data"
    );
    assert!(
        !result.contains("`0` recovery-only, `0` ERROR-node files, `0` catastrophic"),
        "old CPAN receipt must not render missing recovery-shape fields as zero"
    );
    Ok(())
}

/// Verify the TOKEN_HEALTH_TABLE block is rendered correctly from the fixture.
///
/// This is the only test that asserts on the TOKEN_HEALTH_TABLE section —
/// all other tests use `token_metrics_fixture()` but only check unrelated rows.
/// Without this test, a format-string argument transposition in the table
/// builder would go undetected.
#[test]
fn token_health_table_renders_correctly_from_fixture() -> Result<()> {
    let metrics = ParserMetrics {
        syntax_sections: 0,
        system_receipt: None,
        cpan_receipt: None,
        project_corpus: None,
        common_corpus_receipt: None,
        common_corpus_pinned: 0,
        performance_scorecard: None,
        parser_accuracy: None,
        token_metrics: token::token_metrics_fixture(),
    };
    let template = parser_status_template();
    let result = generate_parser_status(&metrics, template)?;

    // The fixture has variant_count=132 and metadata_coverage_count=132.
    // The table row format is: `{count}/{total} ({status})`.
    assert!(result.contains("132/132"), "TOKEN_HEALTH_TABLE must show 132/132 coverage");
    assert!(result.contains("PASS"), "TOKEN_HEALTH_TABLE must show PASS status from fixture");
    // Category partition status from fixture
    assert!(
        result.contains("132 tokens partitioned"),
        "TOKEN_HEALTH_TABLE must show category_partition_status from fixture"
    );
    // Lexer+parser conformance status from fixture
    assert!(
        result.contains("lexer + parser-core"),
        "TOKEN_HEALTH_TABLE must show lexer_parser_conformance_status"
    );
    // Performance row: fixture returns UNVERIFIED (no scorecard file present)
    assert!(
        result.contains("UNVERIFIED"),
        "TOKEN_HEALTH_TABLE must show UNVERIFIED when no perf scorecard"
    );
    // The old marker content must be replaced — the block is not left as "old"
    assert!(
        !result
            .contains("<!-- BEGIN: TOKEN_HEALTH_TABLE -->\nold\n<!-- END: TOKEN_HEALTH_TABLE -->"),
        "TOKEN_HEALTH_TABLE block must be replaced — replace_block did not fire"
    );
    Ok(())
}

#[test]
fn status_marker_contract_parser_md() -> Result<()> {
    let root = crate::utils::project_root()?;
    let target_file = "docs/project/status/parser.md";
    let parser_status_doc = std::fs::read_to_string(root.join(target_file))?;

    for marker_name in PARSER_STATUS_MARKER_NAMES {
        let (begin_marker, end_marker) = parser_marker_bounds(marker_name);
        let begin_count = parser_status_doc.matches(&begin_marker).count();
        let end_count = parser_status_doc.matches(&end_marker).count();

        assert_eq!(
            begin_count, 1,
            "status marker contract violation: missing or duplicate BEGIN marker for {marker_name} in {target_file}; expected exactly one `{begin_marker}` and one `{end_marker}`",
        );
        assert_eq!(
            end_count, 1,
            "status marker contract violation: missing or duplicate END marker for {marker_name} in {target_file}; expected exactly one `{begin_marker}` and one `{end_marker}`",
        );
    }
    Ok(())
}

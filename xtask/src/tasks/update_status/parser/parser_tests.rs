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
    assert!(result.contains("unverified"), "strict-clean no-receipt row should say 'unverified'");
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
        unreadable_files: vec![],
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
        dirty_classification: None,
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
    assert!(result.contains("10 pinned modules"), "strict-clean row missing pinned modules note");
    Ok(())
}

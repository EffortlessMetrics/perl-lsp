use super::*;

#[test]
fn test_collect_per_crate_mutation_from_mock_file() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let out_dir = dir.path().join("mutants.out");
    fs::create_dir_all(&out_dir)?;
    let json = r#"[
        {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs","genre":"FnValue"},
        {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs","genre":"BinaryOperator"},
        {"package":"perl-parser","file":"crates/perl-parser/src/lib.rs","genre":"FnValue"}
    ]"#;
    fs::write(out_dir.join("mutants.json"), json)?;
    let result = collect_per_crate_mutation(dir.path());
    assert_eq!(result.get("perl-quote"), Some(&2));
    assert_eq!(result.get("perl-parser"), Some(&1));
    Ok(())
}

#[test]
fn test_collect_per_crate_mutation_ignores_entries_without_package() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let out_dir = dir.path().join("mutants.out");
    fs::create_dir_all(&out_dir)?;
    let json = r#"[
        {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"},
        {"file":"crates/perl-parser/src/lib.rs","genre":"FnValue"},
        {"package":null,"file":"crates/perl-parser/src/lib.rs"},
        {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"}
    ]"#;
    fs::write(out_dir.join("mutants.json"), json)?;
    let result = collect_per_crate_mutation(dir.path());
    assert_eq!(result.len(), 1);
    assert_eq!(result.get("perl-quote"), Some(&2));
    Ok(())
}

#[test]
fn test_collect_per_crate_mutation_ignores_blank_package_names() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let out_dir = dir.path().join("mutants.out");
    fs::create_dir_all(&out_dir)?;
    let json = r#"[
        {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"},
        {"package":"","file":"crates/perl-parser/src/lib.rs"},
        {"package":"   ","file":"crates/perl-parser/src/lib.rs"}
    ]"#;
    fs::write(out_dir.join("mutants.json"), json)?;
    let result = collect_per_crate_mutation(dir.path());
    assert_eq!(result.len(), 1);
    assert_eq!(result.get("perl-quote"), Some(&1));
    Ok(())
}

#[test]
fn test_collect_per_crate_mutation_invalid_json_returns_empty_map() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let out_dir = dir.path().join("mutants.out");
    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("mutants.json"), "{not-json")?;
    let result = collect_per_crate_mutation(dir.path());
    assert!(result.is_empty());
    Ok(())
}

#[test]
fn test_format_crate_quality_table_has_header_and_data() {
    let mut mutation = BTreeMap::new();
    mutation.insert("perl-quote".to_string(), 249);
    let mut tests = BTreeMap::new();
    tests.insert("perl-quote".to_string(), 42);
    let table = format_crate_quality_table(&mutation, &tests);
    assert!(
        table.contains("Crate")
            && table.contains("perl-quote")
            && table.contains("249")
            && table.contains("42")
    );
}

#[test]
fn test_format_crate_quality_table_empty_maps() {
    let table = format_crate_quality_table(&BTreeMap::new(), &BTreeMap::new());
    assert!(table.contains("no data yet"));
}

#[test]
fn test_parse_per_crate_test_counts_parses_unix_and_windows_paths() {
    let output = "Running unittests src/lib.rs \
        (target/debug/deps/perl_parser_core-abc123)\n\
        lexer_edge_case: test\nparser_smoke: test\n\
        Running unittests src/lib.rs \
        (target\\debug\\deps\\perl_workspace-123def.exe)\n\
        index_builds: test\n";
    let counts = parse_per_crate_test_counts(output);
    assert_eq!(counts.get("perl-parser-core"), Some(&2));
    assert_eq!(counts.get("perl-workspace"), Some(&1));
}

#[test]
fn test_parse_per_crate_test_counts_parses_absolute_external_target_paths() {
    let output = "Running unittests src/lib.rs \
        (C:\\Users\\steven\\AppData\\Local\\Temp\\cargo-out\\debug\\deps\\perl_lsp_rs-cafe123.exe)\n\
        lsp_smoke: test\n\
        Running unittests src/lib.rs \
        (/tmp/cargo-out/debug/deps/perl_workspace_index-feed456)\n\
        workspace_indexes: test\n";
    let counts = parse_per_crate_test_counts(output);
    assert_eq!(counts.get("perl-lsp-rs"), Some(&1));
    assert_eq!(counts.get("perl-workspace-index"), Some(&1));
}

#[test]
fn test_parse_per_crate_test_counts_ignores_tests_without_active_crate() {
    let output = "orphan_test: test\n\
        Running unittests src/lib.rs (target/debug/deps/perl_parser_core-abc123)\n\
        parser_smoke: test\n\
        note: test\n\
        Running unittests src/lib.rs (target/debug/deps/perl_lexer-987def)\n\
        lexer_smoke: test\n";
    let counts = parse_per_crate_test_counts(output);
    assert_eq!(counts.get("perl-parser-core"), Some(&2));
    assert_eq!(counts.get("perl-lexer"), Some(&1));
    assert_eq!(counts.len(), 2);
}

// ---------------------------------------------------------------------------
// Receipt-reading tests
// ---------------------------------------------------------------------------

#[test]
fn test_read_diagnostics_p50_ms_parses_editor_ux_md() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let status_dir = dir.path().join("docs/project/status");
    fs::create_dir_all(&status_dir)?;
    let md = "# Editor UX Scorecard\n\n\
        ## Latency (ms)\n\n\
        | Request class | p50 | p50 baseline | p95 | p95 baseline |\n\
        |---|---:|---:|---:|---:|\n\
        | completion | 27.00 | 27.00 | 35.00 | 35.00 |\n\
        | diagnostics | 53.00 | 53.00 | 66.00 | 66.00 |\n\
        | hover | 24.00 | 24.00 | 31.00 | 31.00 |\n";
    fs::write(status_dir.join("editor_ux.md"), md)?;
    let p50 = read_diagnostics_p50_ms(dir.path());
    assert_eq!(p50, Some(53.0));
    Ok(())
}

#[test]
fn test_read_diagnostics_p50_ms_returns_none_when_file_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = read_diagnostics_p50_ms(dir.path());
    assert!(result.is_none());
}

#[test]
fn test_read_diagnostics_p50_ms_returns_none_when_row_absent() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let status_dir = dir.path().join("docs/project/status");
    fs::create_dir_all(&status_dir)?;
    fs::write(status_dir.join("editor_ux.md"), "# no latency table here\n")?;
    let result = read_diagnostics_p50_ms(dir.path());
    assert!(result.is_none());
    Ok(())
}

#[test]
fn test_read_incremental_parse_range_ns_parses_scorecard_json() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let status_dir = dir.path().join("docs/project/status");
    fs::create_dir_all(&status_dir)?;
    let json = r#"{
        "schema_version": 1,
        "generated_at_epoch_s": 1234567890,
        "metrics": {
            "incremental_small_edit": {"iterations": 35, "median_ns": 73307, "p95_ns": 148249, "mean_ns": 78530},
            "incremental_multiple_edits": {"iterations": 35, "median_ns": 36733, "p95_ns": 182845, "mean_ns": 50285}
        }
    }"#;
    fs::write(status_dir.join("parser_performance_scorecard.json"), json)?;
    let range = read_incremental_parse_range_ns(dir.path());
    // 36733 is lower, 73307 is upper
    assert_eq!(range, Some((36733, 73307)));
    Ok(())
}

#[test]
fn test_read_incremental_parse_range_ns_returns_none_when_file_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = read_incremental_parse_range_ns(dir.path());
    assert!(result.is_none());
}

#[test]
fn test_read_incremental_parse_range_ns_returns_none_on_invalid_json() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let status_dir = dir.path().join("docs/project/status");
    fs::create_dir_all(&status_dir)?;
    fs::write(status_dir.join("parser_performance_scorecard.json"), "{bad}")?;
    let result = read_incremental_parse_range_ns(dir.path());
    assert!(result.is_none());
    Ok(())
}

#[test]
fn test_format_quality_metrics_bullet_with_both_receipts() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let status_dir = dir.path().join("docs/project/status");
    fs::create_dir_all(&status_dir)?;
    // Write mock editor_ux.md
    let md = "# Editor UX Scorecard\n\
        ## Latency (ms)\n\
        | Request class | p50 | p50 baseline | p95 | p95 baseline |\n\
        |---|---:|---:|---:|---:|\n\
        | diagnostics | 53.00 | 53.00 | 66.00 | 66.00 |\n";
    fs::write(status_dir.join("editor_ux.md"), md)?;
    // Write mock parser_performance_scorecard.json
    let json = r#"{
        "schema_version": 1,
        "generated_at_epoch_s": 1234567890,
        "metrics": {
            "incremental_small_edit": {"iterations": 35, "median_ns": 73307, "p95_ns": 148249, "mean_ns": 78530},
            "incremental_multiple_edits": {"iterations": 35, "median_ns": 36733, "p95_ns": 182845, "mean_ns": 50285}
        }
    }"#;
    fs::write(status_dir.join("parser_performance_scorecard.json"), json)?;
    let bullet = format_quality_metrics_bullet(dir.path());
    // Must match the exact format PR #1192 writes into quality.md
    assert_eq!(
        bullet,
        "diagnostics p50 = 53 ms (receipt: `editor_ux.md`); \
         incremental parse median = 37–73 µs (receipt: `parser_performance_scorecard.json`)"
    );
    Ok(())
}

#[test]
fn test_format_quality_metrics_bullet_fallback_when_receipts_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bullet = format_quality_metrics_bullet(dir.path());
    assert!(bullet.contains("unmeasured"));
    assert!(!bullet.contains("931ns"));
    assert!(!bullet.contains("<50ms"));
}

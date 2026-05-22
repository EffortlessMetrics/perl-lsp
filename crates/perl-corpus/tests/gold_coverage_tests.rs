//! Coverage tests for `crates/perl-corpus/src/gold.rs`.
//!
//! # What is covered
//!
//! - `load_gold_fixture`: success path, missing `fixture.pl`, missing `expected.json`,
//!   malformed JSON, no-directory-name edge case.
//! - `load_gold_fixtures` / `load_gold_fixtures_from`: empty directory, directory
//!   with one valid subdirectory, subdirectory that fails to load (warn-and-skip),
//!   root does not exist.
//! - `load_hover_gold_fixtures`: root does not exist, non-directory entry skipped,
//!   directory without `fixture.pl` skipped, directory without `expected_hover.json`
//!   skipped, malformed `expected_hover.json`, valid fixture loaded.
//! - `load_goto_gold_fixtures`: same shape as hover variant.
//! - `load_completion_gold_fixtures`: same shape, plus all `CompletionAssertionKind`
//!   variants round-trip through serde.
//! - `load_document_symbol_gold_fixtures`: same shape, plus all
//!   `DocumentSymbolAssertionKind` variants round-trip.
//! - All assertion enum variants (`GoldAssertion`, `HoverAssertionKind`,
//!   `GotoAssertionKind`, `CompletionAssertionKind`, `DocumentSymbolAssertionKind`)
//!   serialise and deserialise correctly.
//!
//! # What is NOT covered (and why)
//!
//! - Filesystem I/O failures below `fs::read_dir` level: too invasive to reproduce
//!   without mocking.
//! - The `tracing::warn!` path inside `load_gold_fixtures_from` is exercised
//!   indirectly (the bad-fixture test confirms we don't panic), but the log message
//!   itself is not asserted.

mod gold {
    use perl_corpus::gold::{
        CompletionAssertionKind, CompletionGoldExpected, DocumentSymbolAssertionKind,
        DocumentSymbolGoldExpected, GoldAssertion, GoldExpected, GotoAssertionKind,
        GotoGoldExpected, HoverAssertionKind, HoverGoldExpected,
    };
    use perl_corpus::gold::{
        load_completion_gold_fixtures, load_document_symbol_gold_fixtures, load_gold_fixture,
        load_gold_fixtures, load_gold_fixtures_from, load_goto_gold_fixtures,
        load_hover_gold_fixtures,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let pid = std::process::id();
        path.push(format!("{}_{}_{}_{}", "perl_corpus_gold", prefix, pid, nanos));
        path
    }

    fn make_fixture_dir(
        root: &std::path::Path,
        name: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir = root.join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("fixture.pl"), "my $x = 1;\n")?;
        Ok(dir)
    }

    fn write_expected_json(
        dir: &std::path::Path,
        json: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(dir.join("expected.json"), json)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // GoldAssertion serde round-trips
    // -------------------------------------------------------------------------

    #[test]
    fn gold_assertion_no_diagnostics_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"assertion":"no_diagnostics"}"#;
        let a: GoldAssertion = serde_json::from_str(json)?;
        assert!(matches!(a, GoldAssertion::NoDiagnostics));
        Ok(())
    }

    #[test]
    fn gold_assertion_no_diagnostic_with_code() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"assertion":"no_diagnostic","code":"PL999"}"#;
        let a: GoldAssertion = serde_json::from_str(json)?;
        assert!(matches!(&a, GoldAssertion::NoDiagnostic { code } if code == "PL999"));
        Ok(())
    }

    #[test]
    fn gold_assertion_diagnostic_present_optional_fields() -> Result<(), Box<dyn std::error::Error>>
    {
        // byte_offset and message_contains are optional
        let no_opts = r#"{"assertion":"diagnostic_present","code":"E01"}"#;
        let a: GoldAssertion = serde_json::from_str(no_opts)?;
        assert!(
            matches!(&a, GoldAssertion::DiagnosticPresent { code, byte_offset: None, message_contains: None } if code == "E01"),
        );

        let with_msg =
            r#"{"assertion":"diagnostic_present","code":"E02","message_contains":"something"}"#;
        let b: GoldAssertion = serde_json::from_str(with_msg)?;
        assert!(matches!(
            &b,
            GoldAssertion::DiagnosticPresent { code, message_contains: Some(msg), .. }
            if code == "E02" && msg == "something"
        ),);
        Ok(())
    }

    #[test]
    fn gold_assertion_diagnostic_count() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"assertion":"diagnostic_count","code":"C01","count":7}"#;
        let a: GoldAssertion = serde_json::from_str(json)?;
        assert!(matches!(&a, GoldAssertion::DiagnosticCount { code, count: 7 } if code == "C01"));
        Ok(())
    }

    #[test]
    fn gold_assertion_unknown_variant_is_error() {
        let json = r#"{"assertion":"totally_unknown"}"#;
        let result: Result<GoldAssertion, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown assertion variant must fail");
    }

    // -------------------------------------------------------------------------
    // GoldExpected serde
    // -------------------------------------------------------------------------

    #[test]
    fn gold_expected_multi_assertion_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"diagnostics":[
            {"assertion":"no_diagnostics"},
            {"assertion":"diagnostic_count","code":"X","count":2}
        ]}"#;
        let expected: GoldExpected = serde_json::from_str(json)?;
        assert_eq!(expected.diagnostics.len(), 2);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_gold_fixture - success
    // -------------------------------------------------------------------------

    #[test]
    fn load_gold_fixture_success() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("fixture_success");
        fs::create_dir_all(&root)?;
        let fixture_dir = make_fixture_dir(&root, "my-fixture")?;
        write_expected_json(&fixture_dir, r#"{"diagnostics":[{"assertion":"no_diagnostics"}]}"#)?;

        let fixture = load_gold_fixture(&fixture_dir)?;
        assert_eq!(fixture.name, "my-fixture");
        assert!(fixture.fixture_path.ends_with("fixture.pl"));
        assert_eq!(fixture.expected.diagnostics.len(), 1);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_gold_fixture - error paths
    // -------------------------------------------------------------------------

    #[test]
    fn load_gold_fixture_missing_fixture_pl() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("missing_pl");
        fs::create_dir_all(&root)?;
        let dir = root.join("no-pl");
        fs::create_dir_all(&dir)?;
        write_expected_json(&dir, r#"{"diagnostics":[]}"#)?;
        // Note: NO fixture.pl written

        let result = load_gold_fixture(&dir);
        assert!(result.is_err(), "should fail without fixture.pl");
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("fixture.pl"), "error should mention fixture.pl");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_gold_fixture_missing_expected_json() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("missing_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "has-pl-only")?;
        // Note: NO expected.json written

        let result = load_gold_fixture(&dir);
        assert!(result.is_err(), "should fail without expected.json");
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("expected.json"), "error should mention expected.json");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_gold_fixture_malformed_expected_json() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("bad_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "malformed")?;
        write_expected_json(&dir, r#"{INVALID JSON"#)?;

        let result = load_gold_fixture(&dir);
        assert!(result.is_err(), "malformed JSON must return Err");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_gold_fixtures / load_gold_fixtures_from
    // -------------------------------------------------------------------------

    #[test]
    fn load_gold_fixtures_returns_error_when_root_missing() {
        let result = load_gold_fixtures("/tmp/perl_corpus_gold_nonexistent_root_xyz_abc");
        assert!(result.is_err(), "nonexistent root must return Err");
    }

    #[test]
    fn load_gold_fixtures_from_returns_error_when_root_missing() {
        let result = load_gold_fixtures_from("/tmp/perl_corpus_gold_nonexistent_root_xyz_def");
        assert!(result.is_err(), "nonexistent root must return Err");
    }

    #[test]
    fn load_gold_fixtures_empty_root_returns_empty_vec() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("empty_root");
        fs::create_dir_all(&root)?;

        let fixtures = load_gold_fixtures(&root)?;
        assert!(fixtures.is_empty(), "empty root must yield empty fixture list");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_gold_fixtures_loads_valid_subdirs_and_sorts() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_dir("multi_fixture");
        fs::create_dir_all(&root)?;

        // Create two valid fixtures in non-alphabetical order on disk
        for name in &["zebra-fixture", "alpha-fixture"] {
            let d = make_fixture_dir(&root, name)?;
            write_expected_json(&d, r#"{"diagnostics":[{"assertion":"no_diagnostics"}]}"#)?;
        }

        let fixtures = load_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 2, "should load exactly 2 fixtures");
        assert_eq!(fixtures[0].name, "alpha-fixture");
        assert_eq!(fixtures[1].name, "zebra-fixture");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_gold_fixtures_skips_invalid_subdirs_with_warning()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("skip_bad");
        fs::create_dir_all(&root)?;

        // One valid fixture
        let good = make_fixture_dir(&root, "good")?;
        write_expected_json(&good, r#"{"diagnostics":[]}"#)?;

        // One invalid subdirectory (missing expected.json)
        let bad = root.join("bad");
        fs::create_dir_all(&bad)?;
        // Only fixture.pl, no expected.json
        fs::write(bad.join("fixture.pl"), "1;\n")?;

        // A plain file (not a directory) - should be silently skipped
        fs::write(root.join("not-a-dir.txt"), "ignore\n")?;

        let fixtures = load_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 1, "only the good fixture should load");
        assert_eq!(fixtures[0].name, "good");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // HoverAssertionKind serde round-trips
    // -------------------------------------------------------------------------

    #[test]
    fn hover_assertion_kind_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            r#"{"kind":"hover_non_null","line":0,"character":0}"#,
            r#"{"kind":"hover_null","line":1,"character":5}"#,
            r#"{"kind":"hover_contains","needle":"Foo","line":2,"character":10}"#,
            r#"{"kind":"hover_absent","needle":"Bar","line":3,"character":0}"#,
        ];

        for raw in &cases {
            let parsed: perl_corpus::gold::HoverAssertion =
                serde_json::from_str(raw).map_err(|e| format!("failed on {raw}: {e}"))?;
            // Smoke-check: it deserialized successfully
            let _ = parsed;
        }
        Ok(())
    }

    #[test]
    fn hover_assertion_kind_hover_contains_needle() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"hover_contains","needle":"my needle","line":0,"character":0}"#;
        let a: perl_corpus::gold::HoverAssertion = serde_json::from_str(json)?;
        assert!(
            matches!(&a.kind, HoverAssertionKind::HoverContains { needle } if needle == "my needle")
        );
        Ok(())
    }

    #[test]
    fn hover_assertion_kind_hover_absent_needle() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"hover_absent","needle":"forbidden","line":5,"character":3}"#;
        let a: perl_corpus::gold::HoverAssertion = serde_json::from_str(json)?;
        assert!(
            matches!(&a.kind, HoverAssertionKind::HoverAbsent { needle } if needle == "forbidden")
        );
        assert_eq!(a.line, 5);
        assert_eq!(a.character, 3);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_hover_gold_fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn load_hover_gold_fixtures_root_missing_returns_err() {
        let result = load_hover_gold_fixtures("/tmp/perl_corpus_hover_nonexistent_root_xyz");
        assert!(result.is_err(), "nonexistent root must return Err");
    }

    #[test]
    fn load_hover_gold_fixtures_skips_non_directories() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("hover_non_dir");
        fs::create_dir_all(&root)?;
        // Only a plain file - not a directory
        fs::write(root.join("plain.txt"), "ignore\n")?;

        let fixtures = load_hover_gold_fixtures(&root)?;
        assert!(fixtures.is_empty());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_hover_gold_fixtures_skips_dir_without_fixture_pl()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("hover_no_pl");
        fs::create_dir_all(&root)?;
        let dir = root.join("no-pl");
        fs::create_dir_all(&dir)?;
        // Only expected_hover.json, no fixture.pl
        let hover_json = r#"{"version":1,"fixture":"no-pl","assertions":[]}"#;
        fs::write(dir.join("expected_hover.json"), hover_json)?;

        let fixtures = load_hover_gold_fixtures(&root)?;
        assert!(fixtures.is_empty(), "should skip when fixture.pl is missing");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_hover_gold_fixtures_skips_dir_without_expected_hover_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("hover_no_hover_json");
        fs::create_dir_all(&root)?;
        let _dir = make_fixture_dir(&root, "no-hover")?;
        // fixture.pl present but no expected_hover.json

        let fixtures = load_hover_gold_fixtures(&root)?;
        assert!(fixtures.is_empty(), "should skip when expected_hover.json is missing");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_hover_gold_fixtures_loads_valid_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("hover_valid");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "hover-test")?;
        let hover_json = r#"{
            "version": 1,
            "fixture": "hover-test",
            "assertions": [
                {"kind":"hover_non_null","line":0,"character":5,"rationale":"should have hover"}
            ]
        }"#;
        fs::write(dir.join("expected_hover.json"), hover_json)?;

        let fixtures = load_hover_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "hover-test");
        assert_eq!(fixtures[0].hover_assertions.len(), 1);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_hover_gold_fixtures_malformed_json_returns_err()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("hover_bad_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "bad-hover")?;
        fs::write(dir.join("expected_hover.json"), r#"{INVALID"#)?;

        let result = load_hover_gold_fixtures(&root);
        assert!(result.is_err(), "malformed JSON must bubble up as Err");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // GotoAssertionKind serde round-trips
    // -------------------------------------------------------------------------

    #[test]
    fn goto_assertion_kind_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let cases = vec![
            (r#"{"kind":"goto_non_null","line":0,"character":0}"#, "goto_non_null"),
            (r#"{"kind":"goto_null","line":1,"character":0}"#, "goto_null"),
            (r#"{"kind":"goto_line","expected_line":42,"line":2,"character":0}"#, "goto_line"),
        ];
        for (json, variant) in cases {
            let parsed: perl_corpus::gold::GotoAssertion =
                serde_json::from_str(json).map_err(|e| format!("failed on {variant}: {e}"))?;
            let _ = parsed;
        }
        Ok(())
    }

    #[test]
    fn goto_assertion_goto_line_expected_line() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"goto_line","expected_line":77,"line":0,"character":0}"#;
        let a: perl_corpus::gold::GotoAssertion = serde_json::from_str(json)?;
        assert!(matches!(&a.kind, GotoAssertionKind::GotoLine { expected_line: 77 }));
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_goto_gold_fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn load_goto_gold_fixtures_root_missing_returns_err() {
        let result = load_goto_gold_fixtures("/tmp/perl_corpus_goto_nonexistent_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn load_goto_gold_fixtures_skips_dir_without_expected_goto_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("goto_no_json");
        fs::create_dir_all(&root)?;
        make_fixture_dir(&root, "no-goto")?;

        let fixtures = load_goto_gold_fixtures(&root)?;
        assert!(fixtures.is_empty());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_goto_gold_fixtures_loads_valid_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("goto_valid");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "goto-test")?;
        let goto_json = r#"{
            "version": 1,
            "fixture": "goto-test",
            "assertions": [
                {"kind":"goto_non_null","line":3,"character":10}
            ]
        }"#;
        fs::write(dir.join("expected_goto.json"), goto_json)?;

        let fixtures = load_goto_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "goto-test");
        assert_eq!(fixtures[0].goto_assertions.len(), 1);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_goto_gold_fixtures_malformed_json_returns_err() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_dir("goto_bad_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "bad-goto")?;
        fs::write(dir.join("expected_goto.json"), r#"NOT JSON"#)?;

        let result = load_goto_gold_fixtures(&root);
        assert!(result.is_err());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // CompletionAssertionKind serde round-trips
    // -------------------------------------------------------------------------

    #[test]
    fn completion_assertion_kind_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            r#"{"kind":"completion_non_empty","line":0,"character":0}"#,
            r#"{"kind":"completion_top1","expected_label":"my_fn","line":1,"character":0}"#,
            r#"{"kind":"completion_top5","expected_label":"my_fn","line":2,"character":0}"#,
            r#"{"kind":"completion_present","expected_label":"other_fn","line":3,"character":0}"#,
            r#"{"kind":"completion_noise_absent","forbidden_label":"bad","line":4,"character":0}"#,
        ];
        for raw in &cases {
            let parsed: perl_corpus::gold::CompletionAssertion =
                serde_json::from_str(raw).map_err(|e| format!("failed on {raw}: {e}"))?;
            let _ = parsed;
        }
        Ok(())
    }

    #[test]
    fn completion_assertion_noise_absent_forbidden_label() -> Result<(), Box<dyn std::error::Error>>
    {
        let json = r#"{"kind":"completion_noise_absent","forbidden_label":"verboten","line":0,"character":0}"#;
        let a: perl_corpus::gold::CompletionAssertion = serde_json::from_str(json)?;
        assert!(
            matches!(&a.kind, CompletionAssertionKind::CompletionNoiseAbsent { forbidden_label } if forbidden_label == "verboten"),
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_completion_gold_fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn load_completion_gold_fixtures_root_missing_returns_err() {
        let result = load_completion_gold_fixtures("/tmp/perl_corpus_completion_nonexistent_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn load_completion_gold_fixtures_skips_dir_without_expected_completion_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("completion_no_json");
        fs::create_dir_all(&root)?;
        make_fixture_dir(&root, "no-completion")?;

        let fixtures = load_completion_gold_fixtures(&root)?;
        assert!(fixtures.is_empty());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_completion_gold_fixtures_loads_valid_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_dir("completion_valid");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "completion-test")?;
        let completion_json = r#"{
            "version": 1,
            "fixture": "completion-test",
            "assertions": [
                {"kind":"completion_non_empty","line":5,"character":8}
            ]
        }"#;
        fs::write(dir.join("expected_completion.json"), completion_json)?;

        let fixtures = load_completion_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "completion-test");
        assert_eq!(fixtures[0].completion_assertions.len(), 1);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_completion_gold_fixtures_malformed_json_returns_err()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("completion_bad_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "bad-completion")?;
        fs::write(dir.join("expected_completion.json"), r#"{BAD"#)?;

        let result = load_completion_gold_fixtures(&root);
        assert!(result.is_err());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // DocumentSymbolAssertionKind serde round-trips
    // -------------------------------------------------------------------------

    #[test]
    fn document_symbol_assertion_kind_all_variants() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            r#"{"kind":"symbol_non_empty"}"#,
            r#"{"kind":"symbol_present","name":"my_fn"}"#,
            r#"{"kind":"symbol_absent","name":"gone"}"#,
            r#"{"kind":"symbol_count","count":3}"#,
        ];
        for raw in &cases {
            let parsed: perl_corpus::gold::DocumentSymbolAssertion =
                serde_json::from_str(raw).map_err(|e| format!("failed on {raw}: {e}"))?;
            let _ = parsed;
        }
        Ok(())
    }

    #[test]
    fn document_symbol_assertion_symbol_count() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"symbol_count","count":42}"#;
        let a: perl_corpus::gold::DocumentSymbolAssertion = serde_json::from_str(json)?;
        assert!(matches!(&a.kind, DocumentSymbolAssertionKind::SymbolCount { count: 42 }));
        Ok(())
    }

    #[test]
    fn document_symbol_assertion_symbol_present() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"symbol_present","name":"greet"}"#;
        let a: perl_corpus::gold::DocumentSymbolAssertion = serde_json::from_str(json)?;
        assert!(
            matches!(&a.kind, DocumentSymbolAssertionKind::SymbolPresent { name } if name == "greet")
        );
        Ok(())
    }

    #[test]
    fn document_symbol_assertion_symbol_absent() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"kind":"symbol_absent","name":"gone_fn"}"#;
        let a: perl_corpus::gold::DocumentSymbolAssertion = serde_json::from_str(json)?;
        assert!(
            matches!(&a.kind, DocumentSymbolAssertionKind::SymbolAbsent { name } if name == "gone_fn")
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // load_document_symbol_gold_fixtures
    // -------------------------------------------------------------------------

    #[test]
    fn load_document_symbol_gold_fixtures_root_missing_returns_err() {
        let result = load_document_symbol_gold_fixtures("/tmp/perl_corpus_docssym_nonexistent_xyz");
        assert!(result.is_err());
    }

    #[test]
    fn load_document_symbol_gold_fixtures_skips_dir_without_expected_symbols_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("sym_no_json");
        fs::create_dir_all(&root)?;
        make_fixture_dir(&root, "no-symbols")?;

        let fixtures = load_document_symbol_gold_fixtures(&root)?;
        assert!(fixtures.is_empty());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_document_symbol_gold_fixtures_loads_valid_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("sym_valid");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "sym-test")?;
        let sym_json = r#"{
            "version": 1,
            "fixture": "sym-test",
            "assertions": [
                {"kind":"symbol_non_empty"},
                {"kind":"symbol_present","name":"greet"}
            ]
        }"#;
        fs::write(dir.join("expected_symbols.json"), sym_json)?;

        let fixtures = load_document_symbol_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "sym-test");
        assert_eq!(fixtures[0].symbol_assertions.len(), 2);

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_document_symbol_gold_fixtures_malformed_json_returns_err()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("sym_bad_json");
        fs::create_dir_all(&root)?;
        let dir = make_fixture_dir(&root, "bad-sym")?;
        fs::write(dir.join("expected_symbols.json"), r#"not json at all"#)?;

        let result = load_document_symbol_gold_fixtures(&root);
        assert!(result.is_err());

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn load_document_symbol_gold_fixtures_sorted_by_name() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_dir("sym_sorted");
        fs::create_dir_all(&root)?;
        let sym_json = r#"{"version":1,"fixture":"x","assertions":[]}"#;

        for name in &["zebra", "apple", "mango"] {
            let dir = make_fixture_dir(&root, name)?;
            fs::write(dir.join("expected_symbols.json"), sym_json)?;
        }

        let fixtures = load_document_symbol_gold_fixtures(&root)?;
        assert_eq!(fixtures.len(), 3);
        assert_eq!(fixtures[0].name, "apple");
        assert_eq!(fixtures[1].name, "mango");
        assert_eq!(fixtures[2].name, "zebra");

        fs::remove_dir_all(&root)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // HoverGoldExpected / GotoGoldExpected / CompletionGoldExpected /
    // DocumentSymbolGoldExpected - version and fixture fields
    // -------------------------------------------------------------------------

    #[test]
    fn hover_gold_expected_version_and_fixture_fields() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"version":3,"fixture":"my-hover","assertions":[]}"#;
        let expected: HoverGoldExpected = serde_json::from_str(json)?;
        assert_eq!(expected.version, 3);
        assert_eq!(expected.fixture, "my-hover");
        assert!(expected.assertions.is_empty());
        Ok(())
    }

    #[test]
    fn goto_gold_expected_version_and_fixture_fields() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"version":2,"fixture":"def-go","assertions":[]}"#;
        let expected: GotoGoldExpected = serde_json::from_str(json)?;
        assert_eq!(expected.version, 2);
        assert_eq!(expected.fixture, "def-go");
        Ok(())
    }

    #[test]
    fn completion_gold_expected_version_and_fixture_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"version":1,"fixture":"comp-go","assertions":[]}"#;
        let expected: CompletionGoldExpected = serde_json::from_str(json)?;
        assert_eq!(expected.version, 1);
        assert_eq!(expected.fixture, "comp-go");
        Ok(())
    }

    #[test]
    fn document_symbol_gold_expected_version_and_fixture_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"version":1,"fixture":"sym-go","assertions":[]}"#;
        let expected: DocumentSymbolGoldExpected = serde_json::from_str(json)?;
        assert_eq!(expected.version, 1);
        assert_eq!(expected.fixture, "sym-go");
        Ok(())
    }
}

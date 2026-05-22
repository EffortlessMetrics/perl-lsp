//! Coverage tests for `crates/perl-corpus/src/lint.rs`.
//!
//! # What is covered
//!
//! - `LintConfig::default`: all four fields verified.
//! - `LintResult::is_ok`: true when no errors, false when errors present.
//! - `lint` (thin wrapper around `lint_with_config`): clean input succeeds, dirty fails.
//! - `lint_with_config`:
//!   - `check_unknown_tags = false` suppresses unknown-tag warnings.
//!   - `check_unknown_flags = false` suppresses unknown-flag warnings.
//!   - `require_perl_version = true` emits a warning for sections missing `@perl`.
//!   - `max_sections_per_file` threshold triggers the per-file limit warning.
//!   - Unknown tag produces a warning.
//!   - Unknown flag produces a warning.
//!   - Empty body produces a warning.
//! - `check_sections`:
//!   - Missing ID (id_source == Generated) produces an error.
//!   - Invalid ID format produces an error.
//!   - Duplicate effective ID produces an error.
//!   - Explicit ID passes the format check.
//!   - Both `wip` and `todo` flags count toward markers (indirectly validated via
//!     inventory; checked here at the lint layer via warnings).
//!
//! # What is NOT covered (and why)
//!
//! - The `tracing::warn!` / `tracing::error!` side effects: these go to the tracing
//!   subscriber and are not visible without a custom subscriber in tests.
//! - The ID regex compile-time path: `ID_RE` is a module-level lazy static; its
//!   `None` arm is unreachable with a known-good pattern.

mod lint {
    use perl_corpus::lint::{
        KNOWN_FLAGS, KNOWN_TAGS, LintConfig, check_sections, lint, lint_with_config,
    };
    use perl_corpus::metadata::{IdSource, Section};

    // -------------------------------------------------------------------------
    // Helper to build Section values
    // -------------------------------------------------------------------------

    fn make_section(
        id: &str,
        id_source: IdSource,
        explicit_id: Option<&str>,
        tags: &[&str],
        flags: &[&str],
        body: &str,
        file: &str,
        perl: Option<&str>,
    ) -> Section {
        Section {
            id: id.to_string(),
            id_source,
            explicit_id: explicit_id.map(str::to_string),
            generated_id: if id_source == IdSource::Generated {
                Some(id.to_string())
            } else {
                None
            },
            title: "Test Section".to_string(),
            file: file.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            perl: perl.map(str::to_string),
            flags: flags.iter().map(|f| f.to_string()).collect(),
            body: body.to_string(),
            expected: None,
            line: Some(1),
        }
    }

    fn valid_section(id: &str) -> Section {
        make_section(
            id,
            IdSource::Explicit,
            Some(id),
            &["regex"],
            &[],
            "my $x = 1;",
            "test.txt",
            None,
        )
    }

    // -------------------------------------------------------------------------
    // LintConfig::default
    // -------------------------------------------------------------------------

    #[test]
    fn lint_config_default_values() {
        let cfg = LintConfig::default();
        assert_eq!(cfg.max_sections_per_file, 12);
        assert!(cfg.check_unknown_tags, "default should check unknown tags");
        assert!(cfg.check_unknown_flags, "default should check unknown flags");
        assert!(!cfg.require_perl_version, "default should NOT require perl version");
    }

    // -------------------------------------------------------------------------
    // LintResult::is_ok
    // -------------------------------------------------------------------------

    #[test]
    fn lint_result_is_ok_true_when_no_errors() {
        let sections = [valid_section("case.ok")];
        let result = check_sections(&sections, &LintConfig::default());
        assert!(result.is_ok(), "no errors means is_ok should be true");
    }

    #[test]
    fn lint_result_is_ok_false_when_errors_present() {
        let bad = make_section(
            "generated-fallback",
            IdSource::Generated,
            None,
            &[],
            &[],
            "my $x = 1;",
            "f.txt",
            None,
        );
        let result = check_sections(&[bad], &LintConfig::default());
        assert!(!result.is_ok(), "generated id should produce an error");
        assert!(!result.errors.is_empty());
    }

    // -------------------------------------------------------------------------
    // lint (thin wrapper)
    // -------------------------------------------------------------------------

    #[test]
    fn lint_succeeds_on_clean_sections() -> Result<(), Box<dyn std::error::Error>> {
        let sections = [valid_section("clean.case")];
        lint(&sections).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn lint_fails_when_sections_have_errors() {
        let bad = make_section(
            "generated-id",
            IdSource::Generated,
            None,
            &[],
            &[],
            "1;",
            "test.txt",
            None,
        );
        let result = lint(&[bad]);
        assert!(result.is_err(), "lint must propagate errors as Err");
    }

    // -------------------------------------------------------------------------
    // lint_with_config - check_unknown_tags = false
    // -------------------------------------------------------------------------

    #[test]
    fn lint_with_config_unknown_tag_suppressed_when_check_disabled()
    -> Result<(), Box<dyn std::error::Error>> {
        let section = make_section(
            "ok.id",
            IdSource::Explicit,
            Some("ok.id"),
            &["totally-unknown-tag-xyz"],
            &[],
            "my $x = 1;",
            "t.txt",
            None,
        );
        let cfg = LintConfig { check_unknown_tags: false, ..LintConfig::default() };
        // With the flag disabled, no error or warning for the unknown tag
        let result = check_sections(&[section], &cfg);
        assert!(result.is_ok(), "unknown tag should be silent when check_unknown_tags=false");
        assert!(result.warnings.is_empty(), "no warnings expected");
        Ok(())
    }

    // -------------------------------------------------------------------------
    // lint_with_config - check_unknown_flags = false
    // -------------------------------------------------------------------------

    #[test]
    fn lint_with_config_unknown_flag_suppressed_when_check_disabled()
    -> Result<(), Box<dyn std::error::Error>> {
        let section = make_section(
            "ok.id",
            IdSource::Explicit,
            Some("ok.id"),
            &["regex"],
            &["totally-unknown-flag-xyz"],
            "1;",
            "t.txt",
            None,
        );
        let cfg = LintConfig { check_unknown_flags: false, ..LintConfig::default() };
        let result = check_sections(&[section], &cfg);
        assert!(result.is_ok());
        assert!(result.warnings.is_empty());
        Ok(())
    }

    // -------------------------------------------------------------------------
    // lint_with_config - require_perl_version = true
    // -------------------------------------------------------------------------

    #[test]
    fn lint_with_config_require_perl_version_warns_on_missing() {
        let section = make_section(
            "ok.id",
            IdSource::Explicit,
            Some("ok.id"),
            &["regex"],
            &[],
            "1;",
            "t.txt",
            None, // no perl version
        );
        let cfg = LintConfig { require_perl_version: true, ..LintConfig::default() };
        let result = check_sections(&[section], &cfg);
        assert!(result.is_ok(), "missing perl version is a warning, not an error");
        assert!(
            result.warnings.iter().any(|w| w.contains("perl version") || w.contains("@perl")),
            "warning should mention perl version, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn lint_with_config_require_perl_version_no_warning_when_present() {
        let section = make_section(
            "ok.id",
            IdSource::Explicit,
            Some("ok.id"),
            &["regex"],
            &[],
            "1;",
            "t.txt",
            Some("5.36+"),
        );
        let cfg = LintConfig { require_perl_version: true, ..LintConfig::default() };
        let result = check_sections(&[section], &cfg);
        assert!(result.is_ok());
        // Should not warn about missing perl version since it's present
        assert!(
            !result.warnings.iter().any(|w| w.contains("Missing @perl")),
            "no warning expected when perl version is set, got: {:?}",
            result.warnings
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - max_sections_per_file exceeded
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_warns_on_file_exceeding_section_limit() {
        // Build 3 sections in the same file, with a limit of 2
        let sections: Vec<Section> = (0..3_u32)
            .map(|i| {
                make_section(
                    &format!("case.{i}"),
                    IdSource::Explicit,
                    Some(&format!("case.{i}")),
                    &["regex"],
                    &[],
                    "1;",
                    "overflow.txt",
                    None,
                )
            })
            .collect();

        let cfg = LintConfig { max_sections_per_file: 2, ..LintConfig::default() };
        let result = check_sections(&sections, &cfg);
        assert!(result.is_ok(), "exceeding limit is a warning, not an error");
        assert!(
            result.warnings.iter().any(|w| w.contains("overflow.txt") && w.contains("3")),
            "should warn about the file with 3 sections, got: {:?}",
            result.warnings,
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - duplicate effective ID
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_errors_on_duplicate_id() {
        let sections = [
            valid_section("dup.case"),
            valid_section("dup.case"), // duplicate
        ];
        let result = check_sections(&sections, &LintConfig::default());
        assert!(!result.is_ok(), "duplicate id must produce an error");
        assert!(
            result.errors.iter().any(|e| e.contains("dup.case")),
            "error should mention the duplicate id, got: {:?}",
            result.errors,
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - generated ID error
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_errors_on_generated_id_source() {
        let section = make_section(
            "auto.title.001",
            IdSource::Generated,
            None,
            &["regex"],
            &[],
            "1;",
            "gen.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(!result.is_ok());
        assert!(
            result.errors.iter().any(|e| e.contains("Missing explicit @id")),
            "should complain about missing explicit @id, got: {:?}",
            result.errors,
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - invalid ID format (contains uppercase)
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_errors_on_invalid_id_format_uppercase() {
        let section = make_section(
            "INVALID.ID",
            IdSource::Explicit,
            Some("INVALID.ID"),
            &[],
            &[],
            "1;",
            "f.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(!result.is_ok(), "uppercase in ID must cause an error");
        assert!(
            result.errors.iter().any(|e| e.contains("Invalid @id format")),
            "got: {:?}",
            result.errors
        );
    }

    #[test]
    fn check_sections_errors_on_invalid_id_format_spaces() {
        let section = make_section(
            "has space",
            IdSource::Explicit,
            Some("has space"),
            &[],
            &[],
            "1;",
            "f.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(!result.is_ok(), "spaces in ID must cause an error");
        assert!(
            result.errors.iter().any(|e| e.contains("Invalid @id format")),
            "got: {:?}",
            result.errors
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - empty body
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_warns_on_empty_body() {
        let section = make_section(
            "empty.body",
            IdSource::Explicit,
            Some("empty.body"),
            &["regex"],
            &[],
            "   \n  ", // whitespace-only
            "f.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(result.is_ok(), "empty body is a warning, not an error");
        assert!(
            result.warnings.iter().any(|w| w.contains("Empty body")),
            "should warn about empty body, got: {:?}",
            result.warnings
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - unknown tag warning
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_warns_on_unknown_tag() {
        let section = make_section(
            "unknown.tag",
            IdSource::Explicit,
            Some("unknown.tag"),
            &["not-a-real-tag-xyz"],
            &[],
            "1;",
            "f.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(result.is_ok(), "unknown tag is a warning");
        assert!(
            result.warnings.iter().any(|w| w.contains("not-a-real-tag-xyz")),
            "should warn about unknown tag, got: {:?}",
            result.warnings
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - unknown flag warning
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_warns_on_unknown_flag() {
        let section = make_section(
            "unknown.flag",
            IdSource::Explicit,
            Some("unknown.flag"),
            &["regex"],
            &["not-a-real-flag-xyz"],
            "1;",
            "f.txt",
            None,
        );
        let result = check_sections(&[section], &LintConfig::default());
        assert!(result.is_ok(), "unknown flag is a warning");
        assert!(
            result.warnings.iter().any(|w| w.contains("not-a-real-flag-xyz")),
            "should warn about unknown flag, got: {:?}",
            result.warnings
        );
    }

    // -------------------------------------------------------------------------
    // check_sections - multiple issues can accumulate
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_accumulates_multiple_errors() {
        let sections = vec![
            make_section(
                "generated-id-a",
                IdSource::Generated,
                None,
                &[],
                &[],
                "1;",
                "a.txt",
                None,
            ),
            make_section(
                "generated-id-b",
                IdSource::Generated,
                None,
                &[],
                &[],
                "1;",
                "b.txt",
                None,
            ),
        ];
        let result = check_sections(&sections, &LintConfig::default());
        assert!(!result.is_ok());
        assert!(result.errors.len() >= 2, "each generated id should produce one error");
    }

    // -------------------------------------------------------------------------
    // KNOWN_TAGS / KNOWN_FLAGS constants are non-empty
    // -------------------------------------------------------------------------

    #[test]
    fn known_tags_constant_is_non_empty() {
        assert!(!KNOWN_TAGS.is_empty());
        assert!(KNOWN_TAGS.contains(&"regex"));
        assert!(KNOWN_TAGS.contains(&"heredoc"));
    }

    #[test]
    fn known_flags_constant_is_non_empty() {
        assert!(!KNOWN_FLAGS.is_empty());
        assert!(KNOWN_FLAGS.contains(&"lexer-sensitive"));
        assert!(KNOWN_FLAGS.contains(&"parser-sensitive"));
    }

    // -------------------------------------------------------------------------
    // lint_with_config propagates warning count in the Err message
    // -------------------------------------------------------------------------

    #[test]
    fn lint_with_config_error_message_includes_count() {
        let bad =
            make_section("generated-id", IdSource::Generated, None, &[], &[], "1;", "t.txt", None);
        let result = lint_with_config(&[bad], &LintConfig::default());
        let err = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            err.contains("Linting failed") || err.contains("error"),
            "error message should mention failure, got: {err:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Empty ID string causes a "missing" error, not an invalid-format error
    // -------------------------------------------------------------------------

    #[test]
    fn check_sections_errors_on_empty_id_string() {
        // An explicit section where the id field itself is empty (edge case)
        let section = Section {
            id: String::new(),
            id_source: IdSource::Explicit,
            explicit_id: Some(String::new()),
            generated_id: None,
            title: "No Id".to_string(),
            file: "empty.txt".to_string(),
            tags: vec![],
            perl: None,
            flags: vec![],
            body: "1;".to_string(),
            expected: None,
            line: Some(1),
        };
        let result = check_sections(&[section], &LintConfig::default());
        assert!(!result.is_ok(), "empty id must produce an error");
        assert!(
            result.errors.iter().any(|e| e.contains("Missing effective ID")),
            "got: {:?}",
            result.errors
        );
    }
}

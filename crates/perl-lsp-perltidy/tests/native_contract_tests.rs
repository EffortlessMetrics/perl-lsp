use perl_lsp_perltidy::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnostic,
    FormatDiagnosticSeverity, FormatResult, FormatterMode, KeywordSpacing, TextEdit, TextPosition,
    TextRange, TrailingComma,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_format_config_defaults_to_native_safe_profile() {
    let config = FormatConfig::default();

    assert_eq!(config.mode, FormatterMode::Native);
    assert_eq!(config.line_width, 100);
    assert_eq!(config.indent_width, 4);
    assert!(!config.use_tabs);
    assert_eq!(config.final_newline, FinalNewline::Preserve);
    assert_eq!(config.trailing_comma, TrailingComma::Preserve);
    assert_eq!(config.brace_placement, BracePlacement::SameLine);
    assert_eq!(config.else_placement, ElsePlacement::Cuddled);
    assert_eq!(config.keyword_spacing, KeywordSpacing::Space);
}

#[test]
fn native_format_config_exposes_explicit_compat_and_legacy_modes() {
    assert_eq!(FormatConfig::compat().mode, FormatterMode::Compat);
    assert_eq!(FormatConfig::external_legacy().mode, FormatterMode::ExternalLegacy);
}

#[test]
fn whole_document_range_uses_utf16_positions() {
    let range = TextRange::whole_document("my $face = \"😀\";");

    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(0, 16));
}

#[test]
fn whole_document_range_tracks_final_line_after_newline() -> TestResult {
    let range = TextRange::whole_document(
        r#"my $x = 1;
my $face = "😀";"#,
    );

    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(1, 16));

    Ok(())
}

#[test]
fn text_edit_constructor_preserves_range_and_replacement() -> TestResult {
    let range = TextRange::new(TextPosition::new(2, 4), TextPosition::new(2, 10));
    let edit = TextEdit::new(range, "my $value = 42;");

    assert_eq!(edit.range, range);
    assert_eq!(edit.new_text, "my $value = 42;");

    Ok(())
}

#[test]
fn diagnostic_constructor_preserves_severity_range_and_message() -> TestResult {
    let range = TextRange::new(TextPosition::new(3, 0), TextPosition::new(3, 7));
    let diagnostic = FormatDiagnostic::new(
        "native.format.test",
        FormatDiagnosticSeverity::Error,
        Some(range),
        "test diagnostic",
    );

    assert_eq!(diagnostic.code, "native.format.test");
    assert_eq!(diagnostic.severity, FormatDiagnosticSeverity::Error);
    assert_eq!(diagnostic.range, Some(range));
    assert_eq!(diagnostic.message, "test diagnostic");

    Ok(())
}

#[test]
fn diagnostic_severity_serializes_as_kebab_case() -> TestResult {
    assert_eq!(serde_json::to_string(&FormatDiagnosticSeverity::Info)?, "\"info\"");
    assert_eq!(serde_json::to_string(&FormatDiagnosticSeverity::Warning)?, "\"warning\"");
    assert_eq!(serde_json::to_string(&FormatDiagnosticSeverity::Error)?, "\"error\"");

    Ok(())
}

#[test]
fn replace_document_result_distinguishes_changed_from_unchanged() {
    let unchanged = FormatResult::replace_document("my $x = 1;\n", "my $x = 1;\n");
    assert!(!unchanged.changed);
    assert!(unchanged.edits.is_empty());

    let changed = FormatResult::replace_document("my $x=1;\n", "my $x = 1;\n");
    assert!(changed.changed);
    assert_eq!(changed.formatted, "my $x = 1;\n");
    assert_eq!(changed.edits.len(), 1);
    assert_eq!(changed.edits[0].range, TextRange::whole_document("my $x=1;\n"));
    assert_eq!(changed.edits[0].new_text, "my $x = 1;\n");
}

#[test]
fn unsafe_to_format_result_returns_diagnostic_and_no_edits() {
    let result = FormatResult::unsafe_to_format(
        "print <<'EOF';\n",
        "native.format.unsafe_heredoc",
        "heredoc formatting is not enabled yet",
    );

    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].severity, FormatDiagnosticSeverity::Warning);
    assert_eq!(result.diagnostics[0].code, "native.format.unsafe_heredoc");
}

#[test]
fn native_result_serializes_agent_friendly_shape() -> TestResult {
    let result = FormatResult::replace_document("my $x=1;\n", "my $x = 1;\n");
    let value = serde_json::to_value(result)?;

    assert_eq!(value["changed"], true);
    assert_eq!(value["formatted"], "my $x = 1;\n");
    assert_eq!(value["edits"][0]["new_text"], "my $x = 1;\n");
    assert!(value["diagnostics"].as_array().is_some_and(Vec::is_empty));

    Ok(())
}

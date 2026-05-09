use perl_lsp_perltidy::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnosticSeverity,
    FormatResult, FormatterMode, KeywordSpacing, TextPosition, TextRange, TrailingComma,
};

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
fn native_result_serializes_agent_friendly_shape() -> Result<(), Box<dyn std::error::Error>> {
    let result = FormatResult::replace_document("my $x=1;\n", "my $x = 1;\n");
    let value = serde_json::to_value(result)?;

    assert_eq!(value["changed"], true);
    assert_eq!(value["formatted"], "my $x = 1;\n");
    assert_eq!(value["edits"][0]["new_text"], "my $x = 1;\n");
    assert!(value["diagnostics"].as_array().is_some_and(Vec::is_empty));

    Ok(())
}

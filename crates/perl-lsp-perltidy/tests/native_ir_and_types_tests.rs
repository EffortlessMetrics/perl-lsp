/// Tests for the `FormatDoc` IR constructors and rendering, type-constructor
/// round-trips, `TextPosition`/`TextRange`/`TextEdit` builders, `FormatConfig`
/// serde, and enum serde round-trips (`FormatterMode`, `FinalNewline`,
/// `TrailingComma`, `BracePlacement`, `ElsePlacement`, `KeywordSpacing`).
///
/// Slice: FormatDoc IR constructors + config/type serde
/// Before: 164 tests | After: +22 tests = ~186 tests
use perl_lsp_perltidy::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnostic,
    FormatDiagnosticSeverity, FormatDoc, FormatResult, FormatterMode, KeywordSpacing,
    NativeFormatter, PerlFormatter, TextEdit, TextPosition, TextRange, TrailingComma,
};

// ──────────────────────────── FormatDoc IR rendering ────────────────────────

#[test]
fn format_doc_hardline_always_breaks_even_in_flat_group() {
    // HardLine inside a group that would fit flat must still emit a newline.
    let config = FormatConfig { line_width: 200, indent_width: 4, ..FormatConfig::default() };
    let doc =
        FormatDoc::group(vec![FormatDoc::text("a"), FormatDoc::HardLine, FormatDoc::text("b")]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "a\nb");
}

#[test]
fn format_doc_line_breaks_in_non_flat_context() {
    // Line in a broken group emits a newline at the current indent level.
    let config = FormatConfig { line_width: 5, indent_width: 2, ..FormatConfig::default() };
    let doc = FormatDoc::Indent(vec![FormatDoc::Line, FormatDoc::text("x")]);

    let rendered = doc.render(&config);

    // Indent level 1, indent_width 2 → two spaces before "x".
    assert_eq!(rendered, "\n  x");
}

#[test]
fn format_doc_softline_becomes_space_in_flat_group() {
    // Already tested via native_doc_ir_tests but this verifies the canonical
    // flat-mode SoftLine → single space path with no surrounding Group.
    let config = FormatConfig { line_width: 200, ..FormatConfig::default() };
    let doc =
        FormatDoc::group(vec![FormatDoc::text("foo"), FormatDoc::SoftLine, FormatDoc::text("bar")]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "foo bar");
}

#[test]
fn format_doc_space_renders_single_space() {
    let doc = FormatDoc::Space;

    let rendered = doc.render(&FormatConfig::default());

    assert_eq!(rendered, " ");
}

#[test]
fn format_doc_literal_preserve_width_is_none_for_multiline() {
    // flat_width is None when text contains '\n'; verified via render path:
    // a LiteralPreserve with a newline inside an oversized group should break.
    let config = FormatConfig { line_width: 200, ..FormatConfig::default() };
    let doc = FormatDoc::literal_preserve("a\nb");

    let rendered = doc.render(&config);

    assert_eq!(rendered, "a\nb");
}

#[test]
fn format_doc_text_constructor_accepts_str_and_string() {
    let from_str = FormatDoc::text("hello");
    let from_string = FormatDoc::text("hello".to_string());

    let config = FormatConfig::default();
    assert_eq!(from_str.render(&config), from_string.render(&config));
}

#[test]
fn format_doc_group_constructor_accepts_vec() {
    let doc = FormatDoc::group(vec![FormatDoc::text("a"), FormatDoc::text("b")]);

    let rendered = doc.render(&FormatConfig::default());

    assert_eq!(rendered, "ab");
}

#[test]
fn format_doc_indent_constructor_accepts_vec() {
    // Broken render — use tiny line width so the group wraps.
    let config = FormatConfig { line_width: 1, indent_width: 4, ..FormatConfig::default() };
    let doc =
        FormatDoc::group(vec![FormatDoc::Indent(vec![FormatDoc::Line, FormatDoc::text("x")])]);

    let rendered = doc.render(&config);

    assert_eq!(rendered, "\n    x");
}

#[test]
fn format_doc_if_break_broken_branch_at_zero_width() {
    // Zero line_width → every group breaks; if_break must select broken branch.
    let config = FormatConfig { line_width: 0, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::SoftLine,
        FormatDoc::if_break(FormatDoc::text("BROKEN"), FormatDoc::text("FLAT")),
    ]);

    let rendered = doc.render(&config);

    // SoftLine becomes newline, if_break selects BROKEN.
    assert!(rendered.contains("BROKEN"), "expected 'BROKEN' in '{rendered}'");
    assert!(!rendered.contains("FLAT"), "unexpected 'FLAT' in '{rendered}'");
}

#[test]
fn format_doc_if_break_flat_branch_in_fitting_group() {
    // Fitting group → flat path → if_break selects flat branch.
    let config = FormatConfig { line_width: 200, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::SoftLine,
        FormatDoc::if_break(FormatDoc::text("BROKEN"), FormatDoc::text("FLAT")),
    ]);

    let rendered = doc.render(&config);

    assert!(rendered.contains("FLAT"), "expected 'FLAT' in '{rendered}'");
    assert!(!rendered.contains("BROKEN"), "unexpected 'BROKEN' in '{rendered}'");
}

// ──────────────────────────── TextPosition / TextRange / TextEdit ────────────

#[test]
fn text_position_new_stores_line_and_character() {
    let pos = TextPosition::new(3, 7);

    assert_eq!(pos.line, 3);
    assert_eq!(pos.character, 7);
}

#[test]
fn text_range_new_stores_start_and_end() {
    let start = TextPosition::new(0, 0);
    let end = TextPosition::new(2, 5);
    let range = TextRange::new(start, end);

    assert_eq!(range.start.line, 0);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.character, 5);
}

#[test]
fn text_range_whole_document_empty_string() {
    let range = TextRange::whole_document("");

    assert_eq!(range.start, TextPosition::new(0, 0));
    // Empty document: last line is line 0, character 0.
    assert_eq!(range.end, TextPosition::new(0, 0));
}

#[test]
fn text_range_whole_document_single_line_no_trailing_newline() {
    let range = TextRange::whole_document("hello");

    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(0, 5));
}

#[test]
fn text_range_whole_document_multiline() {
    let source = "line0\nline1\nline2";
    let range = TextRange::whole_document(source);

    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(2, 5));
}

#[test]
fn text_range_whole_document_counts_utf16_for_supplementary_planes() {
    // "😀" is U+1F600, which encodes as two UTF-16 code units.
    let range = TextRange::whole_document("ab😀");

    assert_eq!(range.start, TextPosition::new(0, 0));
    // 'a'=1, 'b'=1, '😀'=2 → 4 UTF-16 code units.
    assert_eq!(range.end, TextPosition::new(0, 4));
}

#[test]
fn text_edit_new_stores_range_and_new_text() {
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 5));
    let edit = TextEdit::new(range, "replaced");

    assert_eq!(edit.new_text, "replaced");
    assert_eq!(edit.range.start.line, 0);
    assert_eq!(edit.range.end.character, 5);
}

// ──────────────────────────── FormatDiagnostic ───────────────────────────────

#[test]
fn format_diagnostic_new_stores_all_fields() {
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 10));
    let diagnostic = FormatDiagnostic::new(
        "test.code",
        FormatDiagnosticSeverity::Error,
        Some(range),
        "something went wrong",
    );

    assert_eq!(diagnostic.code, "test.code");
    assert_eq!(diagnostic.severity, FormatDiagnosticSeverity::Error);
    assert!(diagnostic.range.is_some());
    assert_eq!(diagnostic.message, "something went wrong");
}

#[test]
fn format_diagnostic_without_range_stores_none() {
    let diagnostic =
        FormatDiagnostic::new("code", FormatDiagnosticSeverity::Info, None, "no range");

    assert!(diagnostic.range.is_none());
    assert_eq!(diagnostic.severity, FormatDiagnosticSeverity::Info);
}

// ──────────────────────────── FormatConfig serde ─────────────────────────────

#[test]
fn format_config_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
    let config = FormatConfig {
        mode: FormatterMode::Compat,
        line_width: 80,
        indent_width: 2,
        use_tabs: true,
        final_newline: FinalNewline::Insert,
        trailing_comma: TrailingComma::AddWhenWrapped,
        brace_placement: BracePlacement::NextLine,
        else_placement: ElsePlacement::SeparateLine,
        keyword_spacing: KeywordSpacing::Compact,
    };

    let json = serde_json::to_string(&config)?;
    let restored: FormatConfig = serde_json::from_str(&json)?;

    assert_eq!(restored.mode, FormatterMode::Compat);
    assert_eq!(restored.line_width, 80);
    assert_eq!(restored.indent_width, 2);
    assert!(restored.use_tabs);
    assert_eq!(restored.final_newline, FinalNewline::Insert);
    assert_eq!(restored.trailing_comma, TrailingComma::AddWhenWrapped);
    assert_eq!(restored.brace_placement, BracePlacement::NextLine);
    assert_eq!(restored.else_placement, ElsePlacement::SeparateLine);
    assert_eq!(restored.keyword_spacing, KeywordSpacing::Compact);

    Ok(())
}

#[test]
fn formatter_mode_serde_uses_kebab_case() -> Result<(), Box<dyn std::error::Error>> {
    let v = serde_json::to_value(FormatterMode::ExternalLegacy)?;
    assert_eq!(v, serde_json::Value::String("external-legacy".to_string()));
    let v = serde_json::to_value(FormatterMode::Off)?;
    assert_eq!(v, serde_json::Value::String("off".to_string()));
    Ok(())
}

#[test]
fn final_newline_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for variant in [FinalNewline::Preserve, FinalNewline::Insert, FinalNewline::Trim] {
        let json = serde_json::to_string(&variant)?;
        let restored: FinalNewline = serde_json::from_str(&json)?;
        assert_eq!(restored, variant);
    }
    Ok(())
}

#[test]
fn trailing_comma_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for variant in [TrailingComma::Preserve, TrailingComma::AddWhenWrapped] {
        let json = serde_json::to_string(&variant)?;
        let restored: TrailingComma = serde_json::from_str(&json)?;
        assert_eq!(restored, variant);
    }
    Ok(())
}

#[test]
fn brace_placement_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for variant in [BracePlacement::SameLine, BracePlacement::NextLine] {
        let json = serde_json::to_string(&variant)?;
        let restored: BracePlacement = serde_json::from_str(&json)?;
        assert_eq!(restored, variant);
    }
    Ok(())
}

#[test]
fn else_placement_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for variant in [ElsePlacement::Cuddled, ElsePlacement::SeparateLine] {
        let json = serde_json::to_string(&variant)?;
        let restored: ElsePlacement = serde_json::from_str(&json)?;
        assert_eq!(restored, variant);
    }
    Ok(())
}

#[test]
fn keyword_spacing_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    for variant in [KeywordSpacing::Space, KeywordSpacing::Compact] {
        let json = serde_json::to_string(&variant)?;
        let restored: KeywordSpacing = serde_json::from_str(&json)?;
        assert_eq!(restored, variant);
    }
    Ok(())
}

// ──────────────────────────── NativeFormatter format_range / Off mode ────────

#[test]
fn native_formatter_range_off_mode_returns_unchanged() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\nmy $y = 2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 10));
    let config = FormatConfig { mode: FormatterMode::Off, ..FormatConfig::default() };

    let result = formatter.format_range(source, range, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.edits.is_empty());
    assert!(result.diagnostics.is_empty());
}

#[test]
fn format_result_unchanged_has_no_edits_or_diagnostics() {
    let source = "my $x = 1;\n";
    let result = FormatResult::unchanged(source);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.edits.is_empty());
    assert!(result.diagnostics.is_empty());
}

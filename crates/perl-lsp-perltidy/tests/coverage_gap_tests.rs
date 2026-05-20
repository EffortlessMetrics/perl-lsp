/// Tests targeting previously uncovered functions and branches in
/// `perl-lsp-perltidy` (both `lib.rs` and `native.rs`).
///
/// Naming convention: `<fn_name>_<case_description>`.
use perl_lsp_perltidy::{
    BracePlacement, BuiltInFormatter, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnostic,
    FormatDiagnosticSeverity, FormatDoc, FormatResult, FormatterMode, KeywordSpacing,
    NativeFormatter, PerlFormatter, PerlTidyConfig, PerlTidyFormatter, TextEdit, TextPosition,
    TextRange, TrailingComma,
};
use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
use perl_tdd_support::must;
use std::sync::Arc;

// ── TextPosition ─────────────────────────────────────────────────────────────

#[test]
fn text_position_new_stores_line_and_character() {
    let pos = TextPosition::new(3, 7);
    assert_eq!(pos.line, 3);
    assert_eq!(pos.character, 7);
}

// ── TextRange ────────────────────────────────────────────────────────────────

#[test]
fn text_range_new_stores_start_and_end() {
    let start = TextPosition::new(0, 0);
    let end = TextPosition::new(1, 5);
    let range = TextRange::new(start, end);
    assert_eq!(range.start, start);
    assert_eq!(range.end, end);
}

#[test]
fn text_range_whole_document_empty_string() {
    let range = TextRange::whole_document("");
    assert_eq!(range.start, TextPosition::new(0, 0));
    // Empty string: last line 0, character 0.
    assert_eq!(range.end, TextPosition::new(0, 0));
}

#[test]
fn text_range_whole_document_single_line_no_newline() {
    // "hello" has 5 chars, no newline — whole doc is line 0, char 5.
    let range = TextRange::whole_document("hello");
    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(0, 5));
}

#[test]
fn text_range_whole_document_multi_line() {
    // Three lines: "a\nb\nc" — last line is index 2, length 1.
    let range = TextRange::whole_document("a\nb\nc");
    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(2, 1));
}

#[test]
fn text_range_whole_document_trailing_newline_last_line_is_final_content_line() {
    // Rust's str::lines() does not yield an empty line after a trailing newline,
    // so "ab\n" has one line "ab" at index 0 with character offset 2.
    let range = TextRange::whole_document("ab\n");
    assert_eq!(range.start, TextPosition::new(0, 0));
    assert_eq!(range.end, TextPosition::new(0, 2));
}

// ── TextEdit ─────────────────────────────────────────────────────────────────

#[test]
fn text_edit_new_stores_range_and_text() {
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 3));
    let edit = TextEdit::new(range, "replacement");
    assert_eq!(edit.range, range);
    assert_eq!(edit.new_text, "replacement");
}

// ── FormatDiagnostic ─────────────────────────────────────────────────────────

#[test]
fn format_diagnostic_new_without_range() {
    let diag = FormatDiagnostic::new(
        "test.code",
        FormatDiagnosticSeverity::Info,
        None,
        "informational message",
    );
    assert_eq!(diag.code, "test.code");
    assert_eq!(diag.severity, FormatDiagnosticSeverity::Info);
    assert!(diag.range.is_none());
    assert_eq!(diag.message, "informational message");
}

#[test]
fn format_diagnostic_new_with_range() {
    let range = TextRange::new(TextPosition::new(2, 4), TextPosition::new(2, 10));
    let diag = FormatDiagnostic::new(
        "test.error",
        FormatDiagnosticSeverity::Error,
        Some(range),
        "error at range",
    );
    assert_eq!(diag.severity, FormatDiagnosticSeverity::Error);
    assert_eq!(diag.range, Some(range));
}

#[test]
fn format_diagnostic_severity_warning_variant() {
    let diag = FormatDiagnostic::new(
        "test.warn",
        FormatDiagnosticSeverity::Warning,
        None,
        "warning message",
    );
    assert_eq!(diag.severity, FormatDiagnosticSeverity::Warning);
}

// ── FormatResult ─────────────────────────────────────────────────────────────

#[test]
fn format_result_unchanged_has_no_edits_or_diagnostics() {
    let result = FormatResult::unchanged("my $x = 1;\n");
    assert_eq!(result.formatted, "my $x = 1;\n");
    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert!(result.diagnostics.is_empty());
}

// ── FormatDoc IR ─────────────────────────────────────────────────────────────

#[test]
fn format_doc_hardline_renders_newline_with_indent() {
    let config = FormatConfig { indent_width: 4, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("start"),
        FormatDoc::HardLine,
        FormatDoc::text("end"),
    ]);
    let rendered = doc.render(&config);
    assert_eq!(rendered, "start\nend");
}

#[test]
fn format_doc_literal_preserve_is_kept_verbatim() {
    let doc = FormatDoc::literal_preserve("raw # text { }");
    let rendered = doc.render(&FormatConfig::default());
    assert_eq!(rendered, "raw # text { }");
}

#[test]
fn format_doc_space_renders_single_space() {
    let doc = FormatDoc::group(vec![FormatDoc::text("a"), FormatDoc::Space, FormatDoc::text("b")]);
    let rendered = doc.render(&FormatConfig::default());
    assert_eq!(rendered, "a b");
}

#[test]
fn format_doc_softline_renders_as_space_when_flat() {
    // A group that fits: SoftLine becomes a space.
    let doc = FormatDoc::group(vec![
        FormatDoc::text("("),
        FormatDoc::SoftLine,
        FormatDoc::text("x"),
        FormatDoc::SoftLine,
        FormatDoc::text(")"),
    ]);
    let rendered = doc.render(&FormatConfig::default());
    assert_eq!(rendered, "( x )");
}

#[test]
fn format_doc_softline_renders_as_newline_when_broken() {
    // A group that does not fit: SoftLine becomes a newline.
    let config = FormatConfig { line_width: 5, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("("),
        FormatDoc::SoftLine,
        FormatDoc::text("longer_value"),
        FormatDoc::SoftLine,
        FormatDoc::text(")"),
    ]);
    let rendered = doc.render(&config);
    assert!(rendered.contains('\n'));
}

#[test]
fn format_doc_if_break_selects_broken_branch() {
    let config = FormatConfig { line_width: 1, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::if_break(FormatDoc::text("broken"), FormatDoc::text("flat")),
        FormatDoc::SoftLine,
        FormatDoc::text("x"),
    ]);
    let rendered = doc.render(&config);
    assert!(rendered.contains("broken"));
}

#[test]
fn format_doc_indent_indents_parts_at_next_level() {
    // Use line_width=4 so the group (flat_width=5) does not fit and must break.
    let config = FormatConfig { line_width: 4, indent_width: 2, ..FormatConfig::default() };
    let doc = FormatDoc::group(vec![
        FormatDoc::text("{"),
        FormatDoc::indent(vec![FormatDoc::SoftLine, FormatDoc::text("x")]),
        FormatDoc::SoftLine,
        FormatDoc::text("}"),
    ]);
    let rendered = doc.render(&config);
    // When broken, indent level 1 with width 2 => two spaces before "x".
    assert!(rendered.contains("\n  x"), "rendered: {rendered:?}");
}

// ── FormatterMode / enum serde roundtrips ────────────────────────────────────

#[test]
fn formatter_mode_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (mode, expected_json) in [
        (FormatterMode::Native, "\"native\""),
        (FormatterMode::Compat, "\"compat\""),
        (FormatterMode::ExternalLegacy, "\"external-legacy\""),
        (FormatterMode::Off, "\"off\""),
    ] {
        let serialized = serde_json::to_string(&mode)?;
        assert_eq!(serialized, expected_json, "unexpected JSON for {mode:?}");
        let deserialized: FormatterMode = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, mode, "roundtrip failed for {mode:?}");
    }
    Ok(())
}

#[test]
fn final_newline_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (variant, expected_json) in [
        (FinalNewline::Preserve, "\"preserve\""),
        (FinalNewline::Insert, "\"insert\""),
        (FinalNewline::Trim, "\"trim\""),
    ] {
        let serialized = serde_json::to_string(&variant)?;
        assert_eq!(serialized, expected_json);
        let deserialized: FinalNewline = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, variant);
    }
    Ok(())
}

#[test]
fn trailing_comma_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (variant, expected_json) in [
        (TrailingComma::Preserve, "\"preserve\""),
        (TrailingComma::AddWhenWrapped, "\"add-when-wrapped\""),
    ] {
        let serialized = serde_json::to_string(&variant)?;
        assert_eq!(serialized, expected_json);
        let deserialized: TrailingComma = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, variant);
    }
    Ok(())
}

#[test]
fn brace_placement_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (variant, expected_json) in
        [(BracePlacement::SameLine, "\"same-line\""), (BracePlacement::NextLine, "\"next-line\"")]
    {
        let serialized = serde_json::to_string(&variant)?;
        assert_eq!(serialized, expected_json);
        let deserialized: BracePlacement = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, variant);
    }
    Ok(())
}

#[test]
fn else_placement_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (variant, expected_json) in [
        (ElsePlacement::Cuddled, "\"cuddled\""),
        (ElsePlacement::SeparateLine, "\"separate-line\""),
    ] {
        let serialized = serde_json::to_string(&variant)?;
        assert_eq!(serialized, expected_json);
        let deserialized: ElsePlacement = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, variant);
    }
    Ok(())
}

#[test]
fn keyword_spacing_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for (variant, expected_json) in
        [(KeywordSpacing::Space, "\"space\""), (KeywordSpacing::Compact, "\"compact\"")]
    {
        let serialized = serde_json::to_string(&variant)?;
        assert_eq!(serialized, expected_json);
        let deserialized: KeywordSpacing = serde_json::from_str(&serialized)?;
        assert_eq!(deserialized, variant);
    }
    Ok(())
}

// ── NativeFormatter — apply_final_newline edge cases ─────────────────────────

#[test]
fn native_formatter_insert_final_newline_on_crlf_source() {
    // FinalNewline::Insert must strip ALL trailing CR/LF chars, then add \n.
    let formatter = NativeFormatter::new();
    let config = FormatConfig { final_newline: FinalNewline::Insert, ..FormatConfig::default() };
    let source = "my $x = 1;\r\n";
    let result = formatter.format_document(source, &config);
    // After insert: stripped "\r\n", then one "\n" appended.
    assert_eq!(result.formatted, "my $x = 1;\n");
}

#[test]
fn native_formatter_trim_final_newline_multiple_trailing_newlines() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };
    let source = "my $x = 1;\n\n\n";
    let result = formatter.format_document(source, &config);
    assert!(!result.formatted.ends_with('\n'));
    assert!(result.formatted.ends_with("my $x = 1;"));
}

#[test]
fn native_formatter_preserve_final_newline_keeps_source_unchanged() {
    let formatter = NativeFormatter::new();
    // Source already clean: no reformatting change expected.
    let config = FormatConfig { final_newline: FinalNewline::Preserve, ..FormatConfig::default() };
    let source = "my $x = 1;\n";
    let result = formatter.format_document(source, &config);
    assert!(!result.changed);
    assert_eq!(result.formatted, source);
}

// ── PerlTidyFormatter — cache_len ────────────────────────────────────────────

#[test]
fn cache_len_grows_with_each_unique_format_call() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"result_a\n".to_vec()));
    runtime.add_response(MockResponse::success(b"result_b\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    assert_eq!(formatter.cache_len(), 0);

    let _ = must(formatter.format("code_a"));
    assert_eq!(formatter.cache_len(), 1);

    let _ = must(formatter.format("code_b"));
    assert_eq!(formatter.cache_len(), 2);
}

#[test]
fn cache_len_does_not_grow_for_repeated_input() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"result\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let _ = must(formatter.format("same_code"));
    let _ = must(formatter.format("same_code"));
    assert_eq!(formatter.cache_len(), 1);
}

#[test]
fn cache_len_resets_after_clear_cache() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    runtime.add_response(MockResponse::success(b"r\n".to_vec()));
    runtime.add_response(MockResponse::success(b"r\n".to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let _ = must(formatter.format("code"));
    assert_eq!(formatter.cache_len(), 1);
    formatter.clear_cache();
    assert_eq!(formatter.cache_len(), 0);
}

// ── BuiltInFormatter — net_delimiter_delta escape handling ───────────────────

#[test]
fn builtin_formatter_does_not_count_braces_inside_single_quoted_strings() {
    // The "{ }" inside single quotes must not affect indent level.
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($ok) {\nmy $s = '{ not a brace }';\n}\n");
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "if ($ok) {");
    assert_eq!(lines[1], "    my $s = '{ not a brace }';");
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_does_not_count_braces_inside_double_quoted_strings() {
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($ok) {\nmy $s = \"{ not a brace }\";\n}\n");
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[0], "if ($ok) {");
    assert_eq!(lines[1], "    my $s = \"{ not a brace }\";");
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_handles_escaped_quote_inside_string() {
    // An escaped single-quote inside a single-quoted string must not close the string.
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($ok) {\nmy $s = 'it\\'s fine';\n}\n");
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[2], "}");
}

#[test]
fn builtin_formatter_stops_counting_at_comment_hash() {
    // A '#' outside strings ends the effective code for delimiter counting.
    let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
    let formatted = formatter.format("if ($ok) {\nprint 1; # { fake open\n}\n");
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[2], "}");
}

// ── BuiltInFormatter — indent_columns=None fallback ──────────────────────────

#[test]
fn builtin_formatter_default_indent_when_indent_columns_none() {
    // indent_columns = None falls back to 4 spaces.
    let config = PerlTidyConfig { indent_columns: None, ..PerlTidyConfig::default() };
    let formatter = BuiltInFormatter::new(config);
    let formatted = formatter.format("if (1) {\nprint;\n}\n");
    let lines: Vec<&str> = formatted.lines().collect();
    assert_eq!(lines[1], "    print;");
}

// ── NativeFormatter — literal preserve regions ───────────────────────────────

#[test]
fn native_formatter_refuses_format_declaration_start() {
    // "format NAME =" at start of line triggers format body detection.
    let formatter = NativeFormatter::new();
    let source = "format REPORT =\n@<<<\n$name\n.\n";
    let config = FormatConfig::default();

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("format body"));
}

#[test]
fn native_formatter_refuses_pod_head2_and_other_pod_markers() {
    // =head2 is a valid POD start marker.
    let formatter = NativeFormatter::new();
    let source = "=head2 SYNOPSIS\n\nSome text.\n\n=cut\n\nmy $x = 1;\n";
    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("POD"));
}

#[test]
fn native_formatter_refuses_pod_encoding_marker() {
    let formatter = NativeFormatter::new();
    let source = "=encoding utf-8\n\n=cut\n\nmy $x = 1;\n";
    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.diagnostics[0].message.contains("POD"));
}

#[test]
fn native_formatter_refuses_indented_heredoc() {
    // <<~ is the indented heredoc introduced in Perl 5.26.
    let formatter = NativeFormatter::new();
    let source = "print <<~END;\n  body\nEND\n";
    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.diagnostics[0].message.contains("heredoc"));
}

#[test]
fn native_formatter_refuses_double_quoted_heredoc_marker() {
    let formatter = NativeFormatter::new();
    let source = "print <<\"HEREDOC\";\nbody\nHEREDOC\n";
    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.diagnostics[0].message.contains("heredoc"));
}

// ── NativeFormatter — format_range parse gate ────────────────────────────────

#[test]
fn native_format_range_off_mode_returns_unchanged() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { mode: FormatterMode::Off, ..FormatConfig::default() };
    let source = "my $x = ;\n"; // would fail parse
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 10));

    let result = formatter.format_range(source, range, &config);

    assert!(!result.changed);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_format_range_skips_when_source_has_parse_error() {
    let formatter = NativeFormatter::new();
    let source = "my $x = ;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 10));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.diagnostics[0].code, "native.format.parse_error");
}

// ── PerlTidyConfig — serialization roundtrip ─────────────────────────────────

#[test]
fn perl_tidy_config_pbp_serializes_and_deserializes() -> Result<(), Box<dyn std::error::Error>> {
    let original = PerlTidyConfig::pbp();
    let json = serde_json::to_string(&original)?;
    let restored: PerlTidyConfig = serde_json::from_str(&json)?;
    assert_eq!(restored.maximum_line_length, original.maximum_line_length);
    assert_eq!(restored.indent_columns, original.indent_columns);
    assert_eq!(restored.cuddled_else, original.cuddled_else);
    Ok(())
}

#[test]
fn perl_tidy_config_gnu_to_args_contains_expected_flags() {
    let args = PerlTidyConfig::gnu().to_args();
    assert!(args.contains(&"--gnu-style".to_string()));
    assert!(args.contains(&"--opening-brace-on-new-line".to_string()));
    assert!(args.contains(&"--nocuddled-else".to_string()));
    assert!(args.contains(&"--no-vertical-alignment".to_string()));
}

// ── PerlTidyConfig — block_comment_indentation flag ─────────────────────────

#[test]
fn config_to_args_includes_block_comment_indentation_flag() {
    let config = PerlTidyConfig { block_comment_indentation: Some(2), ..PerlTidyConfig::default() };
    let args = config.to_args();
    assert!(args.contains(&"--block-comment-indentation=2".to_string()));
}

// ── FormatSuggestion struct fields ───────────────────────────────────────────

#[test]
fn format_suggestion_description_field_for_line_unchanged_case() {
    // A completely unchanged line does not generate a suggestion.
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my $x = 1;\n";
    runtime.add_response(MockResponse::success(original.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert!(suggestions.is_empty());
}

#[test]
fn format_suggestion_fields_for_changed_line() {
    let runtime = Arc::new(MockSubprocessRuntime::new());
    let original = "my$x=1;\n";
    let formatted = "my $x = 1;\n";
    runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let suggestions = must(formatter.get_suggestions(original));
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].line, 0);
    assert_eq!(suggestions[0].original, "my$x=1;");
    assert_eq!(suggestions[0].formatted, "my $x = 1;");
    assert_eq!(suggestions[0].description, "Line formatting change");
    // Test that FormatSuggestion implements Clone + Debug.
    let cloned = suggestions[0].clone();
    assert_eq!(cloned.line, suggestions[0].line);
    let _ = format!("{:?}", suggestions[0]);
}

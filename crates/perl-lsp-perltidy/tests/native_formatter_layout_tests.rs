use perl_lsp_perltidy::{FinalNewline, FormatConfig, NativeFormatter, PerlFormatter};

#[test]
fn native_formatter_formats_simple_lexical_declarations() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\nour @y;\nstate %z;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1;\nour @y;\nstate %z;\n");
    assert_eq!(result.edits.len(), 1);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_preserves_indent_and_line_endings_for_simple_declarations() {
    let formatter = NativeFormatter::new();
    let source = "  my $x=1;\r\n\tour @y;\r\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert_eq!(result.formatted, "  my $x = 1;\r\n\tour @y;\r\n");
}

#[test]
fn native_formatter_is_idempotent_for_simple_lexical_layout() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\n";

    let once = formatter.format_document(source, &FormatConfig::default());
    let twice = formatter.format_document(&once.formatted, &FormatConfig::default());

    assert_eq!(once.formatted, twice.formatted);
    assert!(!twice.changed);
}

#[test]
fn native_formatter_keeps_unsupported_lines_unchanged() {
    let formatter = NativeFormatter::new();
    let source = "my $x=1;\nprint$x;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert_eq!(result.formatted, "my $x = 1;\nprint$x;\n");
}

#[test]
fn native_formatter_preserves_comment_lines_until_comment_aware_layout_exists() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1; # keep this exact comment\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
}

#[test]
fn native_formatter_combines_simple_layout_with_final_newline_policy() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { final_newline: FinalNewline::Insert, ..FormatConfig::default() };

    let result = formatter.format_document("my $x=1;", &config);

    assert_eq!(result.formatted, "my $x = 1;\n");
}

#[test]
fn native_formatter_formats_compact_keyword_variable_boundary_when_tokenized_safely() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1;\n");
}

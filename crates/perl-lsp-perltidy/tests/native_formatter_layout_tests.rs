use perl_lsp_perltidy::{
    FinalNewline, FormatConfig, NativeFormatter, PerlFormatter, TextPosition, TextRange,
};

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
fn native_formatter_formats_simple_binary_expressions() {
    let formatter = NativeFormatter::new();
    let source = "my$x=$y+1;\nreturn$x*2;\nif($x==2){return$y+1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my $x = $y + 1;\nreturn $x * 2;\nif ($x == 2) {\n    return $y + 1;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_assignments() {
    let formatter = NativeFormatter::new();
    let source = "$x=1;\n$y=$x+2;\nsub bump{$x=$x+1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "$x = 1;\n$y = $x + 2;\nsub bump {\n    $x = $x + 1;\n    return $x;\n}\n"
    );
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_formats_simple_call_expressions() {
    let formatter = NativeFormatter::new();
    let source = "my$x=foo($y,1);\n$z=bar();\nreturn baz($x,$z);\nfoo($x,bar());\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my $x = foo($y, 1);\n$z = bar();\nreturn baz($x, $z);\nfoo($x, bar());\n"
    );
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
fn native_range_formatter_formats_only_selected_simple_declaration_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nmy$y=2;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 7));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nmy $y = 2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(
        result.edits[0].range,
        TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 7))
    );
    assert_eq!(result.edits[0].new_text, "my $y = 2;");
}

#[test]
fn native_range_formatter_formats_selected_simple_binary_expression_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=$y+1;\nreturn$x*2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 10));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = $y + 1;\nreturn$x*2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my $x = $y + 1;");
}

#[test]
fn native_range_formatter_formats_selected_simple_assignment_line() {
    let formatter = NativeFormatter::new();
    let source = "$x=1;\n$y=$x+2;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 8));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "$x=1;\n$y = $x + 2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "$y = $x + 2;");
}

#[test]
fn native_range_formatter_formats_selected_simple_call_expression_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=foo($y,1);\nreturn baz($x,$y);\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 15));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = foo($y, 1);\nreturn baz($x,$y);\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "my $x = foo($y, 1);");
}

#[test]
fn native_range_formatter_treats_end_line_at_character_zero_as_exclusive() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nmy$y=2;\n";
    let range = TextRange::new(TextPosition::new(0, 0), TextPosition::new(1, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert_eq!(result.formatted, "my $x = 1;\nmy$y=2;\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].new_text, "my $x = 1;");
}

#[test]
fn native_formatter_formats_compact_keyword_variable_boundary_when_tokenized_safely() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1;\n");
}

#[test]
fn native_formatter_expands_simple_subroutine_blocks() {
    let formatter = NativeFormatter::new();
    let source = "sub answer{my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "sub answer {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_subroutine_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "sub answer{return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "sub answer {\n  return 1;\n}\n");
}

#[test]
fn native_formatter_expands_simple_if_blocks() {
    let formatter = NativeFormatter::new();
    let source = "if($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "if ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_if_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "  if($ok){return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "  if ($ok) {\n    return 1;\n  }\n");
}

#[test]
fn native_formatter_expands_simple_while_blocks() {
    let formatter = NativeFormatter::new();
    let source = "while($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "while ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_uses_configured_indent_for_simple_while_blocks() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { indent_width: 2, ..FormatConfig::default() };
    let source = "  while($ok){return 1;}\n";

    let result = formatter.format_document(source, &config);

    assert_eq!(result.formatted, "  while ($ok) {\n    return 1;\n  }\n");
}

#[test]
fn native_formatter_expands_simple_unless_blocks() {
    let formatter = NativeFormatter::new();
    let source = "unless($ok){my$x=1;return$x;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "unless ($ok) {\n    my $x = 1;\n    return $x;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_until_blocks() {
    let formatter = NativeFormatter::new();
    let source = "until($done){return 1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "until ($done) {\n    return 1;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_if_else_blocks() {
    let formatter = NativeFormatter::new();
    let source = "if($ok){return 1;}else{return 0;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "if ($ok) {\n    return 1;\n} else {\n    return 0;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_expands_simple_unless_else_blocks() {
    let formatter = NativeFormatter::new();
    let source = "unless($ok){return 0;}else{return 1;}\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "unless ($ok) {\n    return 0;\n} else {\n    return 1;\n}\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_range_formatter_formats_selected_simple_subroutine_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nsub answer{our@y;return@y;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 27));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nsub answer {\n    our @y;\n    return @y;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "sub answer {\n    our @y;\n    return @y;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_if_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nif($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 18));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nif ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "if ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_while_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nwhile($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 21));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nwhile ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "while ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_unless_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nunless($ok){return$x;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 22));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(result.formatted, "my$x=1;\nunless ($ok) {\n    return $x;\n}\n");
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "unless ($ok) {\n    return $x;\n}");
}

#[test]
fn native_range_formatter_formats_selected_simple_if_else_line() {
    let formatter = NativeFormatter::new();
    let source = "my$x=1;\nif($ok){return 1;}else{return 0;}\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 33));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(result.changed);
    assert_eq!(
        result.formatted,
        "my$x=1;\nif ($ok) {\n    return 1;\n} else {\n    return 0;\n}\n"
    );
    assert_eq!(result.edits.len(), 1);
    assert_eq!(result.edits[0].range, range);
    assert_eq!(result.edits[0].new_text, "if ($ok) {\n    return 1;\n} else {\n    return 0;\n}");
}

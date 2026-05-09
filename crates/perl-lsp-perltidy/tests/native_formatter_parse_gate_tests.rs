use perl_lsp_perltidy::{
    FinalNewline, FormatConfig, FormatterMode, NativeFormatter, PerlFormatter, TextPosition,
    TextRange,
};

#[test]
fn native_formatter_leaves_clean_source_unchanged_before_layout_passes_exist() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.edits.is_empty());
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_can_apply_final_newline_policy_after_clean_parse() {
    let formatter = NativeFormatter::new();
    let insert = FormatConfig { final_newline: FinalNewline::Insert, ..FormatConfig::default() };
    let trim = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let inserted = formatter.format_document("my $x = 1;", &insert);
    let trimmed = formatter.format_document("my $x = 1;\n\n", &trim);

    assert!(inserted.changed);
    assert_eq!(inserted.formatted, "my $x = 1;\n");
    assert!(trimmed.changed);
    assert_eq!(trimmed.formatted, "my $x = 1;");
}

#[test]
fn native_formatter_skips_edits_when_source_has_parse_diagnostics() {
    let formatter = NativeFormatter::new();
    let source = "my $x = ;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.edits.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "native.format.parse_error");
    assert!(result.diagnostics[0].message.contains("does not parse cleanly"));
}

#[test]
fn native_formatter_reports_utf16_parse_error_range() {
    let formatter = NativeFormatter::new();
    let source = "my $face = \"😀\";\nmy $x = ;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert_eq!(result.diagnostics.len(), 1);
    assert!(result.diagnostics[0].range.is_some());
}

#[test]
fn native_formatter_refuses_pod_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "=pod\n\n=head1 NAME\n\n=cut\n\nmy $x = 1;\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("POD"));
}

#[test]
fn native_formatter_refuses_heredoc_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "print <<'EOF';\nraw { text }\nEOF\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("heredoc"));
}

#[test]
fn native_formatter_refuses_data_section_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\n__DATA__\nraw\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("DATA/END section"));
}

#[test]
fn native_formatter_refuses_end_section_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\n__END__   \nraw\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("DATA/END section"));
}

#[test]
fn native_formatter_refuses_regex_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "my $matched = $text =~ /needle/i;\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("regex literal"));
}

#[test]
fn native_formatter_refuses_substitution_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "$text =~ s/foo/bar/g;\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("substitution operator"));
}

#[test]
fn native_formatter_refuses_quote_like_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "my @words = qw(alpha beta gamma);\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("quote-like operator"));
}

#[test]
fn native_formatter_refuses_format_body_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "format STDOUT =\n@<<<<\n$name\n.\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("format body"));
}

#[test]
fn native_formatter_does_not_treat_bitshift_as_heredoc() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1 << 2;";
    let config = FormatConfig { final_newline: FinalNewline::Insert, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(result.changed);
    assert_eq!(result.formatted, "my $x = 1 << 2;\n");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_range_formatter_is_parse_gated_but_does_not_rewrite_yet() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\nmy $y = 2;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 10));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.edits.is_empty());
    assert!(result.diagnostics.is_empty());
}

#[test]
fn native_formatter_off_mode_never_parses_or_edits() {
    let formatter = NativeFormatter::new();
    let config = FormatConfig { mode: FormatterMode::Off, ..FormatConfig::default() };
    let source = "my $x = ;\n";

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert!(result.diagnostics.is_empty());
}

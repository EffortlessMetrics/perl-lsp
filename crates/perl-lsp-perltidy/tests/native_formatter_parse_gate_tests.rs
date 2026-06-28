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
fn native_formatter_refuses_transliteration_until_preservation_pass_exists() {
    let formatter = NativeFormatter::new();
    let source = "$text =~ tr/a-z/A-Z/;\n";
    let config = FormatConfig { final_newline: FinalNewline::Trim, ..FormatConfig::default() };

    let result = formatter.format_document(source, &config);

    assert!(!result.changed);
    assert_eq!(result.formatted, source);
    assert_eq!(result.diagnostics[0].code, "native.format.literal_preserve_region");
    assert!(result.diagnostics[0].message.contains("transliteration operator"));
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

// ── range-format preserve-gate scoping (the fix for the over-conservative bail-out) ──

/// Core property: formatting a clean line range succeeds even when the document
/// has a regex on another line.  Before the fix, `validate_clean_parse` was
/// called on the full source, so a regex anywhere in the document would silently
/// abort range formatting — even if the requested lines were completely clean.
#[test]
fn range_format_clean_lines_succeeds_when_regex_is_elsewhere_in_document() {
    let formatter = NativeFormatter::new();
    // Line 0 (0-based) has a regex; line 1 is a clean declaration.
    // A `sub` line with `{` and `}` on one line is something the formatter
    // can reformat — use a simple sub that the formatter will recognise.
    let source = "my $ok = $t =~ /needle/;\nsub   foo{}\n";
    // Request formatting of line 1 only.
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    // Must not produce a literal_preserve_region diagnostic — the regex is not
    // in the requested range.
    assert!(
        result.diagnostics.iter().all(|d| d.code != "native.format.literal_preserve_region"),
        "should not bail with literal_preserve_region when regex is outside the range; \
         got diagnostics: {:?}",
        result.diagnostics,
    );
}

/// Range-format that overlaps a regex must still return unchanged with the
/// preserve-region diagnostic — the bail-out is correct when the construct IS
/// in the requested range.
#[test]
fn range_format_bails_when_range_itself_contains_regex() {
    let formatter = NativeFormatter::new();
    // Line 0 is clean; line 1 has a regex.
    let source = "my $x = 1;\nmy $ok = $t =~ /needle/;\n";
    // Request formatting of line 1 (where the regex lives).
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert_eq!(
        result.diagnostics.iter().find(|d| d.code == "native.format.literal_preserve_region"),
        result.diagnostics.first(),
        "expected a literal_preserve_region diagnostic"
    );
    assert!(
        result.diagnostics.first().is_some_and(|d| d.message.contains("regex literal")),
        "diagnostic should mention regex literal; got: {:?}",
        result.diagnostics,
    );
}

/// Whole-document formatting behavior is unregressed: a document with a regex
/// anywhere must still produce a literal_preserve_region diagnostic when
/// format_document is called.
#[test]
fn document_format_still_bails_on_regex_anywhere_in_document() {
    let formatter = NativeFormatter::new();
    let source = "my $x = 1;\nmy $ok = $t =~ /needle/;\n";

    let result = formatter.format_document(source, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert!(
        result.diagnostics.iter().any(|d| d.code == "native.format.literal_preserve_region"),
        "format_document should still bail for regex anywhere in the document; \
         got diagnostics: {:?}",
        result.diagnostics,
    );
}

/// Heredoc on a different line than the requested range — range-format should
/// proceed (the heredoc is outside the range).
#[test]
fn range_format_clean_lines_succeeds_when_heredoc_is_elsewhere_in_document() {
    let formatter = NativeFormatter::new();
    // Line 0 has a heredoc start; line 1 is a clean declaration.
    let source = "print <<'EOF';\nmy $x = 1;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(
        result.diagnostics.iter().all(|d| d.code != "native.format.literal_preserve_region"),
        "should not bail with literal_preserve_region when heredoc is outside the range; \
         got diagnostics: {:?}",
        result.diagnostics,
    );
}

/// Range-format that covers a line with a heredoc marker must still bail.
#[test]
fn range_format_bails_when_range_contains_heredoc() {
    let formatter = NativeFormatter::new();
    // Line 0 is clean; line 1 has a heredoc start.
    let source = "my $x = 1;\nprint <<'EOF';\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert!(
        result
            .diagnostics
            .first()
            .is_some_and(|d| d.code == "native.format.literal_preserve_region"
                && d.message.contains("heredoc")),
        "expected heredoc literal_preserve_region diagnostic; got: {:?}",
        result.diagnostics,
    );
}

/// A POD block outside the range must not block range-format of clean lines.
#[test]
fn range_format_clean_lines_succeeds_when_pod_is_elsewhere_in_document() {
    let formatter = NativeFormatter::new();
    // Line 0 has a POD marker; line 1 is a clean declaration.
    let source = "=head1 NAME\nmy $x = 1;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(
        result.diagnostics.iter().all(|d| d.code != "native.format.literal_preserve_region"),
        "should not bail with literal_preserve_region when POD is outside the range; \
         got diagnostics: {:?}",
        result.diagnostics,
    );
}

/// qw() (quote-words, a quote-like operator) outside the range — range-format
/// of a clean line should succeed.
#[test]
fn range_format_clean_lines_succeeds_when_qw_is_elsewhere_in_document() {
    let formatter = NativeFormatter::new();
    // Line 0 has qw(); line 1 is clean.
    let source = "my @words = qw(alpha beta);\nmy $x = 1;\n";
    let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(2, 0));

    let result = formatter.format_range(source, range, &FormatConfig::default());

    assert!(
        result.diagnostics.iter().all(|d| d.code != "native.format.literal_preserve_region"),
        "should not bail with literal_preserve_region when qw() is outside the range; \
         got diagnostics: {:?}",
        result.diagnostics,
    );
}

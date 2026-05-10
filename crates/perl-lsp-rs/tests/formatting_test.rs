//! Integration tests for code formatting

use perl_lsp::convert::{WirePosition, WireRange};
use perl_lsp::features::formatting::{CodeFormatter, FormattingOptions};
use perl_tdd_support::must;

#[test]
fn test_basic_formatting() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    // Test simple unformatted code supported by the native formatter.
    let code = "my$x=1;\n";

    let edits = must(formatter.format_document(code, &options));
    assert_eq!(edits.len(), 1, "native default formatting should return one edit");
    assert_eq!(edits[0].new_text, "my $x = 1;\n");
}

#[test]
fn test_range_formatting() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    // Multi-line code
    let code = "my $x = 1;\nsub test{return$x;}\nmy $y = 2;";

    // Format only the middle line
    let range = WireRange { start: WirePosition::new(1, 0), end: WirePosition::new(1, 20) };

    let edits = must(formatter.format_range(code, &range, &options));
    assert_eq!(edits.len(), 1, "native default range formatting should return one edit");
    assert_eq!(edits[0].new_text, "sub test {\n    return $x;\n}");
}

#[test]
fn test_formatting_preserves_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "my$x=1; # keep\n";

    let edits = must(formatter.format_document(code, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "my $x = 1; # keep\n");
}

#[test]
fn test_formatting_preserves_simple_block_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "if($ok){return 1;} # if tail\n";

    let edits = must(formatter.format_document(code, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "if ($ok) {\n    return 1;\n} # if tail\n");
}

#[test]
fn test_range_formatting_preserves_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "my$x=1; # keep\nmy$y=2;\n";
    let range = WireRange { start: WirePosition::new(0, 0), end: WirePosition::new(0, 14) };

    let edits = must(formatter.format_range(code, &range, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "my $x = 1; # keep");
}

#[test]
fn test_range_formatting_keeps_neighboring_leading_comment_outside_selected_line() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "# applies to next declaration\nmy$x=1;\nmy$y=2;\n";
    let range = WireRange { start: WirePosition::new(1, 0), end: WirePosition::new(1, 7) };

    let edits = must(formatter.format_range(code, &range, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "my $x = 1;");
    assert_eq!(edits[0].range.start.line, 1);
    assert_eq!(edits[0].range.end.line, 1);
}

#[test]
fn test_range_formatting_preserves_simple_block_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "if($ok){return 1;} # if tail\nmy$z=3;\n";
    let range = WireRange { start: WirePosition::new(0, 0), end: WirePosition::new(0, 29) };

    let edits = must(formatter.format_range(code, &range, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "if ($ok) {\n    return 1;\n} # if tail");
}

#[test]
fn test_formatting_returns_no_edits_for_literal_preserve_regions() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: None,
    };

    for code in [
        "my $matched = $text =~ /needle/i;   \n",
        "$text =~ s/foo/bar/g;   \n",
        "$text =~ tr/a-z/A-Z/;   \n",
        "my @words = qw(alpha beta gamma);   \n",
        "print <<'EOF';   \nraw { text }\nEOF\n",
        "my $x = 1;   \n__DATA__\nraw fixture bytes\n",
        "my $x = 1;   \n__END__\nraw fixture bytes\n",
        "format STDOUT =\n@<<<<\n$name\n.\nwrite;   \n",
        "=pod\n\n=head1 NAME   \n\n=cut\n\nmy $x = 1;   \n",
    ] {
        let edits = must(formatter.format_document(code, &options));
        assert!(edits.is_empty(), "literal-preserve source should not produce edits: {code:?}");
    }
}

#[test]
fn test_empty_document() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    let edits = must(formatter.format_document("", &options));
    assert!(edits.is_empty(), "native default formatting should not edit an empty document");
}

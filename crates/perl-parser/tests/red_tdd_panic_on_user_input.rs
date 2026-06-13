//! Red TDD tests for issue #1372: Parser must never panic on arbitrary user input.
//!
//! These tests verify that the parser gracefully handles:
//! - Alternative regex delimiters (m#, m!, m|, etc.)
//! - Quote-like operators with various delimiters (q{}, q[], q||, etc.)
//! - Heredoc variants (<<'EOF', <<"EOF", <<`EOF`, <<~EOF)
//! - Unterminated/incomplete constructs
//! - Deep nesting (1000+ levels)
//!
//! Each test MUST NOT panic. Instead, the parser should return either:
//! - A valid AST with appropriate error recovery nodes
//! - A Result::Err that can be inspected
//!
//! THESE TESTS ARE RED (FAILING) UNTIL THE PARSER IS FIXED.

use perl_parser::Parser;
use std::panic;

/// Test that parser doesn't panic on regex with hash delimiter.
#[test]
fn red_test_regex_hash_delimiter_no_panic() {
    let code = r#"if ($text =~ m#pattern#) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on regex with hash delimiter m#pattern#"
    );

    // If parse succeeds, we should have a valid result
    if let Ok(parse_result) = result {
        assert!(
            parse_result.is_ok() || parse_result.is_err(),
            "Parse result should be Ok or Err, not panic"
        );
    }
}

/// Test that parser doesn't panic on regex with exclamation delimiter.
#[test]
fn red_test_regex_bang_delimiter_no_panic() {
    let code = r#"if ($text =~ m!pattern!) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on regex with bang delimiter m!pattern!"
    );
}

/// Test that parser doesn't panic on regex with pipe delimiter.
#[test]
fn red_test_regex_pipe_delimiter_no_panic() {
    let code = r#"if ($text =~ m|pattern|) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on regex with pipe delimiter m|pattern|"
    );
}

/// Test that parser doesn't panic on quote-like with braces.
#[test]
fn red_test_quote_like_braces_no_panic() {
    let code = r#"my $str = q{test string};"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on quote-like with braces q{{...}}"
    );
}

/// Test that parser doesn't panic on quote-like with brackets.
#[test]
fn red_test_quote_like_brackets_no_panic() {
    let code = r#"my $str = q[test string];"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on quote-like with brackets q[...]"
    );
}

/// Test that parser doesn't panic on quote-like with pipe.
#[test]
fn red_test_quote_like_pipe_no_panic() {
    let code = r#"my $str = q|test string|;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on quote-like with pipe q|...|"
    );
}

/// Test that parser doesn't panic on single-quoted heredoc.
#[test]
fn red_test_heredoc_single_quoted_no_panic() {
    let code = r#"my $text = <<'EOF';
Single quoted heredoc
EOF"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on single-quoted heredoc <<'EOF'"
    );
}

/// Test that parser doesn't panic on double-quoted heredoc.
#[test]
fn red_test_heredoc_double_quoted_no_panic() {
    let code = r#"my $text = <<"EOF";
Double quoted heredoc with $variable
EOF"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on double-quoted heredoc <<\"EOF\""
    );
}

/// Test that parser doesn't panic on backtick heredoc.
#[test]
fn red_test_heredoc_backtick_no_panic() {
    let code = r#"my $text = <<`EOF`;
Command heredoc
EOF"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on backtick heredoc <<`EOF`"
    );
}

/// Test that parser doesn't panic on indented heredoc.
#[test]
fn red_test_heredoc_indented_no_panic() {
    let code = r#"my $text = <<~EOF;
    Indented heredoc
    With content
EOF"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on indented heredoc <<~EOF"
    );
}

/// Test that parser doesn't panic on unterminated regex.
#[test]
fn red_test_unterminated_regex_no_panic() {
    let code = r#"if ($text =~ /unclosed_regex) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on unterminated regex /unclosed_regex"
    );
}

/// Test that parser doesn't panic on unclosed quote.
#[test]
fn red_test_unclosed_quote_no_panic() {
    let code = r#"my $str = "unclosed string without closing quote;
my $x = 1;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on unclosed quote");
}

/// Test that parser doesn't panic on unterminated heredoc.
#[test]
fn red_test_unterminated_heredoc_no_panic() {
    let code = r#"my $text = <<EOF;
This is an unterminated heredoc
WRONGEOF
my $x = 1;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on unterminated heredoc");
}

/// Test that parser doesn't panic on deep nesting (100 levels).
#[test]
fn red_test_deep_nesting_100_levels_no_panic() {
    // Build nested structure with 100 levels
    let mut code = String::new();
    for _ in 0..100 {
        code.push_str("{ ");
    }
    code.push_str("1");
    for _ in 0..100 {
        code.push_str(" }");
    }

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(&code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on 100-level deep nesting"
    );
}

/// Test that parser doesn't panic on very deep nesting (1000 levels).
#[test]
fn red_test_deep_nesting_1000_levels_no_panic() {
    // Build nested structure with 1000 levels
    let mut code = String::new();
    for _ in 0..1000 {
        code.push_str("{ ");
    }
    code.push_str("1");
    for _ in 0..1000 {
        code.push_str(" }");
    }

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(&code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on 1000-level deep nesting"
    );
}

/// Test that parser doesn't panic on empty input.
#[test]
fn red_test_empty_input_no_panic() {
    let code = "";

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on empty input");
}

/// Test that parser doesn't panic on single character.
#[test]
fn red_test_single_char_input_no_panic() {
    let code = "x";

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on single character input");
}

/// Test that parser doesn't panic on whitespace only.
#[test]
fn red_test_whitespace_only_no_panic() {
    let code = "    \n    \t    ";

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on whitespace-only input");
}

/// Test that parser doesn't panic on regex with embedded newline.
#[test]
fn red_test_regex_embedded_newline_no_panic() {
    let code = r#"if ($text =~ /pattern
with embedded newline/) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on regex with embedded newline"
    );
}

/// Test that parser doesn't panic on UTF-8 boundary conditions.
#[test]
fn red_test_utf8_boundaries_no_panic() {
    // UTF-8 multi-byte characters
    let code = "my $str = \"こんにちは世界\";";

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on UTF-8 boundaries");
}

/// Test that parser doesn't panic on mixed UTF-8 in strings.
#[test]
fn red_test_mixed_utf8_in_strings_no_panic() {
    let code = r#"my $str1 = "ASCII";
my $str2 = "混合 mixed";
my $str3 = "🎉 emoji";"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on mixed UTF-8 in strings"
    );
}

/// Test that parser doesn't panic on unbalanced parentheses.
#[test]
fn red_test_unbalanced_parens_no_panic() {
    let code = "my $x = (1 + 2;";

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on unbalanced parentheses");
}

/// Test that parser doesn't panic on mixed valid and invalid syntax.
#[test]
fn red_test_mixed_valid_invalid_no_panic() {
    let code = r#"my $valid1 = 42;
my $invalid = (unclosed;
my $valid2 = "string";
if ($text =~ /unclosed) {
my $valid3 = 99;
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on mixed valid and invalid syntax"
    );
}

/// Test that parser doesn't panic on ambiguous bareword regex delimiters.
#[test]
fn red_test_ambiguous_bareword_regex_delim_no_panic() {
    let code = r#"my $result = ambiguous_function;
if ($text =~ m#pattern#) {
    print "Match\n";
}"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on ambiguous bareword with regex delimiters"
    );
}

/// Test that parser doesn't panic on substitution with hash delimiter.
#[test]
fn red_test_substitution_hash_delimiter_no_panic() {
    let code = r#"$text =~ s#old#new#;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on substitution with hash delimiter"
    );
}

/// Test that parser doesn't panic on transliteration with hash delimiter.
#[test]
fn red_test_transliteration_hash_delimiter_no_panic() {
    let code = r#"$text =~ tr#abc#xyz#;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on transliteration with hash delimiter"
    );
}

/// Test that parser doesn't panic on qq with braces.
#[test]
fn red_test_qq_braces_no_panic() {
    let code = r#"my $str = qq{interpolated $variable};"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on qq with braces qq{{...}}"
    );
}

/// Test that parser doesn't panic on qx with braces.
#[test]
fn red_test_qx_backtick_no_panic() {
    let code = r#"my $output = qx`command with $variable`;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on qx with backtick"
    );
}

/// Test that parser doesn't panic on qw with various delimiters.
#[test]
fn red_test_qw_various_delimiters_no_panic() {
    let code = r#"my @items1 = qw(item1 item2 item3);
my @items2 = qw[item1 item2 item3];
my @items3 = qw{item1 item2 item3};"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on qw with various delimiters"
    );
}

/// Test that parser doesn't panic on nested quote-like within heredoc.
#[test]
fn red_test_quote_like_in_heredoc_no_panic() {
    let code = r#"my $text = <<"EOF";
Contains q{quoted} and m#regex# inside
EOF"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on quote-like inside heredoc"
    );
}

/// Test that parser doesn't panic on very long string literal.
#[test]
fn red_test_very_long_string_no_panic() {
    let code = r#"my $long_str = ""#.to_string() + &"x".repeat(10000) + r#"";"#;

    let result = panic::catch_unwind(move || {
        let mut parser = Parser::new(&code);
        parser.parse()
    });

    assert!(result.is_ok(), "Parser panicked on very long string literal");
}

/// Test that parser doesn't panic on many sequential quotes.
#[test]
fn red_test_many_sequential_quotes_no_panic() {
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!("my $str{} = \"string{}\";\n", i, i));
    }

    let result = panic::catch_unwind(move || {
        let mut parser = Parser::new(&code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on many sequential quotes"
    );
}

/// Test that parser doesn't panic on incomplete comment-like syntax.
#[test]
fn red_test_incomplete_comment_like_no_panic() {
    let code = r#"my $x = 1;
# This is a comment
my $y = 2;
=head1 Incomplete POD
my $z = 3;"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on incomplete comment-like syntax"
    );
}

/// Test comprehensive ambiguous syntax from the original test.
#[test]
fn red_test_full_ambiguous_syntax_no_panic() {
    let code = r#"
# Ambiguous bareword vs function call
my $result = ambiguous_function;  # Could be bareword or function call
my $another = another_ambiguous($param);  # Could be method or function

# Ambiguous operator precedence
my $precedence1 = 1 + 2 * 3;  # Should be 1 + (2 * 3)
my $precedence2 = 4 * 5 + 6;  # Should be (4 * 5) + 6

# Ambiguous dereferencing
my $deref1 = $$ref;  # Could be scalar deref or code deref
my $deref2 = $hash->{key}[0];  # Could be hash then array or array then hash

# Ambiguous regex delimiters
if ($text =~ m#pattern#) {  # Using # as delimiter
    print "Match\n";
}
if ($text =~ m!pattern!) {  # Using ! as delimiter
    print "Match\n";
}
if ($text =~ m|pattern|) {  # Using | as delimiter
    print "Match\n";
}

# Ambiguous string delimiters
my $str1 = q{test string};  # Using braces as delimiter
my $str2 = q[test string];  # Using brackets as delimiter
my $str3 = q|test string|;  # Using pipe as delimiter

# Ambiguous heredoc delimiters
my $heredoc1 = <<'EOF';
Single quoted heredoc
EOF

my $heredoc2 = <<"EOF";
Double quoted heredoc with $variable interpolation
EOF

my $heredoc3 = <<`EOF`;
Command heredoc
EOF
"#;

    let result = panic::catch_unwind(|| {
        let mut parser = Parser::new(code);
        parser.parse()
    });

    assert!(
        result.is_ok(),
        "Parser panicked on full ambiguous syntax test case"
    );
}

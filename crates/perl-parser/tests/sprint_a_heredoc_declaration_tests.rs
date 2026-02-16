//! Comprehensive test scaffolding for heredoc declaration parsing
//!
//! ## Specification Reference
//! Tests feature spec: Sprint A Issue #183 - Heredoc Declaration Parser
//!
//! ## Test Coverage
//! This test suite validates:
//! - Bare heredoc labels (<<EOF)
//! - Double-quoted labels (<<"EOF")
//! - Single-quoted labels (<<'EOF')
//! - Backtick labels (<<`EOF`)
//! - Escaped characters in labels
//! - CRLF line endings support
//! - Exact terminator matching (not contains)
//! - Invalid terminator detection
//! - Multiple heredocs on single line
//! - Edge cases and malformed input
//!
//! ## Architecture
//! These tests target the perl-parser crate's heredoc declaration parsing
//! capabilities, ensuring proper recognition of all heredoc label styles
//! and terminator matching semantics.

use perl_parser::Parser;

/// Helper function to parse code and return AST S-expression
fn parse_to_sexp(input: &str) -> String {
    use perl_tdd_support::must;
    let mut parser = Parser::new(input);
    must(parser.parse()).to_sexp()
}

/// Helper function to parse code and verify it succeeds
fn parse_and_verify_success(input: &str, _test_name: &str) {
    use perl_tdd_support::must;
    let mut parser = Parser::new(input);
    must(parser.parse());
}

// ============================================================================
// Test Group 1: Bare Heredoc Labels (<<EOF style)
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - bare heredoc label parsing
///
/// Validates that heredoc declarations with bare (unquoted) labels are
/// correctly recognized and parsed by the parser.
#[test]
fn test_heredoc_decl_bare_label_simple() {
    let input = r#"my $x = <<EOF;
content here
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_bare_label_simple");
    let sexp = parse_to_sexp(input);

    // Verify heredoc declaration is present in AST
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\" \"content here\")"),
        "Expected heredoc declaration with label EOF and content, got: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - bare label alphanumeric variations
///
/// Validates that heredoc labels can contain letters, numbers, and underscores
/// following Perl heredoc identifier rules.
#[test]
fn test_heredoc_decl_bare_label_alphanumeric() {
    let test_cases =
        vec!["<<END_OF_DATA", "<<EOF123", "<<SQL_QUERY", "<<HTML_CONTENT", "<<DATA_2024"];

    for label_decl in test_cases {
        let label = &label_decl[2..];
        let input = format!("my $x = {};\ncontent\n{}\n", label_decl, label);
        parse_and_verify_success(&input, "test_heredoc_decl_bare_label_alphanumeric");

        let sexp = parse_to_sexp(&input);
        assert!(
            sexp.contains(&format!("(heredoc_interpolated \"{}\"", label)),
            "AST should contain heredoc label {}: {}",
            label,
            sexp
        );
    }
}

// ============================================================================
// Test Group 2: Double-Quoted Labels (<<"EOF" style)
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - double-quoted heredoc labels
///
/// Validates that heredoc declarations with double-quoted labels enable
/// variable interpolation in the heredoc body (standard Perl behavior).
#[test]
fn test_heredoc_decl_double_quoted_label() {
    let input = r#"my $x = <<"EOF";
content $var here
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_double_quoted_label");
    let sexp = parse_to_sexp(input);

    // Validate AST indicates interpolation is enabled for this heredoc
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\""),
        "Expected double-quoted heredoc to have interpolated flag, got: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - double-quoted label with escapes
///
/// Validates that escape sequences in double-quoted labels are properly
/// handled according to Perl string interpolation rules.
#[test]
fn test_heredoc_decl_double_quoted_label_with_escapes() {
    let input = r#"my $x = <<"END\nLINE";
content here
END
LINE
"#;
    parse_and_verify_success(input, "test_heredoc_decl_double_quoted_label_with_escapes");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"END\\nLINE\" \"content here\\nEND\\nLINE\")"),
        "Expected label with newline and content, got: {}",
        sexp
    );
    assert!(
        sexp.contains("(UNKNOWN_REST)"),
        "Should contain UNKNOWN_REST due to escape handling: {}",
        sexp
    );
}

// ============================================================================
// Test Group 3: Single-Quoted Labels (<<'EOF' style)
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - single-quoted heredoc labels
///
/// Validates that heredoc declarations with single-quoted labels disable
/// variable interpolation in the heredoc body (standard Perl behavior).
#[test]
fn test_heredoc_decl_single_quoted_label() {
    let input = r#"my $x = <<'EOF';
content $var here (not interpolated)
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_single_quoted_label");
    let sexp = parse_to_sexp(input);

    // Validate AST indicates interpolation is disabled for this heredoc
    assert!(
        sexp.contains("(heredoc \"EOF\""),
        "Expected single-quoted heredoc to NOT have interpolated flag in name, got: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - single-quoted label with special chars
///
/// Validates that special characters within single-quoted labels are treated
/// literally (no escape processing).
#[test]
fn test_heredoc_decl_single_quoted_label_special_chars() {
    let input = r#"my $x = <<'END$DATA';
content here
END$DATA
"#;
    parse_and_verify_success(input, "test_heredoc_decl_single_quoted_label_special_chars");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc \"END$DATA\" \"content here\")"),
        "Special chars in label should be preserved literal: {}",
        sexp
    );
}

// ============================================================================
// Test Group 4: Backtick Labels (<<`EOF` style)
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - backtick heredoc labels
///
/// Validates that heredoc declarations with backtick labels enable command
/// execution with the heredoc body passed to the shell.
#[test]
fn test_heredoc_decl_backtick_label() {
    let input = r#"my $x = <<`EOF`;
echo "command content"
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_backtick_label");
    let sexp = parse_to_sexp(input);

    // Validate AST indicates command execution for this heredoc
    assert!(
        sexp.contains("(heredoc_command \"EOF\""),
        "Expected backtick heredoc to have command flag, got: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - backtick label with interpolation
///
/// Validates that backtick heredocs support variable interpolation before
/// command execution (standard Perl behavior).
#[test]
fn test_heredoc_decl_backtick_label_with_vars() {
    let input = r#"my $x = <<`CMD`;
echo "$var"
CMD
"#;
    parse_and_verify_success(input, "test_heredoc_decl_backtick_label_with_vars");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_command \"CMD\""),
        "Backtick heredoc should have command flag: {}",
        sexp
    );
}

// ============================================================================
// Test Group 5: Escaped Characters in Labels
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - escaped characters in heredoc labels
///
/// Validates that escape sequences in heredoc labels are properly recognized
/// and handled according to the quoting style.
#[test]
fn test_heredoc_decl_label_with_escapes() {
    let input = r#"my $x = <<"END\tTAB";
content with tab in label
END	TAB
"#;
    parse_and_verify_success(input, "test_heredoc_decl_label_with_escapes");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"END\\tTAB\" \"content with tab in label\")"),
        "Expected label with tab and content, got: {}",
        sexp
    );
    assert!(sexp.contains("(UNKNOWN_REST)"), "Should contain UNKNOWN_REST: {}", sexp);
}

/// Tests feature spec: Sprint A Issue #183 - backslash escapes in labels
///
/// Validates that backslash escape sequences are handled correctly in
/// double-quoted heredoc labels.
#[test]
fn test_heredoc_decl_label_backslash_escapes() {
    let input = r#"my $x = <<"END\\SLASH";
content here
END\SLASH
"#;
    parse_and_verify_success(input, "test_heredoc_decl_label_backslash_escapes");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"END\\\\SLASH\" \"content here\")"),
        "Expected label with backslash and content, got: {}",
        sexp
    );
    assert!(sexp.contains("(UNKNOWN_REST)"), "Should contain UNKNOWN_REST: {}", sexp);
}

// ============================================================================
// Test Group 6: CRLF Line Endings
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - CRLF line ending support
///
/// Validates that heredoc declarations correctly handle CRLF line endings
/// (Windows-style \r\n), which is critical for cross-platform compatibility.
#[test]
fn test_heredoc_decl_crlf_line_endings() {
    let input = "my $x = <<EOF;\r\ncontent line 1\r\ncontent line 2\r\nEOF\r\n";
    parse_and_verify_success(input, "test_heredoc_decl_crlf_line_endings");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\" \"content line 1\\ncontent line 2\")"),
        "CRLF heredoc content mismatch: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - mixed line endings
///
/// Validates parser behavior with mixed LF and CRLF line endings, which
/// can occur in files edited on multiple platforms.
#[test]
fn test_heredoc_decl_mixed_line_endings() {
    let input = "my $x = <<EOF;\ncontent with LF\r\nEOF\r\n";
    parse_and_verify_success(input, "test_heredoc_decl_mixed_line_endings");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\" \"content with LF\")"),
        "Mixed line ending content mismatch: {}",
        sexp
    );
}

// ============================================================================
// Test Group 7: Exact Terminator Matching
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - exact terminator matching
///
/// Validates that heredoc terminator matching requires an exact match on a
/// line by itself (not substring matching). This is critical for correct
/// heredoc body parsing.
#[test]
fn test_heredoc_decl_exact_terminator_not_contains() {
    let input = r#"my $x = <<EOF;
This line contains EOF but is not the terminator
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_exact_terminator_not_contains");
    let sexp = parse_to_sexp(input);

    // Validate that "contains EOF" line is treated as body content,
    // not as terminator
    assert!(
        sexp.contains("This line contains EOF but is not the terminator"),
        "Embedded label should be in content: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - terminator with leading whitespace
///
/// Validates that terminators with leading/trailing whitespace are NOT
/// recognized as valid terminators (except for indented heredocs with <<~).
#[test]
fn test_heredoc_decl_terminator_whitespace_invalid() {
    let input = r#"my $x = <<EOF;
content
  EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_terminator_whitespace_invalid");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\" \"content\")")
            && sexp.contains("(UNKNOWN_REST)"),
        "Leading whitespace label should trigger UNKNOWN_REST if not terminated: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - case-sensitive terminator matching
///
/// Validates that heredoc terminators are case-sensitive, so 'EOF' != 'eof'.
#[test]
fn test_heredoc_decl_terminator_case_sensitive() {
    let input = r#"my $x = <<EOF;
eof should not terminate
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_terminator_case_sensitive");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("eof should not terminate"),
        "Case-different label should be part of content: {}",
        sexp
    );
}

// ============================================================================
// Test Group 8: Invalid Terminator Scenarios
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - missing terminator detection
///
/// Validates parser behavior when heredoc terminator is never provided,
/// which should produce appropriate error or warning.
#[test]
fn test_heredoc_decl_missing_terminator() {
    let input = r#"my $x = <<EOF;
content without terminator"#;

    let mut parser = Parser::new(input);
    let _ = parser.parse();
    let errors = parser.errors();
    assert!(
        errors.iter().any(|e| e.to_string().contains("Unterminated heredoc")),
        "Expected missing terminator error, got: {:?}",
        errors
    );
}

/// Tests feature spec: Sprint A Issue #183 - empty label detection
///
/// Validates that empty heredoc labels (<<) are handled appropriately,
/// either with error or by using empty string as label.
#[test]
fn test_heredoc_decl_empty_label() {
    let input = "my $x = <<;\ncontent\n\n";

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok(), "Empty label (<<;) should be accepted");
    let sexp = match result {
        Ok(ast) => ast.to_sexp(),
        Err(e) => format!("(ERROR \"{}\")", e),
    };
    assert!(
        sexp.contains("(heredoc_interpolated \"\" \"content\")") || sexp.contains("(ERROR"),
        "Empty label missing in AST or valid Error: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - malformed label with invalid chars
///
/// Validates parser behavior when heredoc label contains invalid characters
/// for the given quoting style.
#[test]
fn test_heredoc_decl_invalid_label_chars() {
    let input = r#"my $x = <<'EOF WITH SPACES';
content
EOF WITH SPACES
"#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok(), "Quoted label with spaces should be accepted");
}

// ============================================================================
// Test Group 9: Multiple Heredocs
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - multiple heredocs on single line
///
/// Validates that multiple heredoc declarations on a single line are correctly
/// parsed with FIFO body processing (first declared, first body consumed).
#[test]
fn test_heredoc_decl_multiple_on_single_line() {
    let input = r#"print <<FIRST, <<SECOND;
body of first
FIRST
body of second
SECOND
"#;
    parse_and_verify_success(input, "test_heredoc_decl_multiple_on_single_line");
    let sexp = parse_to_sexp(input);

    // Validate that both heredoc declarations are recognized
    // and bodies are associated correctly in FIFO order
    assert!(
        sexp.contains("(heredoc_interpolated \"FIRST\" \"body of first\")"),
        "Missing first heredoc: {}",
        sexp
    );
    assert!(
        sexp.contains("(heredoc_interpolated \"SECOND\" \"body of second\")"),
        "Missing second heredoc: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - three heredocs with different styles
///
/// Validates that multiple heredocs with different quoting styles can be
/// declared on the same line.
#[test]
fn test_heredoc_decl_multiple_mixed_styles() {
    let input = r#"print <<EOF, <<"QUOTED", <<'LITERAL';
unquoted body
EOF
quoted body
QUOTED
literal body
LITERAL
"#;
    parse_and_verify_success(input, "test_heredoc_decl_multiple_mixed_styles");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"EOF\" \"unquoted body\")"),
        "Missing EOF heredoc: {}",
        sexp
    );
    assert!(
        sexp.contains("(heredoc_interpolated \"QUOTED\" \"quoted body\")"),
        "Missing QUOTED heredoc: {}",
        sexp
    );
    assert!(
        sexp.contains("(heredoc \"LITERAL\" \"literal body\")"),
        "Missing LITERAL heredoc: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - nested heredoc declarations
///
/// Validates that heredocs can be used within function calls and complex
/// expressions, with proper declaration and body association.
#[test]
fn test_heredoc_decl_nested_in_expression() {
    let input = r#"my $result = join("\n", <<A, <<B);
first content
A
second content
B
"#;
    parse_and_verify_success(input, "test_heredoc_decl_nested_in_expression");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"A\" \"first content\")"),
        "Missing A heredoc: {}",
        sexp
    );
    assert!(
        sexp.contains("(heredoc_interpolated \"B\" \"second content\")"),
        "Missing B heredoc: {}",
        sexp
    );
}

// ============================================================================
// Test Group 10: Edge Cases
// ============================================================================

/// Tests feature spec: Sprint A Issue #183 - indented heredoc with <<~
///
/// Validates that indented heredocs (<<~) are recognized as declarations
/// with automatic indentation stripping in the body.
#[test]
fn test_heredoc_decl_indented_style() {
    let input = r#"my $x = <<~EOF;
    indented content
    more indented
  EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_indented_style");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains(
            "(heredoc_indented_interpolated \"EOF\" \"  indented content\\n  more indented\")"
        ),
        "Indented heredoc mismatch: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - heredoc in assignment chain
///
/// Validates that heredoc declarations work correctly in assignment chains
/// and complex statement contexts.
#[test]
fn test_heredoc_decl_in_assignment_chain() {
    let input = r#"my ($x, $y) = (<<A, <<B);
first value
A
second value
B
"#;
    parse_and_verify_success(input, "test_heredoc_decl_in_assignment_chain");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"A\" \"first value\")"),
        "Missing A in list: {}",
        sexp
    );
    assert!(
        sexp.contains("(heredoc_interpolated \"B\" \"second value\")"),
        "Missing B in list: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - heredoc with empty body
///
/// Validates that heredocs with empty bodies (immediate terminator) are
/// correctly parsed as valid heredoc declarations.
#[test]
fn test_heredoc_decl_empty_body() {
    let input = r#"my $x = <<EOF;
EOF
"#;
    parse_and_verify_success(input, "test_heredoc_decl_empty_body");

    let sexp = parse_to_sexp(input);
    assert!(sexp.contains("(heredoc_interpolated \"EOF\" \"\")"), "Empty body mismatch: {}", sexp);
}

/// Tests feature spec: Sprint A Issue #183 - heredoc label max length
///
/// Validates that very long heredoc labels are handled correctly by the
/// parser without buffer overflows or performance issues.
#[test]
fn test_heredoc_decl_long_label() {
    let long_label = "A".repeat(255);
    let input = format!("my $x = <<{};\ncontent\n{}\n", long_label, long_label);
    parse_and_verify_success(&input, "test_heredoc_decl_long_label");

    let sexp = parse_to_sexp(&input);
    assert!(
        sexp.contains(&format!("(heredoc_interpolated \"{}\"", long_label)),
        "Long label missing: {}",
        sexp
    );
}

/// Tests feature spec: Sprint A Issue #183 - unicode in heredoc labels
///
/// Validates that Unicode characters in heredoc labels are correctly parsed
/// for full internationalization support.
#[test]
fn test_heredoc_decl_unicode_label() {
    let input = r#"my $x = <<"データ";
content here
データ
"#;
    parse_and_verify_success(input, "test_heredoc_decl_unicode_label");

    let sexp = parse_to_sexp(input);
    assert!(
        sexp.contains("(heredoc_interpolated \"データ\""),
        "Unicode label missing or incorrect: {}",
        sexp
    );
}

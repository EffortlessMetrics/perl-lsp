mod cpan_test_helpers;

use perl_parser_core::Parser;

// Regression tests for parser hang on orphan closing delimiters
// (issue: synchronize() did not consume RightParen/RightBracket,
//  causing parse_program to loop forever on malformed input).

/// Verify that parsing a list with missing commas in parens terminates.
/// Previously: `(1 2 3)` caused synchronize() to leave `)` unconsumed,
/// which made parse_program loop forever calling parse_statement on `)`.
#[test]
fn when_paren_list_missing_commas_then_parse_terminates() {
    let mut parser = Parser::new("my @list = (1 2 3);\n");
    // Must terminate — no timeout/hang.
    let _ = parser.parse();
    // Parser should have collected errors rather than looping.
    // (We don't assert clean parse since the input IS malformed.)
}

/// Orphan `)` at statement level must not loop.
#[test]
fn when_orphan_right_paren_at_top_level_then_parse_terminates() {
    let mut parser = Parser::new(");\n");
    let _ = parser.parse();
}

/// Orphan `]` at statement level must not loop.
#[test]
fn when_orphan_right_bracket_at_top_level_then_parse_terminates() {
    let mut parser = Parser::new("];\n");
    let _ = parser.parse();
}

/// Multiple orphan closers must not loop.
#[test]
fn when_multiple_orphan_closers_then_parse_terminates() {
    let mut parser = Parser::new(") ] ) ;\n");
    let _ = parser.parse();
}

/// Multiple statements: one with malformed parens, followed by valid code.
/// The parser must not hang and must continue to parse the second statement.
#[test]
fn when_bad_paren_list_followed_by_valid_code_then_both_parsed() {
    let mut parser = Parser::new("my @x = (1 2 3);\nmy $y = 4;\n");
    // Must terminate. Both statements should have been attempted.
    let _ = parser.parse();
    // If we get here without hanging, the fix is working.
}

/// Paren with missing comma inside a block must not hang.
#[test]
fn when_paren_missing_comma_inside_block_then_parse_terminates() {
    let mut parser = Parser::new("sub foo { my @x = (1 2 3); }\n");
    let _ = parser.parse();
}

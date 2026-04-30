//! Regression tests for scoped subroutine declarations.
//!
//! Perl allows `my sub`, `our sub`, and `state sub` declarations.
//! These should parse as subroutine statements rather than variable declarations.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_ast::ast::NodeKind;
use perl_parser_core::{ParseError, Parser};

#[test]
fn parses_my_sub_declaration() {
    assert_clean_parse("my sub helper ($x) { $x }");
}

#[test]
fn parses_our_sub_declaration() {
    assert_clean_parse("our sub helper ($x) { $x }");
}

#[test]
fn parses_state_sub_declaration() {
    assert_clean_parse("state sub memo { state $x = 1; $x }");
}

#[test]
fn parses_scoped_sub_forward_declarations() {
    assert_clean_parse("my sub helper; our sub exported; state sub memoized;");
}

#[test]
fn recovers_after_scoped_sub_missing_name() {
    let mut parser = Parser::new("my sub { 1 }; my $x = 2;");
    let output = parser.parse_with_recovery();
    let sexp = output.ast.to_sexp();

    assert!(
        output
            .diagnostics
            .iter()
            .any(|d| {
                matches!(
                    d,
                    ParseError::SyntaxError { message, .. }
                        if message.contains("Expected subroutine name after scoped declarator")
                )
            }),
        "missing-name scoped sub should produce an explicit diagnostic: {:?}",
        output.diagnostics
    );
    assert!(
        sexp.contains("(anonymous_subroutine_expression"),
        "scoped sub recovery should preserve an anonymous-sub structure: {sexp}"
    );

    if let NodeKind::Program { statements } = &output.ast.kind {
        assert!(
            statements.len() >= 2,
            "parser should recover and keep following statement, got {} items",
            statements.len()
        );
    } else {
        panic!("expected Program root, got {:?}", output.ast.kind);
    }
}

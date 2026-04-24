//! Regression tests for declaration keywords used as assignment RHS expressions.
//!
//! Perl allows declaration forms (notably `local`) as expression terms, including
//! as the RHS of another assignment. The parser should preserve declaration
//! structure instead of treating `local` as a plain identifier.

mod cpan_test_helpers;

use cpan_test_helpers::assert_clean_parse;
use perl_parser_core::Parser;

#[test]
fn parses_local_assignment_rhs_with_anonymous_sub_initializer() {
    let source = "my $guard = local $SIG{__WARN__} = sub { 1; };";
    assert_clean_parse(source);

    let mut parser = Parser::new(source);
    let ast = parser.parse_with_recovery().ast;
    let rendered = ast.to_sexp();

    assert!(
        rendered.contains("local_declaration"),
        "expected local declaration in rhs AST, got: {rendered}"
    );
    assert!(
        rendered.contains("anonymous_subroutine_expression"),
        "expected anonymous sub in rhs initializer, got: {rendered}"
    );
}

#[test]
fn keeps_local_autoquoted_before_fat_arrow() {
    assert_clean_parse("my %h = (local => 1, my => 2);");
}

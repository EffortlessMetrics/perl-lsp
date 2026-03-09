//! Parser-level tests for inline POD block handling.
//!
//! Verifies that POD blocks embedded in Perl source do not produce ERROR nodes
//! in the parsed AST.

use perl_parser::Parser;
use perl_tdd_support::must;

/// Helper to parse and assert no ERROR nodes in the AST.
fn assert_parses_without_errors(code: &str) {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "AST should not contain ERROR nodes for code with inline POD.\nCode: {code}\nAST: {sexp}"
    );
}

#[test]
fn inline_pod_between_statements_no_errors() {
    assert_parses_without_errors("my $x = 1;\n=head1 NAME\nFoo\n=cut\nmy $y = 2;");
}

#[test]
fn inline_pod_at_start_of_file_no_errors() {
    assert_parses_without_errors("=head1 NAME\nFoo\n=cut\nmy $x = 1;");
}

#[test]
fn inline_pod_at_eof_without_cut_no_errors() {
    assert_parses_without_errors("my $x = 1;\n=head1 NAME\nFoo");
}

#[test]
fn multiple_inline_pod_sections_no_errors() {
    assert_parses_without_errors(
        "my $a = 1;\n=head1 FIRST\nstuff\n=cut\nmy $b = 2;\n=pod\nmore stuff\n=cut\nmy $c = 3;",
    );
}

#[test]
fn pod_with_sub_definition_no_errors() {
    let code = r#"
sub hello {
    return "world";
}

=head1 NAME

MyModule - A test module

=head1 DESCRIPTION

This is a test.

=cut

sub goodbye {
    return "farewell";
}
"#;
    assert_parses_without_errors(code);
}

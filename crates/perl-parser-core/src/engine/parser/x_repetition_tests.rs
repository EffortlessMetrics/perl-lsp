#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use perl_tdd_support::must;

#[test]
fn test_string_x_number() {
    // "x" x 3 => "xxx"
    let mut parser = Parser::new(r#"my $s = "x" x 3;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_dash_x_80() {
    // "-" x 80
    let mut parser = Parser::new(r#"my $line = "-" x 80;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_space_x_variable() {
    // " " x $indent
    let mut parser = Parser::new(r#"my $spaces = " " x $indent;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_paren_list_x_number() {
    // ($template) x 10
    let mut parser = Parser::new("my @copies = ($template) x 10;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_variable_x_number() {
    // $str x 3
    let mut parser = Parser::new("my $result = $str x 3;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_print_ha_x_3() {
    // print "ha" x 3;
    let mut parser = Parser::new(r#"print "ha" x 3;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

#[test]
fn test_x_not_confused_with_variable() {
    // my $x = 5; -- `x` here is a variable name, not an operator
    let mut parser = Parser::new("my $x = 5;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    // Should NOT contain binary_x -- this is a variable declaration
    assert!(
        !sexp.contains("binary_x"),
        "Should not be binary_x in: {sexp}"
    );
}

#[test]
fn test_x_precedence_with_addition() {
    // In Perl, x has same precedence as *, so: ("ab" x 2) + 1
    let mut parser = Parser::new(r#"my $r = "ab" x 2 + 1;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    // The binary_+ should be the outer op, binary_x should be nested inside
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
    assert!(sexp.contains("binary_+"), "Expected binary_+ in: {sexp}");
}

#[test]
fn test_x_with_concat() {
    // "a" . "b" x 3 should parse as "a" . ("b" x 3) since x has higher precedence than .
    let mut parser = Parser::new(r#"my $r = "a" . "b" x 3;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
    assert!(sexp.contains("binary_."), "Expected binary_. in: {sexp}");
}

#[test]
fn test_x_with_variable_rhs() {
    // "-" x $n -- variable on the right side
    let mut parser = Parser::new(r#"my $s = "-" x $n;"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {sexp}");
}

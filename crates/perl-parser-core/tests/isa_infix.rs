//! Tests for `isa` infix operator (Perl 5.32+ class-check operator).
//!
//! `$obj isa Foo` checks whether `$obj` is an instance of `Foo` (or a subclass).
//! This is distinct from the method call `$obj->isa("Foo")`.
//!
//! Issue #752, finding #3.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ── Helper ──────────────────────────────────────────────────────────────────

/// Walk the AST and find the first Binary node with op == "isa".
fn find_isa_binary(
    node: &perl_parser_core::Node,
) -> Option<(&perl_parser_core::Node, &perl_parser_core::Node)> {
    if let NodeKind::Binary { op, left, right } = &node.kind {
        if op == "isa" {
            return Some((left, right));
        }
    }
    for child in node.children() {
        if let Some(found) = find_isa_binary(child) {
            return Some(found);
        }
    }
    None
}

// ── Primary failing cases (will be red before fix) ──────────────────────────

#[test]
fn test_isa_infix_simple_parses_cleanly() {
    // `$obj isa Foo` — simplest form: fails before fix
    assert_clean_parse(r#"my $r = $obj isa Foo;"#);
}

#[test]
fn test_isa_infix_qualified_parses_cleanly() {
    // `$obj isa Foo::Bar` — qualified class name
    assert_clean_parse(r#"my $r = $obj isa Foo::Bar;"#);
}

#[test]
fn test_isa_infix_scalar_rhs_parses_cleanly() {
    // `$obj isa $class` — dynamic class name
    assert_clean_parse(r#"my $r = $obj isa $class;"#);
}

#[test]
fn test_isa_infix_in_if_condition_parses_cleanly() {
    // `if ($obj isa Foo) {}` — as an if-condition; the original failing example
    assert_clean_parse(r#"if ($obj isa Foo) { 1 }"#);
}

// ── Structural assertions: must produce Binary{op="isa"} ─────────────────────

#[test]
fn test_isa_infix_produces_binary_node_with_op_isa() {
    let mut parser = Parser::new(r#"$obj isa Foo"#);
    let ast = must(parser.parse());
    let (left, _right) = find_isa_binary(&ast).expect("should find a Binary node with op=isa");
    // Left operand must be the $obj variable
    assert!(
        matches!(&left.kind, NodeKind::Variable { sigil, name } if sigil == "$" && name == "obj"),
        "expected left operand to be $obj, got: {:?}",
        left.kind
    );
}

#[test]
fn test_isa_infix_sexp_contains_isa() {
    // to_sexp() for `$obj isa Foo` should include "isa" (the op name)
    let mut parser = Parser::new(r#"$obj isa Foo"#);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(sexp.contains("isa"), "sexp should reference 'isa', got: {}", sexp);
}

// ── Regression: method call must still parse as MethodCall, not infix ────────

#[test]
fn test_isa_method_call_unchanged() {
    // `$obj->isa("Foo")` is a method call — must not be changed
    assert_clean_parse(r#"if ($obj->isa("Foo")) { 1; }"#);
}

#[test]
fn test_isa_method_call_is_method_call_node() {
    let mut parser = Parser::new(r#"$obj->isa("Foo")"#);
    let ast = must(parser.parse());
    // Must NOT produce a Binary{op="isa"} node
    assert!(
        find_isa_binary(&ast).is_none(),
        "method call $obj->isa() must not produce Binary{{op=isa}}, sexp: {}",
        ast.to_sexp()
    );
}

// ── Regression: existing string-comparison operators unchanged ───────────────

#[test]
fn test_eq_operator_unchanged() {
    assert_clean_parse(r#"if ($a eq $b) { 1; }"#);
}

#[test]
fn test_cmp_operator_unchanged() {
    assert_clean_parse(r#"my $r = $a cmp $b;"#);
}

// ── Regression: variable named `isa` still parses ───────────────────────────

#[test]
fn test_variable_named_isa_still_parses() {
    // `my $isa = 1` — `isa` as a variable name (not as an operator)
    assert_clean_parse(r#"my $isa = 1;"#);
}

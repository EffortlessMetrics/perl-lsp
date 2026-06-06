//! Tests for Perl 5.38+ class VERSION syntax.
//!
//! `class Foo VERSION { ... }` allows an optional version after the class name,
//! similar to `package Foo VERSION { ... }`. The parser must consume the version
//! token(s) before proceeding to attributes and the class body block.
//!
//! Covers:
//!   - `class Foo 1.0 { }`        — decimal version
//!   - `class Foo v1.2.3 { }`     — v-string version
//!   - `class Foo { }`            — no version (regression guard)
//!   - `class Foo :isa(Bar) { }`  — attribute without version (regression guard)
//!   - `class Foo v1.0 :isa(Bar) { }` — version then attribute

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ── Core version cases ────────────────────────────────────────────────────────

#[test]
fn class_with_decimal_version_parses_cleanly() {
    assert_clean_parse(r#"class Foo 1.0 { }"#);
}

#[test]
fn class_with_vstring_version_parses_cleanly() {
    assert_clean_parse(r#"class Foo v1.2.3 { }"#);
}

#[test]
fn class_with_integer_version_parses_cleanly() {
    assert_clean_parse(r#"class Foo 2 { }"#);
}

#[test]
fn class_version_produces_a_class_node() {
    let src = r#"class Foo 1.0 { }"#;
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        unreachable!("expected program node, got {}", ast.kind.kind_name());
    };
    let class = statements.iter().find(|s| matches!(s.kind, NodeKind::Class { .. }));
    assert!(class.is_some(), "expected Class node in {}", ast.to_sexp());
}

#[test]
fn class_version_block_is_preserved() {
    // The block must not be dropped — the class node must have a Block child.
    let src = r#"class Foo 1.23 { method greet() { } }"#;
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        unreachable!("expected program node, got {}", ast.kind.kind_name());
    };
    let class = statements
        .iter()
        .find(|s| matches!(s.kind, NodeKind::Class { .. }))
        .unwrap_or_else(|| unreachable!("expected Class node in {}", ast.to_sexp()));
    let NodeKind::Class { body, .. } = &class.kind else {
        unreachable!("expected Class kind, got {}", class.kind.kind_name());
    };
    assert!(
        matches!(body.kind, NodeKind::Block { .. }),
        "class body must be a Block, got {}",
        body.kind.kind_name()
    );
}

#[test]
fn class_with_version_produces_no_parser_errors() {
    let src = r#"class Foo 1.0 { }"#;
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "class with version must produce no parser errors, got: {:?}",
        parser.errors()
    );
}

#[test]
fn class_with_vstring_produces_no_parser_errors() {
    let src = r#"class Foo v1.2.3 { }"#;
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "class with v-string version must produce no parser errors, got: {:?}",
        parser.errors()
    );
}

// ── Version then attribute ────────────────────────────────────────────────────

#[test]
fn class_with_version_then_attribute_parses_cleanly() {
    assert_clean_parse(r#"class Foo v1.0 :isa(Bar) { }"#);
}

#[test]
fn class_with_decimal_version_then_isa_parses_cleanly() {
    assert_clean_parse(r#"class Foo 1.5 :isa(Base) { }"#);
}

// ── Regression guards: existing valid forms must keep working ─────────────────

#[test]
fn class_without_version_still_parses_cleanly() {
    assert_clean_parse(r#"class Foo { }"#);
}

#[test]
fn class_with_attribute_no_version_still_parses_cleanly() {
    assert_clean_parse(r#"class Foo :isa(Bar) { }"#);
}

#[test]
fn class_with_multiple_isa_no_version_still_parses_cleanly() {
    assert_clean_parse(r#"class Foo :isa(Bar) :isa(Baz) { }"#);
}

#[test]
fn class_with_qualified_name_no_version_still_parses_cleanly() {
    assert_clean_parse(r#"class My::App::Widget { }"#);
}

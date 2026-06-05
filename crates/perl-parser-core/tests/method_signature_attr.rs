//! Tests for `method` with BOTH a signature (parens) AND trailing attributes.
//!
//! Issue #752 finding 4: `method foo() :public { }` errors with
//! "expected '{', found ':' at position 13" because `parse_method` goes straight
//! to `parse_block` after the signature, without first checking for trailing
//! attributes (as `parse_subroutine` already does).

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::{must, must_some};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Parse `source` and return the `attributes` vec from the first Method node.
fn method_attributes(source: &str) -> Vec<String> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    find_method_attributes(&ast)
}

fn find_method_attributes(node: &perl_parser_core::Node) -> Vec<String> {
    match &node.kind {
        NodeKind::Method { attributes, .. } => attributes.clone(),
        _ => {
            for child in node.children() {
                let found = find_method_attributes(child);
                if !found.is_empty() {
                    return found;
                }
            }
            vec![]
        }
    }
}

/// Parse `source` and return whether the first Method node has a signature.
fn method_has_signature(source: &str) -> bool {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    find_method_has_signature(&ast)
}

fn find_method_has_signature(node: &perl_parser_core::Node) -> bool {
    match &node.kind {
        NodeKind::Method { signature, .. } => signature.is_some(),
        _ => {
            for child in node.children() {
                if find_method_has_signature(child) {
                    return true;
                }
            }
            false
        }
    }
}

// ── Failing cases: method + signature + trailing attribute ───────────────────
// These tests document the bug: they SHOULD parse cleanly but currently error.

#[test]
fn test_method_with_empty_signature_and_single_attr() {
    // method foo() :public { } — the core failing case from the issue report
    assert_clean_parse(
        r#"
class Foo {
    method foo() :public { }
}
"#,
    );
}

#[test]
fn test_method_with_param_and_single_attr() {
    // method bar($x) :private { } — parameter plus trailing attribute
    assert_clean_parse(
        r#"
class Foo {
    method bar($x) :private { }
}
"#,
    );
}

#[test]
fn test_method_with_empty_signature_and_multiple_attrs() {
    // method foo() :public :other { } — multiple trailing attributes
    assert_clean_parse(
        r#"
class Foo {
    method foo() :public :other { }
}
"#,
    );
}

// ── Attribute capture: trailing attrs are stored on the Method node ──────────

#[test]
fn test_method_signature_attr_captured() {
    // Verify the attribute is stored, not silently dropped
    let source = r#"
class Foo {
    method foo() :public { }
}
"#;
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got: {:?}", ast.kind);
    };
    let class_stmt = must_some(statements.first());
    let NodeKind::Class { body, .. } = &class_stmt.kind else {
        panic!("expected Class, got: {:?}", class_stmt.kind);
    };
    let NodeKind::Block { statements: body_stmts } = &body.kind else {
        panic!("expected Block inside Class, got: {:?}", body.kind);
    };
    let method_node = must_some(body_stmts.first());
    let NodeKind::Method { attributes, signature, .. } = &method_node.kind else {
        panic!("expected Method, got: {:?}", method_node.kind);
    };
    assert!(signature.is_some(), "method foo() should have a signature");
    assert!(
        attributes.iter().any(|a| a == "public"),
        "expected 'public' attribute, got: {:?}",
        attributes,
    );
}

#[test]
fn test_method_param_and_attr_both_captured() {
    // Verify signature has parameters AND attribute is stored
    let source = r#"
class Foo {
    method bar($x) :private { }
}
"#;
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected Program, got: {:?}", ast.kind);
    };
    let class_stmt = must_some(statements.first());
    let NodeKind::Class { body, .. } = &class_stmt.kind else {
        panic!("expected Class, got: {:?}", class_stmt.kind);
    };
    let NodeKind::Block { statements: body_stmts } = &body.kind else {
        panic!("expected Block inside Class, got: {:?}", body.kind);
    };
    let method_node = must_some(body_stmts.first());
    let NodeKind::Method { attributes, signature, .. } = &method_node.kind else {
        panic!("expected Method, got: {:?}", method_node.kind);
    };
    assert!(signature.is_some(), "method bar($x) should have a signature");
    assert!(
        attributes.iter().any(|a| a == "private"),
        "expected 'private' attribute, got: {:?}",
        attributes,
    );
}

// ── Regression: previously-working variants must remain clean ────────────────

#[test]
fn test_method_no_parens_attr_still_works() {
    // method foo :lvalue { } — attribute before (no) signature; already worked
    assert_clean_parse(
        r#"
class Foo {
    method foo :lvalue { }
}
"#,
    );
}

#[test]
fn test_method_empty_sig_no_attr_still_works() {
    // method foo() { } — signature but no attribute; already worked
    assert_clean_parse(
        r#"
class Foo {
    method foo() { }
}
"#,
    );
}

#[test]
fn test_method_param_no_attr_still_works() {
    // method foo($x) { } — parameter but no trailing attribute; already worked
    assert_clean_parse(
        r#"
class Foo {
    method foo($x) { }
}
"#,
    );
}

#[test]
fn test_sub_with_sig_and_attr_unchanged() {
    // sub foo() :lvalue { } — sub with signature + attr; already worked
    assert_clean_parse(r#"sub foo() :lvalue { }"#);
}

#[test]
fn test_sub_with_attr_no_sig_unchanged() {
    // sub foo :lvalue { } — sub attr without signature; already worked
    assert_clean_parse(r#"sub foo :lvalue { }"#);
}

// ── Additional coverage: attributes stored correctly for leading attrs ────────

#[test]
fn test_method_leading_attr_still_captured() {
    // Leading attribute (before body, no signature) still stored correctly after fix
    let attrs = method_attributes(r#"method size :lvalue { }"#);
    assert!(
        attrs.iter().any(|a| a == "lvalue"),
        "expected 'lvalue' in attributes, got: {:?}",
        attrs,
    );
}

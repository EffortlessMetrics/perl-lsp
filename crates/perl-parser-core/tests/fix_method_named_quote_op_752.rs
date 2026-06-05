//! Regression tests for Object::Pad `method` named after quote-like operators.
//!
//! Issue #752 finding 5: `method y() { ... }` errors because the lexer treats `y`
//! (and `s`, `tr`) as transliteration/substitution operators instead of identifiers
//! after `method`. The lexer's `after_sub` mechanism already handles `sub y {}` —
//! this fix extends it to `method`.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::{must, must_some};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn find_first_method(node: &Node) -> Option<&Node> {
    match &node.kind {
        NodeKind::Method { .. } => Some(node),
        _ => {
            for child in node.children() {
                if let Some(found) = find_first_method(child) {
                    return Some(found);
                }
            }
            None
        }
    }
}

fn has_node_kind_name(node: &Node, name: &str) -> bool {
    if node.kind.kind_name() == name {
        return true;
    }
    node.children().iter().any(|child| has_node_kind_name(child, name))
}

// ── Failing cases: method names that are also quote-like operators ─────────────
// These tests document the bug (finding 5 from issue #752).
// They SHOULD produce a proper Method node. Before the fix:
//   - `method y()` and `method tr()` silently produce a transliteration node instead
//   - `method s { }` produces an Error node (substitution parse failure)

/// Verify `method y()` produces a Method node, not a transliteration node.
///
/// Before the fix, the lexer consumes `y(` as a transliteration operator using `(`
/// as its delimiter, resulting in an incorrect AST without a Method node.
#[test]
fn test_method_named_y_produces_method_node() -> Result<(), String> {
    let source = "method y() { return 1; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    if find_first_method(&ast).is_none() {
        let sexp = ast.to_sexp();
        return Err(format!(
            "expected a Method node for 'method y() {{...}}', but got none.\nsexp:\n{}",
            sexp
        ));
    }
    Ok(())
}

#[test]
fn test_method_named_s_parses_cleanly() {
    // method named `s` — lexed as substitution operator without the fix
    assert_clean_parse("method s { 1 }");
}

/// Verify `method tr()` produces a Method node, not a transliteration node.
#[test]
fn test_method_named_tr_produces_method_node() -> Result<(), String> {
    let source = "method tr() {}";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    if find_first_method(&ast).is_none() {
        let sexp = ast.to_sexp();
        return Err(format!(
            "expected a Method node for 'method tr() {{}}', but got none.\nsexp:\n{}",
            sexp
        ));
    }
    Ok(())
}

/// Cascade case: class with two methods, one named `y`.
///
/// Before the fix, `method y()` is consumed as a transliteration operator and
/// `method to_string()` is not reached correctly. Both should parse as Method nodes.
#[test]
fn test_class_with_method_named_y_and_other_method() -> Result<(), String> {
    let source = r#"
class Foo {
    method y() { 1 }
    method to_string() { 2 }
}
"#;
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    if find_first_method(&ast).is_none() {
        let sexp = ast.to_sexp();
        return Err(format!(
            "expected a Method node in class with 'method y()' and 'method to_string()', got none.\nsexp:\n{}",
            sexp
        ));
    }
    Ok(())
}

// ── Method name is correctly stored as identifier ─────────────────────────────

#[test]
fn test_method_named_y_stores_correct_name() -> Result<(), String> {
    let source = "method y() { return 1; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let method_node = must_some(find_first_method(&ast));
    let NodeKind::Method { name, .. } = &method_node.kind else {
        return Err(format!("expected Method node, got {}", method_node.kind.kind_name()));
    };
    if name != "y" {
        return Err(format!("expected method name 'y', got '{}'", name));
    }
    Ok(())
}

#[test]
fn test_method_named_s_stores_correct_name() -> Result<(), String> {
    let source = "method s { 1 }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let method_node = must_some(find_first_method(&ast));
    let NodeKind::Method { name, .. } = &method_node.kind else {
        return Err(format!("expected Method node, got {}", method_node.kind.kind_name()));
    };
    if name != "s" {
        return Err(format!("expected method name 's', got '{}'", name));
    }
    Ok(())
}

// ── Critical regression: body-level transliteration still works ──────────────
//
// The `after_sub` flag is cleared when the lexer sees `{`, so transliteration
// inside a method body must still be parsed as a tr/// operator.

#[test]
fn test_transliteration_in_method_body_still_works() {
    // `y/a/b/` inside the method body must still parse as a Transliteration node.
    assert_clean_parse(
        r#"
class Foo {
    method process($s) { $s =~ y/a/b/; return $s; }
}
"#,
    );
}

#[test]
fn test_transliteration_in_method_body_produces_transliteration_node() -> Result<(), String> {
    let source = r#"
class Foo {
    method process($s) { $s =~ y/a/b/; return $s; }
}
"#;
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    if !has_node_kind_name(&ast, "Transliteration") {
        let sexp = ast.to_sexp();
        return Err(format!(
            "expected a Transliteration node in the AST body, but found none.\nsexp:\n{}",
            sexp
        ));
    }
    Ok(())
}

// ── Regression: bare transliteration/substitution operators still work ────────

#[test]
fn test_bare_tr_still_transliterates() {
    assert_clean_parse(r#"$x =~ tr/a/b/;"#);
}

#[test]
fn test_bare_y_still_transliterates() {
    assert_clean_parse(r#"$x =~ y/a/b/;"#);
}

#[test]
fn test_bare_s_still_substitutes() {
    assert_clean_parse(r#"$x =~ s/a/b/;"#);
}

#[test]
fn test_standalone_tr_still_works() {
    assert_clean_parse(r#"tr/a-z/A-Z/;"#);
}

// ── Regression: `sub y {}` still works (existing sub mechanism unaffected) ────

#[test]
fn test_sub_named_y_still_works() {
    assert_clean_parse(r#"sub y { return 1; }"#);
}

#[test]
fn test_sub_named_s_still_works() {
    assert_clean_parse(r#"sub s { 1 }"#);
}

#[test]
fn test_sub_named_tr_still_works() {
    assert_clean_parse(r#"sub tr { 1 }"#);
}

// ── Regression: method -> dispatch with these names still works ───────────────

#[test]
fn test_arrow_method_y_still_works() {
    assert_clean_parse(r#"$obj->y("foo");"#);
}

#[test]
fn test_arrow_method_s_still_works() {
    assert_clean_parse(r#"$obj->s("foo", "bar");"#);
}

#[test]
fn test_arrow_method_tr_still_works() {
    assert_clean_parse(r#"$obj->tr("old", "new");"#);
}

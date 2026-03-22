//! Tests for Perl version compatibility warnings (PL900)
//!
//! These tests verify that the version_compat lint correctly detects uses of
//! Perl features that are not available in the declared Perl version.

use perl_lsp_diagnostics::version_compat::check_version_compat;
use perl_parser_core::{Node, NodeKind, SourceLocation};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn program(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements: stmts }, loc(0, 200))
}

fn block(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements: stmts }, loc(0, 100))
}

fn use_node(module: &str) -> Node {
    Node::new(
        NodeKind::Use { module: module.to_string(), args: vec![], has_filter_risk: false },
        loc(0, 12),
    )
}

fn use_feature(feature: &str) -> Node {
    Node::new(
        NodeKind::Use {
            module: "feature".to_string(),
            args: vec![format!("'{}'", feature)],
            has_filter_risk: false,
        },
        loc(0, 20),
    )
}

fn class_node(name: &str) -> Node {
    Node::new(
        NodeKind::Class { name: name.to_string(), body: Box::new(block(vec![])) },
        loc(20, 50),
    )
}

fn try_node() -> Node {
    Node::new(
        NodeKind::Try { body: Box::new(block(vec![])), catch_blocks: vec![], finally_block: None },
        loc(20, 60),
    )
}

fn say_call() -> Node {
    Node::new(
        NodeKind::FunctionCall {
            name: "say".to_string(),
            args: vec![Node::new(
                NodeKind::String { value: "hello".to_string(), interpolated: false },
                loc(24, 31),
            )],
        },
        loc(20, 32),
    )
}

fn sub_with_signature() -> Node {
    let sig = Node::new(
        NodeKind::Signature {
            parameters: vec![Node::new(
                NodeKind::MandatoryParameter {
                    variable: Box::new(Node::new(
                        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                        loc(25, 27),
                    )),
                },
                loc(25, 27),
            )],
        },
        loc(24, 28),
    );
    Node::new(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: Some(loc(24, 27)),
            prototype: None,
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        loc(20, 60),
    )
}

fn diagnostics_have_code(diagnostics: &[perl_lsp_diagnostics::Diagnostic], code: &str) -> bool {
    diagnostics.iter().any(|d| d.code.as_deref() == Some(code))
}

fn no_compat_warnings(diagnostics: &[perl_lsp_diagnostics::Diagnostic]) -> bool {
    !diagnostics_have_code(diagnostics, "PL900")
}

// ---------------------------------------------------------------------------
// Test 1: class in v5.36 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_class_warns_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), class_node("Foo")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'class' in v5.36, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("class"), "Message should mention 'class': {}", msg.message);
    assert!(
        msg.message.contains("v5.38") || msg.message.contains("5.38"),
        "Message should mention minimum version v5.38: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: class in v5.38 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_class_ok_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), class_node("Foo")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'class' in v5.38, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: class with `use feature 'class'` (no version) -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_class_ok_with_use_feature_class() -> Result<(), Box<dyn std::error::Error>> {
    // use feature 'class' explicitly enables it regardless of version
    let ast = program(vec![use_feature("class"), class_node("Bar")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'class'' is present, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: try/catch in v5.32 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_try_warns_on_v5_32() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.32"), try_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'try' in v5.32, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("try"), "Message should mention 'try': {}", msg.message);
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: try/catch in v5.34 -> no warn (experimental but bundled)
// ---------------------------------------------------------------------------

#[test]
fn test_try_ok_on_v5_34() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.34"), try_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'try' in v5.34, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6: say in v5.8 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_say_warns_on_v5_8() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), say_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'say' in v5.8, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("say"), "Message should mention 'say': {}", msg.message);
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7: say in v5.10 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_say_ok_on_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), say_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'say' in v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 8: no declared version -> no warnings emitted at all
// ---------------------------------------------------------------------------

#[test]
fn test_no_version_no_warnings() -> Result<(), Box<dyn std::error::Error>> {
    // No `use vN.NN` — checker must skip silently
    let ast = program(vec![class_node("Foo"), try_node(), say_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warnings when no version is declared, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 9: numeric form `use 5.036` treated same as `use v5.36`
// ---------------------------------------------------------------------------

#[test]
fn test_numeric_version_5_036() -> Result<(), Box<dyn std::error::Error>> {
    // `use 5.036` is equivalent to `use v5.36`
    let ast = program(vec![use_node("5.036"), class_node("Foo")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'class' with `use 5.036` (= v5.36), got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 10: explicit `use feature 'signatures'` suppresses warning even on old version
// ---------------------------------------------------------------------------

#[test]
fn test_explicit_use_feature_suppresses_warning() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.18"), use_feature("signatures"), sub_with_signature()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'signatures'' is present, got: {:?}",
        diagnostics
    );
    Ok(())
}

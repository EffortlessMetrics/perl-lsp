//! Tests for Perl version compatibility warnings (PL900)
//!
//! These tests verify that the version_compat lint correctly detects uses of
//! Perl features that are not available in the declared Perl version.

use perl_lsp_diagnostics::{DiagnosticSeverity, version_compat::check_version_compat};
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
        NodeKind::Use {
            module: module.to_string(),
            args: vec![],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(0, 12),
    )
}

fn use_node_at(module: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Use {
            module: module.to_string(),
            args: vec![],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(start, end),
    )
}

fn use_feature(feature: &str) -> Node {
    Node::new(
        NodeKind::Use {
            module: "feature".to_string(),
            args: vec![format!("'{}'", feature)],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(0, 20),
    )
}

fn use_feature_arg(arg: &str) -> Node {
    Node::new(
        NodeKind::Use {
            module: "feature".to_string(),
            args: vec![arg.to_string()],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(0, 20),
    )
}

fn use_feature_arg_at(arg: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Use {
            module: "feature".to_string(),
            args: vec![arg.to_string()],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(start, end),
    )
}

fn no_feature(feature: &str) -> Node {
    Node::new(
        NodeKind::No {
            module: "feature".to_string(),
            args: vec![format!("'{}'", feature)],
            has_filter_risk: false,
        },
        loc(0, 20),
    )
}

fn no_feature_at(feature: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::No {
            module: "feature".to_string(),
            args: vec![format!("'{}'", feature)],
            has_filter_risk: false,
        },
        loc(start, end),
    )
}

fn no_feature_arg(arg: &str) -> Node {
    Node::new(
        NodeKind::No {
            module: "feature".to_string(),
            args: vec![arg.to_string()],
            has_filter_risk: false,
        },
        loc(0, 20),
    )
}

fn class_node(name: &str) -> Node {
    Node::new(
        NodeKind::Class { name: name.to_string(), parents: vec![], body: Box::new(block(vec![])) },
        loc(20, 50),
    )
}

fn num_node(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(20, 21))
}

fn try_node() -> Node {
    Node::new(
        NodeKind::Try { body: Box::new(block(vec![])), catch_blocks: vec![], finally_block: None },
        loc(20, 60),
    )
}

fn given_when_node() -> Node {
    let when = Node::new(
        NodeKind::When {
            condition: Box::new(Node::new(
                NodeKind::Number { value: "1".to_string() },
                loc(34, 35),
            )),
            body: Box::new(block(vec![say_call()])),
        },
        loc(28, 52),
    );

    Node::new(
        NodeKind::Given {
            expr: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
                loc(24, 30),
            )),
            body: Box::new(block(vec![when])),
        },
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

fn given_node() -> Node {
    Node::new(
        NodeKind::Given {
            expr: Box::new(num_node("1")),
            body: Box::new(block(vec![when_node(), default_node()])),
        },
        loc(20, 80),
    )
}

fn when_node() -> Node {
    Node::new(
        NodeKind::When { condition: Box::new(num_node("1")), body: Box::new(block(vec![])) },
        loc(30, 50),
    )
}

fn default_node() -> Node {
    Node::new(NodeKind::Default { body: Box::new(block(vec![])) }, loc(50, 70))
}

fn default_with_class_node() -> Node {
    Node::new(NodeKind::Default { body: Box::new(block(vec![class_node("Nested")])) }, loc(50, 90))
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
// Test 2b: class in v5.40 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_class_ok_on_v5_40() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40"), class_node("Foo")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'class' in v5.40, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2a: class in v5.38_001 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_class_ok_on_v5_38_dev_release() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38_001"), class_node("Foo")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'class' in v5.38_001, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: `use feature 'class'` overrides an old version declaration -> no warn
// ---------------------------------------------------------------------------
//
// This is the real suppression test: a version IS declared (v5.10) and would
// normally trigger a warning for `class`, but `use feature 'class'` explicitly
// enables it and should suppress the warning.
//
// A previous version of this test used only `use_feature("class")` with no
// version declaration, which made the checker return early before reaching the
// suppression logic — the test was vacuous (it tested the early-exit path, not
// the feature override).

#[test]
fn test_class_ok_with_use_feature_class() -> Result<(), Box<dyn std::error::Error>> {
    // version IS declared (v5.10 does NOT bundle class), but explicit `use feature 'class'`
    // should suppress the PL900 warning.
    let ast = program(vec![use_node("v5.10"), use_feature("class"), class_node("Bar")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'class'' is present on v5.10, got: {:?}",
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
// Test 5b: given/when warns on v5.38 (deprecated)
// ---------------------------------------------------------------------------

#[test]
fn test_given_when_warns_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), given_when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(
        diag.severity,
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "Expected warning severity for deprecated given/when on v5.38, got: {:?}",
        diag
    );
    assert!(
        diag.message.contains("given/when"),
        "Message should mention given/when: {}",
        diag.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5bb: default warns on v5.38 (deprecated)
// ---------------------------------------------------------------------------

#[test]
fn test_default_warns_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), default_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(
        diag.severity,
        perl_diagnostics::codes::DiagnosticSeverity::Warning,
        "Expected warning severity for deprecated default on v5.38, got: {:?}",
        diag
    );
    assert!(diag.message.contains("default"), "Message should mention default: {}", diag.message);
    assert!(
        diag.message.contains("deprecated"),
        "Message should mention deprecation: {}",
        diag.message
    );
    Ok(())
}

#[test]
fn test_when_warns_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    assert!(
        diag.message.contains("given/when/default"),
        "Message should mention given/when/default: {}",
        diag.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5c: given/when errors on v5.42 (removed)
// ---------------------------------------------------------------------------

#[test]
fn test_given_when_errors_on_v5_42() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.42"), given_when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(
        diag.severity,
        perl_diagnostics::codes::DiagnosticSeverity::Error,
        "Expected error severity for removed given/when on v5.42, got: {:?}",
        diag
    );
    assert!(
        diag.suggestion
            .as_deref()
            .is_some_and(|suggestion| suggestion.contains("if") && suggestion.contains("elsif")),
        "Expected migration suggestion for given/when removal, got: {:?}",
        diag.suggestion
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5cb: default errors on v5.42 (removed)
// ---------------------------------------------------------------------------

#[test]
fn test_default_errors_on_v5_42() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.42"), default_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(
        diag.severity,
        perl_diagnostics::codes::DiagnosticSeverity::Error,
        "Expected error severity for removed default on v5.42, got: {:?}",
        diag
    );
    assert!(diag.message.contains("default"), "Message should mention default: {}", diag.message);
    assert!(diag.message.contains("removed"), "Message should mention removal: {}", diag.message);
    assert!(
        diag.suggestion
            .as_deref()
            .is_some_and(|suggestion| suggestion.contains("if") && suggestion.contains("elsif")),
        "Expected migration suggestion for default removal, got: {:?}",
        diag.suggestion
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5b: given/when in v5.36 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_given_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'given/when' in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5c: given/when in v5.38 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_given_warns_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'given/when' in v5.38, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(msg.severity, DiagnosticSeverity::Warning);
    assert!(
        msg.message.contains("v5.38") || msg.message.contains("5.38"),
        "Message should mention minimum version v5.38: {}",
        msg.message
    );
    assert!(
        msg.message.contains("deprecated"),
        "Message should mention deprecation: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5d: given/when in v5.42 -> error
// ---------------------------------------------------------------------------

#[test]
fn test_given_errors_on_v5_42() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.42"), given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert_eq!(msg.severity, DiagnosticSeverity::Error);
    assert!(
        msg.message.contains("v5.42") || msg.message.contains("5.42"),
        "Message should mention removal version v5.42: {}",
        msg.message
    );
    assert!(msg.message.contains("removed"), "Message should mention removal: {}", msg.message);
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

#[test]
fn test_say_inside_given_when_is_reached_by_walker() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), given_when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(
        diagnostics
            .iter()
            .find(|d| d.code.as_deref() == Some("PL900") && d.message.contains("say")),
    );
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn test_class_inside_default_is_reached_by_walker() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), default_with_class_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let diag = must_some(
        diagnostics
            .iter()
            .find(|d| d.code.as_deref() == Some("PL900") && d.message.contains("class")),
    );
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
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

// ---------------------------------------------------------------------------
// Test 11: signatures in v5.20 -> warns (not bundled until v5.36)
// ---------------------------------------------------------------------------
//
// Regression guard for the FEATURE_VERSIONS / features_enabled_by_version
// alignment fix.  signatures became experimental in v5.20 but are not in the
// stable feature bundle until v5.36.  Without the fix (signatures min = 5.20
// but bundle threshold = 5.36), `use v5.20` + a signature sub would emit a
// false-positive "requires v5.20; declared v5.20" diagnostic.
// With the fix (min = 5.36) it correctly warns "requires v5.36".

#[test]
fn test_signatures_warns_on_v5_20() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.20"), sub_with_signature()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for signatures on v5.20 (stable bundle requires v5.36), got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    // The minimum version in the warning must be 5.36, not 5.20 (which would be nonsensical)
    assert!(
        msg.message.contains("v5.36") || msg.message.contains("5.36"),
        "Message should mention minimum version v5.36, not v5.20: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 12: signatures in v5.36 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_signatures_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), sub_with_signature()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for signatures in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 13: say nested inside a method call object is detected
// ---------------------------------------------------------------------------
//
// Regression guard for the walker MethodCall fix: before the fix, `object`
// was not traversed — a `say` call used as the object expression of a chain
// (`say("hi")->something`) would be silently skipped.  This test verifies
// the walker now enters the object sub-tree.

fn say_inside_method_call_object() -> Node {
    // Models: say("hi")->foo()  — contrived but exercises the object walk path
    Node::new(
        NodeKind::MethodCall {
            object: Box::new(say_call()),
            method: "foo".to_string(),
            args: vec![],
        },
        loc(20, 70),
    )
}

#[test]
fn test_say_nested_in_method_call_object_detected() -> Result<(), Box<dyn std::error::Error>> {
    // v5.8 file: say (inside method call object) must still be flagged
    let ast = program(vec![use_node("v5.8"), say_inside_method_call_object()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'say' nested in method call object on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 14: say in a ternary branch is detected
// ---------------------------------------------------------------------------
//
// Walker regression guard for Ternary: before the fix, Ternary had no arm
// in the walker and fell into `_ => {}`, silently dropping all three
// sub-expressions.

#[test]
fn test_say_in_ternary_branch_detected() -> Result<(), Box<dyn std::error::Error>> {
    // Models: $x ? say("a") : "b"  — say in ternary then-expr
    let ternary = Node::new(
        NodeKind::Ternary {
            condition: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(20, 22),
            )),
            then_expr: Box::new(say_call()),
            else_expr: Box::new(Node::new(
                NodeKind::String { value: "b".to_string(), interpolated: false },
                loc(40, 43),
            )),
        },
        loc(20, 44),
    );

    let ast = program(vec![use_node("v5.8"), ternary]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'say' inside ternary on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 15: say in a return expression is detected
// ---------------------------------------------------------------------------
//
// Walker regression guard for Return: before the fix, Return{value:Some(...)}
// fell into `_ => {}` so any version-gated construct in a return value
// was never visited.

#[test]
fn test_say_in_return_value_detected() -> Result<(), Box<dyn std::error::Error>> {
    // Models: `return say("hi");` — say in return value
    let ret = Node::new(NodeKind::Return { value: Some(Box::new(say_call())) }, loc(20, 40));
    let ast = program(vec![use_node("v5.8"), ret]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'say' inside return value on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests for given/when/default (#3344) and defer (#3350)
// ---------------------------------------------------------------------------

fn issue_3344_given_node() -> Node {
    Node::new(
        NodeKind::Given {
            expr: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(26, 28),
            )),
            body: Box::new(block(vec![])),
        },
        loc(20, 50),
    )
}

fn issue_3344_when_node() -> Node {
    Node::new(
        NodeKind::When {
            condition: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(26, 28),
            )),
            body: Box::new(block(vec![])),
        },
        loc(20, 50),
    )
}

fn issue_3344_default_node() -> Node {
    Node::new(NodeKind::Default { body: Box::new(block(vec![])) }, loc(20, 40))
}

fn smartmatch_node() -> Node {
    Node::new(
        NodeKind::Binary {
            op: "~~".to_string(),
            left: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
                loc(20, 26),
            )),
            right: Box::new(Node::new(
                NodeKind::String { value: "pattern".to_string(), interpolated: false },
                loc(30, 39),
            )),
        },
        loc(20, 39),
    )
}

fn defer_call() -> Node {
    // `defer { }` is now parsed as NodeKind::Defer (Perl 5.36+ experimental, stable in 5.40).
    Node::new(
        NodeKind::Defer {
            block: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(26, 40))),
        },
        loc(20, 41),
    )
}

fn defer_helper_call() -> Node {
    Node::new(
        NodeKind::FunctionCall {
            name: "defer".to_string(),
            args: vec![Node::new(
                NodeKind::String { value: "cleanup".to_string(), interpolated: false },
                loc(26, 35),
            )],
        },
        loc(20, 36),
    )
}

// ---------------------------------------------------------------------------
// Test 16: given in v5.8 -> warns (requires v5.10+)
// ---------------------------------------------------------------------------

#[test]
fn test_given_warns_on_v5_8() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), issue_3344_given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'given' in v5.8, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("given"), "Message should mention 'given': {}", msg.message);
    assert!(
        msg.message.contains("v5.10") || msg.message.contains("5.10"),
        "Message should mention minimum version v5.10: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 17: when in v5.8 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_when_warns_on_v5_8() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), issue_3344_when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'when' in v5.8, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("when"), "Message should mention 'when': {}", msg.message);
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 18: default in v5.8 -> warns
// ---------------------------------------------------------------------------

#[test]
fn test_default_warns_on_v5_8() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), issue_3344_default_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'default' in v5.8, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("default"), "Message should mention 'default': {}", msg.message);
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 19: given/when/default in v5.10 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_given_ok_on_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), issue_3344_given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'given' in v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_when_ok_on_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), issue_3344_when_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'when' in v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_default_ok_on_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), issue_3344_default_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'default' in v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 20: explicit `use feature 'switch'` suppresses given/when warning
// ---------------------------------------------------------------------------
//
// In Perl, given/when/default are enabled by `use feature 'switch'`.  An
// explicit `use feature 'switch'` on an old-version file must suppress
// the PL900 warning.

#[test]
fn test_given_ok_with_use_feature_switch() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), use_feature("switch"), issue_3344_given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'switch'' is present on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_given_ok_with_use_feature_qw_switch() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("v5.8"), use_feature_arg("qw(switch say)"), issue_3344_given_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature qw(switch say)' is present on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests for smartmatch (`~~`) (#3396)
// ---------------------------------------------------------------------------

#[test]
fn test_smartmatch_warns_on_v5_8() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for smartmatch in v5.8, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(
        msg.message.contains("smartmatch") || msg.message.contains("~~"),
        "Message should mention smartmatch: {}",
        msg.message
    );
    assert!(
        msg.message.contains("v5.10") || msg.message.contains("5.10"),
        "Message should mention minimum version v5.10: {}",
        msg.message
    );
    Ok(())
}

#[test]
fn test_smartmatch_ok_on_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for smartmatch in v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_warns_on_v5_38() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.38"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for smartmatch in v5.38, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(
        msg.message.contains("deprecated") || msg.message.contains("v5.38"),
        "Message should mention deprecation: {}",
        msg.message
    );
    Ok(())
}

#[test]
fn test_smartmatch_errors_on_v5_42() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.42"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 error for smartmatch in v5.42, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(
        msg.message.contains("removed") || msg.message.contains("v5.42"),
        "Message should mention removal: {}",
        msg.message
    );
    assert_eq!(msg.severity, DiagnosticSeverity::Error, "Expected removal severity to be an error");
    Ok(())
}

#[test]
fn test_smartmatch_ok_with_use_feature_switch() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), use_feature("switch"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature \\'switch\\'' is present on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_ok_with_use_feature_bundle_5_10() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.8"), use_feature_arg("':5.10'"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature \":5.10\"' is present on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_warns_after_no_feature_switch() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), no_feature("switch"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning when 'no feature \"switch\"' disables smartmatch on v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_warns_after_no_feature_switch_disables_bundle()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("v5.8"),
        use_feature_arg("':5.10'"),
        no_feature("switch"),
        smartmatch_node(),
    ]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning when 'no feature \"switch\"' disables the ':5.10' bundle on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_warns_after_no_feature_all() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10"), no_feature_arg("':all'"), smartmatch_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning when 'no feature \":all\"' clears the v5.10 bundle, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_given_warns_after_no_feature_switch_disables_bundle()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node_at("v5.8", 0, 8),
        use_feature_arg_at("':5.10'", 9, 18),
        no_feature_at("switch", 19, 29),
        issue_3344_given_node(),
    ]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning when 'no feature \"switch\"' disables the ':5.10' bundle for given/when/default on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

#[test]
fn test_smartmatch_warns_after_no_feature_switch_disables_qw_imports()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node_at("v5.8", 0, 8),
        use_feature_arg_at("qw(switch say)", 9, 18),
        no_feature_at("switch", 19, 29),
        smartmatch_node(),
    ]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning when 'no feature \"switch\"' disables grouped feature imports for smartmatch on v5.8, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 21: defer block in v5.34 -> warns (requires v5.36+)
// ---------------------------------------------------------------------------

#[test]
fn test_defer_warns_on_v5_34() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.34"), defer_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'defer' in v5.34, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("defer"), "Message should mention 'defer': {}", msg.message);
    assert!(
        msg.message.contains("v5.36") || msg.message.contains("5.36"),
        "Message should mention minimum version v5.36: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 22: defer in v5.36 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_defer_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), defer_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'defer' in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 23: explicit `use feature 'defer'` suppresses warning on old version
// ---------------------------------------------------------------------------

#[test]
fn test_defer_ok_with_use_feature_defer() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.34"), use_feature("defer"), defer_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'defer'' is present on v5.34, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 24: ordinary helper call named defer is not treated as the feature
// ---------------------------------------------------------------------------

#[test]
fn test_defer_helper_call_is_not_version_feature() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.34"), defer_helper_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for ordinary helper call named 'defer', got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers for builtin:: tests
// ---------------------------------------------------------------------------

fn builtin_floor_call() -> Node {
    builtin_call("builtin::floor")
}

fn builtin_call(name: &str) -> Node {
    Node::new(
        NodeKind::FunctionCall {
            name: name.to_string(),
            args: vec![Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
                loc(30, 32),
            )],
        },
        loc(20, 33),
    )
}

fn use_builtin_node() -> Node {
    use_builtin_import("'floor'")
}

fn use_builtin_import(import: &str) -> Node {
    Node::new(
        NodeKind::Use {
            module: "builtin".to_string(),
            args: vec![import.to_string()],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(0, 22),
    )
}

fn use_builtin_qw_import(imports: &str) -> Node {
    use_builtin_import(&format!("qw({})", imports))
}

fn isa_node() -> Node {
    Node::new(
        NodeKind::Binary {
            op: "isa".to_string(),
            left: Box::new(Node::new(
                NodeKind::Variable { sigil: "$".to_string(), name: "obj".to_string() },
                loc(24, 28),
            )),
            right: Box::new(Node::new(
                NodeKind::String { value: "MyClass".to_string(), interpolated: false },
                loc(29, 38),
            )),
        },
        loc(20, 38),
    )
}

// ---------------------------------------------------------------------------
// Test 25: builtin::floor in v5.36 -> ok (available since v5.36)
// ---------------------------------------------------------------------------

#[test]
fn test_builtin_qualified_call_floor_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), builtin_floor_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'builtin::floor' in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 26: builtin::floor in v5.40 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_builtin_qualified_call_ok_on_v5_40() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40"), builtin_floor_call()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'builtin::floor' in v5.40, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 27: `use builtin 'floor'` in v5.36 -> ok (available since v5.36)
// ---------------------------------------------------------------------------

#[test]
fn test_use_builtin_floor_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), use_builtin_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'use builtin \"floor\"' in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 28: builtin bundle import in v5.36 -> warns (requires v5.40+)
// ---------------------------------------------------------------------------

#[test]
fn test_use_builtin_bundle_warns_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36"), use_builtin_import("':5.40'")]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'use builtin \":5.40\"' in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 29: builtin import on old version suppresses duplicate qualified-call warning
// ---------------------------------------------------------------------------

#[test]
fn test_use_builtin_suppresses_qualified_call_warning() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("v5.38"),
        use_builtin_import("'load_module'"),
        builtin_call("builtin::load_module"),
    ]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    let pl900_count = diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL900")).count();
    assert!(
        pl900_count <= 1,
        "Expected at most one PL900 for 'use builtin load_module' + 'builtin::load_module' on v5.38, got {} warnings: {:?}",
        pl900_count,
        diagnostics
    );
    Ok(())
}

#[test]
fn test_builtin_qualified_calls_have_distinct_minimum_versions()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("builtin::floor", "v5.36", false),
        ("builtin::is_tainted", "v5.36", true),
        ("builtin::is_tainted", "v5.38", false),
        ("builtin::export_lexically", "v5.36", true),
        ("builtin::export_lexically", "v5.38", false),
        ("builtin::load_module", "v5.38", true),
        ("builtin::load_module", "v5.40", false),
    ];

    for (name, version, should_warn) in cases {
        let ast = program(vec![use_node(version), builtin_call(name)]);
        let mut diagnostics = vec![];
        check_version_compat(&ast, &mut diagnostics);

        assert_eq!(
            diagnostics_have_code(&diagnostics, "PL900"),
            should_warn,
            "Unexpected builtin:: compatibility result for {name} on {version}: {:?}",
            diagnostics
        );
    }

    Ok(())
}

#[test]
fn test_builtin_imports_have_distinct_minimum_versions() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("'floor'", "v5.36", false),
        ("'is_tainted'", "v5.36", true),
        ("'is_tainted'", "v5.38", false),
        ("'export_lexically'", "v5.36", true),
        ("'export_lexically'", "v5.38", false),
        ("'load_module'", "v5.38", true),
        ("'load_module'", "v5.40", false),
        ("':5.40'", "v5.38", true),
        ("':5.40'", "v5.40", false),
    ];

    for (import, version, should_warn) in cases {
        let ast = program(vec![use_node(version), use_builtin_import(import)]);
        let mut diagnostics = vec![];
        check_version_compat(&ast, &mut diagnostics);

        assert_eq!(
            diagnostics_have_code(&diagnostics, "PL900"),
            should_warn,
            "Unexpected builtin import compatibility result for {import} on {version}: {:?}",
            diagnostics
        );
    }

    Ok(())
}

#[test]
fn test_builtin_qw_imports_have_distinct_minimum_versions() -> Result<(), Box<dyn std::error::Error>>
{
    let cases = [
        ("floor", "v5.36", false),
        ("is_tainted", "v5.36", true),
        ("is_tainted", "v5.38", false),
        ("export_lexically", "v5.36", true),
        ("export_lexically", "v5.38", false),
        ("load_module", "v5.38", true),
        ("load_module", "v5.40", false),
        (":5.40", "v5.38", true),
        (":5.40", "v5.40", false),
    ];

    for (imports, version, should_warn) in cases {
        let ast = program(vec![use_node(version), use_builtin_qw_import(imports)]);
        let mut diagnostics = vec![];
        check_version_compat(&ast, &mut diagnostics);

        assert_eq!(
            diagnostics_have_code(&diagnostics, "PL900"),
            should_warn,
            "Unexpected builtin qw import compatibility result for {imports} on {version}: {:?}",
            diagnostics
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 30: `isa` operator in v5.30 -> warns (requires v5.36)
// ---------------------------------------------------------------------------

#[test]
fn test_isa_warns_on_v5_30() -> Result<(), Box<dyn std::error::Error>> {
    // `$obj isa 'MyClass'` with `use v5.30` should produce a PL900 warning
    let ast = program(vec![use_node("v5.30"), isa_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        diagnostics_have_code(&diagnostics, "PL900"),
        "Expected PL900 warning for 'isa' operator in v5.30, got: {:?}",
        diagnostics
    );
    let msg = must_some(diagnostics.iter().find(|d| d.code.as_deref() == Some("PL900")));
    assert!(msg.message.contains("isa"), "Message should mention 'isa': {}", msg.message);
    assert!(
        msg.message.contains("v5.36") || msg.message.contains("5.36"),
        "Message should mention minimum version v5.36: {}",
        msg.message
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 31: `isa` operator in v5.36 -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_isa_ok_on_v5_36() -> Result<(), Box<dyn std::error::Error>> {
    // `$obj isa 'MyClass'` with `use v5.36` should NOT produce a warning
    let ast = program(vec![use_node("v5.36"), isa_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning for 'isa' operator in v5.36, got: {:?}",
        diagnostics
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Test 32: `isa` operator with `use feature 'isa'` on old version -> no warn
// ---------------------------------------------------------------------------

#[test]
fn test_isa_ok_with_use_feature_isa() -> Result<(), Box<dyn std::error::Error>> {
    // Explicit `use feature 'isa'` on v5.10 should suppress PL900
    let ast = program(vec![use_node("v5.10"), use_feature("isa"), isa_node()]);
    let mut diagnostics = vec![];
    check_version_compat(&ast, &mut diagnostics);

    assert!(
        no_compat_warnings(&diagnostics),
        "Expected no PL900 warning when 'use feature 'isa'' is present on v5.10, got: {:?}",
        diagnostics
    );
    Ok(())
}

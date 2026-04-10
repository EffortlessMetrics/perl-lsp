//! Behavior-driven scenarios for `perl-pragma`.
//!
//! These tests validate end-user-visible pragma semantics in a
//! Given/When/Then style.

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::PragmaTracker;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn use_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Use {
            module: module.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

#[test]
fn scenario_lexical_scope_restores_outer_state() -> Result<(), Box<dyn std::error::Error>> {
    // Given: strict is enabled in outer scope
    // When: strict refs is disabled in an inner block
    // Then: refs is disabled inside, but restored outside.
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![no_node("strict", &["refs"], 16, 33)], 14, 36),
        use_node("warnings", &[], 40, 55),
    ]);

    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 20);
    assert!(!inside.strict_refs);
    assert!(inside.strict_vars);

    let after = PragmaTracker::state_for_offset(&map, 45);
    assert!(after.strict_refs);
    assert!(after.warnings);

    Ok(())
}

#[test]
fn scenario_warning_category_suppression_is_granular() -> Result<(), Box<dyn std::error::Error>> {
    // Given: warnings are globally enabled
    // When: one warning category is disabled with `no warnings 'uninitialized'`
    // Then: only that category is inactive and others still fire.
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["'uninitialized'"], 16, 45),
    ]);

    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 30);

    assert!(state.warnings);
    assert!(!state.is_warning_active("uninitialized"));
    assert!(state.is_warning_active("deprecated"));

    Ok(())
}

#[test]
fn scenario_version_pragma_enables_modern_defaults() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a lexical `use v5.40` declaration
    // When: pragma state is queried after that declaration
    // Then: strict + warnings are enabled and version features are present.
    let ast = program(vec![use_node("v5.40", &[], 0, 12)]);

    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 5);

    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    assert!(state.warnings);
    assert!(state.has_feature("signatures"));
    assert!(state.has_feature("builtin"));

    Ok(())
}

#[test]
fn scenario_builtin_imports_stay_lexical() -> Result<(), Box<dyn std::error::Error>> {
    // Given: lexical builtin imports in a nested block
    // When: state is queried inside and outside that block
    // Then: imports are available only in the lexical scope where imported.
    let ast = program(vec![
        block(vec![use_node("builtin", &["qw(true floor)"], 20, 45)], 18, 48),
        use_node("warnings", &[], 52, 67),
    ]);

    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 30);
    assert!(inside.has_builtin_import("true"));
    assert!(inside.has_builtin_import("floor"));

    let outside = PragmaTracker::state_for_offset(&map, 60);
    assert!(!outside.has_builtin_import("true"));
    assert!(!outside.has_builtin_import("floor"));

    Ok(())
}

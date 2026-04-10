//! BDD-style behavior specification tests for `perl-pragma`.
//!
//! These scenarios lock user-observable pragma behavior from the perspective of
//! a caller that builds a pragma map and queries effective state by byte offset.

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
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn given_default_program_when_querying_before_any_pragma_then_state_is_disabled() -> TestResult {
    let ast = program(vec![]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 0);

    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    assert!(!state.warnings);
    Ok(())
}

#[test]
fn given_use_strict_then_no_strict_refs_when_querying_after_second_statement_then_only_refs_are_disabled()
-> TestResult {
    let ast = program(vec![use_node("strict", &[], 0, 11), no_node("strict", &["refs"], 12, 29)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 29);

    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

#[test]
fn given_warnings_with_category_disabled_when_querying_category_activity_then_only_that_category_is_inactive()
-> TestResult {
    let ast = program(vec![
        use_node("warnings", &[], 0, 12),
        no_node("warnings", &["experimental::smartmatch"], 13, 52),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 52);

    assert!(state.warnings, "global warnings should stay on for category-only disable");
    assert!(state.is_warning_active("deprecated"));
    assert!(!state.is_warning_active("experimental::smartmatch"));
    Ok(())
}

#[test]
fn given_warnings_then_no_warnings_without_categories_when_querying_after_disable_then_all_warning_categories_are_inactive()
-> TestResult {
    let ast = program(vec![use_node("warnings", &[], 0, 12), no_node("warnings", &[], 13, 24)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 24);

    assert!(!state.warnings);
    assert!(!state.is_warning_active("deprecated"));
    assert!(!state.is_warning_active("uninitialized"));
    Ok(())
}

#[test]
fn given_version_bundle_then_feature_switch_removed_in_v538_when_querying_features_then_bundle_reflects_removal()
-> TestResult {
    let ast = program(vec![use_node("v5.38", &[], 0, 6)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 6);

    assert!(state.has_feature("class"));
    assert!(state.has_feature("method"));
    assert!(!state.has_feature("switch"));
    Ok(())
}

#[test]
fn given_pragma_inside_block_when_querying_after_block_then_outer_state_is_restored() -> TestResult
{
    let ast = program(vec![
        use_node("strict", &["vars"], 0, 17),
        block(vec![use_node("strict", &["refs"], 20, 37)], 18, 39),
    ]);
    let map = PragmaTracker::build(&ast);

    let in_block = PragmaTracker::state_for_offset(&map, 30);
    let after_block = PragmaTracker::state_for_offset(&map, 39);

    assert!(in_block.strict_vars);
    assert!(in_block.strict_refs);
    assert!(after_block.strict_vars);
    assert!(!after_block.strict_refs, "strict refs should revert at block exit");
    Ok(())
}

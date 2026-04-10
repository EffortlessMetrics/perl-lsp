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

fn block_node(statements: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements }, location: loc(start, end) }
}

fn program(statements: Vec<Node>) -> Node {
    let end = statements.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements }, location: loc(0, end) }
}

#[test]
fn given_strict_scope_changes_when_querying_inside_and_after_block_then_state_is_lexically_restored()
 {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block_node(vec![no_node("strict", &["refs"], 15, 30)], 13, 33),
        use_node("warnings", &[], 34, 49),
    ]);

    let pragma_map = PragmaTracker::build(&ast);

    let inside_state = PragmaTracker::state_for_offset(&pragma_map, 20);
    assert!(!inside_state.strict_refs);
    assert!(inside_state.strict_vars);

    let after_state = PragmaTracker::state_for_offset(&pragma_map, 40);
    assert!(after_state.strict_vars);
    assert!(after_state.strict_subs);
    assert!(after_state.strict_refs);
    assert!(after_state.warnings);
}

#[test]
fn given_warning_categories_disabled_when_checking_activity_then_only_those_categories_are_inactive()
 {
    let ast = program(vec![
        use_node("warnings", &[], 0, 14),
        no_node("warnings", &["uninitialized", "deprecated"], 15, 52),
    ]);

    let pragma_map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&pragma_map, 30);

    assert!(state.warnings, "global warnings should remain enabled");
    assert!(!state.is_warning_active("uninitialized"));
    assert!(!state.is_warning_active("deprecated"));
    assert!(state.is_warning_active("syntax"));
}

#[test]
fn given_no_warnings_without_categories_when_checking_activity_then_all_warning_categories_are_off()
{
    let ast = program(vec![use_node("warnings", &[], 0, 14), no_node("warnings", &[], 15, 27)]);

    let pragma_map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&pragma_map, 20);

    assert!(!state.warnings);
    assert!(!state.is_warning_active("syntax"));
    assert!(!state.is_warning_active("uninitialized"));
}

#[test]
fn given_use_version_5_40_when_building_state_then_strict_warnings_and_bundle_features_are_enabled()
{
    let ast = program(vec![use_node("v5.40", &[], 0, 10)]);

    let pragma_map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&pragma_map, 5);

    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.warnings);
    assert!(state.has_feature("builtin"));
    assert!(state.has_feature("signatures"));
}

#[test]
fn given_use_builtin_qw_imports_when_querying_state_then_all_requested_imports_are_available() {
    let ast = program(vec![use_node("builtin", &["qw(blessed true false)"], 0, 33)]);

    let pragma_map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&pragma_map, 10);

    assert!(state.has_builtin_import("blessed"));
    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("false"));
    assert!(!state.has_builtin_import("weaken"));
}

#[test]
fn given_use_feature_signatures_and_unicode_strings_when_building_state_then_semantic_flags_are_set()
 {
    let ast = program(vec![use_node("feature", &["'qw(signatures unicode_strings)'"], 0, 41)]);

    let pragma_map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&pragma_map, 20);

    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.unicode_strings);
}

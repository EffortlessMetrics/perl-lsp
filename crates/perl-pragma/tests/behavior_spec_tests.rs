//! BDD-style behavior specification tests for perl-pragma.
//!
//! These scenarios describe pragma behavior from a consumer point of view:
//! "Given <context>, when <construct appears>, then <effective state>."

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{PragmaState, PragmaTracker};

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

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn package_block(name: &str, body: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Package {
            name: name.to_string(),
            name_span: loc(start, end),
            block: Some(Box::new(body)),
        },
        location: loc(start, end),
    }
}

fn phase_block(phase: &str, body: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::PhaseBlock {
            phase: phase.to_string(),
            phase_span: Some(loc(start, start + phase.len())),
            block: Box::new(body),
        },
        location: loc(start, end),
    }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

#[test]
fn given_fresh_file_when_no_pragmas_then_default_state_applies() {
    let ast = program(vec![]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 0);
    assert_eq!(state, PragmaState::default());
}

#[test]
fn given_use_strict_when_querying_after_statement_then_all_strict_modes_are_enabled() {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 8);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn given_use_strict_when_no_strict_refs_in_inner_block_then_refs_is_restored_outside_block() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![no_node("strict", &["refs"], 15, 31)], 13, 33),
        use_node("warnings", &[], 34, 49),
    ]);
    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 25);
    assert!(!inside.strict_refs);

    let outside = PragmaTracker::state_for_offset(&map, 40);
    assert!(outside.strict_vars);
    assert!(outside.strict_subs);
    assert!(outside.strict_refs);
    assert!(outside.warnings);
}

#[test]
fn given_use_warnings_when_specific_category_is_disabled_then_other_categories_stay_active() {
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized"], 16, 41),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 30);
    assert!(state.warnings);
    assert!(!state.is_warning_active("uninitialized"));
    assert!(state.is_warning_active("deprecated"));
}

#[test]
fn given_use_v5_40_when_querying_state_then_effective_strict_warnings_and_feature_bundle_apply() {
    let ast = program(vec![use_node("v5.40", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 8);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.warnings);
    assert!(state.has_feature("builtin"));
}

#[test]
fn given_use_builtin_qw_when_querying_scope_then_each_imported_name_is_available() {
    let ast = program(vec![use_node("builtin", &["qw(true false ceil)"], 0, 30)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 12);
    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("false"));
    assert!(state.has_builtin_import("ceil"));
}

#[test]
fn given_no_builtin_when_querying_scope_then_selected_imports_are_removed() {
    let ast = program(vec![
        use_node("builtin", &["qw(true false ceil)"], 0, 30),
        no_node("builtin", &["'false'"], 31, 49),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 40);
    assert!(state.has_builtin_import("true"));
    assert!(!state.has_builtin_import("false"));
    assert!(state.has_builtin_import("ceil"));
}

#[test]
fn given_use_feature_qw_when_querying_state_then_requested_features_and_unicode_strings_are_enabled()
 {
    let ast = program(vec![use_node("feature", &["'qw(signatures unicode_strings)'"], 0, 41)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.unicode_strings);
}

#[test]
fn given_use_if_feature_bundle_when_querying_state_then_bundle_features_are_recorded() {
    let ast = program(vec![use_node("if", &["$]", ">=", "5.036", "feature", "':5.36'"], 0, 44)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(state.has_feature("say"));
    assert!(state.has_feature("signatures"));
    assert!(state.has_feature("isa"));
}

#[test]
fn given_no_feature_all_then_use_feature_bundle_when_querying_state_then_bundle_is_reenabled() {
    let ast = program(vec![
        use_node("v5.40", &[], 0, 12),
        no_node("feature", &["':all'"], 13, 31),
        use_node("feature", &["':5.40'"], 32, 52),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 40);
    assert!(state.has_feature("builtin"));
    assert!(state.has_feature("say"));
}

#[test]
fn given_use_feature_signatures_when_querying_state_then_effective_strict_modes_are_enabled() {
    let ast = program(vec![use_node("feature", &["'signatures'"], 0, 24)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 12);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn given_use_v5_38_when_querying_state_then_switch_feature_is_not_available_but_modern_features_are()
 {
    let ast = program(vec![use_node("v5.38", &[], 0, 10)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 5);
    assert!(state.has_feature("class"));
    assert!(state.has_feature("method"));
    assert!(!state.has_feature("switch"));
}

#[test]
fn given_package_block_with_inner_no_strict_when_execution_continues_then_outer_state_is_restored()
{
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_block("Foo", block(vec![no_node("strict", &["subs"], 20, 36)], 18, 40), 13, 42),
        use_node("warnings", &[], 43, 58),
    ]);
    let map = PragmaTracker::build(&ast);

    let inside_package = PragmaTracker::state_for_offset(&map, 30);
    assert!(!inside_package.strict_subs);

    let after_package = PragmaTracker::state_for_offset(&map, 50);
    assert!(after_package.strict_subs);
    assert!(after_package.warnings);
}

#[test]
fn given_begin_block_with_use_strict_when_querying_inside_block_then_strict_is_active() {
    let ast = program(vec![phase_block(
        "BEGIN",
        block(vec![use_node("strict", &[], 8, 20)], 6, 22),
        0,
        22,
    )]);
    let map = PragmaTracker::build(&ast);

    let inside_begin = PragmaTracker::state_for_offset(&map, 12);
    assert!(inside_begin.strict_vars);
    assert!(inside_begin.strict_subs);
    assert!(inside_begin.strict_refs);
}

#[test]
fn given_begin_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    let ast = program(vec![phase_block(
        "BEGIN",
        block(vec![use_node("strict", &[], 8, 20)], 6, 22),
        0,
        22,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_begin = PragmaTracker::state_for_offset(&map, 24);
    assert!(!after_begin.strict_vars);
    assert!(!after_begin.strict_subs);
    assert!(!after_begin.strict_refs);
}

#[test]
fn given_begin_block_with_use_warnings_when_querying_after_block_then_warnings_is_not_active() {
    let ast = program(vec![phase_block(
        "BEGIN",
        block(vec![use_node("warnings", &[], 8, 22)], 6, 24),
        0,
        24,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_begin = PragmaTracker::state_for_offset(&map, 26);
    assert!(!after_begin.warnings);
}

#[test]
fn given_end_block_with_use_warnings_when_querying_after_block_then_warnings_is_not_active() {
    let ast = program(vec![phase_block(
        "END",
        block(vec![use_node("warnings", &[], 6, 20)], 4, 22),
        0,
        22,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_end = PragmaTracker::state_for_offset(&map, 24);
    assert!(!after_end.warnings);
}

#[test]
fn given_init_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    let ast = program(vec![phase_block(
        "INIT",
        block(vec![use_node("strict", &[], 7, 19)], 5, 21),
        0,
        21,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_init = PragmaTracker::state_for_offset(&map, 23);
    assert!(!after_init.strict_vars);
    assert!(!after_init.strict_subs);
    assert!(!after_init.strict_refs);
}

#[test]
fn given_check_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    let ast = program(vec![phase_block(
        "CHECK",
        block(vec![use_node("strict", &[], 8, 20)], 6, 22),
        0,
        22,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_check = PragmaTracker::state_for_offset(&map, 24);
    assert!(!after_check.strict_vars);
    assert!(!after_check.strict_subs);
    assert!(!after_check.strict_refs);
}

#[test]
fn given_unitcheck_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    let ast = program(vec![phase_block(
        "UNITCHECK",
        block(vec![use_node("strict", &[], 12, 24)], 10, 26),
        0,
        26,
    )]);
    let map = PragmaTracker::build(&ast);

    let after_unitcheck = PragmaTracker::state_for_offset(&map, 28);
    assert!(!after_unitcheck.strict_vars);
    assert!(!after_unitcheck.strict_subs);
    assert!(!after_unitcheck.strict_refs);
}

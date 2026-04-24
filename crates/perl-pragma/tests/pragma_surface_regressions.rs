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

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn eval_node(body_node: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Eval { block: Box::new(body_node) }, location: loc(start, end) }
}

fn string_node(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::String { value: value.to_string(), interpolated },
        location: loc(start, end),
    }
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
fn use_strict_qw_vars_refs_enables_only_requested_flags() {
    let ast = program(vec![use_node("strict", &["qw(vars refs)"], 0, 24)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 12);
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn no_strict_qw_vars_subs_preserves_refs() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        no_node("strict", &["qw(vars subs)"], 13, 36),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 30);
    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn use_feature_all_populates_latest_bundle_surface() {
    let ast = program(vec![use_node("feature", &["':all'"], 0, 19)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(state.has_feature("say"));
    assert!(state.has_feature("signatures"));
    assert!(state.has_feature("builtin"));
}

#[test]
fn no_feature_all_clears_version_bundle_features() {
    let ast = program(vec![use_node("v5.40", &[], 0, 10), no_node("feature", &["':all'"], 11, 30)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(!state.has_feature("say"));
    assert!(!state.has_feature("signatures"));
    assert!(!state.has_feature("builtin"));
}

#[test]
fn use_builtin_qw_true_floor_tracks_lexical_imports() {
    let ast = program(vec![use_node("builtin", &["qw(true floor)"], 0, 27)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
}

#[test]
fn no_builtin_qw_floor_removes_selected_import_only() {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 27),
        no_node("builtin", &["qw(floor)"], 28, 47),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 40);
    assert!(state.has_builtin_import("true"));
    assert!(!state.has_builtin_import("floor"));
}

#[test]
fn no_if_builtin_qw_true_removes_target_import() {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 27),
        no_node("if", &["$cond", "builtin", "qw(true)"], 28, 58),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 50);
    assert!(!state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
}

#[test]
fn no_unless_feature_bundle_disables_bundle_entries() {
    let ast = program(vec![
        use_node("feature", &["':5.36'"], 0, 20),
        no_node("unless", &["$cond", "feature", "':5.36'"], 21, 55),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 45);
    assert!(!state.has_feature("say"));
    assert!(!state.has_feature("isa"));
}

#[test]
fn no_if_locale_scope_clears_locale_state() {
    let ast = program(vec![
        use_node("locale", &["':not_characters'"], 0, 28),
        no_node("if", &["$cond", "locale", "':not_characters'"], 29, 68),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 55);
    assert!(!state.locale);
    assert!(state.locale_scope.is_none());
}

#[test]
fn explicit_feature_toggle_wins_after_version_bundle() {
    let ast = program(vec![
        use_node("v5.40", &[], 0, 10),
        no_node("feature", &["'builtin'"], 11, 31),
        use_node("feature", &["'builtin'"], 32, 53),
    ]);
    let map = PragmaTracker::build(&ast);

    let disabled = PragmaTracker::state_for_offset(&map, 20);
    assert!(!disabled.has_feature("builtin"));

    let reenabled = PragmaTracker::state_for_offset(&map, 45);
    assert!(reenabled.has_feature("builtin"));
}

#[test]
fn nested_eval_block_changes_restore_to_outer_scope() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        eval_node(
            block(
                vec![
                    no_node("strict", &["refs"], 20, 36),
                    eval_node(block(vec![no_node("strict", &["vars"], 45, 61)], 43, 63), 41, 65),
                ],
                18,
                67,
            ),
            16,
            69,
        ),
        use_node("warnings", &[], 70, 85),
    ]);
    let map = PragmaTracker::build(&ast);

    let inner = PragmaTracker::state_for_offset(&map, 50);
    assert!(!inner.strict_refs);
    assert!(!inner.strict_vars);

    let after = PragmaTracker::state_for_offset(&map, 75);
    assert!(after.strict_vars);
    assert!(after.strict_refs);
    assert!(after.warnings);
}

#[test]
fn eval_string_does_not_leak_pragma_changes_to_following_scope() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        eval_node(string_node("no strict 'refs'; use warnings;", false, 14, 44), 13, 45),
        block(vec![], 46, 48),
    ]);
    let map = PragmaTracker::build(&ast);

    let state_after = PragmaTracker::state_for_offset(&map, 47);
    assert!(state_after.strict_refs);
    assert!(!state_after.warnings);
}

#[test]
fn package_and_phase_blocks_restore_lexical_state_after_exit() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_block("P", block(vec![no_node("strict", &["subs"], 20, 36)], 18, 38), 13, 40),
        phase_block("BEGIN", block(vec![no_node("strict", &["vars"], 50, 66)], 48, 68), 41, 69),
        use_node("warnings", &[], 70, 85),
    ]);
    let map = PragmaTracker::build(&ast);

    let in_package = PragmaTracker::state_for_offset(&map, 30);
    assert!(!in_package.strict_subs);

    let in_begin = PragmaTracker::state_for_offset(&map, 58);
    assert!(!in_begin.strict_vars);

    let after = PragmaTracker::state_for_offset(&map, 80);
    assert!(after.strict_vars);
    assert!(after.strict_subs);
    assert!(after.strict_refs);
    assert!(after.warnings);
}

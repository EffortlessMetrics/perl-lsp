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

fn eval_node(body: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Eval { block: Box::new(body) }, location: loc(start, end) }
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

fn string_node(value: &str, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::String { value: value.to_string(), interpolated: false },
        location: loc(start, end),
    }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

#[test]
fn strict_qw_vars_refs_only_enables_those_categories() {
    let ast = program(vec![use_node("strict", &["qw(vars refs)"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);

    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn no_strict_qw_vars_subs_leaves_refs_enabled() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        no_node("strict", &["qw(vars subs)"], 13, 35),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 25);

    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
}

#[test]
fn use_feature_all_enables_broad_feature_surface() {
    let ast = program(vec![use_node("feature", &["':all'"], 0, 20)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);

    assert!(state.has_feature("say"));
    assert!(state.has_feature("signatures"));
    assert!(state.has_feature("builtin"));
}

#[test]
fn no_feature_all_clears_features_after_use_feature_all() {
    let ast = program(vec![
        use_node("feature", &["':all'"], 0, 20),
        no_node("feature", &["':all'"], 21, 40),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 30);

    assert!(!state.has_feature("say"));
    assert!(!state.has_feature("signatures"));
    assert!(!state.has_feature("builtin"));
}

#[test]
fn use_builtin_qw_true_floor_tracks_lexical_imports() {
    let ast = program(vec![use_node("builtin", &["qw(true floor)"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 12);

    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
}

#[test]
fn no_builtin_true_removes_only_named_lexical_import() {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 28),
        no_node("builtin", &["'true'"], 29, 47),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 38);

    assert!(!state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
}

#[test]
fn no_if_builtin_conditionally_removes_builtin_import() {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 28),
        no_node("if", &["$cond", "builtin", "'floor'"], 29, 61),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 45);

    assert!(state.has_builtin_import("true"));
    assert!(!state.has_builtin_import("floor"));
}

#[test]
fn no_unless_locale_clears_locale_state() {
    let ast = program(vec![
        use_node("locale", &["':not_characters'"], 0, 28),
        no_node("unless", &["$cond", "locale"], 29, 56),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 42);

    assert!(!state.locale);
    assert!(state.locale_scope.is_none());
}

#[test]
fn no_if_feature_bundle_clears_version_bundle_features() {
    let ast = program(vec![
        use_node("v5.40", &[], 0, 10),
        no_node("if", &["$cond", "feature", "':5.36'"], 11, 43),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 30);

    assert!(state.has_feature("builtin"));
    assert!(!state.has_feature("signatures"));
    assert!(!state.has_feature("defer"));
}

#[test]
fn version_bundle_and_explicit_feature_can_be_removed_and_readded() {
    let ast = program(vec![
        use_node("v5.40", &[], 0, 10),
        no_node("feature", &["'builtin'"], 11, 32),
        use_node("feature", &["'builtin'"], 33, 54),
    ]);
    let map = PragmaTracker::build(&ast);

    let after_no = PragmaTracker::state_for_offset(&map, 20);
    assert!(!after_no.has_feature("builtin"));

    let after_readd = PragmaTracker::state_for_offset(&map, 45);
    assert!(after_readd.has_feature("builtin"));
}

#[test]
fn nested_eval_block_changes_are_lexically_restored() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        eval_node(
            block(
                vec![
                    no_node("strict", &["refs"], 20, 36),
                    eval_node(block(vec![no_node("strict", &["vars"], 42, 58)], 40, 60), 38, 61),
                ],
                18,
                62,
            ),
            16,
            63,
        ),
        use_node("warnings", &[], 64, 79),
    ]);
    let map = PragmaTracker::build(&ast);

    let inner = PragmaTracker::state_for_offset(&map, 50);
    assert!(!inner.strict_vars);
    assert!(!inner.strict_refs);

    let after = PragmaTracker::state_for_offset(&map, 70);
    assert!(after.strict_vars);
    assert!(after.strict_refs);
    assert!(after.warnings);
}

#[test]
fn eval_string_content_does_not_leak_pragmas_to_outer_scope() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        eval_node(string_node("no strict 'refs'; use warnings;", 20, 50), 18, 52),
        block(vec![], 53, 55),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 54);

    assert!(state.strict_vars);
    assert!(state.strict_refs);
    assert!(!state.warnings);
}

#[test]
fn package_and_phase_blocks_restore_lexical_pragma_state() {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_block("Foo", block(vec![no_node("strict", &["subs"], 20, 36)], 18, 38), 13, 40),
        phase_block("BEGIN", block(vec![no_node("strict", &["refs"], 48, 64)], 46, 66), 41, 67),
        use_node("warnings", &[], 68, 83),
    ]);
    let map = PragmaTracker::build(&ast);

    let in_package = PragmaTracker::state_for_offset(&map, 30);
    assert!(!in_package.strict_subs);

    let in_phase = PragmaTracker::state_for_offset(&map, 55);
    assert!(!in_phase.strict_refs);

    let after = PragmaTracker::state_for_offset(&map, 75);
    assert!(after.strict_subs);
    assert!(after.strict_refs);
    assert!(after.warnings);
}

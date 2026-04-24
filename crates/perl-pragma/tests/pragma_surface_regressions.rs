//! Focused regressions for widened pragma surface and asymmetry edge-cases.

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
            args: args.iter().map(|arg| arg.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
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

fn function_call(name: &str, args: Vec<Node>, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::FunctionCall { name: name.to_string(), args },
        location: loc(start, end),
    }
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
            name_span: loc(start, start + name.len()),
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
    let end = stmts.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

#[test]
fn use_strict_qw_vars_refs_enables_selected_categories() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["qw(vars refs)"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);

    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn no_strict_qw_vars_subs_disables_selected_categories() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        no_node("strict", &["qw(vars subs)"], 13, 35),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 24);

    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn use_feature_all_enables_known_surface_features() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["':all'"], 0, 20)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);

    assert!(state.has_feature("say"));
    assert!(state.has_feature("signatures"));
    assert!(state.has_feature("class"));
    assert!(state.has_feature("builtin"));
    Ok(())
}

#[test]
fn no_feature_all_clears_priorly_enabled_feature_set() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("feature", &["':all'"], 0, 20),
        no_node("feature", &["':all'"], 21, 40),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 32);

    assert!(state.features.is_empty());
    assert!(!state.unicode_strings);
    Ok(())
}

#[test]
fn use_builtin_qw_true_floor_tracks_imported_names() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("builtin", &["qw(true floor)"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 14);

    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
    Ok(())
}

#[test]
fn no_builtin_removes_requested_lexical_imports() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 28),
        no_node("builtin", &["qw(true)"], 29, 48),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 40);

    assert!(!state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
    Ok(())
}

#[test]
fn conditional_no_if_feature_all_clears_features() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("feature", &["':all'"], 0, 20),
        no_node("if", &["$cond", "feature", "':all'"], 21, 55),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 40);

    assert!(state.features.is_empty());
    Ok(())
}

#[test]
fn conditional_no_unless_locale_restores_locale_flags() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("locale", &["':not_characters'"], 0, 31),
        no_node("unless", &["$cond", "locale", "':not_characters'"], 32, 78),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 60);

    assert!(!state.locale);
    assert!(state.locale_scope.is_none());
    Ok(())
}

#[test]
fn conditional_no_if_builtin_removes_requested_imports() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("builtin", &["qw(true floor)"], 0, 28),
        no_node("if", &["$cond", "builtin", "qw(true)"], 29, 60),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 45);

    assert!(!state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
    Ok(())
}

#[test]
fn version_bundle_then_explicit_no_feature_updates_feature_set_only()
-> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("v5.40", &[], 0, 10), no_node("feature", &["'builtin'"], 11, 32)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 20);

    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    assert!(state.warnings);
    assert!(!state.has_feature("builtin"));
    Ok(())
}

#[test]
fn nested_eval_block_state_is_restored_after_exit() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        eval_node(
            block(
                vec![
                    no_node("strict", &["refs"], 20, 36),
                    block(vec![no_node("strict", &["vars"], 40, 56)], 38, 60),
                ],
                18,
                62,
            ),
            15,
            64,
        ),
        use_node("warnings", &[], 65, 80),
    ]);
    let map = PragmaTracker::build(&ast);

    let inner_eval = PragmaTracker::state_for_offset(&map, 45);
    assert!(!inner_eval.strict_vars);
    assert!(inner_eval.strict_subs);
    assert!(!inner_eval.strict_refs);

    let after_eval = PragmaTracker::state_for_offset(&map, 70);
    assert!(after_eval.strict_vars);
    assert!(after_eval.strict_subs);
    assert!(after_eval.strict_refs);
    assert!(after_eval.warnings);
    Ok(())
}

#[test]
fn eval_string_call_does_not_leak_compile_time_pragma_state()
-> Result<(), Box<dyn std::error::Error>> {
    let eval_string = function_call(
        "eval",
        vec![string_node("no strict 'refs'; use warnings;", true, 20, 55)],
        15,
        56,
    );
    let ast = program(vec![use_node("strict", &[], 0, 12), eval_string]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 58);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(!state.warnings);
    Ok(())
}

#[test]
fn package_and_phase_blocks_restore_outer_lexical_state() -> Result<(), Box<dyn std::error::Error>>
{
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_block("Foo", block(vec![no_node("strict", &["subs"], 20, 36)], 18, 38), 13, 40),
        phase_block("BEGIN", block(vec![use_node("warnings", &[], 48, 63)], 46, 65), 41, 66),
        use_node("utf8", &[], 67, 76),
    ]);
    let map = PragmaTracker::build(&ast);

    let in_package = PragmaTracker::state_for_offset(&map, 30);
    assert!(!in_package.strict_subs);

    let in_begin = PragmaTracker::state_for_offset(&map, 55);
    assert!(in_begin.warnings);

    let after_phase = PragmaTracker::state_for_offset(&map, 70);
    assert!(after_phase.strict_vars);
    assert!(after_phase.strict_subs);
    assert!(after_phase.strict_refs);
    assert!(!after_phase.warnings);
    assert!(after_phase.utf8);
    Ok(())
}

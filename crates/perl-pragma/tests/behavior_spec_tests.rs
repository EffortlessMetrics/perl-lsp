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
            phase_span: None,
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

// ── Phase block pragma scoping (issue #4100) ──────────────────────────────────
//
// Perl phase blocks (BEGIN/END/INIT/CHECK/UNITCHECK) share *normal lexical
// pragma scope*.  A pragma declared inside a phase block body applies only to
// that body — it does NOT propagate to the surrounding file scope.  This is
// verified against Perl 5.38.2:
//
//   $ perl -e 'BEGIN { use strict; } $x = 1; print "ok: strict not active\n"'
//   ok: strict not active
//
// The following 6 tests encode this invariant so any regression is caught.

#[test]
fn given_begin_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    // `BEGIN { use strict; }` — strict declared inside the BEGIN block body must NOT
    // propagate to file scope.  PragmaTracker treats PhaseBlock as opaque (no save/restore
    // needed because it never recurses into the body), so the effective state after the
    // phase block is unchanged from before it — the default (no strict).
    let body = block(vec![use_node("strict", &[], 8, 20)], 6, 22);
    let ast = program(vec![
        phase_block("BEGIN", body, 0, 22),
        // A statement after the block so we have an offset to query against.
        use_node("warnings", &[], 23, 36),
    ]);
    let map = PragmaTracker::build(&ast);

    // After the block, strict must NOT be active — pragma inside phase block doesn't leak.
    let after = PragmaTracker::state_for_offset(&map, 30);
    assert!(!after.strict_vars, "strict_vars must NOT propagate out of BEGIN block");
    assert!(!after.strict_subs, "strict_subs must NOT propagate out of BEGIN block");
    assert!(!after.strict_refs, "strict_refs must NOT propagate out of BEGIN block");
}

#[test]
fn given_end_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    // Same invariant for `END { use strict; }`.
    let body = block(vec![use_node("strict", &[], 6, 18)], 4, 20);
    let ast = program(vec![phase_block("END", body, 0, 20), use_node("warnings", &[], 21, 34)]);
    let map = PragmaTracker::build(&ast);

    let after = PragmaTracker::state_for_offset(&map, 28);
    assert!(!after.strict_vars, "strict_vars must NOT propagate out of END block");
    assert!(!after.strict_subs, "strict_subs must NOT propagate out of END block");
    assert!(!after.strict_refs, "strict_refs must NOT propagate out of END block");
}

#[test]
fn given_init_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    // Same invariant for `INIT { use strict; }`.
    let body = block(vec![use_node("strict", &[], 7, 19)], 5, 21);
    let ast = program(vec![phase_block("INIT", body, 0, 21), use_node("warnings", &[], 22, 35)]);
    let map = PragmaTracker::build(&ast);

    let after = PragmaTracker::state_for_offset(&map, 29);
    assert!(!after.strict_vars, "strict_vars must NOT propagate out of INIT block");
    assert!(!after.strict_subs, "strict_subs must NOT propagate out of INIT block");
    assert!(!after.strict_refs, "strict_refs must NOT propagate out of INIT block");
}

#[test]
fn given_check_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    // Same invariant for `CHECK { use strict; }`.
    let body = block(vec![use_node("strict", &[], 8, 20)], 6, 22);
    let ast = program(vec![phase_block("CHECK", body, 0, 22), use_node("warnings", &[], 23, 36)]);
    let map = PragmaTracker::build(&ast);

    let after = PragmaTracker::state_for_offset(&map, 30);
    assert!(!after.strict_vars, "strict_vars must NOT propagate out of CHECK block");
    assert!(!after.strict_subs, "strict_subs must NOT propagate out of CHECK block");
    assert!(!after.strict_refs, "strict_refs must NOT propagate out of CHECK block");
}

#[test]
fn given_unitcheck_block_with_use_strict_when_querying_after_block_then_strict_is_not_active() {
    // Same invariant for `UNITCHECK { use strict; }`.
    let body = block(vec![use_node("strict", &[], 12, 24)], 10, 26);
    let ast =
        program(vec![phase_block("UNITCHECK", body, 0, 26), use_node("warnings", &[], 27, 40)]);
    let map = PragmaTracker::build(&ast);

    let after = PragmaTracker::state_for_offset(&map, 34);
    assert!(!after.strict_vars, "strict_vars must NOT propagate out of UNITCHECK block");
    assert!(!after.strict_subs, "strict_subs must NOT propagate out of UNITCHECK block");
    assert!(!after.strict_refs, "strict_refs must NOT propagate out of UNITCHECK block");
}

#[test]
fn given_begin_block_with_use_warnings_when_querying_after_block_then_warnings_is_not_active() {
    // `BEGIN { use warnings; }` — warnings applies only inside BEGIN, not file-wide.
    // PragmaTracker treats PhaseBlock as opaque so the state after the block is the
    // same as before it entered (strict=true from the preceding `use strict;`, but
    // warnings=false since that only appeared inside the phase block body).
    let body = block(vec![use_node("warnings", &[], 8, 21)], 6, 23);
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        phase_block("BEGIN", body, 13, 35),
        // Statement after block at offset 36+
        use_node("utf8", &[], 36, 44),
    ]);
    let map = PragmaTracker::build(&ast);

    // After the block, warnings must NOT be active — it was only declared inside the
    // phase block body and must not leak to file scope.
    let after = PragmaTracker::state_for_offset(&map, 40);
    // strict WAS declared at file level before the phase block, so it IS active after.
    assert!(after.strict_vars, "strict_vars from before the BEGIN block must still be active");
    // warnings was only inside BEGIN body — must NOT be active after.
    assert!(!after.warnings, "warnings must NOT propagate out of BEGIN block");
}

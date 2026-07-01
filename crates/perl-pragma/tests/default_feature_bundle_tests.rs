//! Spec-alignment tests for the Perl `:default` feature bundle.
//!
//! In Perl, a file outside any `use VERSION` / `use feature` declaration still
//! has the `:default` feature bundle enabled — `indirect`, `multidimensional`,
//! `bareword_filehandles`, `apostrophe_as_package_separator`, and `smartmatch`.
//! These default-on features are not free-standing extras: `use vX.Y` bundles
//! work by *disabling* them (for example `use v5.36` turns off `indirect` and
//! `multidimensional`; `use v5.38` additionally turns off
//! `bareword_filehandles`; `use v5.42` turns off
//! `apostrophe_as_package_separator` and `smartmatch`).
//!
//! These tests pin the baseline ([`PragmaState::default`]) and the way the
//! version bundles compose against it.
//!
//! Reference: <https://perldoc.perl.org/feature#FEATURE-BUNDLES>

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{CompileTimePragmaEnvironment, PragmaState, PragmaTracker};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn function_call(name: &str, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::FunctionCall { name: name.to_string(), args: vec![] },
        location: loc(start, end),
    }
}

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

/// Effective state after the last statement in the program.
fn final_state(stmts: Vec<Node>) -> PragmaState {
    let ast = program(stmts);
    let map = PragmaTracker::build(&ast);
    PragmaTracker::final_state(&map)
}

const DEFAULT_BUNDLE: &[&str] = &[
    "indirect",
    "multidimensional",
    "bareword_filehandles",
    "apostrophe_as_package_separator",
    "smartmatch",
];

fn assert_has_all(state: &PragmaState, names: &[&str]) {
    for name in names {
        assert!(state.has_feature(name), "expected feature {name:?} to be enabled");
    }
}

fn assert_has_none(state: &PragmaState, names: &[&str]) {
    for name in names {
        assert!(!state.has_feature(name), "expected feature {name:?} to be disabled");
    }
}

// ===========================================================================
// Baseline (`:default`) bundle
// ===========================================================================

#[test]
fn default_state_enables_the_default_bundle() -> TestResult {
    let state = PragmaState::default();
    assert_has_all(&state, DEFAULT_BUNDLE);
    // No version-bundle features leak into the baseline.
    assert_has_none(&state, &["say", "state", "signatures", "isa", "module_true", "try"]);
    Ok(())
}

#[test]
fn default_state_leaves_non_feature_flags_cleared() -> TestResult {
    let state = PragmaState::default();
    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    assert!(!state.warnings);
    assert!(!state.utf8);
    assert!(state.encoding.is_none());
    assert!(!state.unicode_strings);
    assert!(!state.locale);
    assert!(state.builtin_imports.is_empty());
    assert!(state.disabled_warning_categories.is_empty());
    Ok(())
}

#[test]
fn plain_file_with_no_pragmas_reports_default_bundle() -> TestResult {
    // A program with content but no `use`/`no` produces an empty transition
    // map; queries fall back to the baseline, which is the `:default` bundle.
    let ast = program(vec![function_call("print", 0, 12)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 6);
    assert_has_all(&state, DEFAULT_BUNDLE);
    assert!(!state.has_feature("say"));
    Ok(())
}

#[test]
fn all_strict_keeps_the_default_bundle() -> TestResult {
    let state = PragmaState::all_strict();
    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    assert!(state.strict_refs, "input that hits the boundary: strict_refs: true");
    assert!(!state.warnings, "all_strict must not enable warnings");
    assert_has_all(&state, DEFAULT_BUNDLE);
    Ok(())
}

#[test]
fn all_strict_boundary_discriminator_input_that_hits_the_boundary_strict_refs_true() -> TestResult {
    let state = PragmaState::all_strict();
    let expected = PragmaState {
        strict_vars: true,
        strict_subs: true,
        strict_refs: true,
        warnings: false,
        utf8: false,
        encoding: None,
        unicode_strings: false,
        locale: false,
        locale_scope: None,
        disabled_warning_categories: Vec::new(),
        signatures_strict: false,
        features: DEFAULT_BUNDLE.to_vec(),
        builtin_imports: Vec::new(),
    };

    assert_eq!(state.strict_refs, true, "input that hits the boundary: strict_refs: true");
    assert_eq!(state, expected, "all_strict should match the exact expected state");
    Ok(())
}

#[test]
fn use_strict_final_state_matches_all_strict() -> TestResult {
    // `use strict` only toggles the strict categories; the `:default` feature
    // bundle is untouched, so the effective state equals `all_strict()`.
    let state = final_state(vec![use_node("strict", &[], 0, 11)]);
    assert_eq!(state, PragmaState::all_strict());
    assert_has_all(&state, DEFAULT_BUNDLE);
    Ok(())
}

// ===========================================================================
// `use vX.Y` bundles disable default-on features
// ===========================================================================

#[test]
fn use_v5_36_disables_indirect_and_multidimensional() -> TestResult {
    let state = final_state(vec![use_node("v5.36", &[], 0, 9)]);
    // Disabled by the 5.36 bundle:
    assert_has_none(&state, &["indirect", "multidimensional", "switch"]);
    // Still on in the 5.36 bundle:
    assert_has_all(
        &state,
        &["bareword_filehandles", "apostrophe_as_package_separator", "smartmatch"],
    );
    // Added by the 5.36 bundle:
    assert_has_all(&state, &["say", "signatures", "isa", "state"]);
    // v5.36 implies strict + warnings.
    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    assert!(state.warnings);
    Ok(())
}

#[test]
fn use_v5_38_additionally_disables_bareword_filehandles() -> TestResult {
    let state = final_state(vec![use_node("v5.38", &[], 0, 9)]);
    assert_has_none(&state, &["indirect", "multidimensional", "bareword_filehandles"]);
    assert_has_all(&state, &["apostrophe_as_package_separator", "smartmatch", "module_true"]);
    Ok(())
}

#[test]
fn use_v5_42_disables_apostrophe_separator_and_smartmatch() -> TestResult {
    let state = final_state(vec![use_node("v5.42", &[], 0, 9)]);
    assert_has_none(
        &state,
        &[
            "indirect",
            "multidimensional",
            "bareword_filehandles",
            "apostrophe_as_package_separator",
            "smartmatch",
        ],
    );
    assert_has_all(&state, &["say", "signatures", "try", "module_true"]);
    Ok(())
}

// ===========================================================================
// `no feature` against the baseline
// ===========================================================================

#[test]
fn no_feature_disables_a_single_default_on_feature() -> TestResult {
    // `no feature 'multidimensional';` lexically turns off one baseline feature
    // while leaving the rest of the `:default` bundle intact.
    let state = final_state(vec![no_node("feature", &["multidimensional"], 0, 30)]);
    assert!(!state.has_feature("multidimensional"));
    assert_has_all(
        &state,
        &["indirect", "bareword_filehandles", "apostrophe_as_package_separator", "smartmatch"],
    );
    Ok(())
}

#[test]
fn bare_no_feature_resets_to_default_bundle() -> TestResult {
    // `use feature 'say'; no feature;` should drop the explicit `say` and leave
    // the `:default` bundle restored.
    let state =
        final_state(vec![use_node("feature", &["say"], 0, 18), no_node("feature", &[], 18, 28)]);
    assert!(!state.has_feature("say"), "bare `no feature` resets the explicit 'say'");
    assert_has_all(&state, DEFAULT_BUNDLE);
    Ok(())
}

#[test]
fn use_feature_say_adds_to_the_default_bundle() -> TestResult {
    // Explicitly enabling a feature augments — not replaces — the baseline.
    let state = final_state(vec![use_node("feature", &["say"], 0, 18)]);
    assert!(state.has_feature("say"));
    assert_has_all(&state, DEFAULT_BUNDLE);
    Ok(())
}

// ===========================================================================
// PragmaState ↔ CompileTimePragmaEnvironment interaction: seeded :default
// ===========================================================================
//
// These two tests drive the NEW code path introduced by this PR: the custom
// `Default for PragmaState` that seeds `features` with `DEFAULT_FEATURES`.
// Both tests would FAIL if the PR's fix were reverted (empty `features` vec).

/// `PragmaMap::snapshot_at` returns `PragmaSnapshot::default()` when the query
/// offset is before any pragma entry (the `idx == 0` fallback path). That
/// snapshot must carry the `:default` feature bundle because it delegates to
/// `PragmaState::default()`, which now seeds `features` with `DEFAULT_FEATURES`.
///
/// Before the fix: `PragmaState::default()` had `features: vec![]`, so every
/// feature query on the pre-pragma snapshot returned `false` — wrong for Perl's
/// file-scope semantics where `:default` features are always active.
#[test]
fn pragma_environment_snapshot_before_first_pragma_has_default_bundle() -> TestResult {
    // Place the pragma well past offset 0 so a query at offset 0 definitely
    // hits the `idx == 0` branch (no entries at or before byte 0).
    let ast = program(vec![use_node("strict", &[], 50, 62)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);

    // Query before the first pragma — exercises PragmaMap::snapshot_at idx==0
    // fallback, which returns PragmaSnapshot::default() → PragmaState::default().
    let snapshot = environment.snapshot_at(0);
    assert_has_all(snapshot.state(), DEFAULT_BUNDLE);
    assert!(!snapshot.state().strict_vars, "pre-pragma snapshot must not have strict_vars");
    Ok(())
}

/// `PragmaQueryCursor::snapshot_at` falls back to `PragmaSnapshot::default()`
/// (via `map_or_else(PragmaSnapshot::default, ...)`) when no entry matches —
/// this is the `entry_for_offset` returning `None` branch.
///
/// Before the fix: `PragmaSnapshot::default()` derived from `PragmaState`
/// which had `features: vec![]`, so `has_feature("indirect")` returned `false`
/// on the cursor path even though Perl's pre-pragma file scope has `:default`
/// features on.  After the fix `PragmaState::default()` seeds `features` with
/// `DEFAULT_FEATURES`, so `PragmaSnapshot::default()` inherits the bundle.
#[test]
fn cursor_snapshot_at_on_empty_map_has_default_bundle() -> TestResult {
    // An empty program produces no pragma entries — the cursor must return a
    // default snapshot that carries the full `:default` bundle.
    let ast = program(vec![]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    let mut cursor = pragma_map.cursor();

    // This exercises PragmaQueryCursor::snapshot_at → entry_for_offset → None
    // → map_or_else(PragmaSnapshot::default, …) → PragmaState::default().
    let snapshot = cursor.snapshot_at(pragma_map, 0);
    assert_has_all(snapshot.state(), DEFAULT_BUNDLE);
    // Strict/warnings must not bleed in from the default baseline.
    assert!(!snapshot.state().strict_vars, "cursor default must not enable strict_vars");
    assert!(!snapshot.state().warnings, "cursor default must not enable warnings");
    Ok(())
}

/// `build_scoped_body` (in `range_builder/walk.rs`) saves the caller's
/// `current_state` before entering a block and restores it on exit.  After the
/// PR the initial `current_state` that `CompileTimePragmaEnvironment::build`
/// passes in is `PragmaState::default()` — which carries the `:default` bundle.
/// The scope-restore pushes that saved state as the post-scope entry, so any
/// query after the block should still see the `:default` bundle.
///
/// Before the fix: the saved state had `features: vec![]`, so the post-scope
/// restore also had empty features — the `:default` bundle would be lost after
/// any lexical scope closed.
#[test]
fn build_scoped_body_restore_preserves_default_bundle() -> TestResult {
    // A scoped block that adds `say`; after the block closes the state should
    // restore to the parent (seeded :default, no `say`).
    let inner_use = use_node("feature", &["say"], 10, 28);
    let scoped = block(vec![inner_use], 5, 35);
    let ast = program(vec![scoped]);
    let environment = CompileTimePragmaEnvironment::build(&ast);

    // Query inside the block — `say` was added on top of the seeded :default.
    let in_scope = environment.snapshot_at(20);
    assert!(in_scope.state().has_feature("say"), "inside block must see 'say'");
    assert_has_all(in_scope.state(), DEFAULT_BUNDLE);

    // Query after the block — state is restored to the seeded :default; `say`
    // is gone but the `:default` bundle must still be present.
    let after_scope = environment.snapshot_at(36);
    assert!(!after_scope.state().has_feature("say"), "after block closes 'say' must be gone");
    assert_has_all(after_scope.state(), DEFAULT_BUNDLE);
    Ok(())
}

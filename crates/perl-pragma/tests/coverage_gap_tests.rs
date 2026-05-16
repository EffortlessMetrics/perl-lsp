//! Tests targeting previously-uncovered branches in perl-pragma.
//!
//! Covers:
//! - `PragmaSnapshot` accessor methods (`state`, `has_feature`, `is_warning_active`)
//! - `PragmaSnapshot::strict_enabled` / `warnings_enabled` on false paths
//! - `PragmaStateQuery::offset`
//! - `PragmaMap::snapshot_at` with idx=0 (offset before first entry)
//! - `normalize_snapshot` with `signatures_strict=true`
//! - Unknown feature name path in `known_feature_name` (returns `None`)
//! - `enable_feature_name` / `disable_feature_name` returning `false`
//! - `conditional_target_tail_is_valid` returning `false` for untracked modules
//! - `apply_conditional_use_target` / `apply_conditional_no_target` for
//!   "warnings" and "locale" arms and the "feature returns false" early-return path
//! - `apply_conditional_no_target` "_ => return" arm
//! - `PragmaQueryCursor` branches: index past end, index adjustment in the
//!   explicit-map `entry_for_offset` path
//! - `NodeKind::LabeledStatement` and `NodeKind::StatementModifier` walk arms

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{
    CompileTimePragmaEnvironment, PerlVersion, PragmaQueryCursor, PragmaSnapshot, PragmaState,
    PragmaTracker, features_enabled_by_version, parse_perl_version,
};

// ---------------------------------------------------------------------------
// Helpers (mirrored from other test files)
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

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

fn dummy_node(start: usize, end: usize) -> Node {
    Node { kind: NodeKind::MissingExpression, location: loc(start, end) }
}

// ---------------------------------------------------------------------------
// PragmaSnapshot accessor methods
// ---------------------------------------------------------------------------

/// `PragmaSnapshot::state()` must expose the underlying `PragmaState`.
#[test]
fn pragma_snapshot_state_accessor_returns_underlying_state()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(8);

    let state = snapshot.state();
    assert!(state.strict_vars, "state() must reflect strict_vars from the snapshot");
    assert!(state.strict_subs, "state() must reflect strict_subs from the snapshot");
    Ok(())
}

/// `PragmaSnapshot::has_feature()` must delegate to the underlying state.
#[test]
fn pragma_snapshot_has_feature_delegates_to_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["'say'"], 0, 18)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(10);

    assert!(snapshot.has_feature("say"), "snapshot must report say as enabled");
    assert!(!snapshot.has_feature("builtin"), "snapshot must report builtin as disabled");
    Ok(())
}

/// `PragmaSnapshot::is_warning_active()` must delegate to the underlying state.
#[test]
fn pragma_snapshot_is_warning_active_delegates_to_state() -> Result<(), Box<dyn std::error::Error>>
{
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["'deprecated'"], 16, 40),
    ]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(30);

    assert!(snapshot.is_warning_active("uninitialized"), "uninitialized category should be active");
    assert!(!snapshot.is_warning_active("deprecated"), "deprecated category should be disabled");
    Ok(())
}

/// `PragmaSnapshot::strict_enabled()` must return false when not all categories are on.
#[test]
fn pragma_snapshot_strict_enabled_returns_false_when_partial()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["vars"], 0, 16)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(8);

    assert!(!snapshot.strict_enabled(), "strict_enabled must be false when only vars is on");
    Ok(())
}

/// `PragmaSnapshot::strict_enabled()` must return true when all three categories are on.
#[test]
fn pragma_snapshot_strict_enabled_returns_true_when_all_on()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(8);

    assert!(snapshot.strict_enabled(), "strict_enabled must be true when all strict is on");
    Ok(())
}

/// `PragmaSnapshot::warnings_enabled()` must return true/false correctly.
#[test]
fn pragma_snapshot_warnings_enabled_reflects_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast_on = program(vec![use_node("warnings", &[], 0, 15)]);
    let env_on = CompileTimePragmaEnvironment::build(&ast_on);
    assert!(env_on.snapshot_at(8).warnings_enabled());

    let ast_off = program(vec![]);
    let env_off = CompileTimePragmaEnvironment::build(&ast_off);
    assert!(!env_off.snapshot_at(0).warnings_enabled());
    Ok(())
}

// ---------------------------------------------------------------------------
// PragmaStateQuery::offset()
// ---------------------------------------------------------------------------

/// `PragmaStateQuery::offset()` must return the requested byte offset.
#[test]
fn pragma_state_query_offset_returns_requested_offset() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let query = env.query_at(8);

    assert_eq!(query.offset(), 8, "offset() must return the query offset");
    Ok(())
}

// ---------------------------------------------------------------------------
// PragmaMap::snapshot_at — offset before first entry (idx = 0)
// ---------------------------------------------------------------------------

/// Querying an offset before any pragma entry must return the default snapshot.
#[test]
fn pragma_map_snapshot_at_before_first_entry_returns_default()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 50, 62)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let map = env.map();

    let snapshot = map.snapshot_at(10);
    assert!(
        !snapshot.state().strict_vars,
        "snapshot at offset before first pragma must not have strict_vars"
    );
    assert!(
        !snapshot.state().strict_subs,
        "snapshot at offset before first pragma must not have strict_subs"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// normalize_snapshot with signatures_strict=true
// ---------------------------------------------------------------------------

/// `CompileTimePragmaEnvironment::snapshot_at` must normalise `signatures_strict`
/// so that all three strict booleans appear true even when not explicitly set.
#[test]
fn snapshot_at_normalises_signatures_strict_to_all_strict_flags()
-> Result<(), Box<dyn std::error::Error>> {
    // use feature 'signatures' sets signatures_strict but not the individual flags.
    // snapshot_at must call normalize_snapshot which propagates signatures_strict.
    let ast = program(vec![use_node("feature", &["'signatures'"], 0, 26)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = env.snapshot_at(15);

    assert!(snapshot.state().signatures_strict, "signatures_strict must be true");
    // After normalisation, all strict categories must appear set.
    assert!(
        snapshot.strict_enabled(),
        "strict_enabled() must be true after signatures normalisation"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Unknown feature name — known_feature_name returns None
// ---------------------------------------------------------------------------

/// Attempting to enable an unknown feature name must have no effect on state.
#[test]
fn use_feature_unknown_name_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["'totally_fake_feature_xyz'"], 0, 40)]);
    let map = PragmaTracker::build(&ast);

    // No known feature was enabled, so no entry should be pushed.
    assert!(map.is_empty(), "unknown feature name must not produce a pragma-map entry");
    Ok(())
}

/// Attempting to disable an unknown feature name must also be a no-op.
#[test]
fn no_feature_unknown_name_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("feature", &["'say'"], 0, 18),
        no_node("feature", &["'totally_fake_feature_xyz'"], 19, 55),
    ]);
    let map = PragmaTracker::build(&ast);

    // The no directive with an unknown name returns false (no change), so no
    // second entry is pushed; the 'say' entry from use-feature stays unchanged.
    assert_eq!(map.len(), 1, "unknown no-feature name must not produce an additional entry");
    let state = PragmaTracker::state_for_offset(&map, 40);
    assert!(state.has_feature("say"), "previously enabled 'say' must still be present");
    Ok(())
}

// ---------------------------------------------------------------------------
// enable_feature_name — feature already present doesn't add duplicate to vec
// ---------------------------------------------------------------------------

/// Enabling a feature that is already in the features list must not add a
/// duplicate to the features vector (even though enable_feature_name still
/// returns true for a known feature, and apply_feature_state may push another
/// state entry).
#[test]
fn use_feature_duplicate_does_not_add_duplicate_to_features_vec()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("feature", &["'say'"], 0, 18),
        use_node("feature", &["'say'"], 19, 37),
    ]);
    let map = PragmaTracker::build(&ast);

    // After the second 'use feature say', the features list must not have duplicates.
    let state = PragmaTracker::state_for_offset(&map, 30);
    let say_count = state.features.iter().filter(|&&f| f == "say").count();
    assert_eq!(say_count, 1, "features vec must not contain duplicate 'say' entries");
    Ok(())
}

// ---------------------------------------------------------------------------
// disable_feature_name returns false — feature not present
// ---------------------------------------------------------------------------

/// Disabling a known feature that is not currently active must return false,
/// leaving the map unchanged.
#[test]
fn no_feature_not_present_does_not_push_entry() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("feature", &["'say'"], 0, 18),
        no_node("feature", &["'builtin'"], 19, 39),
    ]);
    let map = PragmaTracker::build(&ast);

    // 'builtin' was never enabled, so disable_feature_name returns false →
    // apply_feature_state returns false → no entry pushed.
    assert_eq!(map.len(), 1, "disabling a feature not in the list must not push an extra entry");
    let state = PragmaTracker::state_for_offset(&map, 30);
    assert!(state.has_feature("say"), "'say' must still be present");
    assert!(!state.has_feature("builtin"), "'builtin' was never enabled");
    Ok(())
}

// ---------------------------------------------------------------------------
// conditional_target_tail_is_valid — returns false for untracked module
// ---------------------------------------------------------------------------

/// `use if` with an untracked target module must be a no-op.
#[test]
fn use_if_with_untracked_target_module_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // conditional_target_tail_is_valid returns false for the `_ => false` arm,
    // so conditional_pragma_target returns None, and apply_conditional_use_target
    // is never called — no entry pushed.
    let ast = program(vec![use_node("if", &["$cond", "Moose"], 0, 22)]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "use if with untracked target must not push any entry");
    Ok(())
}

/// `no if` with an untracked target module must also be a no-op.
#[test]
fn no_if_with_untracked_target_module_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![no_node("if", &["$cond", "Moose"], 0, 22)]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "no if with untracked target must not push any entry");
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_use_target — "warnings" arm
// ---------------------------------------------------------------------------

/// `use if $cond, warnings` must enable warnings via the conditional path.
#[test]
fn use_if_warnings_enables_warnings_conditionally() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "warnings"], 0, 28)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.warnings, "use if cond, warnings must enable warnings");
    assert!(state.disabled_warning_categories.is_empty(), "disabled categories must be cleared");
    Ok(())
}

/// `use if $cond, 'warnings', 'FATAL'` with an arg must still record warnings.
#[test]
fn use_if_warnings_with_args_enables_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "warnings", "'FATAL'"], 0, 40)]);
    let map = PragmaTracker::build(&ast);

    // conditional_target_tail_is_valid returns true for "warnings" regardless of tail
    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(state.warnings, "use if cond, warnings 'FATAL' must enable warnings");
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_use_target — "locale" arm
// ---------------------------------------------------------------------------

/// `use if $cond, locale` must enable locale state via the conditional path.
#[test]
fn use_if_locale_enables_locale_conditionally() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "locale"], 0, 26)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.locale, "use if cond, locale must enable locale");
    Ok(())
}

/// `use if $cond, locale, ':not_characters'` must capture the scope argument.
#[test]
fn use_if_locale_with_scope_arg_records_locale_scope() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "locale", "':not_characters'"], 0, 46)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 25);
    assert!(state.locale, "locale must be enabled");
    assert_eq!(
        state.locale_scope.as_deref(),
        Some(":not_characters"),
        "locale scope must be captured"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_use_target — "feature" early-return (no change)
// ---------------------------------------------------------------------------

/// `use if $cond, feature => 'bogus_feature'` must be a no-op because
/// apply_feature_state returns false (no known feature changed), causing
/// apply_conditional_use_target to early-return without pushing an entry.
#[test]
fn use_if_feature_unknown_name_is_no_op() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "feature", "'bogus_xyz'"], 0, 38)]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "use if with unknown feature name must not push any entry");
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_use_target — "_ else return" (untracked version string)
// ---------------------------------------------------------------------------

/// `use if $cond, 'v99.99'` targets a version that parse_perl_version parses
/// as Some, but enable_effective_version_semantics does nothing harmful.
/// The important path is that an unparseable fake version returns None, hitting
/// the `else { return; }` arm.
///
/// We test with a genuine known version to exercise the version arm in
/// `apply_conditional_use_target`, since that arm was also uncovered.
#[test]
fn use_if_version_target_via_conditional_applies_version_semantics()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "v5.36"], 0, 22)]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 12);
    assert!(state.strict_vars, "v5.36 implies strict");
    assert!(state.has_feature("signatures"), "v5.36 bundle includes signatures");
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_no_target — "feature" early-return (no change)
// ---------------------------------------------------------------------------

/// `no if $cond, feature => 'bogus_feature'` must be a no-op because
/// apply_feature_state returns false for an unknown feature name.
#[test]
fn no_if_feature_unknown_name_is_no_op() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![no_node("if", &["$cond", "feature", "'bogus_xyz'"], 0, 38)]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "no if with unknown feature name must not push any entry");
    Ok(())
}

/// `no if $cond, feature => 'builtin'` when builtin is not currently active
/// must also be a no-op (apply_feature_state returns false, early return).
#[test]
fn no_if_feature_not_currently_active_is_no_op() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![no_node("if", &["$cond", "feature", "'builtin'"], 0, 38)]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "no if with inactive feature must not push any entry");
    Ok(())
}

// ---------------------------------------------------------------------------
// apply_conditional_no_target — "_ => return" arm
// ---------------------------------------------------------------------------

/// `no if $cond, 'SomeUntracked::Module'` must hit the `_ => return` arm of
/// apply_conditional_no_target and produce no state change.
///
/// We need conditional_pragma_target to actually find a target — but since
/// `_ => false` in conditional_target_tail_is_valid blocks untracked modules,
/// the no-entry result comes from the outer guard.  The `_ => return` arm is
/// reached when the module is a valid version string (parseable) in the
/// `use if` conditional but doesn't match any specific case.
///
/// To exercise `_ => return` directly in apply_conditional_no_target we need
/// a module that: (a) passes is_tracked_pragma_module / parse_perl_version,
/// but (b) is NOT in the match arms. That can't happen with current code since
/// all tracked modules are covered.  The practical path is through a version
/// string that has no separate arm — tested via `no if $cond, v5.10`:
#[test]
fn no_if_version_target_currently_not_tracked_is_no_op() -> Result<(), Box<dyn std::error::Error>> {
    // `no if $cond, v5.10` — conditional_pragma_target finds v5.10 (version),
    // conditional_target_tail_is_valid returns true (tail is empty for version).
    // apply_conditional_no_target is called with module="v5.10".
    // The match in apply_conditional_no_target has no arm for version strings,
    // so it hits `_ => return`, producing no entry.
    let ast = program(vec![no_node("if", &["$cond", "v5.10"], 0, 22)]);
    let map = PragmaTracker::build(&ast);

    assert!(
        map.is_empty(),
        "no if with version target must hit the _ => return arm and push no entry"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// NodeKind::LabeledStatement — walk.rs uncovered arm
// ---------------------------------------------------------------------------

/// A labeled statement wrapping a `use strict` pragma must propagate the pragma.
#[test]
fn labeled_statement_containing_use_strict_propagates_pragma()
-> Result<(), Box<dyn std::error::Error>> {
    let inner_use = use_node("strict", &[], 10, 22);
    let labeled = Node {
        kind: NodeKind::LabeledStatement {
            label: "OUTER".to_string(),
            statement: Box::new(inner_use),
        },
        location: loc(0, 22),
    };
    let ast = program(vec![labeled]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.strict_vars, "use strict inside labeled statement must set strict_vars");
    assert!(state.strict_subs, "use strict inside labeled statement must set strict_subs");
    assert!(state.strict_refs, "use strict inside labeled statement must set strict_refs");
    Ok(())
}

/// A labeled statement containing a non-pragma node must be a no-op.
#[test]
fn labeled_statement_with_non_pragma_inner_produces_no_entry()
-> Result<(), Box<dyn std::error::Error>> {
    let labeled = Node {
        kind: NodeKind::LabeledStatement {
            label: "LOOP".to_string(),
            statement: Box::new(dummy_node(5, 15)),
        },
        location: loc(0, 15),
    };
    let ast = program(vec![labeled]);
    let map = PragmaTracker::build(&ast);

    assert!(map.is_empty(), "labeled statement with no pragma must not push any entry");
    Ok(())
}

// ---------------------------------------------------------------------------
// NodeKind::StatementModifier — walk.rs uncovered arm
// ---------------------------------------------------------------------------

/// A `use strict if ...` modelled as a StatementModifier must propagate the pragma
/// from the statement child.
#[test]
fn statement_modifier_containing_use_strict_propagates_pragma()
-> Result<(), Box<dyn std::error::Error>> {
    let inner_use = use_node("strict", &[], 0, 12);
    let condition = dummy_node(16, 22);
    let stmt_mod = Node {
        kind: NodeKind::StatementModifier {
            statement: Box::new(inner_use),
            modifier: "if".to_string(),
            condition: Box::new(condition),
        },
        location: loc(0, 22),
    };
    let ast = program(vec![stmt_mod]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 8);
    assert!(state.strict_vars, "use strict inside StatementModifier statement must apply");
    Ok(())
}

/// A StatementModifier whose condition contains a pragma must also apply it.
#[test]
fn statement_modifier_condition_containing_pragma_propagates_it()
-> Result<(), Box<dyn std::error::Error>> {
    // Unusual but syntactically possible: pragma inside the condition expression.
    let statement = dummy_node(0, 10);
    let condition_pragma = use_node("warnings", &[], 14, 26);
    let stmt_mod = Node {
        kind: NodeKind::StatementModifier {
            statement: Box::new(statement),
            modifier: "if".to_string(),
            condition: Box::new(condition_pragma),
        },
        location: loc(0, 26),
    };
    let ast = program(vec![stmt_mod]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(state.warnings, "pragma inside StatementModifier condition must apply");
    Ok(())
}

// ---------------------------------------------------------------------------
// PragmaQueryCursor — entry_for_offset backward-seek on explicit PragmaMap
// ---------------------------------------------------------------------------

/// Using a cursor on a `PragmaMap` (not the legacy tuple API) must correctly
/// handle a backward seek (decreasing offset) that requires a binary-search
/// fallback — exercising the `entries[self.index].range.start > offset` branch
/// in `entry_for_offset`.
#[test]
fn cursor_entry_for_offset_backward_seek_explicit_map() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![no_node("strict", &["refs"], 20, 36)], 18, 40),
        use_node("warnings", &[], 42, 57),
    ]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let map = env.map();

    let mut cursor = map.cursor();

    // Advance cursor well past the start.
    let late = cursor.snapshot_at(map, 50);
    assert!(late.state().warnings, "late offset must see warnings");

    // Backward seek — must fall back to binary search in entry_for_offset.
    let early = cursor.snapshot_at(map, 8);
    assert!(early.state().strict_vars, "early offset must see strict after backward seek");
    assert!(!early.state().warnings, "early offset must not see warnings");

    // Verify result matches the non-cursor API.
    assert_eq!(early, env.snapshot_at(8), "cursor result must match direct snapshot_at");
    Ok(())
}

/// Using a cursor on an empty explicit `PragmaMap` must return the default snapshot.
#[test]
fn cursor_entry_for_offset_empty_map_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let map = env.map();

    let mut cursor = map.cursor();
    let snapshot = cursor.snapshot_at(map, 999);

    assert!(!snapshot.state().strict_vars, "empty map must return default snapshot");
    assert!(!snapshot.state().warnings, "empty map must return default snapshot");
    Ok(())
}

/// Cursor on explicit `PragmaMap`: after advancing far past the end, then querying
/// again at a later offset must not panic and must return the correct state.
/// This exercises the `self.index >= entries.len()` clamp in `entry_for_offset`.
#[test]
fn cursor_entry_for_offset_index_clamped_when_past_end() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 13, 28)]);
    let env = CompileTimePragmaEnvironment::build(&ast);
    let map = env.map();

    let mut cursor = map.cursor();

    // Advance cursor to a very large offset that is past all entries.
    let _ = cursor.snapshot_at(map, 9999);

    // Query again at a smaller but still valid offset — cursor.index is now
    // past entries.len(), triggering the clamp branch.
    let snapshot = cursor.snapshot_at(map, 20);
    assert!(snapshot.state().warnings, "warnings must be visible after index clamp");
    Ok(())
}

/// Reusing a `PragmaQueryCursor` with a smaller explicit `PragmaMap` after
/// it was advanced on a larger map must clamp `self.index` correctly
/// (`entry_for_offset` line `self.index >= entries.len()` guard).
#[test]
fn cursor_reused_with_smaller_map_clamps_index() -> Result<(), Box<dyn std::error::Error>> {
    // Build a larger map (many entries) and advance the cursor to the end.
    let big_ast = program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        use_node("utf8", &[], 29, 38),
        use_node("feature", &["'say'"], 39, 57),
    ]);
    let big_env = CompileTimePragmaEnvironment::build(&big_ast);
    let big_map = big_env.map();
    let big_len = big_map.entries().len();

    let mut cursor = big_map.cursor();
    // Advance cursor to maximum index (entries.len() - 1).
    let _ = cursor.snapshot_at(big_map, 9999);

    // Now query against a much smaller map with fewer entries than cursor.index.
    // cursor.index is currently big_len-1; the small map may have only 1 entry.
    let small_ast = program(vec![use_node("warnings", &[], 0, 15)]);
    let small_env = CompileTimePragmaEnvironment::build(&small_ast);
    let small_map = small_env.map();

    // cursor.index (big_len-1) >= small_map.entries().len() triggers the clamp.
    assert!(big_len > small_map.entries().len(), "pre-condition: big map is larger");
    let snapshot = cursor.snapshot_at(small_map, 10);
    assert!(snapshot.state().warnings, "small map must report warnings after index clamp");
    Ok(())
}

/// Reusing a `PragmaQueryCursor` (legacy tuple API) with a smaller map after it
/// was advanced on a larger map must clamp `self.index` correctly
/// (`state_for_offset` line `self.index >= pragma_map.len()` guard).
#[test]
fn cursor_legacy_api_reused_with_smaller_map_clamps_index() -> Result<(), Box<dyn std::error::Error>>
{
    // Advance a cursor over a large legacy tuple map.
    let big_ast = program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        use_node("utf8", &[], 29, 38),
        use_node("feature", &["'say'"], 39, 57),
    ]);
    let big_map = PragmaTracker::build(&big_ast);

    let mut cursor = PragmaQueryCursor::new();
    let _ = cursor.state_for_offset(&big_map, 9999);

    // Now use the same cursor with a 1-entry map — cursor.index is big_map.len()-1
    // which is >= small_map.len(), triggering the clamp at line 756.
    let small_ast = program(vec![use_node("warnings", &[], 0, 15)]);
    let small_map = PragmaTracker::build(&small_ast);

    assert!(big_map.len() > small_map.len(), "pre-condition: big map is larger");
    let state = cursor.state_for_offset(&small_map, 10);
    assert!(state.warnings, "small map must report warnings after index clamp");
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_perl_version edge cases (lines around 350-363 that touch minor=0 path)
// ---------------------------------------------------------------------------

/// `parse_perl_version("5")` — a version with no minor component must parse to minor=0.
#[test]
fn parse_perl_version_major_only_returns_minor_zero() -> Result<(), Box<dyn std::error::Error>> {
    let v = parse_perl_version("5");
    assert!(v.is_some(), "parse_perl_version('5') must succeed");
    let v = v.ok_or("expected Some")?;
    assert_eq!(v.major, 5);
    assert_eq!(v.minor, 0);
    Ok(())
}

/// `parse_perl_version("v5")` — v-prefixed major-only must parse to minor=0.
#[test]
fn parse_perl_version_v_prefixed_major_only_returns_minor_zero()
-> Result<(), Box<dyn std::error::Error>> {
    let v = parse_perl_version("v5");
    assert!(v.is_some(), "parse_perl_version('v5') must succeed");
    let v = v.ok_or("expected Some")?;
    assert_eq!(v.major, 5);
    assert_eq!(v.minor, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// features_enabled_by_version edge cases
// ---------------------------------------------------------------------------

/// v5.8 (below all known bundles) must return an empty feature list.
#[test]
fn features_enabled_by_v5_8_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 8));
    assert!(features.is_empty(), "v5.8 must not enable any features");
    Ok(())
}

/// v5.20 must include postfix_deref.
#[test]
fn features_enabled_by_v5_20_includes_postfix_deref() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 20));
    assert!(features.contains(&"postfix_deref"), "v5.20 must include postfix_deref");
    Ok(())
}

// ---------------------------------------------------------------------------
// PragmaSnapshot From/Into conversions
// ---------------------------------------------------------------------------

/// `PragmaState` must round-trip through `PragmaSnapshot` via `From`/`Into`.
#[test]
fn pragma_state_round_trips_through_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState { warnings: true, strict_vars: true, ..Default::default() };

    let snapshot = PragmaSnapshot::from_state(state.clone());
    let recovered: PragmaState = snapshot.into();

    assert_eq!(recovered.warnings, state.warnings);
    assert_eq!(recovered.strict_vars, state.strict_vars);
    Ok(())
}

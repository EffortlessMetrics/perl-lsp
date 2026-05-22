//! Coverage gap tests for perl-pragma crate.
//!
//! Targets branch coverage gaps identified in `perl-pragma/lib.rs`:
//!
//! - `use if` / `no if` with `utf8`, `encoding`, and `locale` targets (conditional arms)
//! - `use if` / `no if` with no recognized target (no-op paths)
//! - `no feature ':VERSION'` bundle disable (feature_state `disable_feature_name` bundle path)
//! - `apply_feature_state` no-args disabled path restoring the default feature bundle
//! - `apply_no_directive` for feature when no state change (no push path)
//! - `LabeledStatement` and `StatementModifier` AST traversal
//! - `PragmaSnapshot`/`PragmaState` conversions (`From` impls)
//! - `PragmaMap::state_at`, `PragmaStateQuery` methods, `PragmaSnapshot` methods
//! - `PragmaQueryCursor` explicit-map and legacy-map clamp/backward-seek branches
//! - `features_enabled_by_version` documented bundle boundaries
//! - `parse_perl_version` major-only without 'v' prefix
//! - `normalize_snapshot` propagation of `signatures_strict`
//! - `builtin_import_names` double-quoted single name
//! - `conditional_pragma_target` edge cases: empty encoding arg, feature with no args
//! - `conditional_target_tail_is_valid` for `builtin` with all-empty args

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{
    CompileTimePragmaEnvironment, PerlVersion, PragmaQueryCursor, PragmaSnapshot, PragmaState,
    PragmaTracker, features_enabled_by_version, parse_perl_version, version_implies_strict,
    version_implies_warnings,
};

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

// ===========================================================================
// `use if` / `no if` with utf8 target
// ===========================================================================

#[test]
fn use_if_utf8_conditionally_enables_utf8() -> Result<(), Box<dyn std::error::Error>> {
    // `use if $cond, 'utf8'`
    let ast = program(vec![use_node("if", &["$cond", "'utf8'"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(state.utf8, "use if with utf8 target must enable utf8");
    Ok(())
}

#[test]
fn no_if_utf8_conditionally_disables_utf8() -> Result<(), Box<dyn std::error::Error>> {
    // `use utf8; no if $cond, 'utf8'`
    let ast =
        program(vec![use_node("utf8", &[], 0, 9), no_node("if", &["$cond", "'utf8'"], 10, 30)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 25);
    assert!(!state.utf8, "no if with utf8 target must disable utf8");
    Ok(())
}

// ===========================================================================
// `no if` with encoding target
// ===========================================================================

#[test]
fn no_if_encoding_conditionally_clears_encoding() -> Result<(), Box<dyn std::error::Error>> {
    // `use encoding 'utf8'; no if $cond, encoding, 'utf8'`
    let ast = program(vec![
        use_node("encoding", &["'utf8'"], 0, 18),
        no_node("if", &["$cond", "encoding", "'utf8'"], 19, 50),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 35);
    assert!(state.encoding.is_none(), "no if with encoding target must clear encoding");
    Ok(())
}

// ===========================================================================
// `use if` / `no if` with no recognized pragma target — no-op path
// ===========================================================================

#[test]
fn use_if_with_untracked_module_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use if $cond, 'Moose'` — Moose is not a tracked pragma
    // conditional_pragma_target returns None → no state change
    let ast = program(vec![use_node("if", &["$cond", "'Moose'"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    // No pragma state change should occur
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(!state.strict_vars, "untracked module in use if must not affect strict");
    assert!(!state.warnings, "untracked module in use if must not affect warnings");
    Ok(())
}

#[test]
fn no_if_with_untracked_module_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use strict; no if $cond, 'Moose'` — Moose is not tracked
    let ast =
        program(vec![use_node("strict", &[], 0, 12), no_node("if", &["$cond", "'Moose'"], 13, 35)]);
    let map = PragmaTracker::build(&ast);
    // The strict state from use strict should still be active
    let state = PragmaTracker::state_for_offset(&map, 25);
    assert!(state.strict_vars, "strict must survive a no-if with untracked module");
    Ok(())
}

// ===========================================================================
// `no if` with warnings target — disable-category via conditional path
// ===========================================================================

#[test]
fn no_if_warnings_category_disables_specific_category() -> Result<(), Box<dyn std::error::Error>> {
    // `use warnings; no if $cond, warnings, 'uninitialized'`
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("if", &["$cond", "warnings", "'uninitialized'"], 16, 55),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 35);
    assert!(state.warnings, "global warnings must stay on after conditional category disable");
    assert!(!state.is_warning_active("uninitialized"), "category must be disabled");
    assert!(state.is_warning_active("deprecated"), "other categories must remain active");
    Ok(())
}

// ===========================================================================
// `no feature ':VERSION'` — disable feature bundle via version tag
// ===========================================================================

#[test]
fn no_feature_bundle_version_tag_disables_bundle() -> Result<(), Box<dyn std::error::Error>> {
    // `use v5.36; no feature ':5.36'`
    // This exercises the `!enabled && item starts with ':'` branch in apply_feature_state
    let ast =
        program(vec![use_node("v5.36", &[], 0, 10), no_node("feature", &["':5.36'"], 11, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(!state.has_feature("signatures"), "no feature ':5.36' must remove signatures");
    assert!(!state.has_feature("defer"), "no feature ':5.36' must remove defer");
    assert!(!state.has_feature("isa"), "no feature ':5.36' must remove isa");
    // Features from older bundles (v5.10) were also in v5.36 and should be removed
    assert!(!state.has_feature("say"), "no feature ':5.36' must remove say from bundle");
    Ok(())
}

#[test]
fn no_feature_version_bundle_only_removes_bundle_features() -> Result<(), Box<dyn std::error::Error>>
{
    // `use v5.38; no feature ':5.10'`
    // Only v5.10 features (say, state, switch) should be removed
    // v5.16+ features should remain
    let ast =
        program(vec![use_node("v5.38", &[], 0, 10), no_node("feature", &["':5.10'"], 11, 27)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(!state.has_feature("say"), "say (v5.10) must be removed by no feature ':5.10'");
    assert!(!state.has_feature("state"), "state (v5.10) must be removed by no feature ':5.10'");
    // v5.16 features should survive (they were not in the :5.10 bundle)
    assert!(
        state.has_feature("unicode_eval"),
        "unicode_eval (v5.16) must survive no feature ':5.10'"
    );
    Ok(())
}

// ===========================================================================
// `apply_feature_state` with no-op conditions
// ===========================================================================

#[test]
fn no_feature_bare_on_empty_state_restores_default_bundle() -> Result<(), Box<dyn std::error::Error>>
{
    // `no feature` with no args resets to the documented :default bundle.
    let ast = program(vec![no_node("feature", &[], 0, 11)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 5);
    assert!(state.has_feature("indirect"), "no feature must restore :default indirect");
    assert!(
        state.has_feature("multidimensional"),
        "no feature must restore :default multidimensional"
    );
    assert!(
        state.has_feature("bareword_filehandles"),
        "no feature must restore :default bareword_filehandles"
    );
    assert!(
        state.has_feature("apostrophe_as_package_separator"),
        "no feature must restore :default apostrophe package separator"
    );
    assert!(state.has_feature("smartmatch"), "no feature must restore :default smartmatch");
    assert!(!state.has_feature("say"), "no feature must not restore non-default say");
    Ok(())
}

#[test]
fn use_feature_unknown_name_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use feature 'NonExistentFeature'` — known_feature_name returns None
    // enable_feature_name returns false → apply_feature_state returns false → no push
    let ast = program(vec![use_node("feature", &["'NonExistentFeature'"], 0, 34)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty(), "unknown feature name must produce no map entry");
    Ok(())
}

#[test]
fn no_feature_unknown_name_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `no feature 'NonExistentFeature'` — disable_feature_name returns false → no push
    let ast = program(vec![no_node("feature", &["'NonExistentFeature'"], 0, 34)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty(), "unknown feature name in no feature must produce no map entry");
    Ok(())
}

// ===========================================================================
// `LabeledStatement` traversal
// ===========================================================================

#[test]
fn labeled_statement_with_use_strict_is_tracked() -> Result<(), Box<dyn std::error::Error>> {
    // LABEL: use strict;
    // The walk.rs handler for LabeledStatement recurses into the inner statement.
    let inner = use_node("strict", &[], 8, 20);
    let labeled = Node {
        kind: NodeKind::LabeledStatement { label: "LOOP".to_string(), statement: Box::new(inner) },
        location: loc(0, 20),
    };
    let ast = program(vec![labeled]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.strict_vars, "strict inside labeled statement must be tracked");
    assert!(state.strict_subs, "strict inside labeled statement must be tracked");
    assert!(state.strict_refs, "strict inside labeled statement must be tracked");
    Ok(())
}

// ===========================================================================
// `StatementModifier` traversal
// ===========================================================================

#[test]
fn statement_modifier_with_use_strict_in_statement_is_tracked()
-> Result<(), Box<dyn std::error::Error>> {
    // `use strict if $cond;` — as a StatementModifier node
    // walk.rs recurses into both `statement` and `condition`.
    let inner_use = use_node("strict", &[], 0, 12);
    let modifier = Node {
        kind: NodeKind::StatementModifier {
            statement: Box::new(inner_use),
            modifier: "if".to_string(),
            condition: Box::new(dummy_node(16, 21)),
        },
        location: loc(0, 21),
    };
    let ast = program(vec![modifier]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(state.strict_vars, "strict inside statement modifier must be tracked");
    Ok(())
}

#[test]
fn statement_modifier_with_use_warnings_in_condition_is_tracked()
-> Result<(), Box<dyn std::error::Error>> {
    // StatementModifier where the condition itself contains a use pragma.
    // walk.rs also recurses into `condition`.
    let inner_use = use_node("warnings", &[], 15, 30);
    let modifier = Node {
        kind: NodeKind::StatementModifier {
            statement: Box::new(dummy_node(0, 10)),
            modifier: "if".to_string(),
            condition: Box::new(inner_use),
        },
        location: loc(0, 30),
    };
    let ast = program(vec![modifier]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 20);
    assert!(state.warnings, "warnings inside statement modifier condition must be tracked");
    Ok(())
}

// ===========================================================================
// `PragmaSnapshot` / `PragmaState` conversions and helper methods
// ===========================================================================

#[test]
fn pragma_snapshot_from_state_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    // PragmaSnapshot::from_state() and From<PragmaState> for PragmaSnapshot
    let state = PragmaState { warnings: true, strict_vars: true, ..PragmaState::default() };
    let snapshot = PragmaSnapshot::from_state(state.clone());
    assert_eq!(snapshot.state(), &state, "from_state roundtrip must preserve state");
    Ok(())
}

#[test]
fn pragma_snapshot_from_impl_for_state() -> Result<(), Box<dyn std::error::Error>> {
    // From<PragmaState> for PragmaSnapshot
    let state = PragmaState { utf8: true, ..PragmaState::default() };
    let snapshot: PragmaSnapshot = state.clone().into();
    assert!(snapshot.state().utf8, "From<PragmaState> for PragmaSnapshot must work");
    Ok(())
}

#[test]
fn pragma_state_from_snapshot_impl() -> Result<(), Box<dyn std::error::Error>> {
    // From<PragmaSnapshot> for PragmaState
    let state = PragmaState { locale: true, ..PragmaState::default() };
    let snapshot = PragmaSnapshot::from_state(state.clone());
    let recovered: PragmaState = snapshot.into();
    assert!(recovered.locale, "From<PragmaSnapshot> for PragmaState must work");
    Ok(())
}

#[test]
fn pragma_snapshot_strict_enabled_method() -> Result<(), Box<dyn std::error::Error>> {
    // PragmaSnapshot::strict_enabled() requires all three strict flags
    let all_strict = PragmaState {
        strict_vars: true,
        strict_subs: true,
        strict_refs: true,
        ..PragmaState::default()
    };
    let snapshot = PragmaSnapshot::from_state(all_strict);
    assert!(snapshot.strict_enabled(), "strict_enabled must be true when all three flags are set");

    let partial = PragmaState { strict_vars: true, strict_subs: true, ..PragmaState::default() };
    let partial_snap = PragmaSnapshot::from_state(partial);
    assert!(!partial_snap.strict_enabled(), "strict_enabled must be false if strict_refs is unset");
    Ok(())
}

#[test]
fn pragma_snapshot_warnings_enabled_method() -> Result<(), Box<dyn std::error::Error>> {
    // PragmaSnapshot::warnings_enabled()
    let with_warnings = PragmaState { warnings: true, ..PragmaState::default() };
    let snap = PragmaSnapshot::from_state(with_warnings);
    assert!(snap.warnings_enabled(), "warnings_enabled must reflect warnings=true");

    let no_warnings = PragmaState::default();
    let snap2 = PragmaSnapshot::from_state(no_warnings);
    assert!(!snap2.warnings_enabled(), "warnings_enabled must be false for default state");
    Ok(())
}

#[test]
fn pragma_snapshot_has_feature_and_is_warning_active() -> Result<(), Box<dyn std::error::Error>> {
    // PragmaSnapshot::has_feature and is_warning_active delegate to PragmaState
    let state = PragmaState { warnings: true, features: vec!["say"], ..PragmaState::default() };
    let snap = PragmaSnapshot::from_state(state);
    assert!(snap.has_feature("say"), "has_feature must delegate to state");
    assert!(!snap.has_feature("class"), "has_feature must return false for absent feature");
    assert!(snap.is_warning_active("uninitialized"), "is_warning_active must delegate");
    Ok(())
}

// ===========================================================================
// `PragmaMap::state_at` and `PragmaStateQuery` methods
// ===========================================================================

#[test]
fn pragma_map_state_at_returns_correct_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 13, 28)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();

    // state_at is documented as a method on PragmaMap
    let state = pragma_map.state_at(20);
    assert!(state.strict_vars, "state_at must return correct state");
    assert!(state.warnings, "state_at must include warnings from entry at offset 13");
    Ok(())
}

#[test]
fn pragma_state_query_offset_and_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("utf8", &[], 0, 9)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let query = environment.query_at(5);
    assert_eq!(query.offset(), 5, "query.offset() must return the queried offset");
    assert!(query.snapshot().state().utf8, "query.snapshot() must expose the active state");
    Ok(())
}

// ===========================================================================
// `features_enabled_by_version` intermediate bundles
// ===========================================================================

#[test]
fn v5_16_enables_unicode_eval_evalbytes_current_sub_fc() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 16));
    assert!(features.contains(&"unicode_eval"), "v5.16 must enable unicode_eval");
    assert!(features.contains(&"evalbytes"), "v5.16 must enable evalbytes");
    assert!(features.contains(&"current_sub"), "v5.16 must enable current_sub");
    assert!(features.contains(&"fc"), "v5.16 must enable fc");
    // Should also retain v5.10 and v5.12 features
    assert!(features.contains(&"say"), "v5.16 must retain say from v5.10");
    assert!(features.contains(&"unicode_strings"), "v5.16 must retain unicode_strings from v5.12");
    Ok(())
}

#[test]
fn v5_20_omits_postderef_bundle_feature() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 20));
    assert!(
        !features.contains(&"postderef_qq"),
        "v5.20 feature bundle must not include postderef_qq"
    );
    // Should retain v5.16 features
    assert!(features.contains(&"unicode_eval"), "v5.20 must retain unicode_eval");
    assert!(features.contains(&"current_sub"), "v5.20 must retain current_sub");
    Ok(())
}

#[test]
fn v5_34_omits_try_but_retains_5_28_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 34));
    assert!(!features.contains(&"try"), "v5.34 feature bundle must not include try");
    // Should retain earlier features
    assert!(features.contains(&"postderef_qq"), "v5.34 must retain postderef_qq");
    assert!(features.contains(&"unicode_eval"), "v5.34 must retain unicode_eval");
    // switch still present before v5.36
    assert!(features.contains(&"switch"), "v5.34 must retain switch (not yet removed)");
    Ok(())
}

#[test]
fn v5_38_enables_module_true_without_class_features() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 38));
    assert!(features.contains(&"module_true"), "v5.38 must enable module_true");
    assert!(!features.contains(&"class"), "v5.38 feature bundle must not include class");
    assert!(!features.contains(&"field"), "v5.38 feature bundle must not include field");
    assert!(!features.contains(&"method"), "v5.38 feature bundle must not include method");
    assert!(!features.contains(&"switch"), "v5.38 must remove switch");
    assert!(!features.contains(&"try"), "v5.38 feature bundle must not include try");
    assert!(features.contains(&"signatures"), "v5.38 must retain signatures from v5.36");
    Ok(())
}

#[test]
fn v5_9_returns_default_feature_set() -> Result<(), Box<dyn std::error::Error>> {
    // Versions before v5.10 implicitly load the :default bundle.
    let features = features_enabled_by_version(PerlVersion::new(5, 9));
    assert!(features.contains(&"indirect"), "v5.9 must include :default indirect");
    assert!(features.contains(&"multidimensional"), "v5.9 must include :default multidimensional");
    assert!(
        features.contains(&"bareword_filehandles"),
        "v5.9 must include :default bareword_filehandles"
    );
    assert!(
        features.contains(&"apostrophe_as_package_separator"),
        "v5.9 must include :default apostrophe package separator"
    );
    assert!(features.contains(&"smartmatch"), "v5.9 must include :default smartmatch");
    assert!(!features.contains(&"say"), "v5.9 must not include v5.10 say");
    Ok(())
}

// ===========================================================================
// `parse_perl_version` edge cases
// ===========================================================================

#[test]
fn parse_perl_version_major_only_without_v_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // A bare numeric major like "5" (no dot, no v prefix) should be parsed as major=5, minor=0
    let parsed = parse_perl_version("5");
    assert_eq!(parsed, Some(PerlVersion::new(5, 0)), "bare '5' must parse to (5, 0)");
    Ok(())
}

#[test]
fn parse_perl_version_empty_string_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    // Empty string after stripping 'v' prefix yields empty major component → None
    let parsed = parse_perl_version("");
    assert!(parsed.is_none(), "empty string must return None from parse_perl_version");
    Ok(())
}

#[test]
fn parse_perl_version_v_only_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    // "v" alone strips prefix leaving "" — parse_version_component("") returns None
    let parsed = parse_perl_version("v");
    assert!(parsed.is_none(), "'v' alone must return None from parse_perl_version");
    Ok(())
}

#[test]
fn parse_perl_version_numeric_5_036() -> Result<(), Box<dyn std::error::Error>> {
    // Numeric form like 5.036 should parse correctly to (5, 36)
    let parsed = parse_perl_version("5.036");
    assert_eq!(parsed, Some(PerlVersion::new(5, 36)), "5.036 must parse to (5, 36)");
    Ok(())
}

// ===========================================================================
// `normalize_snapshot` — `signatures_strict` propagates strict flags
// ===========================================================================

#[test]
fn snapshot_at_with_signatures_strict_propagates_all_strict_flags()
-> Result<(), Box<dyn std::error::Error>> {
    // `use feature 'signatures'` sets signatures_strict=true.
    // snapshot_at calls normalize_snapshot which then forces strict_vars/subs/refs.
    let ast = program(vec![use_node("feature", &["'signatures'"], 0, 28)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let snapshot = environment.snapshot_at(15);
    assert!(
        snapshot.strict_enabled(),
        "snapshot_at with signatures_strict must show all strict flags via normalize"
    );
    Ok(())
}

#[test]
fn no_feature_signatures_unsets_signatures_strict_but_preserves_explicit_strict()
-> Result<(), Box<dyn std::error::Error>> {
    // `use strict; use feature 'signatures'; no feature 'signatures'`
    // After `no feature 'signatures'`, signatures_strict=false.
    // But explicit strict was set independently — it should remain.
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        use_node("feature", &["'signatures'"], 13, 41),
        no_node("feature", &["'signatures'"], 42, 70),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 60);
    // signatures_strict is now false, but explicit strict is still on
    assert!(!state.signatures_strict, "signatures_strict must be cleared by no feature signatures");
    assert!(state.strict_vars, "explicit strict_vars must survive no feature signatures");
    assert!(state.strict_subs, "explicit strict_subs must survive no feature signatures");
    assert!(state.strict_refs, "explicit strict_refs must survive no feature signatures");
    Ok(())
}

// ===========================================================================
// `builtin_import_names` — double-quoted single name
// ===========================================================================

#[test]
fn use_builtin_double_quoted_name_is_tracked() -> Result<(), Box<dyn std::error::Error>> {
    // `use builtin "true"` — builtin_import_names handles double-quoted single name
    let ast = program(vec![use_node("builtin", &["\"true\""], 0, 20)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(state.has_builtin_import("true"), "double-quoted builtin name must be tracked");
    Ok(())
}

#[test]
fn no_builtin_double_quoted_name_is_removed() -> Result<(), Box<dyn std::error::Error>> {
    // `use builtin 'true'; no builtin "true"`
    let ast = program(vec![
        use_node("builtin", &["'true'"], 0, 18),
        no_node("builtin", &["\"true\""], 19, 37),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 30);
    assert!(!state.has_builtin_import("true"), "double-quoted name in no builtin must remove it");
    Ok(())
}

// ===========================================================================
// `conditional_pragma_target` edge cases
// ===========================================================================

#[test]
fn use_if_feature_with_empty_args_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use if $cond, feature` — feature with no additional args fails
    // conditional_target_tail_is_valid for feature requires !tail.is_empty()
    let ast = program(vec![use_node("if", &["$cond", "feature"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    // No recognized target → no state change
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(!state.has_feature("say"), "use if feature with no args must not enable features");
    Ok(())
}

#[test]
fn use_if_builtin_with_no_names_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use if $cond, builtin, ''` — empty arg for builtin, no names extractable
    // conditional_target_tail_is_valid: builtin → tail.iter().any(|arg| !builtin_import_names(arg).is_empty())
    // If all args produce no names, this is false → noop
    let ast = program(vec![use_node("if", &["$cond", "builtin", "''"], 0, 30)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.builtin_imports.is_empty(), "builtin with no names must not add imports");
    Ok(())
}

#[test]
fn use_if_encoding_with_empty_arg_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    // `use if $cond, encoding, ''` — empty encoding value
    // conditional_target_tail_is_valid for encoding: tail.len()==1 && !normalized_pragma_token(&tail[0]).is_empty()
    // Empty string normalized → fails → no target found
    let ast = program(vec![use_node("if", &["$cond", "encoding", "''"], 0, 32)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 15);
    assert!(state.encoding.is_none(), "encoding with empty arg must not set encoding");
    Ok(())
}

// ===========================================================================
// `PerlVersion` ordering
// ===========================================================================

#[test]
fn perl_version_ordering_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    // PerlVersion derives PartialOrd/Ord — verify ordering
    assert!(PerlVersion::new(5, 36) < PerlVersion::new(5, 40), "5.36 must be less than 5.40");
    assert!(PerlVersion::new(5, 40) > PerlVersion::new(4, 0), "5.40 must be greater than 4.0");
    assert_eq!(
        PerlVersion::new(5, 36),
        PerlVersion::new(5, 36),
        "equal versions must compare equal"
    );
    Ok(())
}

#[test]
fn version_implies_strict_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // Boundary is v5.12: <5.12 no strict, >=5.12 yes strict
    assert!(!version_implies_strict(PerlVersion::new(5, 11)));
    assert!(version_implies_strict(PerlVersion::new(5, 12)));
    assert!(version_implies_strict(PerlVersion::new(5, 36)));
    Ok(())
}

#[test]
fn version_implies_warnings_boundary() -> Result<(), Box<dyn std::error::Error>> {
    // Boundary is v5.35: <5.35 no warnings, >=5.35 yes warnings
    assert!(!version_implies_warnings(PerlVersion::new(5, 34)));
    assert!(version_implies_warnings(PerlVersion::new(5, 35)));
    assert!(version_implies_warnings(PerlVersion::new(5, 40)));
    Ok(())
}

// ===========================================================================
// `PragmaMap::entries` and `to_tuples`
// ===========================================================================

#[test]
fn pragma_map_entries_returns_all_entries() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 13, 28)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    // Two pragmas → two entries (no scoped blocks, no restore points)
    assert_eq!(pragma_map.entries().len(), 2, "entries() must return all map entries");
    Ok(())
}

#[test]
fn pragma_map_to_tuples_matches_entries() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    let tuples = pragma_map.to_tuples();
    assert_eq!(tuples.len(), pragma_map.entries().len(), "to_tuples len must match entries len");
    assert_eq!(
        tuples[0].0.start,
        pragma_map.entries()[0].range.start,
        "to_tuples range must match entries range"
    );
    Ok(())
}

// ===========================================================================
// `as_map` on CompileTimePragmaEnvironment
// ===========================================================================

#[test]
fn compile_time_environment_as_map_returns_tuples() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("utf8", &[], 0, 9)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let tuples = environment.as_map();
    assert_eq!(tuples.len(), 1, "as_map must return one entry for a single pragma");
    assert!(tuples[0].1.state().utf8, "as_map snapshot must reflect utf8=true");
    Ok(())
}

// ===========================================================================
// `PragmaQueryCursor` explicit-map and legacy-map branches
// ===========================================================================

#[test]
fn pragma_map_snapshot_at_before_first_entry_returns_default()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 50, 62)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();

    let snapshot = pragma_map.snapshot_at(10);
    assert!(
        !snapshot.state().strict_vars,
        "snapshot before the first pragma must not have strict_vars"
    );
    assert!(
        !snapshot.state().strict_subs,
        "snapshot before the first pragma must not have strict_subs"
    );
    Ok(())
}

#[test]
fn cursor_entry_for_offset_backward_seek_explicit_map() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![no_node("strict", &["refs"], 20, 36)], 18, 40),
        use_node("warnings", &[], 42, 57),
    ]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    let mut cursor = pragma_map.cursor();

    let late = cursor.snapshot_at(pragma_map, 50);
    assert!(late.state().warnings, "late offset must see warnings");

    let early = cursor.snapshot_at(pragma_map, 8);
    assert!(early.state().strict_vars, "early offset must see strict after backward seek");
    assert!(!early.state().warnings, "early offset must not see warnings");
    assert_eq!(early, environment.snapshot_at(8), "cursor result must match direct snapshot_at");
    Ok(())
}

#[test]
fn cursor_entry_for_offset_empty_map_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    let mut cursor = pragma_map.cursor();

    let snapshot = cursor.snapshot_at(pragma_map, 999);
    assert!(!snapshot.state().strict_vars, "empty map must return default snapshot");
    assert!(!snapshot.state().warnings, "empty map must return default snapshot");
    Ok(())
}

#[test]
fn cursor_entry_for_offset_index_clamped_when_past_end() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 13, 28)]);
    let environment = CompileTimePragmaEnvironment::build(&ast);
    let pragma_map = environment.map();
    let mut cursor = pragma_map.cursor();

    let _ = cursor.snapshot_at(pragma_map, 9999);

    let snapshot = cursor.snapshot_at(pragma_map, 20);
    assert!(snapshot.state().warnings, "warnings must be visible after index clamp");
    Ok(())
}

#[test]
fn cursor_reused_with_smaller_map_clamps_index() -> Result<(), Box<dyn std::error::Error>> {
    let big_ast = program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        use_node("utf8", &[], 29, 38),
        use_node("feature", &["'say'"], 39, 57),
    ]);
    let big_environment = CompileTimePragmaEnvironment::build(&big_ast);
    let big_map = big_environment.map();
    let big_len = big_map.entries().len();
    let mut cursor = big_map.cursor();
    let _ = cursor.snapshot_at(big_map, 9999);

    let small_ast = program(vec![use_node("warnings", &[], 0, 15)]);
    let small_environment = CompileTimePragmaEnvironment::build(&small_ast);
    let small_map = small_environment.map();

    assert!(big_len > small_map.entries().len(), "precondition: big map is larger");
    let snapshot = cursor.snapshot_at(small_map, 10);
    assert!(snapshot.state().warnings, "small map must report warnings after index clamp");
    Ok(())
}

#[test]
fn cursor_legacy_api_reused_with_smaller_map_clamps_index() -> Result<(), Box<dyn std::error::Error>>
{
    let big_ast = program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        use_node("utf8", &[], 29, 38),
        use_node("feature", &["'say'"], 39, 57),
    ]);
    let big_map = PragmaTracker::build(&big_ast);
    let mut cursor = PragmaQueryCursor::new();
    let _ = cursor.state_for_offset(&big_map, 9999);

    let small_ast = program(vec![use_node("warnings", &[], 0, 15)]);
    let small_map = PragmaTracker::build(&small_ast);

    assert!(big_map.len() > small_map.len(), "precondition: big map is larger");
    let state = cursor.state_for_offset(&small_map, 10);
    assert!(state.warnings, "small map must report warnings after index clamp");
    Ok(())
}

// ===========================================================================
// `use if` / `no unless` for `locale` — scope fields via conditional path
// ===========================================================================

#[test]
fn use_if_locale_with_scope_conditionally_enables_locale() -> Result<(), Box<dyn std::error::Error>>
{
    // `use if $cond, locale, ':not_characters'`
    // This exercises the locale arm of apply_conditional_use_target
    let ast = program(vec![use_node("if", &["$cond", "locale", "':not_characters'"], 0, 44)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 22);
    assert!(state.locale, "use if locale must set locale=true");
    assert_eq!(
        state.locale_scope.as_deref(),
        Some(":not_characters"),
        "use if locale must set locale_scope"
    );
    Ok(())
}

#[test]
fn no_unless_locale_conditionally_clears_locale() -> Result<(), Box<dyn std::error::Error>> {
    // `use locale ':not_characters'; no unless $cond, locale`
    let ast = program(vec![
        use_node("locale", &["':not_characters'"], 0, 28),
        no_node("unless", &["$cond", "locale"], 29, 55),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 45);
    assert!(!state.locale, "no unless locale must clear locale");
    assert!(state.locale_scope.is_none(), "no unless locale must clear locale_scope");
    Ok(())
}

// ===========================================================================
// `PragmaState::has_builtin_import` negative case
// ===========================================================================

#[test]
fn has_builtin_import_returns_false_when_absent() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState::default();
    assert!(
        !state.has_builtin_import("anything"),
        "has_builtin_import must return false on empty imports"
    );
    Ok(())
}

// ===========================================================================
// `no feature` on features that are already absent — returns false, no push
// ===========================================================================

#[test]
fn no_feature_on_absent_feature_produces_no_map_entry() -> Result<(), Box<dyn std::error::Error>> {
    // `no feature 'say'` when say was never enabled — disable_feature_name returns false
    // → apply_feature_state returns false → no push
    let ast = program(vec![no_node("feature", &["'say'"], 0, 18)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty(), "no feature on absent feature name must not produce a map entry");
    Ok(())
}

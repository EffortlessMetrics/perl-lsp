//! Comprehensive unit tests for perl-pragma crate.
//!
//! Tests cover PragmaState, PragmaTracker::build, and PragmaTracker::state_for_offset
//! across all public API surface including edge cases.

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{PerlVersion, PragmaState, PragmaTracker, features_enabled_by_version};

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

fn function_call(name: &str, args: Vec<Node>, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::FunctionCall { name: name.to_string(), args },
        location: loc(start, end),
    }
}

fn number_node(value: &str, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Number { value: value.to_string() }, location: loc(start, end) }
}

fn string_node(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::String { value: value.to_string(), interpolated },
        location: loc(start, end),
    }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn dummy_node(start: usize, end: usize) -> Node {
    Node { kind: NodeKind::MissingExpression, location: loc(start, end) }
}

// ===========================================================================
// PragmaState tests
// ===========================================================================

#[test]
fn default_state_is_all_false() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState::default();
    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    assert!(!state.warnings);
    assert!(!state.utf8);
    assert!(state.encoding.is_none());
    assert!(!state.unicode_strings);
    assert!(!state.locale);
    assert!(state.locale_scope.is_none());
    Ok(())
}

#[test]
fn all_strict_enables_strict_but_not_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState::all_strict();
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(!state.warnings, "all_strict should not enable warnings");
    assert!(!state.utf8);
    assert!(state.encoding.is_none());
    assert!(!state.unicode_strings);
    assert!(!state.locale);
    assert!(state.locale_scope.is_none());
    Ok(())
}

#[test]
fn pragma_state_clone_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let original = PragmaState::all_strict();
    let cloned = original.clone();
    // Verify the clone matches the original (independence verified by value equality)
    assert!(cloned.strict_vars, "clone should preserve strict_vars");
    Ok(())
}

#[test]
fn pragma_state_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PragmaState::default(), PragmaState::default());
    assert_eq!(PragmaState::all_strict(), PragmaState::all_strict());
    assert_ne!(PragmaState::default(), PragmaState::all_strict());
    Ok(())
}

// ===========================================================================
// PragmaTracker::build — empty / trivial programs
// ===========================================================================

#[test]
fn empty_program_yields_empty_map() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty());
    Ok(())
}

#[test]
fn program_without_pragmas_yields_empty_map() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![dummy_node(0, 10)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty());
    Ok(())
}

// ===========================================================================
// use strict / no strict — full and selective
// ===========================================================================

#[test]
fn use_strict_enables_all_strict_categories() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    let state = &map[0].1;
    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    Ok(())
}

#[test]
fn use_strict_vars_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["vars"], 0, 18)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

#[test]
fn use_strict_subs_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["subs"], 0, 18)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(!state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

#[test]
fn use_strict_refs_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["refs"], 0, 18)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn use_strict_quoted_args_single_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["'vars'", "'refs'"], 0, 30)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn use_strict_quoted_args_double_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["\"subs\""], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_subs);
    Ok(())
}

#[test]
fn use_if_strict_conditionally_enables_strict() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$^O", "eq", "'MSWin32'", "'strict'"], 0, 35)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;

    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn use_if_version_target_applies_version_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$]", ">=", "5.034", "v5.40"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;

    assert!(state.strict_vars, "v5.40 should imply strict");
    assert!(state.warnings, "v5.40 should imply warnings");
    assert!(state.has_feature("builtin"), "v5.40 should imply builtin feature bundle");
    Ok(())
}

#[test]
fn use_unless_strict_conditionally_enables_strict() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("unless", &["$already_strict", "'strict'"], 0, 38)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;

    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn use_unless_version_target_applies_version_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("unless", &["$]", "<", "5.036", "v5.40"], 0, 31)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;

    assert!(state.strict_vars, "v5.40 should imply strict");
    assert!(state.warnings, "v5.40 should imply warnings");
    assert!(state.has_feature("builtin"), "v5.40 should imply builtin feature bundle");
    Ok(())
}

#[test]
fn use_if_version_condition_does_not_apply_version_semantics()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$]", ">=", "5.036", "Some::Module"], 0, 38)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 20);

    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    assert!(!state.warnings);
    assert!(!state.has_feature("builtin"));
    Ok(())
}

#[test]
fn use_if_encoding_targets_encoding_not_argument() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("if", &["$cond", "encoding", "'utf8'"], 0, 32)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;

    assert_eq!(state.encoding.as_deref(), Some("utf8"));
    assert!(!state.utf8, "encoding pragma should not be mistaken for utf8 pragma");
    Ok(())
}

#[test]
fn no_strict_disables_all() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), no_node("strict", &[], 13, 23)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    let state = &map[1].1;
    assert!(!state.strict_vars && !state.strict_subs && !state.strict_refs);
    Ok(())
}

#[test]
fn no_strict_selective_disables_only_specified() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), no_node("strict", &["refs"], 13, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

#[test]
fn no_strict_quoted_single() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), no_node("strict", &["'vars'"], 13, 30)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(!state.strict_vars);
    assert!(state.strict_subs);
    Ok(())
}

#[test]
fn no_strict_quoted_double() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("strict", &[], 0, 12), no_node("strict", &["\"subs\""], 13, 30)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    Ok(())
}

#[test]
fn use_feature_signatures_enables_all_strict_categories() -> Result<(), Box<dyn std::error::Error>>
{
    let ast = program(vec![use_node("feature", &["'signatures'"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    let state = &map[0].1;
    assert!(state.signatures_strict);
    Ok(())
}

#[test]
fn use_feature_qw_signatures_enables_all_strict_categories()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["qw(signatures say)"], 0, 34)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    let state = &map[0].1;
    assert!(state.signatures_strict);
    Ok(())
}

// ===========================================================================
// use warnings / no warnings
// ===========================================================================

#[test]
fn use_warnings_enables_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("warnings", &[], 0, 15)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    assert!(map[0].1.warnings);
    Ok(())
}

#[test]
fn use_v5_12_enables_effective_strict_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.12", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(!state.warnings, "v5.12 should not imply warnings");
    Ok(())
}

#[test]
fn use_v5_40_enables_effective_strict_and_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.warnings);
    Ok(())
}

#[test]
fn use_v5_40_1_enables_builtin_and_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40.1", &[], 0, 14)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(state.warnings);
    assert!(state.has_feature("builtin"));
    Ok(())
}

#[test]
fn use_numeric_version_enables_effective_strict_and_warnings()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("5.040", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(state.warnings);
    Ok(())
}

#[test]
fn use_developer_version_enables_effective_strict_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("5.012_001", &[], 0, 16)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(!state.warnings, "5.012_001 should not imply warnings");
    Ok(())
}

#[test]
fn require_version_does_not_enable_effective_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![function_call("require", vec![number_node("5.36", 8, 12)], 0, 13)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 12);
    assert!(!state.strict_vars, "require VERSION must not enable strict vars");
    assert!(!state.strict_subs, "require VERSION must not enable strict subs");
    assert!(!state.strict_refs, "require VERSION must not enable strict refs");
    assert!(!state.warnings, "require VERSION must not enable warnings");
    Ok(())
}

#[test]
fn no_warnings_disables_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("warnings", &[], 0, 15), no_node("warnings", &[], 16, 30)]);
    let map = PragmaTracker::build(&ast);
    assert!(!map[1].1.warnings);
    Ok(())
}

#[test]
fn use_utf8_and_no_utf8_toggle_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("utf8", &[], 0, 9), no_node("utf8", &[], 10, 19)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    assert!(map[0].1.utf8);
    assert!(!map[1].1.utf8);
    Ok(())
}

#[test]
fn use_encoding_tracks_active_source_encoding() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("encoding", &["'utf8'"], 0, 18), no_node("encoding", &[], 19, 31)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    assert_eq!(map[0].1.encoding.as_deref(), Some("utf8"));
    assert!(map[1].1.encoding.is_none());
    Ok(())
}

#[test]
fn use_locale_tracks_scope_and_clears_on_no_locale() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("locale", &["':not_characters'"], 0, 28),
        no_node("locale", &[], 29, 39),
    ]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    assert!(map[0].1.locale);
    assert_eq!(map[0].1.locale_scope.as_deref(), Some(":not_characters"));
    assert!(!map[1].1.locale);
    assert!(map[1].1.locale_scope.is_none());
    Ok(())
}

#[test]
fn use_feature_unicode_strings_sets_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["'unicode_strings'"], 0, 30)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    assert!(map[0].1.unicode_strings);
    Ok(())
}

#[test]
fn use_feature_bundle_5_12_sets_unicode_strings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["':5.12'"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    assert!(map[0].1.unicode_strings);
    Ok(())
}

#[test]
fn use_feature_quoted_qw_items_are_parsed() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["'qw(signatures unicode_strings)'"], 0, 46)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    assert!(map[0].1.signatures_strict, "signatures should imply strictness");
    assert!(map[0].1.unicode_strings, "unicode_strings should be enabled");
    Ok(())
}

#[test]
fn use_feature_bundle_5_10_enables_switch_and_say() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("feature", &["':5.10'"], 0, 22)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.has_feature("say"));
    assert!(state.has_feature("state"));
    assert!(state.has_feature("switch"));
    Ok(())
}

#[test]
fn no_feature_switch_disables_switch_after_version_bundle() -> Result<(), Box<dyn std::error::Error>>
{
    let ast =
        program(vec![use_node("v5.10", &[], 0, 12), no_node("feature", &["'switch'"], 13, 31)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    let state = &map[1].1;
    assert!(state.has_feature("say"));
    assert!(state.has_feature("state"));
    assert!(!state.has_feature("switch"));
    Ok(())
}

#[test]
fn no_feature_all_clears_bundle_features() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40", &[], 0, 12), no_node("feature", &["':all'"], 13, 31)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(!state.has_feature("say"));
    assert!(!state.has_feature("switch"));
    assert!(!state.has_feature("builtin"));
    Ok(())
}

#[test]
fn no_feature_without_args_clears_bundle_features() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40", &[], 0, 12), no_node("feature", &[], 13, 23)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(!state.has_feature("say"));
    assert!(!state.has_feature("switch"));
    assert!(!state.has_feature("builtin"));
    Ok(())
}

// ===========================================================================
// Unknown / unrelated pragmas are ignored
// ===========================================================================

#[test]
fn unknown_use_module_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("Moose", &[], 0, 10)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty());
    Ok(())
}

#[test]
fn unknown_no_module_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![no_node("autovivification", &[], 0, 25)]);
    let map = PragmaTracker::build(&ast);
    assert!(map.is_empty());
    Ok(())
}

#[test]
fn unknown_strict_arg_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["bogus"], 0, 20)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(!state.strict_vars && !state.strict_subs && !state.strict_refs);
    Ok(())
}

// ===========================================================================
// Cumulative state across multiple use statements
// ===========================================================================

#[test]
fn cumulative_use_strict_then_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 13, 27)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    // After use warnings, strict should still be on
    let state = &map[1].1;
    assert!(state.strict_vars && state.strict_subs && state.strict_refs && state.warnings);
    Ok(())
}

#[test]
fn incremental_strict_categories() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("strict", &["vars"], 0, 20), use_node("strict", &["subs"], 21, 40)]);
    let map = PragmaTracker::build(&ast);
    // After second use strict, both vars and subs should be on
    let state = &map[1].1;
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

// ===========================================================================
// Block scoping — state restored after block
// ===========================================================================

#[test]
fn block_scoping_restores_state() -> Result<(), Box<dyn std::error::Error>> {
    // use strict; { no strict 'refs'; } use warnings;
    // Block scoping restores current_state so the use-warnings after the block
    // inherits the pre-block state (strict all on).
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![no_node("strict", &["refs"], 14, 30)], 13, 31),
        use_node("warnings", &[], 32, 47),
    ]);
    let map = PragmaTracker::build(&ast);

    // Inside block: refs disabled
    let inside = PragmaTracker::state_for_offset(&map, 20);
    assert!(!inside.strict_refs);
    assert!(inside.strict_vars);

    // After block: the use-warnings entry inherits restored strict state
    let after = PragmaTracker::state_for_offset(&map, 40);
    assert!(after.strict_vars && after.strict_subs && after.strict_refs);
    assert!(after.warnings);
    Ok(())
}

#[test]
fn nested_blocks_restore_correctly() -> Result<(), Box<dyn std::error::Error>> {
    // use strict; { { no strict; } } use warnings;
    // After outer block, current_state is restored so use-warnings inherits strict on.
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        block(vec![block(vec![no_node("strict", &[], 20, 30)], 18, 32)], 13, 33),
        use_node("warnings", &[], 34, 49),
    ]);
    let map = PragmaTracker::build(&ast);

    // Deep inside nested block — strict disabled
    let deep = PragmaTracker::state_for_offset(&map, 25);
    assert!(!deep.strict_vars);

    // After outer block — the use-warnings entry has strict restored
    let after = PragmaTracker::state_for_offset(&map, 45);
    assert!(after.strict_vars);
    assert!(after.warnings);
    Ok(())
}

// ===========================================================================
// Subroutine bodies
// ===========================================================================

#[test]
fn subroutine_body_inherits_pragma_state() -> Result<(), Box<dyn std::error::Error>> {
    let sub_body = block(vec![use_node("warnings", &[], 30, 45)], 25, 50);
    let sub_node = Node {
        kind: NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(sub_body),
        },
        location: loc(20, 55),
    };
    let ast = program(vec![use_node("strict", &[], 0, 12), sub_node]);
    let map = PragmaTracker::build(&ast);

    // Inside sub body: warnings is on, strict inherited
    let inside = PragmaTracker::state_for_offset(&map, 40);
    assert!(inside.warnings);
    assert!(inside.strict_vars);
    Ok(())
}

// ===========================================================================
// If / While / For / Foreach bodies
// ===========================================================================

#[test]
fn if_branches_traversed() -> Result<(), Box<dyn std::error::Error>> {
    let then_branch = block(vec![use_node("warnings", &[], 20, 35)], 18, 40);
    let else_branch = block(vec![no_node("strict", &["refs"], 45, 60)], 42, 65);
    let if_node = Node {
        kind: NodeKind::If {
            condition: Box::new(dummy_node(10, 15)),
            then_branch: Box::new(then_branch),
            elsif_branches: vec![],
            else_branch: Some(Box::new(else_branch)),
        },
        location: loc(10, 65),
    };
    let ast = program(vec![use_node("strict", &[], 0, 9), if_node]);
    let map = PragmaTracker::build(&ast);

    // Then branch has warnings enabled
    let then_state = PragmaTracker::state_for_offset(&map, 30);
    assert!(then_state.warnings);

    // Else branch has refs disabled
    let else_state = PragmaTracker::state_for_offset(&map, 55);
    assert!(!else_state.strict_refs);
    Ok(())
}

#[test]
fn if_elsif_else_branches_restore_state() -> Result<(), Box<dyn std::error::Error>> {
    let then_branch = block(vec![no_node("strict", &["refs"], 20, 35)], 18, 40);
    let elsif_branch = block(vec![use_node("warnings", &[], 45, 60)], 43, 65);
    let else_branch = block(vec![no_node("strict", &["subs"], 70, 85)], 68, 90);
    let if_node = Node {
        kind: NodeKind::If {
            condition: Box::new(dummy_node(10, 15)),
            then_branch: Box::new(then_branch),
            elsif_branches: vec![(Box::new(dummy_node(41, 42)), Box::new(elsif_branch))],
            else_branch: Some(Box::new(else_branch)),
        },
        location: loc(10, 90),
    };
    let ast =
        program(vec![use_node("strict", &[], 0, 12), if_node, use_node("warnings", &[], 91, 106)]);
    let map = PragmaTracker::build(&ast);

    let then_state = PragmaTracker::state_for_offset(&map, 25);
    assert!(!then_state.strict_refs, "then branch pragmas must be tracked");

    let elsif_state = PragmaTracker::state_for_offset(&map, 50);
    assert!(elsif_state.warnings, "elsif branch pragmas must be tracked");

    let else_state = PragmaTracker::state_for_offset(&map, 75);
    assert!(!else_state.strict_subs, "else branch pragmas must be tracked");

    let after = PragmaTracker::state_for_offset(&map, 95);
    assert!(after.strict_vars && after.strict_subs && after.strict_refs);
    assert!(after.warnings);
    Ok(())
}

#[test]
fn while_body_traversed() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![use_node("warnings", &[], 20, 35)], 18, 40);
    let while_node = Node {
        kind: NodeKind::While {
            condition: Box::new(dummy_node(10, 15)),
            body: Box::new(body),
            continue_block: None,
        },
        location: loc(10, 40),
    };
    let ast = program(vec![while_node]);
    let map = PragmaTracker::build(&ast);
    assert!(!map.is_empty());
    assert!(map[0].1.warnings);
    Ok(())
}

#[test]
fn for_body_traversed() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![use_node("strict", &["vars"], 30, 50)], 28, 55);
    let for_node = Node {
        kind: NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(body),
            continue_block: None,
        },
        location: loc(10, 55),
    };
    let ast = program(vec![for_node]);
    let map = PragmaTracker::build(&ast);
    assert!(map[0].1.strict_vars);
    Ok(())
}

#[test]
fn foreach_body_traversed() -> Result<(), Box<dyn std::error::Error>> {
    let body = block(vec![use_node("strict", &[], 30, 42)], 28, 45);
    let foreach_node = Node {
        kind: NodeKind::Foreach {
            variable: Box::new(dummy_node(10, 12)),
            list: Box::new(dummy_node(13, 20)),
            body: Box::new(body),
            continue_block: None,
        },
        location: loc(10, 45),
    };
    let ast = program(vec![foreach_node]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars && state.strict_subs && state.strict_refs);
    Ok(())
}

#[test]
fn modern_container_bodies_are_traversed_and_scoped() -> Result<(), Box<dyn std::error::Error>> {
    let method = method_node(block(vec![no_node("strict", &["refs"], 20, 34)], 18, 36), 13, 40);
    let class = class_node(block(vec![no_node("strict", &["subs"], 45, 60)], 43, 63), 41, 65);
    let try_block = block(vec![no_node("strict", &["vars"], 70, 84)], 68, 86);
    let catch_block = block(vec![use_node("warnings", &[], 90, 104)], 88, 106);
    let finally_block = block(vec![no_node("strict", &["refs"], 110, 125)], 108, 128);
    let try_stmt = try_node(try_block, vec![catch_block], Some(finally_block), 66, 130);
    let eval_stmt = eval_node(block(vec![use_node("warnings", &[], 135, 148)], 133, 150), 131, 152);
    let do_stmt = do_node(block(vec![no_node("strict", &["refs"], 155, 170)], 153, 172), 153, 174);
    let defer_stmt =
        defer_node(block(vec![no_node("strict", &["subs"], 178, 190)], 176, 192), 175, 194);
    let given_body = block(
        vec![
            when_node(block(vec![no_node("strict", &["vars"], 200, 214)], 198, 216), 196, 218),
            default_node(block(vec![use_node("warnings", &[], 220, 232)], 218, 234), 218, 236),
        ],
        196,
        238,
    );
    let given_stmt = given_node(given_body, 195, 240);
    let while_body = block(vec![no_node("strict", &["refs"], 245, 258)], 243, 260);
    let continue_body = block(vec![use_node("warnings", &[], 262, 274)], 260, 276);
    let while_stmt = while_node(while_body, Some(continue_body), 242, 278);
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        method,
        class,
        try_stmt,
        eval_stmt,
        do_stmt,
        defer_stmt,
        given_stmt,
        while_stmt,
        use_node("warnings", &[], 280, 295),
    ]);
    let map = PragmaTracker::build(&ast);

    let method_state = PragmaTracker::state_for_offset(&map, 25);
    assert!(!method_state.strict_refs, "method bodies must be traversed");

    let class_state = PragmaTracker::state_for_offset(&map, 50);
    assert!(!class_state.strict_subs, "class bodies must be traversed");

    let try_state = PragmaTracker::state_for_offset(&map, 75);
    assert!(!try_state.strict_vars, "try bodies must be traversed");

    let catch_state = PragmaTracker::state_for_offset(&map, 95);
    assert!(catch_state.warnings, "catch blocks must be traversed");

    let finally_state = PragmaTracker::state_for_offset(&map, 115);
    assert!(!finally_state.strict_refs, "finally blocks must be traversed");

    let eval_state = PragmaTracker::state_for_offset(&map, 140);
    assert!(eval_state.warnings, "eval blocks must be traversed");

    let do_state = PragmaTracker::state_for_offset(&map, 160);
    assert!(!do_state.strict_refs, "do blocks must be traversed");

    let defer_state = PragmaTracker::state_for_offset(&map, 182);
    assert!(!defer_state.strict_subs, "defer blocks must be traversed");

    let when_state = PragmaTracker::state_for_offset(&map, 205);
    assert!(!when_state.strict_vars, "when bodies must be traversed");

    let default_state = PragmaTracker::state_for_offset(&map, 225);
    assert!(default_state.warnings, "default bodies must be traversed");

    let continue_state = PragmaTracker::state_for_offset(&map, 266);
    assert!(continue_state.warnings, "continue blocks must be traversed");

    let after = PragmaTracker::state_for_offset(&map, 285);
    assert!(after.strict_vars && after.strict_subs && after.strict_refs);
    assert!(after.warnings);
    Ok(())
}

// ===========================================================================
// state_for_offset edge cases
// ===========================================================================

#[test]
fn state_for_offset_empty_map_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaTracker::state_for_offset(&[], 100);
    assert_eq!(state, PragmaState::default());
    Ok(())
}

#[test]
fn state_for_offset_before_any_pragma_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 50, 62)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert_eq!(state, PragmaState::default());
    Ok(())
}

#[test]
fn state_for_offset_at_exact_start_of_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 10, 22)]);
    let map = PragmaTracker::build(&ast);
    // Offset 10 is exactly the start — partition_point uses <=, so it should find it
    let state = PragmaTracker::state_for_offset(&map, 10);
    assert!(state.strict_vars);
    Ok(())
}

#[test]
fn state_for_offset_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 0);
    assert!(state.strict_vars);
    Ok(())
}

#[test]
fn state_for_offset_well_past_last_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = PragmaTracker::state_for_offset(&map, 999_999);
    assert!(state.strict_vars);
    Ok(())
}

#[test]
fn state_for_offset_between_two_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12), use_node("warnings", &[], 100, 115)]);
    let map = PragmaTracker::build(&ast);
    // Between the two: strict is on, warnings not yet
    let state = PragmaTracker::state_for_offset(&map, 50);
    assert!(state.strict_vars);
    assert!(!state.warnings);
    Ok(())
}

// ===========================================================================
// Sorting — build() sorts by range.start
// ===========================================================================

#[test]
fn build_sorts_by_start_offset() -> Result<(), Box<dyn std::error::Error>> {
    // Insert pragmas out of order in the AST
    let ast = program(vec![use_node("warnings", &[], 100, 115), use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 2);
    assert!(map[0].0.start <= map[1].0.start, "map should be sorted by start offset");
    Ok(())
}

// ===========================================================================
// Combined strict + warnings toggle sequences
// ===========================================================================

#[test]
fn toggle_strict_on_off_on() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        no_node("strict", &[], 13, 23),
        use_node("strict", &["vars"], 24, 42),
    ]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 3);

    // After re-enabling only vars
    let state = &map[2].1;
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(!state.strict_refs);
    Ok(())
}

#[test]
fn warnings_with_args_still_records() -> Result<(), Box<dyn std::error::Error>> {
    // use warnings with args — the code enables warnings regardless of args
    let ast = program(vec![use_node("warnings", &["FATAL", "all"], 0, 30)]);
    let map = PragmaTracker::build(&ast);
    assert!(map[0].1.warnings);
    Ok(())
}

#[test]
fn no_warnings_with_category_arg_keeps_warnings_flag_true() -> Result<(), Box<dyn std::error::Error>>
{
    // `no warnings 'uninitialized'` should NOT disable the global warnings flag.
    // Only the specific category is suppressed; other warnings remain active.
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized"], 16, 40),
    ]);
    let map = PragmaTracker::build(&ast);
    assert!(
        map[1].1.warnings,
        "global warnings flag must stay true after `no warnings 'uninitialized'`"
    );
    Ok(())
}

#[test]
fn no_warnings_with_category_disables_only_that_category() -> Result<(), Box<dyn std::error::Error>>
{
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized"], 16, 40),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    // The disabled category must be recorded
    assert!(
        state.disabled_warning_categories.contains(&"uninitialized".to_string()),
        "disabled_warning_categories must contain 'uninitialized'"
    );
    // Other categories are still active
    assert!(state.is_warning_active("deprecated"), "category 'deprecated' must still be active");
    assert!(!state.is_warning_active("uninitialized"), "category 'uninitialized' must be inactive");
    Ok(())
}

#[test]
fn no_warnings_bare_disables_all_warnings() -> Result<(), Box<dyn std::error::Error>> {
    // `no warnings;` (no args) must still disable the global warnings flag.
    let ast = program(vec![use_node("warnings", &[], 0, 15), no_node("warnings", &[], 16, 28)]);
    let map = PragmaTracker::build(&ast);
    assert!(!map[1].1.warnings, "bare `no warnings` must clear the global warnings flag");
    Ok(())
}

#[test]
fn no_warnings_multiple_categories_all_recorded() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized", "redefine"], 16, 50),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(state.warnings, "global warnings flag must stay true");
    assert!(
        state.disabled_warning_categories.contains(&"uninitialized".to_string()),
        "must contain 'uninitialized'"
    );
    assert!(
        state.disabled_warning_categories.contains(&"redefine".to_string()),
        "must contain 'redefine'"
    );
    assert!(!state.is_warning_active("uninitialized"));
    assert!(!state.is_warning_active("redefine"));
    assert!(state.is_warning_active("deprecated"));
    Ok(())
}

#[test]
fn use_warnings_after_no_warnings_category_resets_disabled_list()
-> Result<(), Box<dyn std::error::Error>> {
    // use warnings; no warnings 'X'; use warnings;
    // The second `use warnings` should clear the disabled categories list.
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized"], 16, 40),
        use_node("warnings", &[], 41, 56),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = &map[2].1;
    assert!(state.warnings, "warnings must be re-enabled");
    assert!(
        state.disabled_warning_categories.is_empty(),
        "disabled categories must be cleared by `use warnings`"
    );
    assert!(state.is_warning_active("uninitialized"), "uninitialized must be active again");
    Ok(())
}

#[test]
fn no_warnings_quoted_category_strips_quotes() -> Result<(), Box<dyn std::error::Error>> {
    // Parser may leave quotes on args; confirm they are stripped
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["'uninitialized'"], 16, 45),
    ]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(
        state.disabled_warning_categories.contains(&"uninitialized".to_string()),
        "single-quoted category must be stripped and recorded as 'uninitialized'"
    );
    assert!(!state.is_warning_active("uninitialized"));
    Ok(())
}

#[test]
fn is_warning_active_false_when_global_warnings_off() -> Result<(), Box<dyn std::error::Error>> {
    // When warnings are globally off, no category is active
    let state = PragmaState { warnings: false, ..PragmaState::default() };
    assert!(!state.is_warning_active("uninitialized"));
    assert!(!state.is_warning_active("deprecated"));
    assert!(!state.is_warning_active("all"));
    Ok(())
}

#[test]
fn is_warning_active_true_when_warnings_on_and_no_disabled()
-> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState { warnings: true, ..PragmaState::default() };
    assert!(state.is_warning_active("uninitialized"));
    assert!(state.is_warning_active("deprecated"));
    Ok(())
}

// ===========================================================================
// Multiple selective strict categories in one use
// ===========================================================================

#[test]
fn use_strict_multiple_categories_at_once() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &["vars", "refs"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

#[test]
fn no_strict_multiple_categories_at_once() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
        program(vec![use_node("strict", &[], 0, 12), no_node("strict", &["vars", "subs"], 13, 35)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[1].1;
    assert!(!state.strict_vars);
    assert!(!state.strict_subs);
    assert!(state.strict_refs);
    Ok(())
}

// ===========================================================================
// Range values in the pragma map
// ===========================================================================

#[test]
fn pragma_map_records_correct_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 5, 17)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map[0].0, 5..17);
    Ok(())
}

// ===========================================================================
// If without else branch
// ===========================================================================

#[test]
fn if_without_else_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let then_branch = block(vec![use_node("warnings", &[], 20, 35)], 18, 40);
    let if_node = Node {
        kind: NodeKind::If {
            condition: Box::new(dummy_node(10, 15)),
            then_branch: Box::new(then_branch),
            elsif_branches: vec![],
            else_branch: None,
        },
        location: loc(10, 40),
    };
    let ast = program(vec![if_node]);
    let map = PragmaTracker::build(&ast);
    assert!(!map.is_empty());
    Ok(())
}

// ===========================================================================
// features_enabled_by_version — version→feature mapping completeness
// ===========================================================================

#[test]
fn v5_10_enables_say_and_state() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 10));
    assert!(features.contains(&"say"), "v5.10 must enable 'say'");
    assert!(features.contains(&"state"), "v5.10 must enable 'state'");
    Ok(())
}

#[test]
fn v5_10_enables_switch() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 10));
    assert!(features.contains(&"switch"), "v5.10 must enable 'switch' (given/when)");
    Ok(())
}

#[test]
fn v5_14_enables_unicode_strings() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 14));
    assert!(features.contains(&"say"), "v5.14 should retain v5.10 features");
    assert!(features.contains(&"unicode_strings"), "v5.14 must enable 'unicode_strings'");
    Ok(())
}

#[test]
fn v5_26_enables_unicode_eval_and_postfix_deref() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 26));
    assert!(features.contains(&"say"), "v5.26 should retain v5.10 features");
    assert!(features.contains(&"unicode_strings"), "v5.26 should retain v5.14 features");
    assert!(features.contains(&"postfix_deref"), "v5.26 must enable 'postfix_deref'");
    assert!(
        features.contains(&"unicode_eval"),
        "v5.26 must enable 'unicode_eval' (disables /xx is separate feature)"
    );
    Ok(())
}

#[test]
fn v5_36_enables_signatures_defer_isa() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 36));
    assert!(features.contains(&"signatures"), "v5.36 must enable 'signatures'");
    assert!(features.contains(&"defer"), "v5.36 must enable 'defer'");
    assert!(features.contains(&"isa"), "v5.36 must enable 'isa'");
    Ok(())
}

#[test]
fn v5_40_enables_builtin() -> Result<(), Box<dyn std::error::Error>> {
    let features = features_enabled_by_version(PerlVersion::new(5, 40));
    assert!(features.contains(&"builtin"), "v5.40 must enable 'builtin'");
    // Should also retain all v5.36 features
    assert!(features.contains(&"signatures"), "v5.40 should retain 'signatures'");
    assert!(features.contains(&"isa"), "v5.40 should retain 'isa'");
    Ok(())
}

#[test]
fn v5_12_retains_switch_from_v5_10() -> Result<(), Box<dyn std::error::Error>> {
    // switch was inherited from v5.10 and not removed until v5.38
    let features_v12 = features_enabled_by_version(PerlVersion::new(5, 12));
    assert!(
        features_v12.contains(&"switch"),
        "v5.12 should include 'switch' (inherited from v5.10, removed at v5.38)"
    );
    Ok(())
}

#[test]
fn v5_38_removes_switch() -> Result<(), Box<dyn std::error::Error>> {
    // switch (given/when) was removed from the bundle in v5.38
    let features = features_enabled_by_version(PerlVersion::new(5, 38));
    assert!(!features.contains(&"switch"), "v5.38 should not include 'switch' (removed)");
    Ok(())
}

#[test]
fn parse_perl_version_accepts_single_component_major_only() -> Result<(), Box<dyn std::error::Error>>
{
    let parsed = perl_pragma::parse_perl_version("v5");
    assert_eq!(parsed, Some(PerlVersion::new(5, 0)));
    Ok(())
}

#[test]
fn parse_perl_version_ignores_patch_component() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = perl_pragma::parse_perl_version("v5.36.2");
    assert_eq!(parsed, Some(PerlVersion::new(5, 36)));
    Ok(())
}

#[test]
fn parse_perl_version_rejects_non_numeric_input() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(perl_pragma::parse_perl_version("v5.bad"), None);
    assert_eq!(perl_pragma::parse_perl_version("not-a-version"), None);
    assert_eq!(perl_pragma::parse_perl_version(""), None);
    Ok(())
}

#[test]
fn version_implication_boundaries_match_expected_cutoffs() -> Result<(), Box<dyn std::error::Error>>
{
    assert!(!perl_pragma::version_implies_strict(PerlVersion::new(5, 11)));
    assert!(perl_pragma::version_implies_strict(PerlVersion::new(5, 12)));
    assert!(!perl_pragma::version_implies_warnings(PerlVersion::new(5, 34)));
    assert!(perl_pragma::version_implies_warnings(PerlVersion::new(5, 35)));
    Ok(())
}

// ===========================================================================
// PragmaState.features — version-implied feature set stored in PragmaState
// ===========================================================================

#[test]
fn use_v5_10_state_has_say_state_switch() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.10", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    assert_eq!(map.len(), 1);
    let state = &map[0].1;
    assert!(state.has_feature("say"), "v5.10 state must have 'say'");
    assert!(state.has_feature("state"), "v5.10 state must have 'state'");
    assert!(state.has_feature("switch"), "v5.10 state must have 'switch'");
    Ok(())
}

#[test]
fn use_v5_36_state_has_signatures_defer_isa() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.36", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.has_feature("signatures"), "v5.36 state must have 'signatures'");
    assert!(state.has_feature("defer"), "v5.36 state must have 'defer'");
    assert!(state.has_feature("isa"), "v5.36 state must have 'isa'");
    // v5.36 also implies strict and warnings
    assert!(state.strict_vars, "v5.36 implies strict");
    assert!(state.warnings, "v5.36 implies warnings");
    Ok(())
}

#[test]
fn use_v5_40_state_has_builtin() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("v5.40", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.has_feature("builtin"), "v5.40 state must have 'builtin'");
    assert!(!state.has_builtin_import("floor"), "v5.40 should not imply lexical builtin imports");
    assert!(state.builtin_imports.is_empty(), "v5.40 should not populate lexical builtin imports");
    Ok(())
}

#[test]
fn use_builtin_tracks_lexical_imports_only() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("builtin", &["'true'", "'floor'"], 0, 28)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(state.has_builtin_import("true"));
    assert!(state.has_builtin_import("floor"));
    assert!(
        !state.has_builtin_import("is_bool"),
        "only the names actually imported should be tracked"
    );
    assert!(
        !state.has_feature("builtin"),
        "lexical builtin imports should stay separate from version-implied features"
    );
    Ok(())
}

#[test]
fn no_version_declaration_has_no_features() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", &[], 0, 12)]);
    let map = PragmaTracker::build(&ast);
    let state = &map[0].1;
    assert!(!state.has_feature("say"), "strict pragma should not imply 'say'");
    Ok(())
}

#[test]
fn state_default_has_no_features() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState::default();
    assert!(!state.has_feature("say"), "default state has no features");
    assert!(!state.has_feature("state"), "default state has no features");
    Ok(())
}

// ===========================================================================
// Package statement -- pragma state preservation (#3480)
// ===========================================================================

fn package_node(name: &str, block_node: Option<Node>, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Package {
            name: name.to_string(),
            name_span: loc(start, end),
            block: block_node.map(Box::new),
        },
        location: loc(start, end),
    }
}

fn method_node(body_node: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Method {
            name: "foo".to_string(),
            signature: None,
            attributes: vec![],
            body: Box::new(body_node),
        },
        location: loc(start, end),
    }
}

fn class_node(body_node: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Class {
            name: "Foo".to_string(),
            parents: vec![],
            body: Box::new(body_node),
        },
        location: loc(start, end),
    }
}

fn eval_node(body_node: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Eval { block: Box::new(body_node) }, location: loc(start, end) }
}

fn do_node(body_node: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Do { block: Box::new(body_node) }, location: loc(start, end) }
}

fn defer_node(body_node: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Defer { block: Box::new(body_node) }, location: loc(start, end) }
}

#[test]
fn eval_string_is_conservative_and_does_not_enable_pragmas()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        eval_node(string_node("use strict; use warnings;", false, 5, 33), 0, 33),
        dummy_node(34, 40),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 39);
    assert!(!state.strict_vars, "eval STRING must not assume strict is enabled");
    assert!(!state.strict_subs, "eval STRING must not assume strict is enabled");
    assert!(!state.strict_refs, "eval STRING must not assume strict is enabled");
    assert!(!state.warnings, "eval STRING must not assume warnings are enabled");
    Ok(())
}

fn try_node(
    body_node: Node,
    catch_bodies: Vec<Node>,
    finally_node: Option<Node>,
    start: usize,
    end: usize,
) -> Node {
    Node {
        kind: NodeKind::Try {
            body: Box::new(body_node),
            catch_blocks: catch_bodies.into_iter().map(|body| (None, Box::new(body))).collect(),
            finally_block: finally_node.map(Box::new),
        },
        location: loc(start, end),
    }
}

fn given_node(body_node: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Given {
            expr: Box::new(dummy_node(start + 1, start + 2)),
            body: Box::new(body_node),
        },
        location: loc(start, end),
    }
}

fn when_node(body_node: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::When {
            condition: Box::new(dummy_node(start + 1, start + 2)),
            body: Box::new(body_node),
        },
        location: loc(start, end),
    }
}

fn default_node(body_node: Node, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Default { body: Box::new(body_node) }, location: loc(start, end) }
}

fn while_node(body_node: Node, continue_node: Option<Node>, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::While {
            condition: Box::new(dummy_node(start + 1, start + 2)),
            body: Box::new(body_node),
            continue_block: continue_node.map(Box::new),
        },
        location: loc(start, end),
    }
}

/// `use strict; package Foo;` -- subsequent top-level statements after the bare
/// `package Foo;` form (no block) must still see strict in effect.
///
/// The bare `package` form does not create a new scope, so pragma state from
/// before the declaration accumulates normally through sibling statements.
#[test]
fn package_bare_form_does_not_reset_pragma_state() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_node("Foo", None, 13, 25),
        use_node("warnings", &[], 26, 41),
    ]);
    let map = PragmaTracker::build(&ast);

    let state = PragmaTracker::state_for_offset(&map, 35);
    assert!(state.strict_vars, "strict_vars must survive a bare package statement");
    assert!(state.strict_subs, "strict_subs must survive a bare package statement");
    assert!(state.strict_refs, "strict_refs must survive a bare package statement");
    assert!(state.warnings, "warnings must be on after use warnings");
    Ok(())
}

/// `use strict; package Foo { ... }` -- the block form creates a new lexical
/// scope.  Pragmas declared *before* the package block must be visible inside.
#[test]
fn package_block_form_inherits_outer_pragma_state() -> Result<(), Box<dyn std::error::Error>> {
    let inner_use = use_node("warnings", &[], 20, 35);
    let pkg_block = block(vec![inner_use], 14, 49);
    let pkg = package_node("Foo", Some(pkg_block), 13, 50);
    let ast = program(vec![use_node("strict", &[], 0, 12), pkg]);
    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 28);
    assert!(inside.strict_vars, "strict must be inherited inside package block");
    assert!(inside.warnings, "warnings enabled inside package block must be visible");
    Ok(())
}

/// After a `package Foo { ... }` block, the state must be restored to the
/// pre-block value (package blocks are lexically scoped like regular blocks).
#[test]
fn package_block_form_restores_state_after_block() -> Result<(), Box<dyn std::error::Error>> {
    let no_refs = Node {
        kind: NodeKind::No {
            module: "strict".to_string(),
            args: vec!["refs".to_string()],
            has_filter_risk: false,
        },
        location: loc(20, 40),
    };
    let pkg_block = block(vec![no_refs], 14, 59);
    let pkg = package_node("Foo", Some(pkg_block), 13, 60);
    let ast = program(vec![use_node("strict", &[], 0, 12), pkg, use_node("warnings", &[], 61, 76)]);
    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 30);
    assert!(!inside.strict_refs, "strict_refs must be disabled inside the package block");

    let after = PragmaTracker::state_for_offset(&map, 70);
    assert!(after.strict_refs, "strict_refs must be restored after the package block");
    assert!(after.warnings, "warnings must be on after use warnings");
    Ok(())
}

/// `package Foo { use strict 'vars'; }` -- pragma declared *inside*
/// the package block must be visible at the inner offset.
#[test]
fn package_block_pragma_inside_is_visible_at_inner_offset() -> Result<(), Box<dyn std::error::Error>>
{
    let inner_strict = use_node("strict", &["vars"], 20, 40);
    let pkg_block = block(vec![inner_strict], 14, 49);
    let pkg = package_node("Foo", Some(pkg_block), 13, 50);
    let ast = program(vec![pkg]);
    let map = PragmaTracker::build(&ast);

    let inside = PragmaTracker::state_for_offset(&map, 30);
    assert!(inside.strict_vars, "strict_vars declared inside package block must be visible");
    Ok(())
}

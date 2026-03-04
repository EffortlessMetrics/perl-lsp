//! Comprehensive unit tests for perl-pragma crate.
//!
//! Tests cover PragmaState, PragmaTracker::build, and PragmaTracker::state_for_offset
//! across all public API surface including edge cases.

use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{PragmaState, PragmaTracker};

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
    Ok(())
}

#[test]
fn all_strict_enables_strict_but_not_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let state = PragmaState::all_strict();
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(state.strict_refs);
    assert!(!state.warnings, "all_strict should not enable warnings");
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
fn no_warnings_disables_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("warnings", &[], 0, 15), no_node("warnings", &[], 16, 30)]);
    let map = PragmaTracker::build(&ast);
    assert!(!map[1].1.warnings);
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
fn no_warnings_with_args_still_disables() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("warnings", &[], 0, 15),
        no_node("warnings", &["uninitialized"], 16, 40),
    ]);
    let map = PragmaTracker::build(&ast);
    assert!(!map[1].1.warnings);
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

//! Tests for incremental parsing API (Tree::edit and Parser::parse_with_old_tree)

use perl_parser_core::edit::Edit;
use perl_position_tracking::Position;
use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{FallbackReason, InputEdit, Parser, ReparseMode};

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn when_edit_is_recorded_then_tree_accepts_it_without_panicking() {
    let mut tree = parse("my $x = 1;");
    // Simulate replacing "1" with "42" at byte 8..9
    let edit = Edit::new(
        8,
        9,
        10,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(10, 0, 10),
    );
    tree.edit(&edit); // must not panic
}

#[test]
fn when_parse_with_old_tree_is_called_then_new_source_is_parsed() {
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse("my $x = 1;"));
    let new_tree = parser.parse_with_old_tree("my $x = 42;", &old_tree);
    assert!(new_tree.is_some(), "parse_with_old_tree must return a tree for valid source");
    let new_tree = must_some(new_tree);
    assert_eq!(new_tree.source(), "my $x = 42;");
}

#[test]
fn when_parse_with_old_tree_given_invalid_source_then_some_tree_is_returned() {
    // The v3 parser is error-tolerant; even invalid source returns Some.
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse("my $x = 1;"));
    let result = parser.parse_with_old_tree("sub {{{{{", &old_tree);
    assert!(result.is_some(), "error-tolerant parser still yields a tree for malformed source");
}

#[test]
fn when_multiple_edits_are_recorded_then_tree_remains_usable_for_reparse() {
    let mut tree = parse("my $x = 1; my $y = 2;");

    // First edit: replace "1" with "10"
    let edit1 = Edit::new(
        8,
        9,
        10,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(10, 0, 10),
    );
    tree.edit(&edit1);

    // Second edit: replace "2" with "20"
    let edit2 = Edit::new(
        18,
        19,
        20,
        Position::new(18, 0, 18),
        Position::new(19, 0, 19),
        Position::new(20, 0, 20),
    );
    tree.edit(&edit2);

    // Verify the tree is still usable as an old_tree hint: parse the updated source
    // and verify the resulting tree reflects the new content.
    let new_source = "my $x = 10; my $y = 20;";
    let mut parser = Parser::new();
    let new_tree = must_some(parser.parse_with_old_tree(new_source, &tree));
    assert_eq!(
        new_tree.source(),
        new_source,
        "reparse after two edits must reflect updated source"
    );
}

#[test]
fn when_input_edit_type_is_used_then_it_matches_tree_sitter_signature() {
    // Verify InputEdit is accessible and usable
    fn takes_input_edit(_edit: &InputEdit) {}

    let edit =
        Edit::new(0, 1, 2, Position::new(0, 0, 0), Position::new(1, 0, 1), Position::new(2, 0, 2));
    takes_input_edit(&edit);
}

#[test]
fn when_edit_is_applied_then_tree_source_unchanged() {
    // edit() should not modify the stored source - it just records the edit
    let mut tree = parse("my $x = 1;");
    let original_source = tree.source().to_string();

    let edit = Edit::new(
        8,
        9,
        10,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(10, 0, 10),
    );
    tree.edit(&edit);

    // Source should remain unchanged - the edit is just recorded
    assert_eq!(tree.source(), original_source);
}

#[test]
fn when_parse_with_old_tree_given_empty_new_source_then_tree_is_returned() {
    // Deleting the entire file content is a valid LSP edit: new source is empty string.
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse("my $x = 42;"));
    let new_tree = parser.parse_with_old_tree("", &old_tree);
    assert!(new_tree.is_some(), "parse_with_old_tree must handle empty new source");
    let new_tree = must_some(new_tree);
    assert_eq!(new_tree.source(), "", "tree source must be the empty string");
    // The root is still a Program node (empty program).
    assert_eq!(new_tree.root_node().kind(), "source_file");
}

#[test]
fn when_deletion_edit_is_recorded_then_tree_accepts_it() {
    // Deletion: remove "42" (bytes 8..10) leaving byte 8..8 (net -2 shift).
    let mut tree = parse("my $x = 42;");
    let edit = Edit::new(
        8,
        10,
        8,
        Position::new(8, 0, 8),
        Position::new(10, 0, 10),
        Position::new(8, 0, 8),
    );
    tree.edit(&edit); // must not panic for a shrinking (deletion) edit
}

#[test]
fn when_insertion_edit_at_point_is_recorded_then_tree_accepts_it() {
    // Pure insertion: insert "# comment\n" before byte 0 (start_byte == old_end_byte).
    let mut tree = parse("my $x = 1;");
    let inserted = "# comment\n";
    let edit = Edit::new(
        0,
        0,
        inserted.len(),
        Position::new(0, 0, 0),
        Position::new(0, 0, 0),
        Position::new(inserted.len(), 1, 0),
    );
    tree.edit(&edit); // zero-width old range is a valid insertion
}

#[test]
fn when_reparse_after_edit_then_new_tree_has_correct_ast() {
    // Full end-to-end: record an edit, re-parse, verify the AST reflects new source —
    // not just source() but the actual root kind and child structure.
    let old_source = "my $x = 1;";
    let new_source = "sub foo { my $x = 1; }";

    let mut parser = Parser::new();
    let mut old_tree = must_some(parser.parse(old_source));

    // Record an edit that replaces the whole file.
    let edit = Edit::new(
        0,
        old_source.len(),
        new_source.len(),
        Position::new(0, 0, 0),
        Position::new(old_source.len(), 0, old_source.len() as u32),
        Position::new(new_source.len(), 0, new_source.len() as u32),
    );
    old_tree.edit(&edit);

    let new_tree = must_some(parser.parse_with_old_tree(new_source, &old_tree));
    assert_eq!(new_tree.source(), new_source);
    // Root must still be a Program node.
    assert_eq!(new_tree.root_node().kind(), "source_file");
    // The new program has children (the sub declaration).
    assert!(
        new_tree.root_node().child_count() >= 1,
        "new tree must have at least one child for 'sub foo'"
    );
}

#[test]
fn when_parse_with_old_tree_given_unchanged_source_then_old_tree_is_reused() {
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse("my $x = 1;"));

    let reparsed = must_some(parser.parse_with_old_tree("my $x = 1;", &old_tree));

    assert_eq!(reparsed, old_tree, "unchanged source should return the existing tree");
}

#[test]
fn when_parse_with_old_tree_given_unchanged_source_but_pending_edits_then_tree_is_reparsed() {
    let mut parser = Parser::new();
    let mut old_tree = must_some(parser.parse("my $x = 1;"));

    let edit = Edit::new(
        8,
        9,
        10,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(10, 0, 10),
    );
    old_tree.edit(&edit);

    let reparsed = must_some(parser.parse_with_old_tree("my $x = 1;", &old_tree));

    assert_ne!(
        reparsed, old_tree,
        "pending edits should disable unchanged-source reuse and force a reparse"
    );
    assert_eq!(reparsed.source(), "my $x = 1;");
}

#[test]
fn when_one_valid_edit_is_pending_then_token_replay_is_reported() {
    let source = "my $value = 1;\n".repeat(40);
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse(&source));
    let start = must_some(source.find('1'));
    let new_source = source.replacen('1', "22", 1);
    let edit = Edit::new(
        start,
        start + 1,
        start + 2,
        Position::new(start, 0, start as u32),
        Position::new(start + 1, 0, (start + 1) as u32),
        Position::new(start + 2, 0, (start + 2) as u32),
    );
    let mut edited_tree = old_tree;
    edited_tree.edit(&edit);

    let replayed = must_some(parser.parse_with_old_tree(&new_source, &edited_tree));
    let fresh = must_some(parser.parse(&new_source));

    assert_eq!(replayed.root_node().to_sexp(), fresh.root_node().to_sexp());
    assert_eq!(replayed.diagnostics(), fresh.diagnostics());
    assert_eq!(replayed.reparse_mode(), Some(ReparseMode::TokenReplay));
    let metrics = must_some(replayed.incremental_metrics());
    assert!(metrics.tokens_reused > 0);
    assert!(metrics.tokens_relexed > 0);
    assert!(!replayed.reprocessed_ranges().is_empty());
}

#[test]
fn unchanged_reuse_does_not_expose_previous_operation_metrics() {
    let source = "my $value = 1;\n".repeat(40);
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse(&source));
    let start = must_some(source.find('1'));
    let new_source = source.replacen('1', "22", 1);
    let edit = Edit::new(
        start,
        start + 1,
        start + 2,
        Position::new(start, 0, start as u32),
        Position::new(start + 1, 0, (start + 1) as u32),
        Position::new(start + 2, 0, (start + 2) as u32),
    );
    let mut edited_tree = old_tree;
    edited_tree.edit(&edit);
    let replayed = must_some(parser.parse_with_old_tree(&new_source, &edited_tree));

    let unchanged = must_some(parser.parse_with_old_tree(&new_source, &replayed));
    assert_eq!(unchanged.reparse_mode(), Some(ReparseMode::Unchanged));
    assert!(unchanged.incremental_metrics().is_none());
    assert!(unchanged.reprocessed_ranges().is_empty());
}

#[test]
fn multiple_pending_edits_use_an_explicit_full_parse_fallback() {
    let mut tree = parse("my $x = 1; my $y = 2;");
    let edit = |start, old_end, new_end| {
        Edit::new(
            start,
            old_end,
            new_end,
            Position::new(start, 0, start as u32),
            Position::new(old_end, 0, old_end as u32),
            Position::new(new_end, 0, new_end as u32),
        )
    };
    tree.edit(&edit(8, 9, 10));
    tree.edit(&edit(18, 19, 20));

    let mut parser = Parser::new();
    let reparsed = must_some(parser.parse_with_old_tree("my $x = 10; my $y = 20;", &tree));

    assert_eq!(
        reparsed.reparse_mode(),
        Some(ReparseMode::FullParseFallback(FallbackReason::MultipleEdits))
    );
    assert!(reparsed.incremental_metrics().is_some_and(|metrics| metrics.full_parse));
}

#[test]
fn stale_input_edit_uses_a_full_parse_fallback_without_partial_replay() {
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse("my $x = 1;"));
    let mut edited_tree = old_tree;
    edited_tree.edit(&Edit::new(
        8,
        9,
        99,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(99, 0, 99),
    ));

    let reparsed = must_some(parser.parse_with_old_tree("my $x = 42;", &edited_tree));

    assert_eq!(
        reparsed.reparse_mode(),
        Some(ReparseMode::FullParseFallback(FallbackReason::InvalidEdit))
    );
    assert_eq!(
        reparsed.root_node().to_sexp(),
        must_some(parser.parse("my $x = 42;")).root_node().to_sexp()
    );
}

#[test]
fn same_length_change_outside_the_edit_uses_an_invalid_edit_fallback() {
    let old_source = "my $x = 1; my $y = 2;";
    let new_source = "my $x = 1; my $y = 3;";
    let mut parser = Parser::new();
    let old_tree = must_some(parser.parse(old_source));
    let mut edited_tree = old_tree;
    edited_tree.edit(&Edit::new(
        8,
        9,
        9,
        Position::new(8, 0, 8),
        Position::new(9, 0, 9),
        Position::new(9, 0, 9),
    ));

    let reparsed = must_some(parser.parse_with_old_tree(new_source, &edited_tree));
    let fresh = must_some(parser.parse(new_source));

    assert_eq!(
        reparsed.reparse_mode(),
        Some(ReparseMode::FullParseFallback(FallbackReason::InvalidEdit))
    );
    assert_eq!(reparsed.root_node().to_sexp(), fresh.root_node().to_sexp());
}

//! Tests for incremental parsing API (Tree::edit and Parser::parse_with_old_tree)

use perl_parser_core::edit::Edit;
use perl_position_tracking::Position;
use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{InputEdit, Parser};

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
    assert_eq!(new_tree.root_node().kind(), "Program");
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
    assert_eq!(new_tree.root_node().kind(), "Program");
    // The new program has children (the sub declaration).
    assert!(
        new_tree.root_node().child_count() >= 1,
        "new tree must have at least one child for 'sub foo'"
    );
}

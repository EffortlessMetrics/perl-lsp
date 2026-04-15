//! Tests for incremental parsing API (Tree::edit and Parser::parse_with_old_tree)

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{Parser, InputEdit};
use perl_parser_core::edit::Edit;
use perl_position_tracking::Position;

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
    let old_tree = parser.parse("my $x = 1;").unwrap();
    let new_tree = parser.parse_with_old_tree("my $x = 42;", &old_tree);
    assert!(new_tree.is_some(), "parse_with_old_tree must return a tree for valid source");
    let new_tree = new_tree.unwrap();
    assert_eq!(new_tree.source(), "my $x = 42;");
}

#[test]
fn when_parse_with_old_tree_given_invalid_source_then_some_tree_is_returned() {
    // The v3 parser is error-tolerant; even invalid source returns Some.
    let mut parser = Parser::new();
    let old_tree = parser.parse("my $x = 1;").unwrap();
    let result = parser.parse_with_old_tree("sub {{{{{", &old_tree);
    assert!(result.is_some(), "error-tolerant parser still yields a tree for malformed source");
}

#[test]
fn when_multiple_edits_are_recorded_then_tree_stores_all() {
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
    // Test passes if we reach here without panic
}

#[test]
fn when_input_edit_type_is_used_then_it_matches_tree_sitter_signature() {
    // Verify InputEdit is accessible and usable
    fn takes_input_edit(_edit: &InputEdit) {}
    
    let edit = Edit::new(
        0,
        1,
        2,
        Position::new(0, 0, 0),
        Position::new(1, 0, 1),
        Position::new(2, 0, 2),
    );
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

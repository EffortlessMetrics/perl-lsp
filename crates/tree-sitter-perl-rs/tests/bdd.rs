//! Behavior-driven tests for the `tree-sitter-perl-rs` facade.
//!
//! These scenarios emphasize end-user expectations in Given/When/Then form.

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

#[test]
fn given_valid_perl_when_parsed_then_tree_and_root_shape_are_available() {
    // Given
    let source = "my $x = 42;";
    let mut parser = Parser::new();

    // When
    let tree = must_some(parser.parse(source));
    let root = tree.root_node();

    // Then
    assert_eq!(root.kind(), "Program");
    assert_eq!(tree.source(), source);
    assert!(root.to_sexp().starts_with("(source_file"));
}

#[test]
fn given_malformed_perl_when_parsed_then_partial_tree_is_still_returned() {
    // Given
    let malformed = "sub broken { my $x = ; if ($x { print $x;";
    let mut parser = Parser::new();

    // When
    let tree = parser.parse(malformed);

    // Then
    assert!(tree.is_some(), "parser should return a partial tree for malformed input");
    let parsed = must_some(tree);
    let root = parsed.root_node();
    assert_eq!(root.kind(), "Program");
}

#[test]
fn given_a_root_node_when_traversing_children_then_access_patterns_are_consistent() {
    // Given
    let source = "my $x = 1;\nmy $y = 2;\n";
    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));
    let root = tree.root_node();

    // When
    let child_count = root.child_count();
    let iter_count = root.children().count();
    let first_child = root.child(0);

    // Then
    assert_eq!(child_count, iter_count);
    assert!(child_count > 0, "expected program root to expose children");
    assert!(first_child.is_some(), "expected first child to exist");
}

#[test]
fn given_node_and_shorter_external_buffer_when_utf8_text_is_requested_then_range_is_clamped() {
    // Given
    let source = "my $x = 42;";
    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));
    let root = tree.root_node();
    let shorter_buffer = b"my";

    // When
    let extracted = root.utf8_text(shorter_buffer);

    // Then
    assert!(extracted.is_ok(), "clamped byte range should not panic or fail utf8 conversion");
    assert_eq!(extracted.ok(), Some("my"));
}

#[test]
fn given_one_parser_instance_when_reused_then_multiple_inputs_can_be_parsed() {
    // Given
    let mut parser = Parser::new();

    // When
    let first = parser.parse("package Demo;\nsub one { 1 }\n");
    let second = parser.parse("for my $item (@items) { print $item; }\n");

    // Then
    assert!(first.is_some(), "first parse should produce a tree");
    assert!(second.is_some(), "second parse should produce a tree");
}

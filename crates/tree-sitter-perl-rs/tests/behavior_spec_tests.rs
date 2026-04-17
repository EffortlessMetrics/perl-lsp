//! BDD-style behavior specification tests for `tree-sitter-perl-rs`.
//!
//! These scenarios lock facade-level behavior from a user perspective:
//! parser ergonomics, traversal, source extraction, and resilience on malformed input.

use perl_tdd_support::{must, must_some};
use tree_sitter_perl_rs::Parser;

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn when_parsing_valid_perl_then_tree_and_source_are_available() {
    let source = "my $x = 42;";

    let tree = parse(source);

    assert_eq!(tree.source(), source);
    assert_eq!(tree.root_node().start_byte(), 0);
}

#[test]
fn when_requesting_root_kind_then_program_is_returned() {
    let tree = parse("my $x = 42;");

    assert_eq!(tree.root_node().kind(), "Program");
}

#[test]
fn when_rendering_root_as_sexp_then_output_uses_source_file_shape() {
    let tree = parse("my $x = 42;");
    let sexp = tree.root_node().to_sexp();

    assert!(sexp.starts_with("(source_file"), "unexpected sexp: {sexp}");
}

#[test]
fn when_iterating_children_then_iterator_count_matches_indexed_access() {
    let tree = parse("my $x = 1; my $y = 2;");
    let root = tree.root_node();

    let children: Vec<_> = root.children().collect();
    assert_eq!(children.len(), root.child_count());

    if let Some(first_from_iter) = children.first() {
        let first_from_index = must_some(root.child(0));
        assert_eq!(first_from_index.kind(), first_from_iter.kind());
    }
}

#[test]
fn when_requesting_out_of_bounds_child_then_none_is_returned() {
    let tree = parse("my $x = 1;");

    assert!(tree.root_node().child(usize::MAX).is_none());
}

#[test]
fn when_extracting_utf8_text_then_root_round_trips_source_bytes() {
    let source = "my $x = 'café';";
    let tree = parse(source);

    let text = must(tree.root_node().utf8_text(source.as_bytes()));
    assert_eq!(text, source);
}

#[test]
fn when_utf8_text_uses_shorter_buffer_then_it_clamps_without_panicking() {
    let tree = parse("my $x = 42;");

    let result = tree.root_node().utf8_text(b"my");
    assert!(result.is_ok());
    assert_eq!(must(result), "my");
}

#[test]
fn when_parsing_malformed_input_then_error_tolerant_tree_is_still_produced() {
    let mut parser = Parser::new();

    let tree = parser.parse("sub { @@@@invalid{{{{");

    assert!(tree.is_some());
}

#[test]
fn when_reusing_one_parser_for_multiple_inputs_then_each_parse_still_returns_a_tree() {
    let mut parser = Parser::new();

    let first = parser.parse("package Demo;\nsub one { 1 }\n");
    let second = parser.parse("for my $item (@items) { print $item; }\n");

    assert!(first.is_some());
    assert!(second.is_some());
}

#[test]
fn when_requesting_grammar_kind_of_root_then_source_file_is_returned() {
    let tree = parse("my $x = 42;");
    assert_eq!(tree.root_node().grammar_kind(), "source_file");
}

#[test]
fn when_requesting_grammar_kind_of_subroutine_then_sub_is_returned() {
    let tree = parse("sub greet { 1 }");
    let root = tree.root_node();
    // Find the subroutine child
    let sub_node = must_some(root.children().find(|n| n.kind() == "Subroutine"));
    assert_eq!(sub_node.grammar_kind(), "sub");
}

#[test]
fn when_v3_kind_and_grammar_kind_are_both_available_then_they_differ_for_program() {
    let tree = parse("1;");
    let root = tree.root_node();
    assert_eq!(root.kind(), "Program");
    assert_eq!(root.grammar_kind(), "source_file");
    assert_ne!(root.kind(), root.grammar_kind());
}

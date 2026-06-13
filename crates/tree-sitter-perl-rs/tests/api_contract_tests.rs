//! Public API contract coverage for `tree-sitter-perl-rs`.
//!
//! These tests exercise facade methods that are easy for downstream tree-sitter-style
//! consumers to rely on but were not fully covered by behavior or snapshot tests.

use perl_tdd_support::{must, must_some};
use tree_sitter_perl_rs::{LANGUAGE, Parser, PerlLanguage, PerlNodeKind, language};

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn language_descriptor_reports_stable_named_kind_catalog() {
    let descriptor = language();

    assert_eq!(descriptor, LANGUAGE);
    assert_eq!(PerlLanguage::default(), LANGUAGE);
    assert_eq!(descriptor.node_kind_count(), descriptor.node_kind_names().len());
    assert!(descriptor.node_kind_count() > 0);
    assert!(descriptor.node_kind_is_named("Program"));
    assert!(descriptor.node_kind_is_named("Subroutine"));
    assert!(descriptor.node_kind_is_named("Variable"));
    assert!(!descriptor.node_kind_is_named("not_a_perl_node_kind"));
}

#[test]
fn language_descriptor_kind_names_are_sorted_and_unique() {
    let descriptor = language();
    let names = descriptor.node_kind_names();

    for pair in names.windows(2) {
        assert!(
            pair[0] < pair[1],
            "kind names must be sorted and unique; saw {:?} before {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn node_positions_use_zero_based_rows_and_byte_columns() {
    let source = "my $first = 1;\nmy $café = 2;\n";
    let tree = parse(source);
    let root = tree.root_node();
    let second_statement = must_some(root.child(1));

    let root_start = root.start_position();
    assert_eq!(root_start.row, 0);
    assert_eq!(root_start.column, 0);

    let root_end = root.end_position();
    assert_eq!(root_end.row, 1);
    assert_eq!(root_end.column, 14);

    let second_start = second_statement.start_position();
    assert_eq!(second_start.row, 1);
    assert_eq!(second_start.column, 0);

    let second_end = second_statement.end_position();
    assert_eq!(second_end.row, 1);
    assert_eq!(second_end.column, 9);
}

#[test]
fn cursor_navigation_covers_boundaries_siblings_and_reset() {
    let tree = parse("my $x = 1;\nmy $y = 2;\nmy $z = 3;\n");
    let mut cursor = tree.walk();

    assert_eq!(cursor.node().kind(), "source_file");
    assert!(!cursor.goto_parent(), "root cursor must not move above root");
    assert!(!cursor.goto_next_sibling(), "root cursor has no siblings");
    assert!(!cursor.goto_previous_sibling(), "root cursor has no siblings");

    assert!(cursor.goto_first_child(), "root should have a first statement");
    assert_eq!(cursor.node().start_byte(), must_some(tree.root_node().child(0)).start_byte());
    assert!(!cursor.goto_previous_sibling(), "first child has no previous sibling");

    assert!(cursor.goto_next_sibling(), "first child should have a next sibling");
    assert_eq!(cursor.node().start_byte(), must_some(tree.root_node().child(1)).start_byte());
    assert!(cursor.goto_previous_sibling(), "second child should move back to first sibling");
    assert_eq!(cursor.node().start_byte(), must_some(tree.root_node().child(0)).start_byte());

    assert!(cursor.goto_parent(), "child cursor should move back to root");
    assert!(cursor.goto_last_child(), "root should have a last statement");
    assert_eq!(cursor.node().start_byte(), must_some(tree.root_node().child(2)).start_byte());
    assert!(!cursor.goto_next_sibling(), "last child has no next sibling");

    cursor.reset();
    assert_eq!(cursor.node().kind(), "source_file");
}

#[test]
fn node_walk_is_rooted_at_that_node_not_the_whole_tree() {
    let tree = parse("sub outer { my $x = 1; }\nmy $after = 2;\n");
    let sub_node = must_some(tree.root_node().child(0));
    let mut cursor = sub_node.walk();

    assert_eq!(cursor.node().start_byte(), sub_node.start_byte());
    assert!(!cursor.goto_parent(), "node-rooted cursor cannot move to the tree root");
    assert!(cursor.goto_first_child(), "subroutine node should have children");
    assert!(cursor.goto_parent(), "subroutine child should move back to subroutine root");
    assert_eq!(cursor.node().start_byte(), sub_node.start_byte());
}

#[test]
fn utf8_text_reports_invalid_caller_buffers() {
    let tree = parse("my $x = 1;");
    let invalid_utf8 = [b'm', 0xff, b'x'];

    let text = tree.root_node().utf8_text(&invalid_utf8);

    assert!(text.is_err(), "invalid caller-supplied UTF-8 must be reported");
}

#[test]
fn inner_escape_hatch_and_node_kind_reexport_expose_native_program()
-> Result<(), Box<dyn std::error::Error>> {
    let tree = parse("1;");
    let root = tree.root_node();

    match &root.inner().kind {
        PerlNodeKind::Program { statements } => assert_eq!(statements.len(), root.child_count()),
        other => {
            return Err(format!("expected Program root, got {}", other.kind_name()).into());
        }
    }

    Ok(())
}

#[test]
fn tree_source_is_carried_to_child_nodes() {
    let source = "my $answer = 42;";
    let tree = parse(source);
    let child = must_some(tree.root_node().child(0));

    assert_eq!(tree.root_node().tree_source(), source);
    assert_eq!(child.tree_source(), source);
    assert_eq!(
        must(child.utf8_text(source.as_bytes())),
        &source[child.start_byte()..child.end_byte()]
    );
}

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{Node, Parser};

fn subtree_contains_leaf(node: Node<'_>) -> bool {
    if node.is_leaf() {
        return true;
    }

    node.children().any(subtree_contains_leaf)
}

#[test]
fn given_valid_perl_when_parsing_then_tree_and_source_are_available() {
    let source = "package Demo;\nsub greet { return 'hi'; }\n";
    let mut parser = Parser::new();

    let tree = must_some(parser.parse(source));

    assert_eq!(tree.source(), source);
    assert_eq!(tree.root_node().kind(), "Program");
}

#[test]
fn given_program_with_multiple_statements_when_iterating_children_then_child_access_patterns_agree()
{
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("my $x = 1; my $y = 2; my $z = 3;"));
    let root = tree.root_node();

    let child_count = root.child_count();
    let children: Vec<_> = root.children().collect();

    assert_eq!(children.len(), child_count);
    assert!(child_count >= 1);
    assert!(root.child(0).is_some());
    assert!(root.child(child_count).is_none());
}

#[test]
fn given_parsed_tree_when_requesting_utf8_text_then_byte_ranges_are_clamped_safely() {
    let source = "my $value = 42;";
    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));
    let root = tree.root_node();

    let full = root.utf8_text(source.as_bytes());
    let short = root.utf8_text(b"my");

    assert_eq!(full.ok(), Some(source));
    assert_eq!(short.ok(), Some("my"));
}

#[test]
fn given_invalid_perl_when_parsing_then_error_recovery_tree_is_returned() {
    let mut parser = Parser::new();

    let tree = parser.parse("sub broken { if ( { @@@");

    assert!(tree.is_some());
}

#[test]
fn given_tree_sitter_style_facade_when_rendering_then_sexp_uses_source_file_root() {
    let mut parser = Parser::default();
    let tree = must_some(parser.parse("my $count = scalar @items;"));

    let sexp = tree.root_node().to_sexp();

    assert!(sexp.starts_with("(source_file"));
}

#[test]
fn given_leaf_and_non_leaf_nodes_when_checking_is_leaf_then_facade_reflects_ast_structure() {
    let mut parser = Parser::new();
    let tree = must_some(parser.parse("my $x = 1;"));
    let root = tree.root_node();

    assert!(!root.is_leaf());

    assert!(subtree_contains_leaf(root), "expected at least one leaf node in declaration AST");
}

//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::Node;

/// Walk the AST and call a function for each node
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
#[allow(clippy::only_used_in_recursion)]
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    func(node);
    node.for_each_child(|child| walk_node(child, func));
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser_core::{Node, Parser};

    fn collect_via_walk_node(root: &Node) -> Vec<String> {
        let mut visited = Vec::new();
        walk_node(root, &mut |node| visited.push(node.kind.kind_name().to_string()));
        visited
    }

    fn collect_via_node_children(root: &Node) -> Vec<String> {
        fn walk(node: &Node, visited: &mut Vec<String>) {
            visited.push(node.kind.kind_name().to_string());
            node.for_each_child(|child| walk(child, visited));
        }

        let mut visited = Vec::new();
        walk(root, &mut visited);
        visited
    }

    fn assert_traversal_conformance(source: &str) {
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("source should parse in traversal conformance test");
        assert_eq!(collect_via_walk_node(&ast), collect_via_node_children(&ast));
    }

    #[test]
    fn statement_modifier_visits_statement_and_condition() {
        assert_traversal_conformance("print \"ok\" if $x = 5;");
    }

    #[test]
    fn nested_statement_modifiers_traverse_correctly() {
        assert_traversal_conformance("print \"ok\" if $x while $y;");
    }

    #[test]
    fn all_statement_modifier_variants_traverse_correctly() {
        for source in [
            "print \"ok\" if $x;",
            "print \"ok\" unless $x;",
            "print \"ok\" while $x;",
            "print \"ok\" until $x;",
            "print \"ok\" for @items;",
            "print \"ok\" foreach @items;",
        ] {
            assert_traversal_conformance(source);
        }
    }

    #[test]
    fn traversal_conformance_for_child_bearing_node_kinds() {
        // Table-driven snippets that force parser output to include the
        // major child-bearing node families, then verify walker parity
        // against canonical `Node::for_each_child` traversal.
        for source in [
            "my $x = 1; my ($a,$b) = (2,3);",
            "if ($x) { $x = 1 } elsif ($y) { $y = 2 } else { $z = 3 }",
            "while ($x) { $x-- } continue { $x++ }",
            "for (my $i = 0; $i < 3; $i++) { print $i }",
            "foreach my $item (@items) { print $item } continue { print \"c\" }",
            "my $x = $a ? $b : $c; return $x;",
            "sub f($x, $y = 1, :$z, *@rest) { $x + $y }",
            "method m($x) { $self->n($x) }",
            "try { $x } catch ($e) { $e } finally { $x = 0 }",
            "given ($x) { when ($y) { $z } default { $x } }",
            "my $z = foo($a, bar($b)); $obj->m($z);",
            "my $arr = [1, 2]; my $h = { a => 1, b => 2 };",
            "my $x = do { 1 }; eval { 1 }; defer { 1 };",
            "LABEL: print \"ok\" if $x =~ /a/;",
        ] {
            assert_traversal_conformance(source);
        }
    }
}

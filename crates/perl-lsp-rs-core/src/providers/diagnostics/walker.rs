//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::{Node, walk_preorder};

/// Walk the AST and call a function for each node
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
#[allow(clippy::only_used_in_recursion)]
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    walk_preorder(node, func);
}

#[cfg(test)]
mod tests {
    use super::walk_node;
    use perl_parser::Parser;
    use perl_parser_core::NodeKind;

    #[test]
    fn traversal_visits_statement_and_condition_for_all_statement_modifiers() {
        let source = r#"
print "ok" if $x = 5;
print "ok" unless $x = 5;
print "ok" while $x = 5;
print "ok" until $x = 5;
print "ok" for @items;
print "ok" foreach @items;
print "ok" if ($x = 5) unless ($y = 7);
"#;

        let ast = Parser::new(source).parse_with_recovery().ast;

        let mut print_calls = 0usize;
        let mut seen_modifiers = std::collections::BTreeSet::new();
        let mut visited = std::collections::BTreeSet::new();
        let mut expected_statement_nodes = std::collections::BTreeSet::new();
        let mut expected_condition_nodes = std::collections::BTreeSet::new();

        walk_node(&ast, &mut |node| match &node.kind {
            NodeKind::FunctionCall { name, .. } if name == "print" => {
                print_calls += 1;
                visited.insert(node as *const _ as usize);
            }
            NodeKind::StatementModifier { modifier, statement, condition } => {
                seen_modifiers.insert(modifier.clone());
                expected_statement_nodes.insert(statement.as_ref() as *const _ as usize);
                expected_condition_nodes.insert(condition.as_ref() as *const _ as usize);
                visited.insert(node as *const _ as usize);
            }
            _ => {
                visited.insert(node as *const _ as usize);
            }
        });

        assert_eq!(print_calls, 7, "expected every statement subtree to be traversed");
        assert!(expected_statement_nodes.is_subset(&visited));
        assert!(expected_condition_nodes.is_subset(&visited));
        assert_eq!(
            seen_modifiers,
            ["for", "foreach", "if", "unless", "until", "while"]
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect()
        );
    }
}

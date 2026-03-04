//! AST traversal helpers for cursor-based navigation features.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use perl_parser_core::ast::{Node, NodeKind};

/// Finds the most specific AST node containing the given byte offset.
pub fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    if offset < node.location.start || offset > node.location.end {
        return None;
    }

    for child in get_node_children(node) {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    Some(node)
}

/// Returns direct child nodes for a given AST node.
pub fn get_node_children(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } => statements.iter().collect(),
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut children = vec![variable.as_ref()];
            if let Some(init) = initializer {
                children.push(init.as_ref());
            }
            children
        }
        NodeKind::Assignment { lhs, rhs, .. } => vec![lhs.as_ref(), rhs.as_ref()],
        NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        NodeKind::FunctionCall { args, .. } => args.iter().collect(),
        NodeKind::Subroutine { body, .. } => {
            vec![body.as_ref()]
        }
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        _ => vec![],
    }
}

/// Determines the current package context at the given offset.
pub fn current_package_at(ast: &Node, offset: usize) -> &str {
    fn scan<'a>(node: &'a Node, offset: usize, last: &mut Option<&'a str>) {
        if let NodeKind::Package { name, .. } = &node.kind {
            if node.location.start <= offset {
                *last = Some(name.as_str());
            }
        }
        for child in get_node_children(node) {
            if child.location.start <= offset {
                scan(child, offset, last);
            }
        }
    }

    let mut last_pkg: Option<&str> = None;
    scan(ast, offset, &mut last_pkg);
    last_pkg.unwrap_or("main")
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn finds_node_inside_variable_use() {
        let source = "my $count = 1; $count++;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let offset = source.find("$count++").unwrap_or(0) + 2;

        let node = must_some(find_node_at_offset(&ast, offset));
        assert!(node.location.start <= offset && node.location.end >= offset);

    }

    #[test]
    fn resolves_current_package_before_offset() {
        let source = "package Alpha; sub one {}\npackage Beta; sub two {}";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let offset = source.find("two").unwrap_or(0);

        let package = current_package_at(&ast, offset);
        assert_eq!(package, "Beta");
    }
}

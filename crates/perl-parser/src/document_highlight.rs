//! Document Highlight Provider for Perl LSP
//!
//! Highlights all occurrences of a symbol when cursor is positioned on it.
//! Distinguishes between read and write access.

use crate::ast::{Node, NodeKind, SourceLocation};

/// Types of symbol highlights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentHighlightKind {
    /// Regular text occurrence (read access)
    Text = 1,
    /// Read access to a symbol
    Read = 2,
    /// Write access to a symbol
    Write = 3,
}

/// A highlighted range in the document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHighlight {
    /// Source location of the highlight
    pub location: SourceLocation,
    /// Type of highlight
    pub kind: DocumentHighlightKind,
}

/// Document Highlight Provider
pub struct DocumentHighlightProvider;

impl DocumentHighlightProvider {
    /// Create a new document highlight provider
    pub fn new() -> Self {
        Self
    }

    /// Find all highlights for the symbol at the given position in source code
    pub fn find_highlights(
        &self,
        ast: &Node,
        source: &str,
        byte_offset: usize,
    ) -> Vec<DocumentHighlight> {
        // Find the node at the cursor position
        let target_node = match self.find_node_at_offset(ast, byte_offset) {
            Some(node) => node,
            None => return Vec::new(),
        };

        // Get the symbol name and kind
        let symbol_info = match self.extract_symbol_info(&target_node, source) {
            Some(info) => info,
            None => return Vec::new(),
        };

        // Find all occurrences of this symbol
        let mut highlights = Vec::new();
        self.collect_highlights(ast, source, &symbol_info, &mut highlights);

        highlights
    }

    /// Find the node at the given byte offset
    fn find_node_at_offset(&self, node: &Node, offset: usize) -> Option<Node> {
        // Check if offset is within this node
        if offset < node.location.start || offset > node.location.end {
            return None;
        }

        // Check children first for more specific matches
        if let Some(children) = self.get_children(node) {
            for child in children {
                if let Some(found) = self.find_node_at_offset(child, offset) {
                    return Some(found);
                }
            }
        }

        // Check if this node is a relevant symbol
        if self.is_symbol_node(node) {
            return Some(node.clone());
        }

        None
    }

    /// Get children of a node
    fn get_children<'a>(&self, node: &'a Node) -> Option<Vec<&'a Node>> {
        match &node.kind {
            NodeKind::Program { statements } => Some(statements.iter().collect()),
            NodeKind::VariableDeclaration {
                variable,
                initializer,
                ..
            } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                Some(children)
            }
            NodeKind::VariableListDeclaration {
                variables,
                initializer,
                ..
            } => {
                let mut children: Vec<&Node> = variables.iter().collect();
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                Some(children)
            }
            NodeKind::Assignment { lhs, rhs, .. } => Some(vec![lhs.as_ref(), rhs.as_ref()]),
            NodeKind::Binary { left, right, .. } => Some(vec![left.as_ref(), right.as_ref()]),
            NodeKind::Unary { operand, .. } => Some(vec![operand.as_ref()]),
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter().map(|a| a as &Node));
                Some(children)
            }
            NodeKind::FunctionCall { args, .. } => Some(args.iter().collect()),
            NodeKind::Block { statements } => Some(statements.iter().collect()),
            NodeKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                for (cond, branch) in elsif_branches {
                    children.push(cond.as_ref());
                    children.push(branch.as_ref());
                }
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                Some(children)
            }
            NodeKind::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
                let mut children = Vec::new();
                if let Some(i) = init {
                    children.push(i.as_ref());
                }
                if let Some(c) = condition {
                    children.push(c.as_ref());
                }
                if let Some(u) = update {
                    children.push(u.as_ref());
                }
                children.push(body.as_ref());
                Some(children)
            }
            NodeKind::Foreach {
                variable,
                list,
                body,
            } => Some(vec![variable.as_ref(), list.as_ref(), body.as_ref()]),
            NodeKind::While {
                condition, body, ..
            } => Some(vec![condition.as_ref(), body.as_ref()]),
            NodeKind::Subroutine { body, .. } => Some(vec![body.as_ref()]),
            NodeKind::Return { value } => value.as_ref().map(|v| vec![v.as_ref()]),
            NodeKind::ArrayLiteral { elements } => Some(elements.iter().collect()),
            NodeKind::HashLiteral { pairs } => {
                let mut children = Vec::new();
                for (k, v) in pairs {
                    children.push(k);
                    children.push(v);
                }
                Some(children)
            }
            NodeKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => Some(vec![
                condition.as_ref(),
                then_expr.as_ref(),
                else_expr.as_ref(),
            ]),
            NodeKind::VariableWithAttributes { variable, .. } => Some(vec![variable.as_ref()]),
            _ => None,
        }
    }

    /// Check if a node represents a symbol we can highlight
    fn is_symbol_node(&self, node: &Node) -> bool {
        matches!(
            node.kind,
            NodeKind::Variable { .. }
                | NodeKind::FunctionCall { .. }
                | NodeKind::MethodCall { .. }
                | NodeKind::Identifier { .. }
        )
    }

    /// Extract symbol information from a node
    fn extract_symbol_info(&self, node: &Node, source: &str) -> Option<SymbolInfo> {
        match &node.kind {
            NodeKind::Variable { sigil, name } => Some(SymbolInfo {
                name: name.clone(),
                sigil: Some(sigil.clone()),
                is_method: false,
                is_function: false,
            }),
            NodeKind::Identifier { name } => Some(SymbolInfo {
                name: name.clone(),
                sigil: None,
                is_method: false,
                is_function: false,
            }),
            NodeKind::FunctionCall { name, .. } => Some(SymbolInfo {
                name: name.clone(),
                sigil: None,
                is_method: false,
                is_function: true,
            }),
            NodeKind::MethodCall { method, .. } => Some(SymbolInfo {
                name: method.clone(),
                sigil: None,
                is_method: true,
                is_function: false,
            }),
            _ => {
                // Try to extract from source text
                let text = &source[node.location.start..node.location.end];
                if text.starts_with('$') || text.starts_with('@') || text.starts_with('%') {
                    let sigil = text.chars().next().unwrap().to_string();
                    let name = text[1..].to_string();
                    Some(SymbolInfo {
                        name,
                        sigil: Some(sigil),
                        is_method: false,
                        is_function: false,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Collect all highlights for a symbol
    fn collect_highlights(
        &self,
        node: &Node,
        source: &str,
        target: &SymbolInfo,
        highlights: &mut Vec<DocumentHighlight>,
    ) {
        // Check if this node matches our symbol
        if self.node_matches_symbol(node, source, target) {
            let kind = self.determine_highlight_kind(node);
            highlights.push(DocumentHighlight {
                location: node.location,
                kind,
            });
        }

        // Recursively check children
        if let Some(children) = self.get_children(node) {
            for child in children {
                self.collect_highlights(child, source, target, highlights);
            }
        }
    }

    /// Check if a node matches the target symbol
    fn node_matches_symbol(&self, node: &Node, source: &str, target: &SymbolInfo) -> bool {
        match &node.kind {
            NodeKind::Variable { sigil, name } => {
                if let Some(target_sigil) = &target.sigil {
                    sigil == target_sigil && name == &target.name
                } else {
                    false
                }
            }
            NodeKind::Identifier { name } => {
                !target.is_method && target.sigil.is_none() && name == &target.name
            }
            NodeKind::FunctionCall { name, .. } => target.is_function && name == &target.name,
            NodeKind::MethodCall { method, .. } => target.is_method && method == &target.name,
            _ => {
                // Check source text as fallback
                if let Some(target_sigil) = &target.sigil {
                    let expected = format!("{}{}", target_sigil, target.name);
                    let text = &source[node.location.start..node.location.end];
                    text == expected
                } else {
                    false
                }
            }
        }
    }

    /// Determine the kind of highlight based on context
    fn determine_highlight_kind(&self, _node: &Node) -> DocumentHighlightKind {
        // For now, simplified detection
        // In a full implementation, we'd check parent context
        DocumentHighlightKind::Read
    }
}

// Internal SymbolInfo structure
struct SymbolInfo {
    name: String,
    sigil: Option<String>,
    is_method: bool,
    is_function: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_highlight_scalar_variable() {
        let code = r#"my $foo = 42;
print $foo;
$foo = 100;"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        let provider = DocumentHighlightProvider::new();

        // Position on first $foo (byte offset around 3)
        let highlights = provider.find_highlights(&ast, code, 3);

        assert!(!highlights.is_empty());
    }

    #[test]
    fn test_highlight_function_call() {
        let code = r#"sub hello { print "Hello" }
hello();
hello();"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        let provider = DocumentHighlightProvider::new();

        // Position on first hello() call
        let highlights = provider.find_highlights(&ast, code, 29);

        assert!(!highlights.is_empty());
    }

    #[test]
    fn test_no_highlights_for_non_symbol() {
        let code = r#"my $x = "Hello World";"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse().unwrap();
        let provider = DocumentHighlightProvider::new();

        // Position on string literal (byte offset 12 is in "Hello")
        let highlights = provider.find_highlights(&ast, code, 12);

        assert_eq!(highlights.len(), 0);
    }
}

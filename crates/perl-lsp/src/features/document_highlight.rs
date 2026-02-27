//! Document Highlight Provider for Perl LSP
//!
//! Highlights all occurrences of a symbol when cursor is positioned on it.
//! Distinguishes between read and write access.

use perl_parser::ast::{Node, NodeKind, SourceLocation};

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

impl Default for DocumentHighlightProvider {
    fn default() -> Self {
        Self::new()
    }
}

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

        // Deduplicate highlights by location, preferring Write over Read
        self.deduplicate_highlights(highlights)
    }

    /// Deduplicate highlights by location, preferring Write kind over Read
    fn deduplicate_highlights(&self, highlights: Vec<DocumentHighlight>) -> Vec<DocumentHighlight> {
        use std::collections::HashMap;

        // Group by location (start, end)
        let mut by_location: HashMap<(usize, usize), DocumentHighlight> = HashMap::new();

        for h in highlights {
            let key = (h.location.start, h.location.end);
            by_location
                .entry(key)
                .and_modify(|existing| {
                    // Prefer Write (3) over Read (2) over Text (1)
                    if (h.kind as u8) > (existing.kind as u8) {
                        *existing = h.clone();
                    }
                })
                .or_insert(h);
        }

        // Return sorted by position
        let mut result: Vec<_> = by_location.into_values().collect();
        result.sort_by_key(|h| h.location.start);
        result
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
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                Some(children)
            }
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
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
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
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
            NodeKind::For { init, condition, update, body, .. } => {
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
            NodeKind::Foreach { variable, list, body, continue_block } => {
                if let Some(cb) = continue_block {
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref(), cb.as_ref()])
                } else {
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref()])
                }
            }
            NodeKind::While { condition, body, .. } => {
                Some(vec![condition.as_ref(), body.as_ref()])
            }
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
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                Some(vec![condition.as_ref(), then_expr.as_ref(), else_expr.as_ref()])
            }
            NodeKind::VariableWithAttributes { variable, .. } => Some(vec![variable.as_ref()]),
            NodeKind::ExpressionStatement { expression } => Some(vec![expression.as_ref()]),
            // Statement modifiers (Issue #191)
            NodeKind::StatementModifier { statement, condition, .. } => {
                Some(vec![statement.as_ref(), condition.as_ref()])
            }
            // Regex operations - only expr is a child node, patterns are strings (Issue #191)
            NodeKind::Match { expr, .. }
            | NodeKind::Substitution { expr, .. }
            | NodeKind::Transliteration { expr, .. } => Some(vec![expr.as_ref()]),
            // Control flow (Issue #191)
            NodeKind::Given { expr, body } => Some(vec![expr.as_ref(), body.as_ref()]),
            NodeKind::When { condition, body } => Some(vec![condition.as_ref(), body.as_ref()]),
            NodeKind::Default { body } => Some(vec![body.as_ref()]),
            NodeKind::LabeledStatement { statement, .. } => Some(vec![statement.as_ref()]),
            // Code evaluation (Issue #191)
            NodeKind::Eval { block } | NodeKind::Do { block } => Some(vec![block.as_ref()]),
            // Error handling (Issue #191)
            NodeKind::Try { body, catch_blocks, finally_block } => {
                let mut children = vec![body.as_ref()];
                for (_, catch_body) in catch_blocks {
                    children.push(catch_body.as_ref());
                }
                if let Some(finally) = finally_block {
                    children.push(finally.as_ref());
                }
                Some(children)
            }
            // Method declarations (Issue #191)
            NodeKind::Method { body, .. } => Some(vec![body.as_ref()]),
            // Indirect calls (Issue #191)
            NodeKind::IndirectCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                Some(children)
            }
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
                // Check for sigil prefix and extract safely
                let first = text.chars().next();
                match first {
                    Some(sigil @ ('$' | '@' | '%')) => Some(SymbolInfo {
                        name: text.get(1..).unwrap_or("").to_string(),
                        sigil: Some(sigil.to_string()),
                        is_method: false,
                        is_function: false,
                    }),
                    _ => None,
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
        self.collect_highlights_with_parent(node, source, target, highlights, None);
    }

    /// Collect all highlights for a symbol with parent context
    fn collect_highlights_with_parent(
        &self,
        node: &Node,
        source: &str,
        target: &SymbolInfo,
        highlights: &mut Vec<DocumentHighlight>,
        parent: Option<&Node>,
    ) {
        // Check if this node matches our symbol
        if self.node_matches_symbol(node, source, target) {
            let kind = self.determine_highlight_kind_with_parent(node, parent);
            // Use the full location including the sigil
            highlights.push(DocumentHighlight { location: node.location, kind });
        }

        // Recursively check children with this node as parent
        if let Some(children) = self.get_children(node) {
            for child in children {
                self.collect_highlights_with_parent(child, source, target, highlights, Some(node));
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

    /// Determine the kind of highlight based on context with parent information
    fn determine_highlight_kind_with_parent(
        &self,
        node: &Node,
        parent: Option<&Node>,
    ) -> DocumentHighlightKind {
        // Check if this variable is being written to (declaration or assignment)
        // Look for parent nodes that indicate write access
        match &node.kind {
            NodeKind::Variable { .. } => {
                // Check parent context to determine if this is a write or read
                if let Some(parent_node) = parent {
                    match &parent_node.kind {
                        // Variable declarations are writes
                        NodeKind::VariableDeclaration { variable, .. } => {
                            if std::ptr::eq(variable.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Variables in list declarations are writes
                        NodeKind::VariableListDeclaration { variables, .. } => {
                            if variables.iter().any(|v| std::ptr::eq(v, node)) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Left side of assignment is write
                        NodeKind::Assignment { lhs, .. } => {
                            if std::ptr::eq(lhs.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Increment/decrement operations are writes
                        NodeKind::Unary { op, operand, .. } => {
                            if (op == "++" || op == "--") && std::ptr::eq(operand.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Default to read for other contexts
                        _ => DocumentHighlightKind::Read,
                    }
                } else {
                    // If we don't have parent context, default to read
                    DocumentHighlightKind::Read
                }
            }
            _ => DocumentHighlightKind::Read,
        }
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
    fn test_highlight_scalar_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $foo = 42;
print $foo;
$foo = 100;"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = DocumentHighlightProvider::new();

        // Position on first $foo (byte offset around 3)
        let highlights = provider.find_highlights(&ast, code, 3);

        assert!(!highlights.is_empty());
        Ok(())
    }

    #[test]
    fn test_highlight_function_call() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"sub hello { print "Hello" }
hello();
hello();"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = DocumentHighlightProvider::new();

        // Position on first hello() call (byte offset 29)
        let highlights = provider.find_highlights(&ast, code, 29);

        // Should find both hello() calls (fixed in Issue #191)
        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights for function calls, found {}",
            highlights.len()
        );
        Ok(())
    }

    #[test]
    fn test_no_highlights_for_non_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"my $x = "Hello World";"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = DocumentHighlightProvider::new();

        // Position on string literal (byte offset 12 is in "Hello")
        let highlights = provider.find_highlights(&ast, code, 12);

        assert_eq!(highlights.len(), 0);
        Ok(())
    }

    #[test]
    fn test_highlight_statement_modifier() -> Result<(), Box<dyn std::error::Error>> {
        // Test statement modifiers with new AST structure (Issue #191)
        let code = r#"my $x = 5;
print $x if $x > 0;"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = DocumentHighlightProvider::new();

        // Position on first $x
        let highlights = provider.find_highlights(&ast, code, 3);

        // Should find all 3 occurrences of $x
        assert!(
            highlights.len() >= 3,
            "Expected at least 3 highlights for $x, found {}",
            highlights.len()
        );
        Ok(())
    }
}

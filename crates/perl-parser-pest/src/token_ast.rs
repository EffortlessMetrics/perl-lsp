//! Simplified AST for the token-based parser

use std::sync::Arc;

/// Simple AST node structure for token parser
#[derive(Debug, Clone, PartialEq)]
pub struct AstNode {
    pub node_type: String,
    pub start_position: usize,
    pub end_position: usize,
    pub value: Option<Arc<str>>,
    pub children: Vec<AstNode>,
}

impl AstNode {
    /// Convert to S-expression format
    pub fn to_sexp(&self) -> String {
        if self.children.is_empty() {
            if let Some(value) = &self.value {
                format!("({} \"{}\")", self.node_type, value)
            } else {
                format!("({})", self.node_type)
            }
        } else {
            let children_sexp: Vec<String> =
                self.children.iter().map(|child| child.to_sexp()).collect();
            format!("({} {})", self.node_type, children_sexp.join(" "))
        }
    }
}

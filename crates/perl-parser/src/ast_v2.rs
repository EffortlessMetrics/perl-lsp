//! Enhanced AST with full position tracking for incremental parsing
//!
//! This module provides an updated AST that uses Range instead of SourceLocation
//! to support incremental parsing and better error reporting.

use crate::position::Range;

/// A unique identifier for AST nodes to support incremental parsing
pub type NodeId = usize;

/// Enhanced AST node with full position tracking
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Unique identifier for this node
    pub id: NodeId,
    /// The kind of syntax node
    pub kind: NodeKind,
    /// Source range with line/column information
    pub range: Range,
}

impl Node {
    /// Create a new AST node
    pub fn new(id: NodeId, kind: NodeKind, range: Range) -> Self {
        Node { id, kind, range }
    }
    
    /// Convert to tree-sitter compatible S-expression
    pub fn to_sexp(&self) -> String {
        // Delegate to existing implementation
        self.kind.to_sexp()
    }
}

/// Node kinds - same as original but can be extended
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // Program structure
    Program { statements: Vec<Node> },
    Block { statements: Vec<Node> },
    
    // Declarations
    VariableDeclaration {
        declarator: String, // my, our, local, state
        variable: Box<Node>,
        attributes: Vec<String>,
        initializer: Option<Box<Node>>,
    },
    
    VariableListDeclaration {
        declarator: String,
        variables: Vec<Node>,
        attributes: Vec<String>,
        initializer: Option<Box<Node>>,
    },
    
    // Variables
    Variable {
        sigil: String, // $, @, %, *, &
        name: String,
    },
    
    // Error recovery nodes
    Error {
        message: String,
        expected: Vec<String>,
        partial: Option<Box<Node>>,
    },
    
    MissingExpression,
    MissingStatement,
    MissingIdentifier,
    MissingBlock,
    
    // Include all other variants from original AST...
    // (Abbreviated for example - would include all original variants)
    
    // Expressions
    Binary {
        op: String,
        left: Box<Node>,
        right: Box<Node>,
    },
    
    Unary {
        op: String,
        operand: Box<Node>,
    },
    
    // Control flow
    If {
        condition: Box<Node>,
        then_branch: Box<Node>,
        elsif_branches: Vec<(Node, Node)>,
        else_branch: Option<Box<Node>>,
    },
    
    // Literals
    Number { value: String },
    String { value: String, interpolated: bool },
    Identifier { name: String },
    
    // Other essential variants...
}

impl NodeKind {
    /// Convert to S-expression format
    pub fn to_sexp(&self) -> String {
        use NodeKind::*;
        
        match self {
            Program { statements } => {
                let stmts = statements
                    .iter()
                    .map(|s| s.to_sexp())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(program {})", stmts)
            }
            
            Block { statements } => {
                let stmts = statements
                    .iter()
                    .map(|s| s.to_sexp())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(block {})", stmts)
            }
            
            Variable { sigil, name } => {
                format!("(variable {} {})", sigil, name)
            }
            
            Number { value } => format!("(number {})", value),
            
            String { value, interpolated } => {
                if *interpolated {
                    format!("(string_interpolated {:?})", value)
                } else {
                    format!("(string {:?})", value)
                }
            }
            
            Binary { op, left, right } => {
                format!("(binary_{} {} {})", op, left.to_sexp(), right.to_sexp())
            }
            
            Error { message, .. } => format!("(ERROR {})", message),
            
            MissingExpression => "(MISSING_EXPRESSION)".to_string(),
            MissingStatement => "(MISSING_STATEMENT)".to_string(),
            
            // Add other variants...
            _ => format!("({:?})", self),
        }
    }
}

/// Node ID generator for unique identifiers
pub struct NodeIdGenerator {
    next_id: NodeId,
}

impl NodeIdGenerator {
    pub fn new() -> Self {
        NodeIdGenerator { next_id: 0 }
    }
    
    pub fn next(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Default for NodeIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    
    #[test]
    fn test_node_creation() {
        let mut id_gen = NodeIdGenerator::new();
        let range = Range::new(
            Position::new(0, 1, 1),
            Position::new(5, 1, 6)
        );
        
        let node = Node::new(
            id_gen.next(),
            NodeKind::Number { value: "42".to_string() },
            range
        );
        
        assert_eq!(node.id, 0);
        assert_eq!(node.to_sexp(), "(number 42)");
    }
    
    #[test]
    fn test_error_nodes() {
        let mut id_gen = NodeIdGenerator::new();
        let range = Range::new(
            Position::new(0, 1, 1),
            Position::new(0, 1, 1)
        );
        
        let error = Node::new(
            id_gen.next(),
            NodeKind::Error {
                message: "Unexpected token".to_string(),
                expected: vec!["identifier".to_string()],
                partial: None,
            },
            range
        );
        
        assert_eq!(error.to_sexp(), "(ERROR Unexpected token)");
    }
}
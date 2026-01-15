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

/// The kinds of AST nodes used by the parser.
///
/// Each variant represents a specific syntactic construct in the Perl source
/// and carries the child nodes or data needed to represent that construct.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // Program structure
    /// Top-level program containing a list of statements.
    Program {
        /// Statements contained by the program/root node.
        statements: Vec<Node>,
    },
    /// Block node containing a list of statements.
    Block {
        /// Statements inside a block.
        statements: Vec<Node>,
    },

    // Declarations
    /// A single variable declaration (`my`, `our`, `local`, `state`, ...).
    VariableDeclaration {
        /// The declarator keyword (e.g. `my`, `our`).
        declarator: String, // my, our, local, state
        /// The variable node being declared.
        variable: Box<Node>,
        /// Any attributes attached to the declaration.
        attributes: Vec<String>,
        /// Optional initializer expression.
        initializer: Option<Box<Node>>,
    },

    /// A list-style variable declaration (e.g. `my ($a, $b) = ...`).
    VariableListDeclaration {
        /// The declarator keyword.
        declarator: String,
        /// Variables declared in the list.
        variables: Vec<Node>,
        /// Any attributes attached to the declaration.
        attributes: Vec<String>,
        /// Optional initializer for the list.
        initializer: Option<Box<Node>>,
    },

    // Variables
    /// A variable usage with sigil and name (e.g. `$foo`, `@arr`).
    Variable {
        /// The sigil character (e.g. `$`, `@`, `%`).
        sigil: String, // $, @, %, *
        /// The identifier/name of the variable.
        name: String,
    },

    // Error recovery nodes
    /// An error/recovery node produced during parsing.
    Error {
        /// Human readable error message.
        message: String,
        /// Tokens or node kinds that were expected at this location.
        expected: Vec<String>,
        /// Optional partially parsed node for recovery contexts.
        partial: Option<Box<Node>>,
    },

    /// Placeholder for a missing expression during error recovery.
    MissingExpression,
    /// Placeholder for a missing statement during error recovery.
    MissingStatement,
    /// Placeholder for a missing identifier during error recovery.
    MissingIdentifier,
    /// Placeholder for a missing block during error recovery.
    MissingBlock,

    // Include all other variants from original AST...
    // (Abbreviated for example - would include all original variants)

    // Expressions
    /// A binary expression (e.g. `a + b`).
    Binary {
        /// The operator token as text.
        op: String,
        /// Left-hand side expression.
        left: Box<Node>,
        /// Right-hand side expression.
        right: Box<Node>,
    },

    /// A unary expression (e.g. `-x`, `!flag`).
    Unary {
        /// The operator token.
        op: String,
        /// The operand expression.
        operand: Box<Node>,
    },

    // Control flow
    /// An `if` control-flow construct, including `elsif` and `else` branches.
    If {
        /// The conditional expression.
        condition: Box<Node>,
        /// The then-branch block node.
        then_branch: Box<Node>,
        /// Zero or more `elsif` branches represented as (condition, block).
        elsif_branches: Vec<(Node, Node)>,
        /// Optional else branch.
        else_branch: Option<Box<Node>>,
    },

    // Literals
    /// Numeric literal node.
    Number {
        /// The literal text of the number.
        value: String,
    },
    /// String literal node; may be interpolated.
    String {
        /// The string contents.
        value: String,
        /// Whether the string contains interpolation.
        interpolated: bool,
    },
    /// An identifier token.
    Identifier {
        /// The identifier text.
        name: String,
    },
    // Other essential variants...
}

impl NodeKind {
    /// Convert to S-expression format
    pub fn to_sexp(&self) -> String {
        use NodeKind::*;

        match self {
            Program { statements } => {
                let stmts = statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(source_file {})", stmts)
            }

            Block { statements } => {
                let stmts = statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ");
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

/// Generator for producing unique `NodeId` values used across the AST.
///
/// This utility ensures each constructed `Node` receives a distinct identifier
/// which is useful for incremental parsing, diffing and node references.
pub struct NodeIdGenerator {
    /// The next identifier to hand out.
    next_id: NodeId,
}

impl NodeIdGenerator {
    /// Create a new `NodeIdGenerator` starting at zero.
    pub fn new() -> Self {
        NodeIdGenerator { next_id: 0 }
    }

    /// Return the next unique `NodeId` and advance the generator.
    pub fn next_id(&mut self) -> NodeId {
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
        let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));

        let node = Node::new(id_gen.next_id(), NodeKind::Number { value: "42".to_string() }, range);

        assert_eq!(node.id, 0);
        assert_eq!(node.to_sexp(), "(number 42)");
    }

    #[test]
    fn test_error_nodes() {
        let mut id_gen = NodeIdGenerator::new();
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));

        let error = Node::new(
            id_gen.next_id(),
            NodeKind::Error {
                message: "Unexpected token".to_string(),
                expected: vec!["identifier".to_string()],
                partial: None,
            },
            range,
        );

        assert_eq!(error.to_sexp(), "(ERROR Unexpected token)");
    }
}

//! Enhanced AST with full position tracking for incremental parsing
//!
//! This module provides an updated AST that uses Range instead of SourceLocation
//! to support incremental parsing and better error reporting.

use perl_position_tracking::Range;

/// A unique identifier for AST nodes to support incremental parsing.
pub type NodeId = usize;

/// Index into the diagnostics array in `ParseOutput`.
///
/// This type enables lightweight error nodes that reference diagnostics
/// stored separately from the AST, reducing memory overhead and decoupling
/// tree structure from human-readable messages.
pub type DiagnosticId = u32;

/// Kinds of missing syntax elements for error recovery.
///
/// This enum provides specific information about what was expected
/// but not found, enabling better IDE diagnostics and recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingKind {
    /// Missing expression (e.g., after `=` in assignment)
    Expression,
    /// Missing statement
    Statement,
    /// Missing identifier/name
    Identifier,
    /// Missing block `{ ... }`
    Block,
    /// Missing closing delimiter
    ClosingDelimiter(char),
    /// Missing semicolon
    Semicolon,
    /// Missing condition (e.g., in `if`)
    Condition,
    /// Missing argument
    Argument,
    /// Missing operator
    Operator,
}

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
    /// An error/recovery node produced during parsing (legacy, rich payload).
    ///
    /// This variant embeds the error information directly in the AST node.
    /// For new code, prefer `ErrorRef` which stores only a diagnostic index.
    Error {
        /// Human readable error message.
        message: String,
        /// Tokens or node kinds that were expected at this location.
        expected: Vec<String>,
        /// Optional partially parsed node for recovery contexts.
        partial: Option<Box<Node>>,
    },

    /// Lightweight error node referencing a diagnostic by index.
    ///
    /// This is the preferred error representation for memory efficiency.
    /// The actual diagnostic information is stored in `ParseOutput.diagnostics`
    /// and can be looked up by the `diag_id`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = parser.parse_with_recovery();
    /// for node in output.ast.walk() {
    ///     if let NodeKind::ErrorRef { diag_id } = &node.kind {
    ///         let diagnostic = &output.diagnostics[*diag_id as usize];
    ///         println!("Error at {:?}: {}", node.range, diagnostic);
    ///     }
    /// }
    /// ```
    ErrorRef {
        /// Index into the diagnostics array in `ParseOutput`.
        diag_id: DiagnosticId,
    },

    /// Placeholder for a missing expression during error recovery.
    MissingExpression,
    /// Placeholder for a missing statement during error recovery.
    MissingStatement,
    /// Placeholder for a missing identifier during error recovery.
    MissingIdentifier,
    /// Placeholder for a missing block during error recovery.
    MissingBlock,

    /// Specific kind of missing syntax element.
    ///
    /// This provides more granular information about what's missing
    /// without embedding full diagnostic details in the AST.
    Missing(MissingKind),

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

            Identifier { name } => format!("(identifier {})", name),

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

            Unary { op, operand } => format!("(unary_{} {})", op, operand.to_sexp()),

            If { condition, then_branch, elsif_branches, else_branch } => {
                let mut s = format!("(if {} {}", condition.to_sexp(), then_branch.to_sexp());
                for (elsif_cond, elsif_block) in elsif_branches {
                    s.push_str(&format!(
                        " (elsif {} {})",
                        elsif_cond.to_sexp(),
                        elsif_block.to_sexp()
                    ));
                }
                if let Some(eb) = else_branch {
                    s.push_str(&format!(" (else {})", eb.to_sexp()));
                }
                s.push(')');
                s
            }

            VariableDeclaration { declarator, variable, initializer, .. } => match initializer {
                Some(init) => format!(
                    "(variable_declaration {} {} {})",
                    declarator,
                    variable.to_sexp(),
                    init.to_sexp()
                ),
                None => format!("(variable_declaration {} {})", declarator, variable.to_sexp()),
            },

            VariableListDeclaration { declarator, variables, initializer, .. } => {
                let vars = variables.iter().map(|v| v.to_sexp()).collect::<Vec<_>>().join(" ");
                match initializer {
                    Some(init) => format!(
                        "(variable_list_declaration {} {} {})",
                        declarator,
                        vars,
                        init.to_sexp()
                    ),
                    None => format!("(variable_list_declaration {} {})", declarator, vars),
                }
            }

            Error { message, .. } => format!("(ERROR {})", message),
            ErrorRef { diag_id } => format!("(ERROR_REF #{})", diag_id),

            MissingExpression => "(MISSING_EXPRESSION)".to_string(),
            MissingStatement => "(MISSING_STATEMENT)".to_string(),
            MissingIdentifier => "(MISSING_IDENTIFIER)".to_string(),
            MissingBlock => "(MISSING_BLOCK)".to_string(),
            Missing(kind) => format!("(MISSING {:?})", kind),
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
    use perl_position_tracking::{Position, Range};

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

    // --- Tests for previously-unhandled variants (wildcard was silently swallowing these) ---

    #[test]
    fn test_identifier_sexp_explicit_not_debug() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(3, 1, 4));
        let node = Node::new(0, NodeKind::Identifier { name: "foo".to_string() }, range);
        let sexp = node.to_sexp();
        // Must produce the proper form, NOT the old wildcard Debug output like `(Identifier { name: "foo" })`
        assert_eq!(sexp, "(identifier foo)");
        assert!(!sexp.contains('{'), "sexp must not contain Debug struct syntax: {sexp}");
        Ok(())
    }

    #[test]
    fn test_unary_sexp_explicit() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(2, 1, 3));
        let inner = Node::new(0, NodeKind::Number { value: "1".to_string() }, range.clone());
        let node =
            Node::new(1, NodeKind::Unary { op: "!".to_string(), operand: Box::new(inner) }, range);
        let sexp = node.to_sexp();
        assert_eq!(sexp, "(unary_! (number 1))");
        assert!(!sexp.contains('{'), "sexp must not contain Debug struct syntax: {sexp}");
        Ok(())
    }

    #[test]
    fn test_if_sexp_no_branches() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let cond = Node::new(0, NodeKind::Number { value: "1".to_string() }, range.clone());
        let then_block = Node::new(1, NodeKind::Block { statements: vec![] }, range.clone());
        let node = Node::new(
            2,
            NodeKind::If {
                condition: Box::new(cond),
                then_branch: Box::new(then_block),
                elsif_branches: vec![],
                else_branch: None,
            },
            range,
        );
        let sexp = node.to_sexp();
        assert_eq!(sexp, "(if (number 1) (block ))");
        assert!(!sexp.contains('{'), "sexp must not contain Debug struct syntax: {sexp}");
        Ok(())
    }

    #[test]
    fn test_if_sexp_with_elsif_and_else() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let cond = Node::new(0, NodeKind::Number { value: "1".to_string() }, range.clone());
        let then_block = Node::new(1, NodeKind::Block { statements: vec![] }, range.clone());
        let elsif_cond = Node::new(2, NodeKind::Number { value: "0".to_string() }, range.clone());
        let elsif_block = Node::new(3, NodeKind::Block { statements: vec![] }, range.clone());
        let else_block = Node::new(4, NodeKind::Block { statements: vec![] }, range.clone());
        let node = Node::new(
            5,
            NodeKind::If {
                condition: Box::new(cond),
                then_branch: Box::new(then_block),
                elsif_branches: vec![(elsif_cond, elsif_block)],
                else_branch: Some(Box::new(else_block)),
            },
            range,
        );
        let sexp = node.to_sexp();
        assert_eq!(sexp, "(if (number 1) (block ) (elsif (number 0) (block )) (else (block )))");
        Ok(())
    }

    #[test]
    fn test_variable_declaration_sexp_no_init() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let var = Node::new(
            0,
            NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
            range.clone(),
        );
        let node = Node::new(
            1,
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var),
                attributes: vec![],
                initializer: None,
            },
            range,
        );
        let sexp = node.to_sexp();
        assert_eq!(sexp, "(variable_declaration my (variable $ x))");
        assert!(!sexp.contains('{'), "sexp must not contain Debug struct syntax: {sexp}");
        Ok(())
    }

    #[test]
    fn test_variable_declaration_sexp_with_init() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let var = Node::new(
            0,
            NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
            range.clone(),
        );
        let init = Node::new(1, NodeKind::Number { value: "42".to_string() }, range.clone());
        let node = Node::new(
            2,
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var),
                attributes: vec![],
                initializer: Some(Box::new(init)),
            },
            range,
        );
        assert_eq!(node.to_sexp(), "(variable_declaration my (variable $ x) (number 42))");
        Ok(())
    }

    #[test]
    fn test_variable_list_declaration_sexp_no_init() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let var_a = Node::new(
            0,
            NodeKind::Variable { sigil: "$".to_string(), name: "a".to_string() },
            range.clone(),
        );
        let var_b = Node::new(
            1,
            NodeKind::Variable { sigil: "$".to_string(), name: "b".to_string() },
            range.clone(),
        );
        let node = Node::new(
            2,
            NodeKind::VariableListDeclaration {
                declarator: "my".to_string(),
                variables: vec![var_a, var_b],
                attributes: vec![],
                initializer: None,
            },
            range,
        );
        let sexp = node.to_sexp();
        assert_eq!(sexp, "(variable_list_declaration my (variable $ a) (variable $ b))");
        assert!(!sexp.contains('{'), "sexp must not contain Debug struct syntax: {sexp}");
        Ok(())
    }

    #[test]
    fn test_variable_list_declaration_sexp_with_init() -> Result<(), std::convert::Infallible> {
        let range = Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1));
        let var_a = Node::new(
            0,
            NodeKind::Variable { sigil: "$".to_string(), name: "a".to_string() },
            range.clone(),
        );
        let init = Node::new(1, NodeKind::Number { value: "0".to_string() }, range.clone());
        let node = Node::new(
            2,
            NodeKind::VariableListDeclaration {
                declarator: "our".to_string(),
                variables: vec![var_a],
                attributes: vec![],
                initializer: Some(Box::new(init)),
            },
            range,
        );
        assert_eq!(node.to_sexp(), "(variable_list_declaration our (variable $ a) (number 0))");
        Ok(())
    }
}

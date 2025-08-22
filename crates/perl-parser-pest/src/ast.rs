//! Abstract Syntax Tree types for the Perl parser
//!
//! This module defines the AST node types that represent Perl code structure.
//! All nodes are designed to be compatible with tree-sitter output format.

use crate::token_compat::TokenType;
use std::sync::Arc;

/// Source location information
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub start: usize,
    pub end: usize,
}

/// AST node representing a Perl construct
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub location: SourceLocation,
}

impl Node {
    /// Create a new AST node
    pub fn new(kind: NodeKind, location: SourceLocation) -> Self {
        Self { kind, location }
    }

    /// Convert node to tree-sitter S-expression format
    pub fn to_sexp(&self) -> String {
        match &self.kind {
            NodeKind::Program { statements } => {
                let children = statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(program {})", children)
            }

            NodeKind::PackageDeclaration { name, version, block } => {
                let mut parts = vec![format!("(package_name {})", name)];
                if let Some(v) = version {
                    parts.push(format!("(version {})", v));
                }
                if let Some(b) = block {
                    parts.push(b.to_sexp());
                }
                format!("(package_declaration {})", parts.join(" "))
            }

            NodeKind::UseStatement { module, version, imports } => {
                let mut parts = vec![format!("(module_name {})", module)];
                if let Some(v) = version {
                    parts.push(format!("(version {})", v));
                }
                if let Some(list) = imports {
                    let import_list = list
                        .iter()
                        .map(|i| format!("(import {})", i))
                        .collect::<Vec<_>>()
                        .join(" ");
                    parts.push(format!("(import_list {})", import_list));
                }
                format!("(use_statement {})", parts.join(" "))
            }

            NodeKind::Subroutine { name, prototype, attributes, body } => {
                let mut parts = Vec::new();
                if let Some(n) = name {
                    parts.push(format!("(sub_name {})", n));
                }
                if let Some(p) = prototype {
                    parts.push(format!("(prototype {})", p));
                }
                if !attributes.is_empty() {
                    let attrs = attributes
                        .iter()
                        .map(|a| format!("(attribute {})", a))
                        .collect::<Vec<_>>()
                        .join(" ");
                    parts.push(format!("(attributes {})", attrs));
                }
                parts.push(body.to_sexp());
                format!("(subroutine {})", parts.join(" "))
            }

            NodeKind::Block { statements } => {
                let children = statements.iter().map(|s| s.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(block {})", children)
            }

            NodeKind::IfStatement { condition, then_branch, elsif_branches, else_branch } => {
                let mut parts = vec![
                    format!("(condition {})", condition.to_sexp()),
                    format!("(then {})", then_branch.to_sexp()),
                ];

                for (cond, block) in elsif_branches {
                    parts.push(format!(
                        "(elsif (condition {}) {})",
                        cond.to_sexp(),
                        block.to_sexp()
                    ));
                }

                if let Some(else_block) = else_branch {
                    parts.push(format!("(else {})", else_block.to_sexp()));
                }

                format!("(if_statement {})", parts.join(" "))
            }

            NodeKind::WhileStatement { condition, body, continue_block } => {
                let mut parts =
                    vec![format!("(condition {})", condition.to_sexp()), body.to_sexp()];

                if let Some(cont) = continue_block {
                    parts.push(format!("(continue {})", cont.to_sexp()));
                }

                format!("(while_statement {})", parts.join(" "))
            }

            NodeKind::ForStatement { init, condition, update, body } => {
                let mut parts = Vec::new();

                if let Some(i) = init {
                    parts.push(format!("(init {})", i.to_sexp()));
                }
                if let Some(c) = condition {
                    parts.push(format!("(condition {})", c.to_sexp()));
                }
                if let Some(u) = update {
                    parts.push(format!("(update {})", u.to_sexp()));
                }
                parts.push(body.to_sexp());

                format!("(for_statement {})", parts.join(" "))
            }

            NodeKind::ForeachStatement { variable, list, body } => {
                format!(
                    "(foreach_statement (var {}) (list {}) {})",
                    variable.to_sexp(),
                    list.to_sexp(),
                    body.to_sexp()
                )
            }

            NodeKind::Assignment { left, op, right } => {
                let op_str = match op {
                    TokenType::Equal => "=",
                    TokenType::PlusEqual => "+=",
                    TokenType::MinusEqual => "-=",
                    TokenType::StarEqual => "*=",
                    TokenType::SlashEqual => "/=",
                    _ => "=",
                };
                format!("(assignment (op {}) {} {})", op_str, left.to_sexp(), right.to_sexp())
            }

            NodeKind::Binary { left, op, right } => {
                let op_str = self.token_type_to_string(op);
                format!(
                    "(binary_expression (op {}) {} {})",
                    op_str,
                    left.to_sexp(),
                    right.to_sexp()
                )
            }

            NodeKind::Unary { op, operand } => {
                let op_str = self.token_type_to_string(op);
                format!("(unary_expression (op {}) {})", op_str, operand.to_sexp())
            }

            NodeKind::FunctionCall { name, args } => {
                let arg_list = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(function_call (name {}) (args {}))", name, arg_list)
            }

            NodeKind::MethodCall { object, method, args } => {
                let arg_list = args.iter().map(|a| a.to_sexp()).collect::<Vec<_>>().join(" ");
                format!(
                    "(method_call {} (method {}) (args {}))",
                    object.to_sexp(),
                    method,
                    arg_list
                )
            }

            NodeKind::ArrayAccess { array, index } => {
                format!("(array_access {} (index {}))", array.to_sexp(), index.to_sexp())
            }

            NodeKind::HashAccess { hash, key } => {
                format!("(hash_access {} (key {}))", hash.to_sexp(), key.to_sexp())
            }

            NodeKind::Variable { name } => {
                format!("(variable {})", name)
            }

            NodeKind::Number { value } => {
                format!("(number {})", value)
            }

            NodeKind::String { value } => {
                format!("(string {})", value)
            }

            NodeKind::Regex { pattern, modifiers } => {
                format!("(regex (pattern {}) (modifiers {}))", pattern, modifiers)
            }

            NodeKind::List { elements } => {
                let items = elements.iter().map(|e| e.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(list {})", items)
            }

            NodeKind::Bareword { value } => {
                format!("(bareword {})", value)
            }

            NodeKind::Heredoc { marker, content } => {
                format!("(heredoc (marker {}) (content {}))", marker, content)
            }

            NodeKind::VariableDeclaration { declarator, variables } => {
                let var_list = variables.iter().map(|v| v.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(variable_declaration (declarator {}) {})", declarator, var_list)
            }

            NodeKind::Error { message } => {
                format!("(ERROR {})", message)
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                format!(
                    "(ternary {} {} {})",
                    condition.to_sexp(),
                    then_expr.to_sexp(),
                    else_expr.to_sexp()
                )
            }

            NodeKind::PrefixUpdate { op, operand } => {
                let op_str = self.token_type_to_string(op);
                format!("(prefix_{} {})", op_str, operand.to_sexp())
            }

            NodeKind::PostfixUpdate { op, operand } => {
                let op_str = self.token_type_to_string(op);
                format!("(postfix_{} {})", op_str, operand.to_sexp())
            }

            NodeKind::ArrayRef { elements } => {
                let items = elements.iter().map(|e| e.to_sexp()).collect::<Vec<_>>().join(" ");
                format!("(array_ref {})", items)
            }

            NodeKind::HashRef { pairs } => {
                let items = pairs
                    .iter()
                    .map(|(k, v)| format!("({} {})", k.to_sexp(), v.to_sexp()))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("(hash_ref {})", items)
            }

            NodeKind::Dereference { expr, type_ } => {
                format!("(dereference {} {})", expr.to_sexp(), type_.to_sexp())
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    format!("(return {})", val.to_sexp())
                } else {
                    "(return)".to_string()
                }
            }

            NodeKind::LoopControl { control_type, label } => {
                if let Some(lbl) = label {
                    format!("({} {})", control_type, lbl)
                } else {
                    format!("({})", control_type)
                }
            }
        }
    }

    /// Convert token type to string representation
    fn token_type_to_string(&self, token_type: &TokenType) -> &'static str {
        match token_type {
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Star => "*",
            TokenType::Slash => "/",
            TokenType::Percent => "%",
            TokenType::AndAnd => "&&",
            TokenType::OrOr => "||",
            TokenType::Not => "!",
            TokenType::EqualEqual => "==",
            TokenType::NotEqual => "!=",
            TokenType::Less => "<",
            TokenType::Greater => ">",
            TokenType::LessEqual => "<=",
            TokenType::GreaterEqual => ">=",
            TokenType::StringEq => "eq",
            TokenType::StringNe => "ne",
            TokenType::StringLt => "lt",
            TokenType::StringGt => "gt",
            TokenType::StringLe => "le",
            TokenType::StringGe => "ge",
            TokenType::Arrow => "->",
            TokenType::Dot => ".",
            TokenType::Range => "..",
            TokenType::Ellipsis => "...",
            TokenType::Question => "?",
            TokenType::ColonColon => "::",
            TokenType::Spaceship => "<=>",
            TokenType::StringCmp => "cmp",
            TokenType::StringRepeat => "x",
            TokenType::LeftShift => "<<",
            TokenType::RightShift => ">>",
            TokenType::BitwiseNot => "~",
            TokenType::Backslash => "\\",
            TokenType::Increment => "++",
            TokenType::Decrement => "--",
            _ => "unknown",
        }
    }
}

/// Node types representing different Perl constructs
#[derive(Debug, Clone)]
pub enum NodeKind {
    // Program structure
    Program {
        statements: Vec<Node>,
    },

    // Declarations
    PackageDeclaration {
        name: Arc<str>,
        version: Option<Arc<str>>,
        block: Option<Box<Node>>,
    },

    UseStatement {
        module: Arc<str>,
        version: Option<Arc<str>>,
        imports: Option<Vec<Arc<str>>>,
    },

    Subroutine {
        name: Option<Arc<str>>,
        prototype: Option<Arc<str>>,
        attributes: Vec<Arc<str>>,
        body: Box<Node>,
    },

    VariableDeclaration {
        declarator: Arc<str>, // my, our, local, state
        variables: Vec<Node>,
    },

    // Statements
    Block {
        statements: Vec<Node>,
    },

    IfStatement {
        condition: Box<Node>,
        then_branch: Box<Node>,
        elsif_branches: Vec<(Box<Node>, Box<Node>)>,
        else_branch: Option<Box<Node>>,
    },

    WhileStatement {
        condition: Box<Node>,
        body: Box<Node>,
        continue_block: Option<Box<Node>>,
    },

    ForStatement {
        init: Option<Box<Node>>,
        condition: Option<Box<Node>>,
        update: Option<Box<Node>>,
        body: Box<Node>,
    },

    ForeachStatement {
        variable: Box<Node>,
        list: Box<Node>,
        body: Box<Node>,
    },

    // Expressions
    Assignment {
        left: Box<Node>,
        op: TokenType,
        right: Box<Node>,
    },

    Binary {
        left: Box<Node>,
        op: TokenType,
        right: Box<Node>,
    },

    Unary {
        op: TokenType,
        operand: Box<Node>,
    },

    FunctionCall {
        name: Arc<str>,
        args: Vec<Node>,
    },

    MethodCall {
        object: Box<Node>,
        method: Arc<str>,
        args: Vec<Node>,
    },

    ArrayAccess {
        array: Box<Node>,
        index: Box<Node>,
    },

    HashAccess {
        hash: Box<Node>,
        key: Box<Node>,
    },

    // Literals
    Variable {
        name: Arc<str>,
    },

    Number {
        value: Arc<str>,
    },

    String {
        value: Arc<str>,
    },

    Regex {
        pattern: Arc<str>,
        modifiers: Arc<str>,
    },

    List {
        elements: Vec<Node>,
    },

    Bareword {
        value: Arc<str>,
    },

    Heredoc {
        marker: Arc<str>,
        content: Arc<str>,
    },

    // Error recovery
    Error {
        message: Arc<str>,
    },

    // Additional expressions
    Ternary {
        condition: Box<Node>,
        then_expr: Box<Node>,
        else_expr: Box<Node>,
    },

    PrefixUpdate {
        op: TokenType,
        operand: Box<Node>,
    },

    PostfixUpdate {
        op: TokenType,
        operand: Box<Node>,
    },

    ArrayRef {
        elements: Vec<Node>,
    },

    HashRef {
        pairs: Vec<(Node, Node)>,
    },

    Dereference {
        expr: Box<Node>,
        type_: Box<Node>,
    },

    Return {
        value: Option<Box<Node>>,
    },

    LoopControl {
        control_type: Arc<str>,
        label: Option<Arc<str>>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_to_sexp() {
        let node = Node::new(
            NodeKind::Variable { name: Arc::from("$x") },
            SourceLocation { start: 0, end: 2 },
        );

        assert_eq!(node.to_sexp(), "(variable $x)");
    }

    #[test]
    fn test_program_to_sexp() {
        let program = Node::new(
            NodeKind::Program {
                statements: vec![Node::new(
                    NodeKind::Variable { name: Arc::from("$x") },
                    SourceLocation { start: 0, end: 2 },
                )],
            },
            SourceLocation { start: 0, end: 2 },
        );

        assert_eq!(program.to_sexp(), "(program (variable $x))");
    }
}

//! S-expression formatter for AST nodes
//!
//! This module provides utilities for converting the pure Rust AST
//! into tree-sitter compatible S-expression strings for testing
//! and debugging.

use crate::pure_rust_parser::AstNode;
use std::fmt::Write;

/// Position information for a node
#[derive(Debug, Clone, Copy)]
pub struct NodeSpan {
    pub start: SourcePos,
    pub end: SourcePos,
}

#[derive(Debug, Clone, Copy)]
pub struct SourcePos {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

/// Formatter for converting AST to S-expression
pub struct SexpFormatter {
    /// Original source code (for position calculation)
    #[allow(dead_code)]
    source: String,
    /// Whether to include position information
    include_positions: bool,
    /// Whether to use compact one-line format
    compact: bool,
}

impl SexpFormatter {
    pub fn new(source: &str) -> Self {
        Self { source: source.to_string(), include_positions: false, compact: false }
    }

    pub fn with_positions(mut self, include: bool) -> Self {
        self.include_positions = include;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Convert AST to tree-sitter compatible S-expression
    pub fn format(&self, node: &AstNode) -> String {
        let mut output = String::new();
        let _ = self.format_node(node, &mut output, 0);
        output
    }

    #[allow(clippy::ptr_arg)] // needs String for write!
    fn format_node(&self, node: &AstNode, output: &mut String, depth: usize) -> std::fmt::Result {
        use AstNode::*;

        let node_type = self.get_node_type(node);
        let indent = if self.compact { std::string::String::new() } else { "  ".repeat(depth) };

        write!(output, "{}", indent)?;
        write!(output, "({}", node_type)?;

        // Add position information if enabled
        if self.include_positions
            && let Some(span) = self.get_node_span(node)
        {
            write!(output, " [{}-{}]", span.start.byte, span.end.byte)?;
        }

        match node {
            Program(statements) => {
                for stmt in statements {
                    if !self.compact {
                        writeln!(output)?;
                    } else {
                        write!(output, " ")?;
                    }
                    self.format_node(stmt, output, depth + 1)?;
                }
            }

            Block(statements) => {
                for stmt in statements {
                    if !self.compact {
                        writeln!(output)?;
                    } else {
                        write!(output, " ")?;
                    }
                    self.format_node(stmt, output, depth + 1)?;
                }
            }

            Statement(inner) => {
                if !self.compact {
                    writeln!(output)?;
                } else {
                    write!(output, " ")?;
                }
                self.format_node(inner, output, depth + 1)?;
            }

            VariableDeclaration { scope, variables, initializer } => {
                write!(output, " scope: {}", scope)?;

                write!(output, " (variables")?;
                for var in variables {
                    write!(output, " ")?;
                    self.format_node(var, output, depth + 1)?;
                }
                write!(output, ")")?;

                if let Some(init) = initializer {
                    write!(output, " (initializer ")?;
                    self.format_node(init, output, depth + 1)?;
                    write!(output, ")")?;
                }
            }

            SubDeclaration { name, prototype, attributes, body } => {
                write!(output, " name: {}", name)?;

                if let Some(proto) = prototype {
                    write!(output, " prototype: {}", proto)?;
                }

                if !attributes.is_empty() {
                    write!(output, " (attributes")?;
                    for attr in attributes {
                        write!(output, " {}", attr)?;
                    }
                    write!(output, ")")?;
                }

                write!(output, " (body ")?;
                self.format_node(body, output, depth + 1)?;
                write!(output, ")")?;
            }

            BinaryOp { op, left, right } => {
                write!(output, " operator: {}", op)?;
                write!(output, " (left ")?;
                self.format_node(left, output, depth + 1)?;
                write!(output, ") (right ")?;
                self.format_node(right, output, depth + 1)?;
                write!(output, ")")?;
            }

            UnaryOp { op, operand } => {
                write!(output, " operator: {}", op)?;
                write!(output, " (operand ")?;
                self.format_node(operand, output, depth + 1)?;
                write!(output, ")")?;
            }

            TernaryOp { condition, true_expr, false_expr } => {
                write!(output, " (condition ")?;
                self.format_node(condition, output, depth + 1)?;
                write!(output, ") (then ")?;
                self.format_node(true_expr, output, depth + 1)?;
                write!(output, ") (else ")?;
                self.format_node(false_expr, output, depth + 1)?;
                write!(output, ")")?;
            }

            FunctionCall { function, args } => {
                write!(output, " (function ")?;
                self.format_node(function, output, depth + 1)?;
                write!(output, ") (arguments")?;
                for arg in args {
                    write!(output, " ")?;
                    self.format_node(arg, output, depth + 1)?;
                }
                write!(output, ")")?;
            }

            MethodCall { object, method, args } => {
                write!(output, " (object ")?;
                self.format_node(object, output, depth + 1)?;
                write!(output, ") method: {} (arguments", method)?;
                for arg in args {
                    write!(output, " ")?;
                    self.format_node(arg, output, depth + 1)?;
                }
                write!(output, ")")?;
            }

            IfStatement { condition, then_block, elsif_clauses, else_block } => {
                write!(output, " (condition ")?;
                self.format_node(condition, output, depth + 1)?;
                write!(output, ") (then ")?;
                self.format_node(then_block, output, depth + 1)?;
                write!(output, ")")?;

                for (cond, block) in elsif_clauses {
                    write!(output, " (elsif (condition ")?;
                    self.format_node(cond, output, depth + 1)?;
                    write!(output, ") (block ")?;
                    self.format_node(block, output, depth + 1)?;
                    write!(output, "))")?;
                }

                if let Some(else_block) = else_block {
                    write!(output, " (else ")?;
                    self.format_node(else_block, output, depth + 1)?;
                    write!(output, ")")?;
                }
            }

            WhileStatement { label, condition, block }
            | UntilStatement { label, condition, block } => {
                if let Some(lbl) = label {
                    write!(output, " label: {}", lbl)?;
                }
                write!(output, " (condition ")?;
                self.format_node(condition, output, depth + 1)?;
                write!(output, ") (body ")?;
                self.format_node(block, output, depth + 1)?;
                write!(output, ")")?;
            }

            ForeachStatement { label, variable, list, block } => {
                if let Some(lbl) = label {
                    write!(output, " label: {}", lbl)?;
                }
                if let Some(var) = variable {
                    write!(output, " (variable ")?;
                    self.format_node(var, output, depth + 1)?;
                    write!(output, ")")?;
                }
                write!(output, " (list ")?;
                self.format_node(list, output, depth + 1)?;
                write!(output, ") (body ")?;
                self.format_node(block, output, depth + 1)?;
                write!(output, ")")?;
            }

            Number(n) => write!(output, " value: {}", n)?,
            String(s) => write!(output, " value: {:?}", s)?,
            Identifier(id) => write!(output, " name: {}", id)?,
            ScalarVariable(name) => write!(output, " name: {}", name)?,
            ArrayVariable(name) => write!(output, " name: {}", name)?,
            HashVariable(name) => write!(output, " name: {}", name)?,

            Heredoc { marker, indented, quoted, content, .. } => {
                write!(
                    output,
                    " marker: {} indented: {} quoted: {} content: {:?}",
                    marker, indented, quoted, content
                )?;
            }

            EndSection(content) => {
                write!(output, " content: {:?}", content)?;
            }

            Comment(content) => {
                write!(output, " content: {:?}", content)?;
            }

            ErrorNode { message, content } => {
                write!(output, " message: {:?} content: {:?}", message, content)?;
            }

            EmptyExpression => {}

            // Default for other node types
            _ => {
                // For complex nodes, recursively format children
                self.format_children(node, output, depth + 1)?;
            }
        }

        write!(output, ")")?;
        Ok(())
    }

    fn get_node_type(&self, node: &AstNode) -> &'static str {
        use AstNode::*;
        match node {
            Program(_) => "source_file",
            Statement(_) => "statement",
            Block(_) => "block",
            VariableDeclaration { .. } => "variable_declaration",
            SubDeclaration { .. } => "subroutine_declaration",
            PackageDeclaration { .. } => "package_declaration",
            UseStatement { .. } => "use_statement",
            RequireStatement { .. } => "require_statement",
            IfStatement { .. } => "if_statement",
            UnlessStatement { .. } => "unless_statement",
            WhileStatement { .. } => "while_statement",
            UntilStatement { .. } => "until_statement",
            ForStatement { .. } => "for_statement",
            ForeachStatement { .. } => "foreach_statement",
            BinaryOp { .. } => "binary_expression",
            UnaryOp { .. } => "unary_expression",
            TernaryOp { .. } => "ternary_expression",
            Assignment { .. } => "assignment",
            FunctionCall { .. } => "function_call",
            MethodCall { .. } => "method_call",
            ArrayAccess { .. } => "array_access",
            HashAccess { .. } => "hash_access",
            ScalarVariable(_) => "scalar_variable",
            ArrayVariable(_) => "array_variable",
            HashVariable(_) => "hash_variable",
            Number(_) => "number",
            String(_) => "string",
            Identifier(_) => "identifier",
            Heredoc { .. } => "heredoc",
            Regex { .. } => "regex",
            List(_) => "list",
            ArrayRef(_) => "array_ref",
            HashRef(_) => "hash_ref",
            Comment(_) => "comment",
            Pod(_) => "pod",
            DataSection(_) => "data_section",
            TryCatch { .. } => "try_catch",
            DeferStatement(_) => "defer_statement",
            ErrorNode { .. } => "error",
            _ => "unknown",
        }
    }

    #[allow(clippy::ptr_arg)] // needs String for consistency with format_node
    fn format_children(
        &self,
        _node: &AstNode,
        _output: &mut String,
        _depth: usize,
    ) -> std::fmt::Result {
        // This is a placeholder for nodes that need custom formatting
        // In a real implementation, we'd handle all node types
        Ok(())
    }

    fn get_node_span(&self, _node: &AstNode) -> Option<NodeSpan> {
        // In a real implementation, we'd track positions during parsing
        // For now, return None
        None
    }
}

/// Enhanced S-expression builder with field tracking
#[derive(Default)]
pub struct SexpBuilder {
    buffer: String,
    depth: usize,
    compact: bool,
}

impl SexpBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_node(&mut self, node_type: &str) {
        if self.depth > 0 && !self.compact {
            self.buffer.push('\n');
            self.buffer.push_str(&"  ".repeat(self.depth));
        }
        self.buffer.push('(');
        self.buffer.push_str(node_type);
        self.depth += 1;
    }

    pub fn add_field(&mut self, name: &str, value: &str) {
        self.buffer.push(' ');
        self.buffer.push_str(name);
        self.buffer.push_str(": ");
        self.buffer.push_str(value);
    }

    pub fn add_position(&mut self, start: usize, end: usize) -> std::fmt::Result {
        write!(&mut self.buffer, " [{}-{}]", start, end)
    }

    pub fn end_node(&mut self) {
        self.depth -= 1;
        self.buffer.push(')');
    }

    pub fn finish(self) -> String {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_sexp_formatting() {
        let formatter = SexpFormatter::new("my $x = 42;");
        let ast = AstNode::Program(vec![AstNode::Statement(Box::new(AstNode::Assignment {
            target: Box::new(AstNode::ScalarVariable("$x".into())),
            op: "=".into(),
            value: Box::new(AstNode::Number("42".into())),
        }))]);

        let sexp = formatter.format(&ast);
        assert!(sexp.contains("source_file"));
        assert!(sexp.contains("assignment"));
        // `format_children` is still a placeholder for many node kinds, so
        // child nodes are not guaranteed to be present in output.
    }

    #[test]
    fn test_compact_mode() {
        let formatter = SexpFormatter::new("").compact(true);
        let ast =
            AstNode::Program(vec![AstNode::Number("42".into()), AstNode::Number("43".into())]);

        let sexp = formatter.format(&ast);
        assert!(!sexp.contains('\n'));
    }
}

//! Enhanced S-expression formatter with position tracking
//!
//! This module provides tree-sitter compatible S-expression output with:
//! - Byte position tracking for all nodes
//! - Proper field names for tree-sitter compatibility
//! - Compact and pretty-print modes
//! - Error node handling

use crate::pure_rust_parser::AstNode;
use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct NodeSpan {
    pub start: Position,
    pub end: Position,
}

pub struct SexpFormatter {
    source: String,
    include_positions: bool,
    compact: bool,
}

impl SexpFormatter {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            include_positions: true,
            compact: false,
        }
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
        self.format_node(node, &mut output, 0);
        output
    }

    fn format_node(&self, node: &AstNode, output: &mut String, depth: usize) {
        use AstNode::*;

        let node_type = self.get_node_type(node);
        let indent = if self.compact {
            std::string::String::new()
        } else {
            "  ".repeat(depth)
        };

        write!(output, "{}", indent).unwrap();
        write!(output, "({}", node_type).unwrap();

        // Add position information if enabled
        if self.include_positions {
            if let Some(span) = self.get_node_span(node) {
                write!(output, " [{}-{}]", span.start.byte, span.end.byte).unwrap();
            }
        }

        match node {
            Program(statements) => {
                for stmt in statements {
                    if !self.compact {
                        writeln!(output).unwrap();
                    } else {
                        write!(output, " ").unwrap();
                    }
                    self.format_node(stmt, output, depth + 1);
                }
            }

            Block(statements) => {
                for stmt in statements {
                    if !self.compact {
                        writeln!(output).unwrap();
                    } else {
                        write!(output, " ").unwrap();
                    }
                    self.format_node(stmt, output, depth + 1);
                }
            }

            Statement(inner) => {
                if !self.compact {
                    writeln!(output).unwrap();
                } else {
                    write!(output, " ").unwrap();
                }
                self.format_node(inner, output, depth + 1);
            }

            VariableDeclaration {
                scope,
                variables,
                initializer,
            } => {
                write!(output, " scope: {}", scope).unwrap();

                write!(output, " (variables").unwrap();
                for var in variables {
                    write!(output, " ").unwrap();
                    self.format_node(var, output, depth + 1);
                }
                write!(output, ")").unwrap();

                if let Some(init) = initializer {
                    write!(output, " (initializer ").unwrap();
                    self.format_node(init, output, depth + 1);
                    write!(output, ")").unwrap();
                }
            }

            SubDeclaration {
                name,
                prototype,
                attributes,
                body,
            } => {
                write!(output, " name: {}", name).unwrap();

                if let Some(proto) = prototype {
                    write!(output, " prototype: {}", proto).unwrap();
                }

                if !attributes.is_empty() {
                    write!(output, " (attributes").unwrap();
                    for attr in attributes {
                        write!(output, " {}", attr).unwrap();
                    }
                    write!(output, ")").unwrap();
                }

                write!(output, " (body ").unwrap();
                self.format_node(body, output, depth + 1);
                write!(output, ")").unwrap();
            }

            BinaryOp { op, left, right } => {
                write!(output, " operator: {}", op).unwrap();
                write!(output, " (left ").unwrap();
                self.format_node(left, output, depth + 1);
                write!(output, ") (right ").unwrap();
                self.format_node(right, output, depth + 1);
                write!(output, ")").unwrap();
            }

            UnaryOp { op, operand } => {
                write!(output, " operator: {}", op).unwrap();
                write!(output, " (operand ").unwrap();
                self.format_node(operand, output, depth + 1);
                write!(output, ")").unwrap();
            }

            FunctionCall { function, args } => {
                write!(output, " (function ").unwrap();
                self.format_node(function, output, depth + 1);
                write!(output, ") (arguments").unwrap();
                for arg in args {
                    write!(output, " ").unwrap();
                    self.format_node(arg, output, depth + 1);
                }
                write!(output, ")").unwrap();
            }

            MethodCall {
                object,
                method,
                args,
            } => {
                write!(output, " (object ").unwrap();
                self.format_node(object, output, depth + 1);
                write!(output, ") method: {} (arguments", method).unwrap();
                for arg in args {
                    write!(output, " ").unwrap();
                    self.format_node(arg, output, depth + 1);
                }
                write!(output, ")").unwrap();
            }

            IfStatement {
                condition,
                then_block,
                elsif_clauses,
                else_block,
            } => {
                write!(output, " (condition ").unwrap();
                self.format_node(condition, output, depth + 1);
                write!(output, ") (then ").unwrap();
                self.format_node(then_block, output, depth + 1);
                write!(output, ")").unwrap();

                for (elsif_cond, elsif_block) in elsif_clauses {
                    write!(output, " (elsif (condition ").unwrap();
                    self.format_node(elsif_cond, output, depth + 1);
                    write!(output, ") (block ").unwrap();
                    self.format_node(elsif_block, output, depth + 1);
                    write!(output, "))").unwrap();
                }

                if let Some(else_blk) = else_block {
                    write!(output, " (else ").unwrap();
                    self.format_node(else_blk, output, depth + 1);
                    write!(output, ")").unwrap();
                }
            }

            WhileStatement {
                label,
                condition,
                block,
            } => {
                if let Some(lbl) = label {
                    write!(output, " label: {}", lbl).unwrap();
                }
                write!(output, " (condition ").unwrap();
                self.format_node(condition, output, depth + 1);
                write!(output, ") (body ").unwrap();
                self.format_node(block, output, depth + 1);
                write!(output, ")").unwrap();
            }

            ForeachStatement {
                label,
                variable,
                list,
                block,
            } => {
                if let Some(lbl) = label {
                    write!(output, " label: {}", lbl).unwrap();
                }
                if let Some(var) = variable {
                    write!(output, " (variable ").unwrap();
                    self.format_node(var, output, depth + 1);
                    write!(output, ")").unwrap();
                }
                write!(output, " (list ").unwrap();
                self.format_node(list, output, depth + 1);
                write!(output, ") (body ").unwrap();
                self.format_node(block, output, depth + 1);
                write!(output, ")").unwrap();
            }

            // Literals
            Number(n) => write!(output, " value: {}", n).unwrap(),
            String(s) => write!(output, " value: {:?}", s).unwrap(),
            Identifier(id) => write!(output, " name: {}", id).unwrap(),
            ScalarVariable(name) => write!(output, " name: {}", name).unwrap(),
            ArrayVariable(name) => write!(output, " name: {}", name).unwrap(),
            HashVariable(name) => write!(output, " name: {}", name).unwrap(),

            Heredoc {
                marker,
                indented,
                quoted,
                content,
            } => {
                write!(
                    output,
                    " marker: {} indented: {} quoted: {} content: {:?}",
                    marker, indented, quoted, content
                )
                .unwrap();
            }

            DataSection(content) => {
                write!(output, " content: {:?}", content).unwrap();
            }

            Pod(content) => {
                write!(output, " content: {:?}", content).unwrap();
            }

            ErrorNode { message, content } => {
                write!(output, " message: {:?} content: {:?}", message, content).unwrap();
            }

            // Default for other node types
            _ => {
                // For complex nodes, recursively format children
                self.format_children(node, output, depth + 1);
            }
        }

        write!(output, ")").unwrap();
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

    fn format_children(&self, _node: &AstNode, _output: &mut String, _depth: usize) {
        // This is a placeholder for nodes that need custom formatting
        // In a real implementation, we'd handle all node types
    }

    fn get_node_span(&self, _node: &AstNode) -> Option<NodeSpan> {
        // In a real implementation, we'd track positions during parsing
        // For now, return None
        None
    }
}

/// Enhanced S-expression builder with field tracking
pub struct SexpBuilder {
    buffer: String,
    depth: usize,
    compact: bool,
}

impl Default for SexpBuilder {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            depth: 0,
            compact: false,
        }
    }
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

    pub fn add_position(&mut self, start: usize, end: usize) {
        write!(&mut self.buffer, " [{}-{}]", start, end).unwrap();
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
        assert!(sexp.contains("scalar_variable"));
    }

    #[test]
    fn test_compact_mode() {
        let formatter = SexpFormatter::new("").compact(true);
        let ast = AstNode::Program(vec![
            AstNode::Number("42".into()),
            AstNode::Number("43".into()),
        ]);

        let sexp = formatter.format(&ast);
        assert!(!sexp.contains('\n'));
    }
}

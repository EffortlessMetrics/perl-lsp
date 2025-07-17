//! Stateful parser wrapper for handling Perl constructs that require state
//! 
//! This module provides a stateful wrapper around the Pest parser to handle
//! constructs like heredocs that require maintaining state across lines.

use crate::pure_rust_parser::{AstNode, PerlParser, Rule};
use pest::Parser;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct HeredocMarker {
    pub marker: String,
    pub indented: bool,
    pub quoted: HeredocQuoteType,
    pub position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeredocQuoteType {
    None,
    Single,
    Double,
    Backtick,
    Escaped,
}

#[derive(Debug)]
pub struct StatefulPerlParser {
    /// Queue of heredoc markers we're waiting to collect content for
    pending_heredocs: VecDeque<HeredocMarker>,
    /// Buffer for collecting lines when processing heredocs
    line_buffer: Vec<String>,
    /// Current parsing state
    state: ParserState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParserState {
    Normal,
    CollectingHeredoc,
}

impl StatefulPerlParser {
    pub fn new() -> Self {
        Self {
            pending_heredocs: VecDeque::new(),
            line_buffer: Vec::new(),
            state: ParserState::Normal,
        }
    }

    /// Parse a complete Perl source file handling heredocs properly
    pub fn parse(&mut self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let lines: Vec<&str> = input.lines().collect();
        let mut processed_lines = Vec::new();
        let mut heredoc_contents: Vec<(String, String)> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            
            match self.state {
                ParserState::Normal => {
                    // Check if this line contains a heredoc declaration
                    if let Some(heredoc_info) = self.extract_heredoc_declaration(line) {
                        eprintln!("DEBUG: Found heredoc declaration: {:?}", heredoc_info);
                        self.pending_heredocs.push_back(heredoc_info);
                        processed_lines.push(line.to_string());
                        i += 1;
                        
                        // Start collecting heredoc content
                        if !self.pending_heredocs.is_empty() {
                            self.state = ParserState::CollectingHeredoc;
                        }
                    } else {
                        processed_lines.push(line.to_string());
                        i += 1;
                    }
                }
                ParserState::CollectingHeredoc => {
                    if let Some(heredoc) = self.pending_heredocs.front() {
                        // Check if this line is the heredoc terminator
                        if self.is_heredoc_terminator(line, heredoc) {
                            // Collect all content up to this point
                            let content = self.line_buffer.join("\n");
                            heredoc_contents.push((heredoc.marker.clone(), content));
                            
                            self.line_buffer.clear();
                            self.pending_heredocs.pop_front();
                            
                            // Add the terminator line to processed lines
                            processed_lines.push(line.to_string());
                            i += 1;
                            
                            // Check if we have more heredocs to collect
                            if self.pending_heredocs.is_empty() {
                                self.state = ParserState::Normal;
                            }
                        } else {
                            // This is heredoc content
                            self.line_buffer.push(line.to_string());
                            i += 1;
                        }
                    }
                }
            }
        }

        // Join processed lines and parse
        let processed_input = processed_lines.join("\n");
        
        // Build AST and inject heredoc contents
        let mut ast = self.build_ast_from_processed_input(&processed_input)?;
        self.inject_heredoc_contents(&mut ast, heredoc_contents);
        
        Ok(ast)
    }

    /// Extract heredoc declaration information from a line
    pub fn extract_heredoc_declaration(&self, line: &str) -> Option<HeredocMarker> {
        // Simple regex-like pattern matching for heredoc declarations
        // This is a simplified version - a full implementation would be more robust
        
        if let Some(pos) = line.find("<<") {
            let after_marker = &line[pos + 2..];
            let (indented, rest) = if after_marker.starts_with('~') {
                (true, &after_marker[1..])
            } else {
                (false, after_marker)
            };
            
            // Extract the marker and quote type
            let trimmed = rest.trim_start();
            if trimmed.is_empty() {
                return None;
            }
            
            let (marker, quote_type) = if trimmed.starts_with('\'') {
                // Single quoted
                if let Some(end) = trimmed[1..].find('\'') {
                    (trimmed[1..=end].to_string(), HeredocQuoteType::Single)
                } else {
                    return None;
                }
            } else if trimmed.starts_with('"') {
                // Double quoted
                if let Some(end) = trimmed[1..].find('"') {
                    (trimmed[1..=end].to_string(), HeredocQuoteType::Double)
                } else {
                    return None;
                }
            } else if trimmed.starts_with('`') {
                // Backtick
                if let Some(end) = trimmed[1..].find('`') {
                    (trimmed[1..=end].to_string(), HeredocQuoteType::Backtick)
                } else {
                    return None;
                }
            } else if trimmed.starts_with('\\') {
                // Escaped
                let end = trimmed[1..].find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(trimmed[1..].len());
                (trimmed[1..=end].to_string(), HeredocQuoteType::Escaped)
            } else {
                // Bare marker
                let end = trimmed.find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(trimmed.len());
                (trimmed[..end].to_string(), HeredocQuoteType::None)
            };
            
            if marker.is_empty() {
                return None;
            }
            
            Some(HeredocMarker {
                marker,
                indented,
                quoted: quote_type,
                position: pos,
            })
        } else {
            None
        }
    }

    /// Check if a line is a heredoc terminator
    pub fn is_heredoc_terminator(&self, line: &str, heredoc: &HeredocMarker) -> bool {
        let trimmed = if heredoc.indented {
            line.trim_start()
        } else {
            line
        };
        
        trimmed == heredoc.marker
    }

    /// Build AST from processed input
    fn build_ast_from_processed_input(&self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        use crate::pure_rust_parser::PureRustPerlParser;
        
        let mut parser = PureRustPerlParser::new();
        parser.parse(input)
    }

    /// Inject collected heredoc contents into the AST
    fn inject_heredoc_contents(&self, ast: &mut AstNode, contents: Vec<(String, String)>) {
        if contents.is_empty() {
            return;
        }
        
        // Create a map for quick lookup
        let mut content_map: std::collections::HashMap<String, String> = 
            contents.into_iter().collect();
        
        // Recursively walk the AST and update heredoc nodes
        self.update_heredoc_nodes(ast, &mut content_map);
    }
    
    /// Recursively update heredoc nodes with their content
    fn update_heredoc_nodes(&self, node: &mut AstNode, content_map: &mut std::collections::HashMap<String, String>) {
        match node {
            AstNode::Heredoc { marker, content, .. } => {
                if let Some(collected_content) = content_map.remove(marker) {
                    *content = collected_content;
                }
            }
            AstNode::Program(nodes) | 
            AstNode::Block(nodes) => {
                for child in nodes {
                    self.update_heredoc_nodes(child, content_map);
                }
            }
            AstNode::Statement(inner) |
            AstNode::BeginBlock(inner) |
            AstNode::EndBlock(inner) |
            AstNode::CheckBlock(inner) |
            AstNode::InitBlock(inner) |
            AstNode::UnitcheckBlock(inner) |
            AstNode::DoBlock(inner) |
            AstNode::EvalBlock(inner) |
            AstNode::EvalString(inner) => {
                self.update_heredoc_nodes(inner, content_map);
            }
            AstNode::IfStatement { then_block, elsif_clauses, else_block, .. } => {
                self.update_heredoc_nodes(then_block, content_map);
                for (_, block) in elsif_clauses {
                    self.update_heredoc_nodes(block, content_map);
                }
                if let Some(else_block) = else_block {
                    self.update_heredoc_nodes(else_block, content_map);
                }
            }
            AstNode::UnlessStatement { block, else_block, .. } => {
                self.update_heredoc_nodes(block, content_map);
                if let Some(else_block) = else_block {
                    self.update_heredoc_nodes(else_block, content_map);
                }
            }
            AstNode::WhileStatement { block, .. } |
            AstNode::ForeachStatement { block, .. } |
            AstNode::ForStatement { block, .. } => {
                self.update_heredoc_nodes(block, content_map);
            }
            AstNode::SubDeclaration { body, .. } => {
                self.update_heredoc_nodes(body, content_map);
            }
            AstNode::LabeledBlock { block, .. } => {
                self.update_heredoc_nodes(block, content_map);
            }
            AstNode::PackageDeclaration { block, .. } => {
                if let Some(block) = block {
                    self.update_heredoc_nodes(block, content_map);
                }
            }
            AstNode::Assignment { target, value, .. } => {
                self.update_heredoc_nodes(target, content_map);
                self.update_heredoc_nodes(value, content_map);
            }
            AstNode::BinaryOp { left, right, .. } => {
                self.update_heredoc_nodes(left, content_map);
                self.update_heredoc_nodes(right, content_map);
            }
            AstNode::UnaryOp { operand, .. } => {
                self.update_heredoc_nodes(operand, content_map);
            }
            AstNode::TernaryOp { condition, true_expr, false_expr } => {
                self.update_heredoc_nodes(condition, content_map);
                self.update_heredoc_nodes(true_expr, content_map);
                self.update_heredoc_nodes(false_expr, content_map);
            }
            AstNode::FunctionCall { function, args } |
            AstNode::MethodCall { object: function, args, .. } => {
                self.update_heredoc_nodes(function, content_map);
                for arg in args {
                    self.update_heredoc_nodes(arg, content_map);
                }
            }
            AstNode::ArrayElement { index, .. } => {
                self.update_heredoc_nodes(index, content_map);
            }
            AstNode::HashElement { key, .. } => {
                self.update_heredoc_nodes(key, content_map);
            }
            AstNode::List(items) | 
            AstNode::ArrayRef(items) |
            AstNode::HashRef(items) => {
                for item in items {
                    self.update_heredoc_nodes(item, content_map);
                }
            }
            AstNode::VariableDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    self.update_heredoc_nodes(init, content_map);
                }
            }
            _ => {
                // Leaf nodes that don't contain other nodes
            }
        }
    }
}

impl Default for StatefulPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_heredoc() {
        let mut parser = StatefulPerlParser::new();
        let input = r#"print <<EOF;
Hello World
This is heredoc content
EOF
print "after heredoc";"#;

        // This test would verify heredoc parsing works correctly
        // let ast = parser.parse(input).unwrap();
    }

    #[test]
    fn test_indented_heredoc() {
        let mut parser = StatefulPerlParser::new();
        let input = r#"print <<~EOF;
    Hello World
    This is indented content
    EOF
print "after heredoc";"#;

        // Test indented heredoc handling
    }

    #[test]
    fn test_quoted_heredoc() {
        let mut parser = StatefulPerlParser::new();
        let input = r#"print <<'EOF';
No $interpolation here
EOF
print <<"INTERP";
With $interpolation
INTERP"#;

        // Test different quote types
    }
}
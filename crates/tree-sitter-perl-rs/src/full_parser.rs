//! Full Perl parser with slash disambiguation and heredoc support
//!
//! This module provides a complete Perl parser that handles both
//! context-sensitive slash disambiguation and multi-line heredoc parsing.

use crate::pure_rust_parser::{PerlParser, Rule, AstNode, PureRustPerlParser};
use crate::lexer_adapter::LexerAdapter;
use crate::heredoc_parser::{parse_with_heredocs, HeredocDeclaration};
use crate::error::ParseError;
use pest::Parser;
use std::sync::Arc;
use std::collections::HashMap;

/// A complete Perl parser that handles all context-sensitive features
pub struct FullPerlParser {
    /// Stored heredoc declarations for AST enrichment
    heredoc_declarations: Vec<HeredocDeclaration>,
}

impl FullPerlParser {
    /// Create a new full parser instance
    pub fn new() -> Self {
        Self {
            heredoc_declarations: Vec::new(),
        }
    }

    /// Parse Perl code with full preprocessing
    pub fn parse(&mut self, input: &str) -> Result<AstNode, ParseError> {
        // Phase 1: Handle heredocs
        let (heredoc_processed, declarations) = parse_with_heredocs(input);
        self.heredoc_declarations = declarations;
        
        // Phase 2: Handle slash disambiguation
        let fully_processed = LexerAdapter::preprocess(&heredoc_processed);
        
        // Phase 3: Parse with Pest
        let pairs = PerlParser::parse(Rule::program, &fully_processed)
            .map_err(|_| ParseError::ParseFailed)?;
        
        // Phase 4: Build AST
        let mut parser = PureRustPerlParser::new();
        let mut ast = None;
        for pair in pairs {
            ast = parser.build_node(pair).map_err(|_| ParseError::ParseFailed)?;
        }
        
        // Phase 5: Postprocess to restore original tokens and heredoc content
        if let Some(ref mut node) = ast {
            LexerAdapter::postprocess(node);
            self.restore_heredoc_content(node);
        }
        
        ast.ok_or(ParseError::ParseFailed)
    }
    
    /// Parse and return S-expression format
    pub fn parse_to_sexp(&mut self, input: &str) -> Result<String, ParseError> {
        let ast = self.parse(input)?;
        let parser = PureRustPerlParser::new();
        Ok(parser.to_sexp(&ast))
    }
    
    /// Restore heredoc content in the AST
    fn restore_heredoc_content(&self, node: &mut AstNode) {
        // Map placeholder IDs to heredoc content
        let placeholder_map: HashMap<String, Arc<str>> = self.heredoc_declarations
            .iter()
            .filter_map(|decl| {
                decl.content.as_ref().map(|content| {
                    (decl.placeholder_id.clone(), content.clone())
                })
            })
            .collect();
        
        self.restore_node_content(node, &placeholder_map);
    }
    
    fn restore_node_content(&self, node: &mut AstNode, placeholder_map: &HashMap<String, Arc<str>>) {
        match node {
            AstNode::String(value) => {
                // Check if this is a heredoc placeholder
                if value.contains("__HEREDOC__") {
                    // Extract the original heredoc content
                    for (placeholder_id, content) in placeholder_map {
                        let marker = format!("__HEREDOC__{}__HEREDOC__", placeholder_id);
                        if value.contains(&marker) {
                            *value = content.clone();
                            // Heredocs maintain their interpolation status
                            break;
                        }
                    }
                }
            }
            AstNode::Block(statements) |
            AstNode::Program(statements) => {
                for stmt in statements {
                    self.restore_node_content(stmt, placeholder_map);
                }
            }
            AstNode::BinaryOp { left, right, .. } => {
                self.restore_node_content(left, placeholder_map);
                self.restore_node_content(right, placeholder_map);
            }
            AstNode::FunctionCall { args, .. } => {
                for arg in args {
                    self.restore_node_content(arg, placeholder_map);
                }
            }
            AstNode::Assignment { target, value, .. } => {
                self.restore_node_content(target, placeholder_map);
                self.restore_node_content(value, placeholder_map);
            }
            AstNode::List(elements) => {
                for elem in elements {
                    self.restore_node_content(elem, placeholder_map);
                }
            }
            AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
                self.restore_node_content(condition, placeholder_map);
                self.restore_node_content(then_block, placeholder_map);
                for (cond, block) in elsif_clauses {
                    self.restore_node_content(cond, placeholder_map);
                    self.restore_node_content(block, placeholder_map);
                }
                if let Some(block) = else_block {
                    self.restore_node_content(block, placeholder_map);
                }
            }
            _ => {
                // For other node types, recursively process any child nodes
                // This is a simplified version - in production, you'd handle all node types
            }
        }
    }
}

impl Default for FullPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heredoc_with_division() {
        let input = r#"my $x = <<'EOF';
Hello / World
EOF
my $y = $x / 2;"#;

        let mut parser = FullPerlParser::new();
        match parser.parse_to_sexp(input) {
            Ok(result) => {
                println!("Parse result:\n{}", result);
                // Should contain heredoc content
                assert!(result.contains("Hello / World"));
                // Should also parse the division correctly
                assert!(result.contains("binary_expression"));
            }
            Err(e) => {
                panic!("Failed to parse: {:?}", e);
            }
        }
    }

    #[test]
    fn test_heredoc_with_regex() {
        let input = r#"print <<EOF =~ /pattern/;
Test content
EOF"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse_to_sexp(input).unwrap();
        
        // Should parse both heredoc and regex match
        assert!(result.contains("Test content"));
        assert!(result.contains("regex_match"));
    }

    #[test]
    fn test_multiple_features() {
        let input = r#"my $data = <<'DATA';
Line 1: a/b
Line 2: s/foo/bar/
DATA

if ($data =~ /Line 1: (.*)/) {
    my $result = $1;
    $result =~ s/a\/b/x\/y/g;
}"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_indented_heredoc_with_slash() {
        let input = r#"if (1) {
    my $config = <<~'CONFIG';
        path: /usr/local/bin
        regex: /\w+/
        CONFIG
    print $config;
}"#;

        let mut parser = FullPerlParser::new();
        let result = parser.parse(input);
        assert!(result.is_ok());
    }
}
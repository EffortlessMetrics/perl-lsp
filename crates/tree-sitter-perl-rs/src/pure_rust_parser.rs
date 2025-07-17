//! Pure Rust Perl parser implementation
//!
//! This module provides a complete Rust-native implementation of the Perl parser
//! using Pest for grammar parsing, without any dependency on tree-sitter's C code.

use pest::{iterators::Pairs, Parser};
use pest_derive::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct PerlParser;

/// AST node types for the pure Rust parser
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Program(Vec<AstNode>),
    Statement(Vec<AstNode>),
    Expression(Vec<AstNode>),
    ScalarVariable(String),
    ArrayVariable(String),
    HashVariable(String),
    GlobVariable(String),
    Identifier(String),
    Number(String),
    SingleQuotedString(String),
    DoubleQuotedString(String),
    BacktickString(String),
    Keyword(String),
    Operator(String),
    Comment(String),
}

/// Pure Rust Perl parser implementation
pub struct PureRustPerlParser {
    // Add any parser state here
    symbol_table: HashMap<String, String>,
}

impl PureRustPerlParser {
    /// Create a new pure Rust Perl parser
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
        }
    }

    /// Parse Perl source code into an AST
    pub fn parse(&mut self, source: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let pairs = PerlParser::parse(Rule::program, source)?;
        self.build_ast(pairs)
    }

    /// Build AST from Pest parse pairs
    fn build_ast(&mut self, pairs: Pairs<Rule>) -> Result<AstNode, Box<dyn std::error::Error>> {
        let mut nodes = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::program => {
                    // Recursively parse child nodes
                    nodes.push(self.build_ast(pair.into_inner())?);
                }
                Rule::statement => {
                    nodes.push(self.build_ast(pair.into_inner())?);
                }
                Rule::expression => {
                    nodes.push(self.build_ast(pair.into_inner())?);
                }
                Rule::scalar_variable => {
                    let var_name = pair.as_str().to_string();
                    nodes.push(AstNode::ScalarVariable(var_name));
                }
                Rule::array_variable => {
                    let var_name = pair.as_str().to_string();
                    nodes.push(AstNode::ArrayVariable(var_name));
                }
                Rule::hash_variable => {
                    let var_name = pair.as_str().to_string();
                    nodes.push(AstNode::HashVariable(var_name));
                }
                Rule::glob_variable => {
                    let var_name = pair.as_str().to_string();
                    nodes.push(AstNode::GlobVariable(var_name));
                }
                Rule::identifier => {
                    let name = pair.as_str().to_string();
                    nodes.push(AstNode::Identifier(name));
                }
                Rule::number => {
                    let value = pair.as_str().to_string();
                    nodes.push(AstNode::Number(value));
                }
                Rule::single_quoted_string => {
                    let content = pair.as_str().to_string();
                    nodes.push(AstNode::SingleQuotedString(content));
                }
                Rule::double_quoted_string => {
                    let content = pair.as_str().to_string();
                    nodes.push(AstNode::DoubleQuotedString(content));
                }
                Rule::backtick_string => {
                    let content = pair.as_str().to_string();
                    nodes.push(AstNode::BacktickString(content));
                }
                Rule::keyword => {
                    let keyword = pair.as_str().to_string();
                    nodes.push(AstNode::Keyword(keyword));
                }
                Rule::operator => {
                    let op = pair.as_str().to_string();
                    nodes.push(AstNode::Operator(op));
                }
                Rule::COMMENT => {
                    let comment = pair.as_str().to_string();
                    nodes.push(AstNode::Comment(comment));
                }
                Rule::WHITESPACE => {
                    // Skip whitespace in AST
                    continue;
                }
                _ => {
                    // For other rules, recursively parse
                    if !pair.as_str().trim().is_empty() {
                        nodes.push(self.build_ast(pair.into_inner())?);
                    }
                }
            }
        }

        // Determine the appropriate node type based on context
        if nodes.len() == 1 {
            Ok(nodes.remove(0))
        } else {
            Ok(AstNode::Program(nodes))
        }
    }

    /// Convert AST to S-expression format (for compatibility with tree-sitter output)
    pub fn to_sexp(&self, node: &AstNode) -> String {
        match node {
            AstNode::Program(children) => {
                let child_sexps: Vec<String> = children.iter().map(|c| self.to_sexp(c)).collect();
                format!("(source_file, child_sexps.join(" "))")
            }
            AstNode::Statement(children) => {
                let child_sexps: Vec<String> = children.iter().map(|c| self.to_sexp(c)).collect();
                format!("(expression_statement, child_sexps.join(" "))")
            }
            AstNode::Expression(children) => {
                let child_sexps: Vec<String> = children.iter().map(|c| self.to_sexp(c)).collect();
                child_sexps.join(")")
            }
            AstNode::ScalarVariable(name) => {
                format!("(scalar (varname {}), name.trim_start_matches('$'))")
            }
            AstNode::ArrayVariable(name) => {
                format!("(array (varname {}), name.trim_start_matches('@'))")
            }
            AstNode::HashVariable(name) => {
                format!("(hash (varname {}), name.trim_start_matches('%'))")
            }
            AstNode::GlobVariable(name) => {
                format!("(glob (varname {}), name.trim_start_matches('*'))")
            }
            AstNode::Identifier(name) => {
                format!("(identifier {})", name)
            }
            AstNode::Number(value) => {
                format!("(number {})", value)
            }
            AstNode::SingleQuotedString(content) => {
                format!("(string_literal content: (string_content {}))", content)
            }
            AstNode::DoubleQuotedString(content) => {
                format!("(interpolated_string_literal content: (string_content {}))", content)
            }
            AstNode::BacktickString(content) => {
                format!("(command_string content: (string_content {}))", content)
            }
            AstNode::Keyword(keyword) => {
                format!("(keyword {})", keyword)
            }
            AstNode::Operator(op) => {
                format!("(operator {}),op)")
            }
            AstNode::Comment(content) => {
                format!("(comment {})", content)
            }
        }
    }
}

impl Default for PureRustPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "my $var = 42;";
        
        let result = parser.parse(source);
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("AST: {:?}", ast);
        println!("S-expression: {}", sexp);
    }

    #[test]
    fn test_variable_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "$scalar @array %hash";
        
        let result = parser.parse(source);
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("S-expression: {}", sexp);
    }

    #[test]
    fn test_string_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = 'hello' \world\" `command`";
        
        let result = parser.parse(source);
        assert!(result.is_ok());
        
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("S-expression: {}", sexp);
    }
} 
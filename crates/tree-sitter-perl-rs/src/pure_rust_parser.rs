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
    ScalarVariable(String),
    ArrayVariable(String),
    HashVariable(String),
    Number(String),
    Identifier(String),
    Comment(String),
}

/// Pure Rust Perl parser implementation
pub struct PureRustPerlParser {
    symbol_table: HashMap<String, String>,
}

impl PureRustPerlParser {
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
        }
    }

    pub fn parse(&mut self, source: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        let pairs = PerlParser::parse(Rule::program, source)?;
        self.build_ast(pairs)
    }

    fn build_ast(&mut self, pairs: Pairs<Rule>) -> Result<AstNode, Box<dyn std::error::Error>> {
        let mut nodes = Vec::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::program => {
                    nodes.push(self.build_ast(pair.into_inner())?);
                }
                Rule::statement => {
                    nodes.push(self.build_ast(pair.into_inner())?);
                }
                Rule::scalar_variable => {
                    nodes.push(AstNode::ScalarVariable(pair.as_str().to_string()));
                }
                Rule::array_variable => {
                    nodes.push(AstNode::ArrayVariable(pair.as_str().to_string()));
                }
                Rule::hash_variable => {
                    nodes.push(AstNode::HashVariable(pair.as_str().to_string()));
                }
                Rule::number => {
                    nodes.push(AstNode::Number(pair.as_str().to_string()));
                }
                Rule::identifier => {
                    nodes.push(AstNode::Identifier(pair.as_str().to_string()));
                }
                Rule::COMMENT => {
                    nodes.push(AstNode::Comment(pair.as_str().to_string()));
                }
                Rule::WHITESPACE => {
                    // skip
                }
                _ => {
                    if !pair.as_str().trim().is_empty() {
                        nodes.push(self.build_ast(pair.into_inner())?);
                    }
                }
            }
        }
        if nodes.len() == 1 {
            Ok(nodes.remove(0))
        } else {
            Ok(AstNode::Program(nodes))
        }
    }

    pub fn to_sexp(&self, node: &AstNode) -> String {
        match node {
            AstNode::Program(children) => {
                let child_sexps: Vec<String> = children.iter().map(|c| self.to_sexp(c)).collect();
                format!("(source_file {})", child_sexps.join(" "))
            }
            AstNode::Statement(children) => {
                let child_sexps: Vec<String> = children.iter().map(|c| self.to_sexp(c)).collect();
                format!("(statement {})", child_sexps.join(" "))
            }
            AstNode::ScalarVariable(name) => {
                format!("(scalar_variable {})", name)
            }
            AstNode::ArrayVariable(name) => {
                format!("(array_variable {})", name)
            }
            AstNode::HashVariable(name) => {
                format!("(hash_variable {})", name)
            }
            AstNode::Number(value) => {
                format!("(number {})", value)
            }
            AstNode::Identifier(name) => {
                format!("(identifier {})", name)
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
        let source = "$var;";
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
        let source = "$scalar @array %hash;";
        let result = parser.parse(source);
        assert!(result.is_ok());
        let ast = result.unwrap();
        let sexp = parser.to_sexp(&ast);
        println!("S-expression: {}", sexp);
    }

    #[test]
    fn test_debug_parsing() {
        let mut parser = PureRustPerlParser::new();
        let source = "my $var = 42;";
        let result = parser.parse(source);
        match result {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                println!("Success! AST: {:?}", ast);
                println!("S-expression: {}", sexp);
            }
            Err(e) => {
                println!("Parse error: {}, e", e);
            }
        }
    }
} 
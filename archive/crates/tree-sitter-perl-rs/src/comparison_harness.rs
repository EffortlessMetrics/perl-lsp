//! Comparative test harness for C vs Rust parsers
//!
//! This module provides a unified interface to test and benchmark
//! both the C/tree-sitter parser and the pure Rust parser

use std::collections::HashMap;
use std::time::Instant;

#[cfg(feature = "pure-rust")]
use crate::pure_rust_parser::PureRustPerlParser;

#[cfg(not(feature = "pure-rust"))]
use tree_sitter::{Node, Parser};

pub struct ParseResult {
    pub parser_type: String,
    pub success: bool,
    pub parse_time: std::time::Duration,
    pub s_expression: String,
    pub error: Option<String>,
}

pub struct ComparisonHarness {
    #[cfg(not(feature = "pure-rust"))]
    tree_sitter_parser: Parser,
    #[cfg(feature = "pure-rust")]
    pure_rust_parser: PureRustPerlParser,
}

impl Default for ComparisonHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl ComparisonHarness {
    pub fn new() -> Self {
        use perl_tdd_support::must;
        must(Self::try_new())
    }

    pub fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(not(feature = "pure-rust"))]
        {
            let mut parser = Parser::new();
            let language = crate::language();
            parser.set_language(&language)?;
            Ok(Self { tree_sitter_parser: parser })
        }

        #[cfg(feature = "pure-rust")]
        {
            Ok(Self { pure_rust_parser: PureRustPerlParser::new() })
        }
    }

    pub fn parse_with_tree_sitter(&mut self, source: &str) -> ParseResult {
        #[cfg(not(feature = "pure-rust"))]
        {
            let start = Instant::now();
            let tree_result = self.tree_sitter_parser.parse(source, None);
            let parse_time = start.elapsed();

            match tree_result {
                Some(tree) => {
                    let root = tree.root_node();
                    let s_expr = self.node_to_sexp(root, source);
                    ParseResult {
                        parser_type: "tree-sitter".to_string(),
                        success: true,
                        parse_time,
                        s_expression: s_expr,
                        error: None,
                    }
                }
                None => ParseResult {
                    parser_type: "tree-sitter".to_string(),
                    success: false,
                    parse_time,
                    s_expression: String::new(),
                    error: Some("Failed to parse".to_string()),
                },
            }
        }

        #[cfg(feature = "pure-rust")]
        {
            let _ = source; // Avoid unused variable warning
            ParseResult {
                parser_type: "tree-sitter".to_string(),
                success: false,
                parse_time: std::time::Duration::from_secs(0),
                s_expression: String::new(),
                error: Some("Tree-sitter parser not available with pure-rust feature".to_string()),
            }
        }
    }

    pub fn parse_with_pure_rust(&mut self, source: &str) -> ParseResult {
        #[cfg(feature = "pure-rust")]
        {
            let start = Instant::now();
            let parse_result = self.pure_rust_parser.parse(source);
            let parse_time = start.elapsed();

            match parse_result {
                Ok(ast) => {
                    let s_expr = self.pure_rust_parser.to_sexp(&ast);
                    ParseResult {
                        parser_type: "pure-rust".to_string(),
                        success: true,
                        parse_time,
                        s_expression: s_expr,
                        error: None,
                    }
                }
                Err(e) => ParseResult {
                    parser_type: "pure-rust".to_string(),
                    success: false,
                    parse_time,
                    s_expression: String::new(),
                    error: Some(e.to_string()),
                },
            }
        }

        #[cfg(not(feature = "pure-rust"))]
        {
            ParseResult {
                parser_type: "pure-rust".to_string(),
                success: false,
                parse_time: std::time::Duration::from_secs(0),
                s_expression: String::new(),
                error: Some("Pure Rust parser not available without pure-rust feature".to_string()),
            }
        }
    }

    pub fn compare_parsers(&mut self, source: &str) -> (ParseResult, ParseResult) {
        let tree_sitter_result = self.parse_with_tree_sitter(source);
        let pure_rust_result = self.parse_with_pure_rust(source);
        (tree_sitter_result, pure_rust_result)
    }

    pub fn run_benchmark(
        &mut self,
        source: &str,
        iterations: usize,
    ) -> HashMap<String, Vec<std::time::Duration>> {
        let mut results = HashMap::new();

        // Benchmark tree-sitter parser
        let mut tree_sitter_times = Vec::new();
        for _ in 0..iterations {
            let result = self.parse_with_tree_sitter(source);
            if result.success {
                tree_sitter_times.push(result.parse_time);
            }
        }
        results.insert("tree-sitter".to_string(), tree_sitter_times);

        // Benchmark pure Rust parser
        let mut pure_rust_times = Vec::new();
        for _ in 0..iterations {
            let result = self.parse_with_pure_rust(source);
            if result.success {
                pure_rust_times.push(result.parse_time);
            }
        }
        results.insert("pure-rust".to_string(), pure_rust_times);

        results
    }

    #[cfg(not(feature = "pure-rust"))]
    fn node_to_sexp(&self, node: Node, source: &str) -> String {
        let kind = node.kind();
        let child_count = node.child_count();

        if child_count == 0 {
            if node.is_named() {
                format!("({} {})", kind, node.utf8_text(source.as_bytes()).unwrap_or(""))
            } else {
                node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
            }
        } else {
            let mut children = vec![];
            for i in 0..child_count {
                if let Some(child) = node.child(i) {
                    children.push(self.node_to_sexp(child, source));
                }
            }
            format!("({} {})", kind, children.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_harness() {
        let mut harness = ComparisonHarness::new();
        let source = "$var = 42;";

        let (tree_sitter_result, pure_rust_result) = harness.compare_parsers(source);

        println!("Tree-sitter result:");
        println!("  Success: {}", tree_sitter_result.success);
        println!("  Time: {:?}", tree_sitter_result.parse_time);
        println!("  S-expr: {}", tree_sitter_result.s_expression);
        if let Some(error) = &tree_sitter_result.error {
            println!("  Error: {}", error);
        }

        println!("\nPure Rust result:");
        println!("  Success: {}", pure_rust_result.success);
        println!("  Time: {:?}", pure_rust_result.parse_time);
        println!("  S-expr: {}", pure_rust_result.s_expression);
        if let Some(error) = &pure_rust_result.error {
            println!("  Error: {}", error);
        }
    }

    #[test]
    fn test_benchmark() {
        let mut harness = ComparisonHarness::new();
        let source = "my $x = 10; sub foo { return $x * 2; } foo();";

        let results = harness.run_benchmark(source, 100);

        for (parser, times) in &results {
            if !times.is_empty() {
                let total: std::time::Duration = times.iter().sum();
                let avg = total / times.len() as u32;
                println!(
                    "{} parser - Average time: {:?} over {} iterations",
                    parser,
                    avg,
                    times.len()
                );
            } else {
                println!("{} parser - No successful parses", parser);
            }
        }
    }
}

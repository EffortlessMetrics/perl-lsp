//! Unified test framework for all Perl parsers
//! 
//! This crate provides a common test infrastructure to test:
//! 1. perl-lexer + perl-parser (Modern Rust)
//! 2. tree-sitter-perl-rs (Pure Rust Pest)
//! 3. tree-sitter-perl-c (Legacy C)

use anyhow::Result;
use std::time::Duration;

pub mod corpus;
pub mod corpus_converter;

/// A test case for parser testing
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub input: String,
    pub description: Option<String>,
    pub should_parse: bool,
    pub expected_sexp: Option<String>,
}

/// Results from running a parser on a test case
#[derive(Debug)]
pub struct TestResult {
    pub parser_name: String,
    pub test_name: String,
    pub success: bool,
    pub parse_time: Duration,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Trait that all parsers must implement to be testable
pub trait TestableParser {
    /// Name of the parser for reporting
    fn name(&self) -> &'static str;
    
    /// Parse the input and return S-expression output
    fn parse_to_sexp(&self, input: &str) -> Result<String>;
}

/// Wrapper for perl-parser
pub struct PerlParserWrapper;

impl TestableParser for PerlParserWrapper {
    fn name(&self) -> &'static str {
        "perl-parser"
    }
    
    fn parse_to_sexp(&self, input: &str) -> Result<String> {
        use perl_parser::Parser;
        
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        Ok(ast.to_sexp())
    }
}

/// Wrapper for tree-sitter-perl-rs (Pest)
pub struct TreeSitterPerlRsWrapper;

impl TestableParser for TreeSitterPerlRsWrapper {
    fn name(&self) -> &'static str {
        "tree-sitter-perl-rs"
    }
    
    fn parse_to_sexp(&self, input: &str) -> Result<String> {
        use tree_sitter_perl::PureRustParser;
        
        let parser = PureRustParser::new();
        let ast = parser.parse(input)
            .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
        // The Pest parser returns an AstNode, need to convert to S-expression
        Ok(format!("{:?}", ast)) // TODO: Implement proper to_sexp for Pest AST
    }
}

/// Wrapper for tree-sitter-perl-c
pub struct TreeSitterPerlCWrapper;

impl TestableParser for TreeSitterPerlCWrapper {
    fn name(&self) -> &'static str {
        "tree-sitter-perl-c"
    }
    
    fn parse_to_sexp(&self, input: &str) -> Result<String> {
        use tree_sitter_perl_c::create_parser;
        
        let mut parser = create_parser();
        let tree = parser.parse(input, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
        
        let sexp = tree.root_node().to_sexp();
        Ok(sexp)
    }
}

/// Run a test case against all parsers
pub fn run_test_on_all_parsers(test: &TestCase) -> Vec<TestResult> {
    let parsers: Vec<Box<dyn TestableParser>> = vec![
        Box::new(PerlParserWrapper),
        Box::new(TreeSitterPerlRsWrapper),
        Box::new(TreeSitterPerlCWrapper),
    ];
    
    let mut results = Vec::new();
    
    for parser in parsers {
        let start = std::time::Instant::now();
        let parse_result = parser.parse_to_sexp(&test.input);
        let parse_time = start.elapsed();
        
        let (success, output, error) = match parse_result {
            Ok(sexp) => {
                let success = if test.should_parse {
                    // If we expect it to parse, check if output matches expected
                    if let Some(expected) = &test.expected_sexp {
                        sexp == *expected
                    } else {
                        true // No expected output, just check it parsed
                    }
                } else {
                    false // Expected failure but it parsed
                };
                (success, Some(sexp), None)
            }
            Err(e) => {
                let success = !test.should_parse; // Success if we expected failure
                (success, None, Some(e.to_string()))
            }
        };
        
        results.push(TestResult {
            parser_name: parser.name().to_string(),
            test_name: test.name.clone(),
            success,
            parse_time,
            output,
            error,
        });
    }
    
    results
}

/// Load test cases from the corpus directory
pub fn load_corpus_tests() -> Result<Vec<TestCase>> {
    // TODO: Implement loading from test/corpus
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_parse() {
        let test = TestCase {
            name: "simple_variable".to_string(),
            input: "my $x = 42;".to_string(),
            description: Some("Simple variable declaration".to_string()),
            should_parse: true,
            expected_sexp: None,
        };
        
        let results = run_test_on_all_parsers(&test);
        
        for result in &results {
            println!("{}: {} ({:?})", 
                result.parser_name, 
                if result.success { "PASS" } else { "FAIL" },
                result.parse_time
            );
            if let Some(output) = &result.output {
                println!("  Output: {}", output);
            }
            if let Some(error) = &result.error {
                println!("  Error: {}", error);
            }
        }
    }
}
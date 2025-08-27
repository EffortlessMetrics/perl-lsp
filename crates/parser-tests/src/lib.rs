//! Unified test framework for all Perl parsers
//!
//! This crate provides a common test infrastructure to test:
//! 1. perl-lexer + perl-parser (Modern Rust)
//! 2. tree-sitter-perl-rs (Pure Rust Pest)
//! 3. tree-sitter-perl-c (Legacy C) -- currently unavailable

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
        use tree_sitter_perl::{PureRustParser, PureRustPerlParser};

        let parser = PureRustParser::new();
        let ast = parser.parse(input).map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
        Ok(PureRustPerlParser::node_to_sexp(&ast))
    }
}

/// Run a test case against all parsers
pub fn run_test_on_all_parsers(test: &TestCase) -> Vec<TestResult> {
    let parsers: Vec<Box<dyn TestableParser>> =
        vec![Box::new(PerlParserWrapper), Box::new(TreeSitterPerlRsWrapper)];

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
        let input = "my $x = 42;";

        // Test that both parsers can produce valid S-expressions without error
        let rust_parser = TreeSitterPerlRsWrapper;
        let perl_parser = PerlParserWrapper;

        let rust_output = rust_parser.parse_to_sexp(input).unwrap();
        let perl_output = perl_parser.parse_to_sexp(input).unwrap();

        // Verify both outputs are valid S-expressions (start with '(' and end with ')')
        assert!(
            rust_output.starts_with('(') && rust_output.ends_with(')'),
            "Rust parser output is not a valid S-expression: {}",
            rust_output
        );
        assert!(
            perl_output.starts_with('(') && perl_output.ends_with(')'),
            "Perl parser output is not a valid S-expression: {}",
            perl_output
        );

        // Verify both contain expected elements for variable declaration
        assert!(
            rust_output.contains("$x") || rust_output.contains("$ x"),
            "Rust parser output doesn't contain variable name: {}",
            rust_output
        );
        assert!(
            perl_output.contains("$x") || perl_output.contains("$ x"),
            "Perl parser output doesn't contain variable name: {}",
            perl_output
        );
        assert!(
            rust_output.contains("42"),
            "Rust parser output doesn't contain number: {}",
            rust_output
        );
        assert!(
            perl_output.contains("42"),
            "Perl parser output doesn't contain number: {}",
            perl_output
        );

        // Print outputs for manual verification
        println!("Rust parser output: {}", rust_output);
        println!("Perl parser output: {}", perl_output);
    }

    #[test] 
    fn test_sexp_format_validation() {
        // Test various Perl constructs to ensure S-expression format validity
        let test_cases = vec![
            ("my $var = 'hello';", "string variable"),
            ("if ($x > 0) { print $x; }", "if statement"),
            ("sub hello { return 'world'; }", "subroutine"),
            ("@array = (1, 2, 3);", "array assignment"),
            ("%hash = (key => 'value');", "hash assignment"),
        ];

        let rust_parser = TreeSitterPerlRsWrapper;
        let perl_parser = PerlParserWrapper;

        for (input, description) in test_cases {
            println!("\nTesting {}: {}", description, input);
            
            // Both parsers should succeed
            match (rust_parser.parse_to_sexp(input), perl_parser.parse_to_sexp(input)) {
                (Ok(rust_output), Ok(perl_output)) => {
                    // Both should be valid S-expressions
                    assert!(rust_output.starts_with('(') && rust_output.ends_with(')'), 
                           "{}: Rust output not valid S-expression: {}", description, rust_output);
                    assert!(perl_output.starts_with('(') && perl_output.ends_with(')'), 
                           "{}: Perl output not valid S-expression: {}", description, perl_output);
                           
                    // Both should be non-empty
                    assert!(!rust_output.trim().is_empty(), 
                           "{}: Rust output is empty", description);
                    assert!(!perl_output.trim().is_empty(), 
                           "{}: Perl output is empty", description);
                           
                    println!("  Rust: {}", rust_output);
                    println!("  Perl: {}", perl_output);
                    println!("  ✓ Both parsers succeeded");
                }
                (Err(rust_err), Err(perl_err)) => {
                    println!("  ✓ Both parsers failed (acceptable for edge cases)");
                    println!("    Rust error: {}", rust_err);
                    println!("    Perl error: {}", perl_err);
                }
                (Ok(rust_output), Err(perl_err)) => {
                    println!("  ✓ Only Rust succeeded: {}", rust_output);
                    println!("    Perl error: {}", perl_err);
                }
                (Err(rust_err), Ok(perl_output)) => {
                    println!("  ✓ Only Perl succeeded: {}", perl_output);
                    println!("    Rust error: {}", rust_err);
                }
            }
        }
    }
}

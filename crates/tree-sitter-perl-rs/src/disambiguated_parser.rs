use crate::pure_rust_parser::{PerlParser, Rule, AstNode, PureRustPerlParser};
use crate::lexer_adapter::LexerAdapter;
use crate::error::ParseError;
use pest::Parser;
use std::sync::Arc;

/// A Perl parser that handles context-sensitive constructs
/// by preprocessing the input to disambiguate slashes
pub struct DisambiguatedParser;

impl DisambiguatedParser {
    /// Parse Perl code with slash disambiguation
    pub fn parse(input: &str) -> Result<AstNode, ParseError> {
        // Step 1: Preprocess the input to disambiguate slashes
        let preprocessed = LexerAdapter::preprocess(input);
        
        // Step 2: Parse with the modified input
        let pairs = PerlParser::parse(Rule::program, &preprocessed)
            .map_err(|_| ParseError::ParseFailed)?;
        
        // Step 3: Build AST
        let mut parser = PureRustPerlParser::new();
        let mut ast = None;
        for pair in pairs {
            ast = parser.build_node(pair).map_err(|_| ParseError::ParseFailed)?;
        }
        
        // Step 4: Postprocess to restore original tokens
        if let Some(ref mut node) = ast {
            LexerAdapter::postprocess(node);
        }
        
        ast.ok_or(ParseError::ParseFailed)
    }
    
    /// Parse and return S-expression format
    pub fn parse_to_sexp(input: &str) -> Result<String, ParseError> {
        let ast = Self::parse(input)?;
        let parser = PureRustPerlParser::new();
        Ok(parser.to_sexp(&ast))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_division_vs_regex() {
        // Test case from the document: "1/ /abc/"
        let input = "1/ /abc/";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("binary_expression"));
        assert!(result.contains("number 1"));
        assert!(result.contains("regex_match"));
        
        // Test simple division
        let input = "x / 2";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("binary_expression"));
        assert!(result.contains("identifier x"));
        assert!(result.contains("number 2"));
    }
    
    #[test]
    fn test_regex_after_operator() {
        let input = "$x =~ /pattern/";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("binary_expression"));
        assert!(result.contains("=~"));
        assert!(result.contains("regex_match"));
    }
    
    #[test]
    fn test_substitution() {
        let input = "s/foo/bar/g";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("substitution"));
        
        // Test with different delimiters
        let input = "s{foo}{bar}g";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("substitution"));
    }
    
    #[test]
    fn test_complex_expressions() {
        // From the document's edge cases
        let input = "print 1/ /foo/";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        assert!(result.contains("function_call"));
        assert!(result.contains("binary_expression"));
        assert!(result.contains("regex_match"));
        
        // Multiple divisions and regexes
        let input = "a/b/c =~ /x/y/";
        let result = DisambiguatedParser::parse_to_sexp(input).unwrap();
        // Should parse as: (a/b)/c =~ (/x/)y/
        assert!(result.contains("binary_expression"));
    }
}
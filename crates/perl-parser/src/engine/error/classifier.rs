//! Error classification and diagnostic generation for Perl parsing workflow pipeline
//!
//! This module provides intelligent error classification for parsing failures in Perl scripts,
//! offering specific error types and recovery suggestions for LSP workflow operations.
//!
//! # LSP Workflow Integration
//!
//! Error classification supports robust Perl parsing across LSP stages:
//! - **Extract**: Classify parsing errors in embedded Perl scripts
//! - **Normalize**: Provide specific error types for standardization failures
//! - **Thread**: Identify control flow parsing issues in Perl parsing logic
//! - **Render**: Classify syntax errors during output generation
//! - **Index**: Handle parsing errors during symbol extraction and indexing
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::error_classifier::{ErrorClassifier, ParseErrorKind};
//! use perl_parser::{Parser, ast::Node};
//!
//! let classifier = ErrorClassifier::new();
//! let source = "my $email = \"unclosed string...";
//! let mut parser = Parser::new(source);
//! let _result = parser.parse(); // This will fail due to unclosed string
//!
//! // Classify parsing errors for better user feedback
//! // let error_kind = classifier.classify(&error_node, source);
//! // let message = classifier.get_diagnostic_message(&error_kind);
//! // let suggestion = classifier.get_suggestion(&error_kind);
//! ```

use crate::ast::Node;

/// Specific types of parse errors found in Perl script content
///
/// Provides detailed categorization of parsing failures to enable targeted
/// error recovery strategies during Perl parsing workflow workflows.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Parser encountered unexpected token during Perl script analysis
    UnexpectedToken {
        /// Token type that was expected during parsing
        expected: String,
        /// Actual token found in Perl script content
        found: String,
    },
    /// String literal not properly closed in Perl script
    UnclosedString,
    /// Regular expression pattern not properly closed
    UnclosedRegex,
    /// Code block (braces) not properly closed
    UnclosedBlock,
    /// Required semicolon missing in Perl script
    MissingSemicolon,
    /// General syntax error in Perl parsing code
    InvalidSyntax,
    /// Parenthesis not properly closed in expression
    UnclosedParenthesis,
    /// Array or hash bracket not properly closed
    UnclosedBracket,
    /// Hash or block brace not properly closed
    UnclosedBrace,
    /// Heredoc block not properly terminated
    UnterminatedHeredoc,
    /// Variable name does not follow Perl naming rules
    InvalidVariableName,
    /// Subroutine name does not follow Perl naming rules
    InvalidSubroutineName,
    /// Required operator missing in expression
    MissingOperator,
    /// Required operand missing in expression
    MissingOperand,
    /// Unexpected end of file during parsing
    UnexpectedEof,
}

/// Email script error classification engine for LSP workflow operations
///
/// Analyzes parsing errors and provides specific error types with recovery suggestions
/// for robust Perl parsing workflows within enterprise LSP environments.
pub struct ErrorClassifier;

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorClassifier {
    /// Create new error classifier for Perl script analysis
    ///
    /// # Returns
    ///
    /// Configured classifier ready for LSP workflow error analysis
    pub fn new() -> Self {
        ErrorClassifier
    }

    /// Classify parsing error based on AST node and source context
    ///
    /// Analyzes error patterns in Perl script content to provide specific
    /// error types for targeted recovery strategies during LSP workflow.
    ///
    /// # Arguments
    ///
    /// * `error_node` - AST node where error occurred
    /// * `source` - Complete Perl script source code for context analysis
    ///
    /// # Returns
    ///
    /// Specific error type for targeted recovery during Perl parsing
    pub fn classify(&self, error_node: &Node, source: &str) -> ParseErrorKind {
        // Get the error text if available based on location
        let error_text = {
            let start = error_node.location.start;
            let end = (start + 10).min(source.len()); // Look at next 10 chars
            if start < source.len() && end <= source.len() && start <= end {
                &source[start..end]
            } else {
                ""
            }
        };

        // Check for common patterns - check the entire source for unclosed quotes
        let quote_count = source.matches('"').count();
        let single_quote_count = source.matches('\'').count();

        // Check if we have unclosed quotes
        if !quote_count.is_multiple_of(2) {
            return ParseErrorKind::UnclosedString;
        }
        if !single_quote_count.is_multiple_of(2) {
            return ParseErrorKind::UnclosedString;
        }

        // Also check the error text itself
        if error_text.starts_with('"') && !error_text.ends_with('"') {
            return ParseErrorKind::UnclosedString;
        }

        if error_text.starts_with('\'') && !error_text.ends_with('\'') {
            return ParseErrorKind::UnclosedString;
        }

        if error_text.starts_with('/') && !error_text.contains("//") {
            // Could be unclosed regex
            if !error_text[1..].contains('/') {
                return ParseErrorKind::UnclosedRegex;
            }
        }

        // Check context around error
        {
            let pos = error_node.location.start;
            let line_start = source[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = source[pos..].find('\n').map(|i| pos + i).unwrap_or(source.len());

            let line = &source[line_start..line_end];

            // Check for missing semicolon
            if !line.trim().is_empty()
                && !line.trim().ends_with(';')
                && !line.trim().ends_with('{')
                && !line.trim().ends_with('}')
            {
                // Look for common statement patterns
                if line.contains("my ")
                    || line.contains("our ")
                    || line.contains("local ")
                    || line.contains("print ")
                    || line.contains("say ")
                    || line.contains("return ")
                {
                    return ParseErrorKind::MissingSemicolon;
                }
            }

            // Check for unclosed delimiters
            let open_parens = line.matches('(').count();
            let close_parens = line.matches(')').count();
            if open_parens > close_parens {
                return ParseErrorKind::UnclosedParenthesis;
            }

            let open_brackets = line.matches('[').count();
            let close_brackets = line.matches(']').count();
            if open_brackets > close_brackets {
                return ParseErrorKind::UnclosedBracket;
            }

            let open_braces = line.matches('{').count();
            let close_braces = line.matches('}').count();
            if open_braces > close_braces {
                return ParseErrorKind::UnclosedBrace;
            }
        }

        // Check if we're at EOF
        if error_node.location.start >= source.len() - 1 {
            return ParseErrorKind::UnexpectedEof;
        }

        // Default to invalid syntax
        ParseErrorKind::InvalidSyntax
    }

    /// Generate user-friendly diagnostic message for classified error
    ///
    /// Converts error classification into readable message for Perl script developers
    /// during LSP workflow processing and debugging operations.
    ///
    /// # Arguments
    ///
    /// * `kind` - Classified error type from Perl script analysis
    ///
    /// # Returns
    ///
    /// Human-readable error message describing the parsing issue
    pub fn get_diagnostic_message(&self, kind: &ParseErrorKind) -> String {
        match kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                format!("Expected {} but found {}", expected, found)
            }
            ParseErrorKind::UnclosedString => "Unclosed string literal".to_string(),
            ParseErrorKind::UnclosedRegex => "Unclosed regular expression".to_string(),
            ParseErrorKind::UnclosedBlock => "Unclosed code block - missing '}'".to_string(),
            ParseErrorKind::MissingSemicolon => "Missing semicolon at end of statement".to_string(),
            ParseErrorKind::InvalidSyntax => "Invalid syntax".to_string(),
            ParseErrorKind::UnclosedParenthesis => "Unclosed parenthesis - missing ')'".to_string(),
            ParseErrorKind::UnclosedBracket => "Unclosed bracket - missing ']'".to_string(),
            ParseErrorKind::UnclosedBrace => "Unclosed brace - missing '}'".to_string(),
            ParseErrorKind::UnterminatedHeredoc => "Unterminated heredoc".to_string(),
            ParseErrorKind::InvalidVariableName => "Invalid variable name".to_string(),
            ParseErrorKind::InvalidSubroutineName => "Invalid subroutine name".to_string(),
            ParseErrorKind::MissingOperator => "Missing operator".to_string(),
            ParseErrorKind::MissingOperand => "Missing operand".to_string(),
            ParseErrorKind::UnexpectedEof => "Unexpected end of file".to_string(),
        }
    }

    /// Generate recovery suggestion for classified parsing error
    ///
    /// Provides actionable recovery suggestions for Perl script developers
    /// to resolve parsing issues during LSP workflow development.
    ///
    /// # Arguments
    ///
    /// * `kind` - Classified error type requiring recovery suggestion
    ///
    /// # Returns
    ///
    /// Optional recovery suggestion or None if no specific suggestion available
    pub fn get_suggestion(&self, kind: &ParseErrorKind) -> Option<String> {
        match kind {
            ParseErrorKind::MissingSemicolon => {
                Some("Add a semicolon ';' at the end of the statement".to_string())
            }
            ParseErrorKind::UnclosedString => {
                Some("Add a closing quote to terminate the string".to_string())
            }
            ParseErrorKind::UnclosedParenthesis => {
                Some("Add a closing parenthesis ')'".to_string())
            }
            ParseErrorKind::UnclosedBracket => Some("Add a closing bracket ']'".to_string()),
            ParseErrorKind::UnclosedBrace => Some("Add a closing brace '}'".to_string()),
            ParseErrorKind::UnclosedRegex => {
                Some("Add a closing delimiter to terminate the regex".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeKind, Parser, SourceLocation};

    #[test]
    fn test_classify_unclosed_string() {
        let classifier = ErrorClassifier::new();
        let source = r#"my $x = "hello"#;
        let mut parser = Parser::new(source);
        let ast = parser.parse().unwrap_or_else(|_| {
            Node::new(
                NodeKind::Error { message: "Parse error".to_string() },
                SourceLocation { start: 0, end: source.len() },
            )
        });

        // Find error nodes
        let mut errors = vec![];
        find_errors(&ast, &mut errors);

        if let Some(error) = errors.first() {
            let kind = classifier.classify(error, source);
            assert_eq!(kind, ParseErrorKind::UnclosedString);
        }
    }

    #[test]
    fn test_classify_missing_semicolon() {
        let classifier = ErrorClassifier::new();
        let source = "my $x = 42\nmy $y = 10";

        // Simulate an error node at the end of first line
        let error = Node::new(
            NodeKind::Error { message: "".to_string() },
            SourceLocation { start: 10, end: 11 },
        );
        let kind = classifier.classify(&error, source);
        assert_eq!(kind, ParseErrorKind::MissingSemicolon);
    }

    fn find_errors(node: &Node, errors: &mut Vec<Node>) {
        if matches!(&node.kind, NodeKind::Error { .. }) {
            errors.push(node.clone());
        }
        // Recursively check children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    find_errors(stmt, errors);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    find_errors(stmt, errors);
                }
            }
            _ => {} // Other node types don't have children we need to check
        }
    }
}

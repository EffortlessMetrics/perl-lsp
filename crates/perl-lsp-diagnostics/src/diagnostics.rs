//! Diagnostics provider for Perl code
//!
//! This module provides the core diagnostic generation functionality.

use perl_parser_core::Node;
use perl_parser_core::error::ParseError;
use perl_pragma::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::ScopeAnalyzer;

use crate::scope::scope_issues_to_diagnostics;

// Re-export diagnostic types from the shared SRP microcrate.
pub use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};

/// Diagnostics provider
///
/// Analyzes Perl source code and generates diagnostic messages for
/// parse errors, scope issues, and lint warnings.
pub struct DiagnosticsProvider {
    _ast: std::sync::Arc<Node>,
    _source: String,
}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new(ast: &std::sync::Arc<Node>, source: String) -> Self {
        Self { _ast: ast.clone(), _source: source }
    }

    /// Generate diagnostics for the given AST
    ///
    /// Analyzes the AST and parse errors to produce a list of diagnostics
    /// including parse errors, semantic issues, and lint warnings.
    pub fn get_diagnostics(
        &self,
        ast: &std::sync::Arc<Node>,
        parse_errors: &[ParseError],
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let source_len = source.len();

        // Convert parse errors to diagnostics
        for error in parse_errors {
            let (location, message) = match error {
                ParseError::UnexpectedToken { location, expected, found } => {
                    let found = format_found_token(found);
                    (*location, format!("Expected {expected}, found {found}"))
                }
                ParseError::SyntaxError { location, message } => (*location, message.clone()),
                ParseError::UnexpectedEof => (source.len(), "Unexpected end of input".to_string()),
                ParseError::LexerError { message } => (0, message.clone()),
                _ => (0, error.to_string()),
            };

            let range_start = location.min(source_len);
            let range_end = range_start.saturating_add(1).min(source_len.saturating_add(1));

            let suggestion = build_parse_error_suggestion(error);

            diagnostics.push(Diagnostic {
                range: (range_start, range_end),
                severity: DiagnosticSeverity::Error,
                code: Some("parse-error".to_string()),
                message,
                related_information: Vec::new(),
                tags: Vec::new(),
                suggestion,
            });
        }

        // Run scope analysis to detect undeclared/unused/shadowing issues
        let pragma_map = PragmaTracker::build(ast);
        let scope_analyzer = ScopeAnalyzer::new();
        let scope_issues = scope_analyzer.analyze(ast, source, &pragma_map);
        diagnostics.extend(scope_issues_to_diagnostics(scope_issues));

        diagnostics
    }
}

fn format_found_token(found: &str) -> String {
    if found.is_empty() || found == "<EOF>" {
        "end of input".to_string()
    } else {
        format!("`{found}`")
    }
}

/// Build a contextual suggestion for a parse error based on the expected/found tokens.
fn build_parse_error_suggestion(error: &ParseError) -> Option<String> {
    match error {
        ParseError::UnexpectedToken { expected, found, .. } => {
            // Missing semicolon: parser expected ';' or found something when ';' was expected
            if expected.contains(';') || expected.contains("semicolon") {
                return Some("Add a ';' at the end of the statement".to_string());
            }
            // Found ';' when expecting something else often means missing expression
            if found == ";" {
                return Some(format!(
                    "A {expected} is required here -- the statement appears incomplete"
                ));
            }
            // Unexpected closing brace/paren
            if found == "}" || found == ")" || found == "]" {
                return Some(format!("Check for a missing {expected} before '{found}'"));
            }
            None
        }
        ParseError::UnexpectedEof => Some(
            "The file ended unexpectedly -- check for unclosed delimiters or missing semicolons"
                .to_string(),
        ),
        ParseError::UnclosedDelimiter { delimiter } => {
            Some(format!("Add a matching closing '{delimiter}'"))
        }
        _ => None,
    }
}

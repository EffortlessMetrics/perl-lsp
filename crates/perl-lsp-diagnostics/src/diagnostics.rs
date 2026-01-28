//! Diagnostics provider for Perl code
//!
//! This module provides the core diagnostic generation functionality.

use perl_parser_core::Node;
use perl_parser_core::error::ParseError;
use perl_pragma::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::ScopeAnalyzer;

use crate::scope::scope_issues_to_diagnostics;

// Re-export types from types module
pub use crate::types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};

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

        // Convert parse errors to diagnostics
        for error in parse_errors {
            let (location, message) = match error {
                ParseError::UnexpectedToken { location, expected, found } => {
                    (*location, format!("Expected {expected}, found {found}"))
                }
                ParseError::SyntaxError { location, message } => (*location, message.clone()),
                ParseError::UnexpectedEof => (source.len(), "Unexpected end of input".to_string()),
                ParseError::LexerError { message } => (0, message.clone()),
                _ => (0, error.to_string()),
            };

            diagnostics.push(Diagnostic {
                range: (location, location.saturating_add(1)),
                severity: DiagnosticSeverity::Error,
                code: Some("parse-error".to_string()),
                message,
                related_information: Vec::new(),
                tags: Vec::new(),
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

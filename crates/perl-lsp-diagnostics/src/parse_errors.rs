//! Parse error to diagnostic conversion
//!
//! This module provides functionality for converting parser errors into diagnostic messages.

use perl_parser_core::error::ParseError;

use super::types::{Diagnostic, DiagnosticSeverity};

/// Convert a parse error to a diagnostic
#[allow(dead_code)]
pub fn parse_error_to_diagnostic(error: &ParseError) -> Diagnostic {
    let message = error.to_string();
    let location = match error {
        ParseError::UnexpectedToken { location, .. } => *location,
        ParseError::SyntaxError { location, .. } => *location,
        _ => 0,
    };

    Diagnostic {
        range: (location, location + 1),
        severity: DiagnosticSeverity::Error,
        code: Some("syntax-error".to_string()),
        message,
        related_information: Vec::new(),
        tags: Vec::new(),
    }
}

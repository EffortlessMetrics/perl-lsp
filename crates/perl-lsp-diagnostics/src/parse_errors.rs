//! Parse error to diagnostic conversion
//!
//! This module provides functionality for converting parser errors into diagnostic messages.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `syntax-error` | Error | Generic syntax error from the parser |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::error::ParseError;

use crate::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Derive the canonical diagnostic code for a parser error.
pub fn parse_error_code(error: &ParseError) -> DiagnosticCode {
    match error {
        ParseError::UnexpectedEof => DiagnosticCode::UnexpectedEof,
        ParseError::SyntaxError { message, .. } => {
            DiagnosticCode::from_message(message).unwrap_or(DiagnosticCode::SyntaxError)
        }
        _ => DiagnosticCode::ParseError,
    }
}

/// Convert a parse error to a diagnostic with actionable suggestions.
///
/// Every diagnostic includes:
/// - A clear human-readable message describing what went wrong
/// - An appropriate severity level
/// - A diagnostic code (`syntax-error`) for IDE quick-fix integration
/// - An optional suggestion describing how to fix the issue
#[allow(dead_code)]
pub fn parse_error_to_diagnostic(error: &ParseError) -> Diagnostic {
    let message = error.to_string();
    let location = match error {
        ParseError::UnexpectedToken { location, .. } => *location,
        ParseError::SyntaxError { location, .. } => *location,
        _ => 0,
    };

    let suggestion = match error {
        ParseError::UnexpectedToken {
            expected, found, ..
        } => {
            if expected.contains(';') || expected.contains("semicolon") {
                Some("Add a ';' at the end of the statement".to_string())
            } else if found == ";" {
                Some(format!(
                    "A {} is required here -- the statement appears incomplete",
                    expected
                ))
            } else if found == "}" || found == ")" || found == "]" {
                Some(format!(
                    "Check for a missing {} before '{}'",
                    expected, found
                ))
            } else {
                None
            }
        }
        ParseError::UnexpectedEof => {
            Some("Check for unclosed delimiters or missing semicolons".to_string())
        }
        ParseError::UnclosedDelimiter { delimiter } => {
            Some(format!("Add a matching closing '{}'", delimiter))
        }
        ParseError::InvalidString => {
            Some("Check for a missing closing quote or an invalid escape sequence".to_string())
        }
        ParseError::InvalidRegex { .. } => {
            Some("Check the regex pattern for unmatched delimiters or invalid syntax".to_string())
        }
        ParseError::InvalidNumber { literal } => Some(format!(
            "'{}' is not a valid number -- check for misplaced underscores or invalid digits",
            literal
        )),
        ParseError::RecursionLimit | ParseError::NestingTooDeep { .. } => Some(
            "The code is too deeply nested -- consider refactoring into smaller subroutines"
                .to_string(),
        ),
        ParseError::LexerError { message: msg } => {
            let lower = msg.to_lowercase();
            if lower.contains("unterminated") || lower.contains("unclosed") {
                Some(
                    "Check for an unclosed string, regex, or heredoc near this position"
                        .to_string(),
                )
            } else {
                None
            }
        }
        ParseError::SyntaxError { .. } => None,
        ParseError::Cancelled => None,
        // Recovered errors: the parser continued with a synthetic node.
        // No user-facing suggestion is needed — the partial AST is still usable.
        ParseError::Recovered { .. } => None,
    };

    Diagnostic {
        range: (location, location + 1),
        severity: parse_error_severity(error),
        code: Some(parse_error_code(error).as_str().to_string()),
        message,
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion,
    }
}

/// Derive the user-facing severity for a parser error.
pub fn parse_error_severity(error: &ParseError) -> DiagnosticSeverity {
    if matches!(error, ParseError::SyntaxError { .. })
        && (matches!(parse_error_code(error), DiagnosticCode::InvalidPrototype)
            || matches!(
                error,
                ParseError::SyntaxError { message, .. }
                    if is_unknown_subroutine_attribute_warning(message)
            ))
    {
        return DiagnosticSeverity::Warning;
    }

    DiagnosticSeverity::Error
}

fn is_unknown_subroutine_attribute_warning(message: &str) -> bool {
    message.starts_with("unknown subroutine attribute ':")
}

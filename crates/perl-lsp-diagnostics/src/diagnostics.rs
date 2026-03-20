//! Diagnostics provider for Perl code
//!
//! This module provides the core diagnostic generation functionality.

use perl_diagnostics_codes::DiagnosticCode;
use perl_parser_core::Node;
use perl_parser_core::error::ParseError;
use perl_pragma::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::ScopeAnalyzer;

use crate::scope::scope_issues_to_diagnostics;

// Re-export diagnostic types from the shared SRP microcrate.
pub use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

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
                    let found_display = format_found_token(found);
                    let msg = build_enhanced_message(expected, found, &found_display);
                    (*location, msg)
                }
                ParseError::SyntaxError { location, message } => (*location, message.clone()),
                ParseError::UnexpectedEof => (source.len(), "Unexpected end of input".to_string()),
                ParseError::LexerError { message } => (0, message.clone()),
                _ => (0, error.to_string()),
            };

            let range_start = location.min(source_len);
            let range_end = range_start.saturating_add(1).min(source_len.saturating_add(1));

            let suggestion = build_parse_error_suggestion(error);

            // Surface the suggestion as relatedInformation for IDE integration
            let related_information = suggestion
                .as_ref()
                .map(|s| {
                    vec![RelatedInformation {
                        location: (range_start, range_end),
                        message: format!("Suggestion: {s}"),
                    }]
                })
                .unwrap_or_default();

            let code = match error {
                ParseError::UnexpectedEof => DiagnosticCode::UnexpectedEof,
                ParseError::SyntaxError { .. } => DiagnosticCode::SyntaxError,
                _ => DiagnosticCode::ParseError,
            };

            diagnostics.push(Diagnostic {
                range: (range_start, range_end),
                severity: DiagnosticSeverity::Error,
                code: Some(code.as_str().to_string()),
                message,
                related_information,
                tags: Vec::new(),
                suggestion,
            });
        }

        // Run scope analysis to detect undeclared/unused/shadowing issues
        let pragma_map = PragmaTracker::build(ast);
        let scope_analyzer = ScopeAnalyzer::new();
        let scope_issues = scope_analyzer.analyze(ast, source, &pragma_map);
        diagnostics.extend(scope_issues_to_diagnostics(scope_issues));

        // Detect heredoc anti-patterns
        let heredoc_diags = crate::heredoc_antipatterns::detect_heredoc_antipatterns(source);
        diagnostics.extend(heredoc_diags);

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

/// Build an enhanced error message with Perl-specific context.
fn build_enhanced_message(expected: &str, found: &str, found_display: &str) -> String {
    let expected_lower = expected.to_lowercase();

    // Missing semicolon
    if expected.contains(';') || expected_lower.contains("semicolon") {
        return format!("Missing semicolon after statement. Add `;` here (found {found_display})");
    }

    // Expected variable after my/our/local/state
    if expected_lower.contains("variable") {
        return format!(
            "Expected a variable like `$foo`, `@bar`, or `%hash` here, found {found_display}"
        );
    }

    // Unexpected closing delimiter -- possible mismatch
    if found == "}" || found == ")" || found == "]" {
        let opener = match found {
            "}" => "{",
            ")" => "(",
            "]" => "[",
            _ => "",
        };
        return format!(
            "Unexpected `{found}` -- possible unmatched brace. \
             Check the opening `{opener}` earlier in this scope"
        );
    }

    // Default
    format!("Expected {expected}, found {found_display}")
}

/// Build a contextual suggestion for a parse error based on the expected/found tokens.
///
/// Each suggestion is designed to be actionable: the user should be able to read
/// the suggestion and know exactly what to change.
fn build_parse_error_suggestion(error: &ParseError) -> Option<String> {
    match error {
        ParseError::UnexpectedToken { expected, found, .. } => {
            // Missing semicolon: parser expected ';' or found something when ';' was expected
            if expected.contains(';') || expected.contains("semicolon") {
                return Some("Missing semicolon after statement. Add `;` here.".to_string());
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
            // Missing opening brace after sub/if/while/for
            if expected.contains('{') || expected.contains("block") {
                return Some(format!(
                    "Add an opening '{{' to start the block (found {found})"
                ));
            }
            // Missing closing paren in function call or condition
            if expected.contains(')') {
                return Some(
                    "Add a closing ')' -- there may be an unmatched opening '('".to_string(),
                );
            }
            // Missing closing bracket
            if expected.contains(']') {
                return Some(
                    "Add a closing ']' -- there may be an unmatched opening '['".to_string(),
                );
            }
            // Expected a variable (e.g. after my/our/local/state)
            if expected.to_lowercase().contains("variable") {
                return Some(
                    "Expected a variable like `$foo`, `@bar`, or `%hash` after the declaration keyword".to_string(),
                );
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
        ParseError::SyntaxError { message, .. } => {
            // Provide targeted suggestions for known syntax error patterns
            let msg_lower = message.to_lowercase();
            if msg_lower.contains("semicolon") || msg_lower.contains("missing ;") {
                Some("Add a ';' at the end of the statement".to_string())
            } else if msg_lower.contains("heredoc") {
                Some(
                    "Check that the heredoc terminator appears on its own line with no extra whitespace"
                        .to_string(),
                )
            } else {
                None
            }
        }
        ParseError::LexerError { message } => {
            let msg_lower = message.to_lowercase();
            if msg_lower.contains("unterminated") || msg_lower.contains("unclosed") {
                Some(
                    "Check for an unclosed string, regex, or heredoc near this position"
                        .to_string(),
                )
            } else if msg_lower.contains("invalid") && msg_lower.contains("character") {
                Some(
                    "Remove or replace the invalid character -- Perl source should be valid UTF-8 or the encoding declared with 'use utf8;'"
                        .to_string(),
                )
            } else {
                None
            }
        }
        ParseError::RecursionLimit => Some(
            "The code is too deeply nested -- consider refactoring into smaller subroutines"
                .to_string(),
        ),
        ParseError::InvalidNumber { literal } => Some(format!(
            "'{literal}' is not a valid number -- check for misplaced underscores or invalid digits"
        )),
        ParseError::InvalidString => Some(
            "Check for a missing closing quote or an invalid escape sequence".to_string(),
        ),
        ParseError::InvalidRegex { .. } => Some(
            "Check the regex pattern for unmatched delimiters, invalid quantifiers, or unescaped metacharacters"
                .to_string(),
        ),
        ParseError::NestingTooDeep { .. } => Some(
            "Reduce nesting depth by extracting inner logic into named subroutines".to_string(),
        ),
        ParseError::Cancelled => None,
    }
}

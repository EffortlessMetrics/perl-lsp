//! Diagnostics provider for Perl code
//!
//! This module provides the core diagnostic generation functionality.

use std::path::Path;

use perl_parser_core::Node;
use perl_parser_core::error::ParseError;
use perl_pragma::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::ScopeAnalyzer;
use perl_semantic_analyzer::symbol::SymbolExtractor;

use crate::dedup::deduplicate_diagnostics;
use crate::lints::common_mistakes::check_common_mistakes;
use crate::lints::deprecated::check_deprecated_syntax;
use crate::lints::duplicate_hash_keys::check_duplicate_hash_keys;
use crate::lints::eval_error_flow::check_eval_error_flow;
use crate::lints::ffi_checklib::check_ffi_checklib;
use crate::lints::goto_label::check_goto_labels;
use crate::lints::package_subroutine::{
    check_duplicate_package, check_duplicate_subroutine, check_missing_package_declaration,
};
use crate::lints::printf_format::check_printf_format;
use crate::lints::role_conflicts::check_role_conflicts;
use crate::lints::security::check_security;
use crate::lints::strict_warnings::check_strict_warnings;
use crate::lints::unreachable_code::check_unreachable_code;
use crate::lints::unused_imports::check_unused_imports;
use crate::lints::version_compat::check_version_compat;
use crate::parse_errors::{parse_error_code, parse_error_severity};
use crate::scope::scope_issues_to_diagnostics;

// Re-export diagnostic types from the shared SRP microcrate.
#[allow(unused_imports)]
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
    ///
    /// `module_resolver` is an optional callback used by the missing-module lint
    /// (PL701). When `Some`, it is called with a bare module name and should return
    /// `true` if the module is resolvable (workspace or configured include paths).
    /// When `None`, the missing-module lint is skipped entirely.
    pub fn get_diagnostics(
        &self,
        ast: &std::sync::Arc<Node>,
        parse_errors: &[ParseError],
        source: &str,
        module_resolver: Option<&dyn Fn(&str) -> bool>,
    ) -> Vec<Diagnostic> {
        self.get_diagnostics_with_path(ast, parse_errors, source, module_resolver, &[], None)
    }

    /// Generate diagnostics for the given AST with optional source-path context.
    ///
    /// `module_search_paths` is the list of `@INC` paths that were searched during
    /// module resolution. When non-empty, PL701 diagnostics include these paths so
    /// the user can see where perl-lsp looked. Pass `&[]` when the paths are not
    /// available.
    pub fn get_diagnostics_with_path(
        &self,
        ast: &std::sync::Arc<Node>,
        parse_errors: &[ParseError],
        source: &str,
        module_resolver: Option<&dyn Fn(&str) -> bool>,
        module_search_paths: &[String],
        source_path: Option<&Path>,
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

            let code = parse_error_code(error);

            diagnostics.push(Diagnostic {
                range: (range_start, range_end),
                severity: parse_error_severity(error),
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

        // Run lint checks
        check_strict_warnings(ast, &mut diagnostics);
        check_deprecated_syntax(ast, &mut diagnostics);
        let symbol_table = SymbolExtractor::new_with_source(source).extract(ast);
        check_common_mistakes(ast, &symbol_table, &mut diagnostics);
        check_printf_format(ast, &mut diagnostics);

        // Package and subroutine diagnostics (PL200, PL201, PL300)
        check_missing_package_declaration(ast, source, source_path, &mut diagnostics);
        check_duplicate_package(ast, &mut diagnostics);
        check_duplicate_subroutine(ast, &mut diagnostics);

        // Moo/Moose role conflict diagnostics (same-file only)
        check_role_conflicts(ast, &symbol_table, &mut diagnostics);
        check_goto_labels(ast, &symbol_table, &mut diagnostics);

        // Security anti-pattern detection (string eval, two-arg open, backtick exec)
        check_security(ast, &mut diagnostics);
        check_ffi_checklib(ast, &mut diagnostics);
        check_eval_error_flow(ast, &mut diagnostics);

        // Unused import detection
        check_unused_imports(ast, source, &mut diagnostics);

        // Version compatibility lint (PL900)
        check_version_compat(ast, &mut diagnostics);

        // Unreachable code detection (PL406)
        check_unreachable_code(ast, &mut diagnostics);

        // Duplicate hash key detection (PL408)
        check_duplicate_hash_keys(ast, &mut diagnostics);

        // Missing module lint (PL701) — only when a resolver is provided
        if let Some(resolver) = module_resolver {
            crate::lints::missing_module::check_missing_modules(
                ast,
                source,
                resolver,
                module_search_paths,
                &mut diagnostics,
            );
        }

        // Remove duplicate diagnostics before returning
        deduplicate_diagnostics(&mut diagnostics);

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
    build_parse_error_hint(error, "")
}

/// Build an actionable hint for a parse error.
///
/// This is the shared implementation used by both the AST-present and fallback diagnostic
/// paths. `base_message` is the human-readable error text already derived from the error
/// variant; it is used for pattern-matching on `SyntaxError` cases where the variant's
/// `message` field may differ from what was already formatted for display.
///
/// Returns `None` when no targeted hint is available for this error pattern.
pub fn build_parse_error_hint(error: &ParseError, base_message: &str) -> Option<String> {
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
            // Comma expected between list elements
            if expected.contains(',') || expected.to_lowercase().contains("comma") {
                return Some(
                    "Expected `,` between list elements -- check for a missing comma".to_string(),
                );
            }
            // Unexpected token that looks like a lexer failure (e.g. from an unclosed string)
            if found.contains("unknown token") {
                return Some(
                    "Check for an unclosed string, regex, or heredoc near this position"
                        .to_string(),
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
            // Provide targeted suggestions for known syntax error patterns.
            // Check both the stored message and the pre-formatted base_message.
            let msg_lower = message.to_lowercase();
            let base_lower = base_message.to_lowercase();
            if msg_lower.contains("semicolon") || msg_lower.contains("missing ;") {
                Some("Add a ';' at the end of the statement".to_string())
            } else if msg_lower.contains("heredoc") || base_lower.contains("heredoc") {
                Some(
                    "Check that the heredoc terminator appears on its own line with no extra whitespace"
                        .to_string(),
                )
            } else if msg_lower.contains("unclosed")
                || (msg_lower.contains("block") && msg_lower.contains("expected"))
                || msg_lower.contains("missing '}'")
            {
                Some(
                    "Unclosed `{` -- check for a missing `}` to close the block".to_string(),
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
        // Recovered errors: the parser inserted a synthetic node and continued.
        // No user-facing suggestion is needed — the partial AST is still usable.
        ParseError::Recovered { .. } => None,
    }
}

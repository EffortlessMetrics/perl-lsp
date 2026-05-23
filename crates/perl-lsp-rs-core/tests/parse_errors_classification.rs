//! Coverage tests for parse_error_code and parse_error_severity.
//!
//! These tests exercise each `ParseError` variant against the two public
//! classification functions exported by the diagnostics provider, covering
//! the message-inference fallback path and the warning-vs-error severity
//! branch for unknown subroutine attributes and invalid prototype messages.

use perl_diagnostics::codes::DiagnosticCode;
use perl_lsp_rs_core::providers::diagnostics::{
    DiagnosticSeverity, parse_error_code, parse_error_severity,
};
use perl_parser_core::error::{ParseError, RecoveryKind, RecoverySite};

// ---- parse_error_code ------------------------------------------------------

#[test]
fn code_unexpected_eof_maps_to_unexpected_eof() {
    assert_eq!(parse_error_code(&ParseError::UnexpectedEof), DiagnosticCode::UnexpectedEof);
}

#[test]
fn code_unexpected_token_falls_through_to_parse_error() {
    let err = ParseError::UnexpectedToken {
        expected: "semicolon".into(),
        found: "}".into(),
        location: 0,
    };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_lexer_error_falls_through_to_parse_error() {
    let err = ParseError::LexerError { message: "unterminated string".into() };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_recursion_limit_falls_through_to_parse_error() {
    assert_eq!(parse_error_code(&ParseError::RecursionLimit), DiagnosticCode::ParseError);
}

#[test]
fn code_invalid_number_falls_through_to_parse_error() {
    let err = ParseError::InvalidNumber { literal: "1_2_a".into() };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_invalid_string_falls_through_to_parse_error() {
    assert_eq!(parse_error_code(&ParseError::InvalidString), DiagnosticCode::ParseError);
}

#[test]
fn code_unclosed_delimiter_falls_through_to_parse_error() {
    let err = ParseError::UnclosedDelimiter { delimiter: '{' };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_invalid_regex_falls_through_to_parse_error() {
    let err = ParseError::InvalidRegex { message: "bad escape".into() };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_nesting_too_deep_falls_through_to_parse_error() {
    let err = ParseError::NestingTooDeep { depth: 100, max_depth: 50 };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_cancelled_falls_through_to_parse_error() {
    assert_eq!(parse_error_code(&ParseError::Cancelled), DiagnosticCode::ParseError);
}

#[test]
fn code_recovered_falls_through_to_parse_error() {
    let err = ParseError::Recovered {
        site: RecoverySite::ArgList,
        kind: RecoveryKind::InsertedCloser,
        location: 0,
    };
    assert_eq!(parse_error_code(&err), DiagnosticCode::ParseError);
}

#[test]
fn code_syntax_error_unknown_message_defaults_to_syntax_error() {
    let err = ParseError::SyntaxError {
        message: "completely unrecognized rumblings".into(),
        location: 0,
    };
    assert_eq!(parse_error_code(&err), DiagnosticCode::SyntaxError);
}

#[test]
fn code_syntax_error_with_prototype_message_maps_to_invalid_prototype() {
    let err = ParseError::SyntaxError {
        message: "Illegal character in prototype for foo : '?'".into(),
        location: 0,
    };
    assert_eq!(parse_error_code(&err), DiagnosticCode::InvalidPrototype);
}

#[test]
fn code_syntax_error_with_use_strict_maps_to_missing_strict() {
    let err = ParseError::SyntaxError { message: "use strict suggested".into(), location: 0 };
    assert_eq!(parse_error_code(&err), DiagnosticCode::MissingStrict);
}

#[test]
fn code_syntax_error_with_unused_variable_maps_to_unused_variable() {
    let err =
        ParseError::SyntaxError { message: "unused variable $foo at line 1".into(), location: 0 };
    assert_eq!(parse_error_code(&err), DiagnosticCode::UnusedVariable);
}

// ---- parse_error_severity --------------------------------------------------

#[test]
fn severity_unexpected_eof_is_error() {
    assert_eq!(parse_error_severity(&ParseError::UnexpectedEof), DiagnosticSeverity::Error);
}

#[test]
fn severity_unexpected_token_is_error() {
    let err = ParseError::UnexpectedToken {
        expected: "semicolon".into(),
        found: "}".into(),
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_recursion_limit_is_error() {
    assert_eq!(parse_error_severity(&ParseError::RecursionLimit), DiagnosticSeverity::Error);
}

#[test]
fn severity_invalid_number_is_error() {
    let err = ParseError::InvalidNumber { literal: "1_2_a".into() };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_invalid_string_is_error() {
    assert_eq!(parse_error_severity(&ParseError::InvalidString), DiagnosticSeverity::Error);
}

#[test]
fn severity_unclosed_delimiter_is_error() {
    let err = ParseError::UnclosedDelimiter { delimiter: '(' };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_invalid_regex_is_error() {
    let err = ParseError::InvalidRegex { message: "bad".into() };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_lexer_error_is_error() {
    let err = ParseError::LexerError { message: "tokenizer failure".into() };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_cancelled_is_error() {
    assert_eq!(parse_error_severity(&ParseError::Cancelled), DiagnosticSeverity::Error);
}

#[test]
fn severity_nesting_too_deep_is_error() {
    let err = ParseError::NestingTooDeep { depth: 100, max_depth: 50 };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_recovered_is_error() {
    let err = ParseError::Recovered {
        site: RecoverySite::ArgList,
        kind: RecoveryKind::InsertedCloser,
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_syntax_error_default_is_error() {
    let err = ParseError::SyntaxError { message: "unrecognized syntax".into(), location: 0 };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

#[test]
fn severity_invalid_prototype_syntax_error_is_warning() {
    // Messages that map to DiagnosticCode::InvalidPrototype should be downgraded
    // to Warning so editors don't paint a hard error for a stylistic pragma.
    let err = ParseError::SyntaxError {
        message: "Illegal character in prototype for foo : '?'".into(),
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Warning);
}

#[test]
fn severity_prototype_mismatch_message_is_warning() {
    let err = ParseError::SyntaxError {
        message: "Prototype mismatch: sub foo ($$) vs sub foo ($)".into(),
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Warning);
}

#[test]
fn severity_unknown_subroutine_attribute_is_warning() {
    // The branch is gated on the exact message prefix
    // `unknown subroutine attribute ':...`; verify the warning downgrade.
    let err = ParseError::SyntaxError {
        message: "unknown subroutine attribute ':MyAttr'".into(),
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Warning);
}

#[test]
fn severity_unknown_attribute_with_different_prefix_is_error() {
    // Negative test for the warning branch: the helper checks the exact prefix,
    // so a message that merely mentions "attribute" must stay as Error.
    let err = ParseError::SyntaxError {
        message: "attribute is unknown for subroutine".into(),
        location: 0,
    };
    assert_eq!(parse_error_severity(&err), DiagnosticSeverity::Error);
}

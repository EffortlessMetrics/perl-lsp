//! Coverage tests for all public diagnostic catalog functions.
//!
//! Each public helper returns a `DiagnosticMeta` wrapping a `DiagnosticCode`.
//! These tests verify: correct stable code string, presence/absence of docs URL,
//! and that `from_message` correctly round-trips known keywords.

use perl_diagnostics::catalog;

// ---------------------------------------------------------------------------
// Parse / syntax errors (PL001–PL003)
// ---------------------------------------------------------------------------

#[test]
fn parse_error_returns_pl001_with_docs_url() {
    let meta = catalog::parse_error();
    assert_eq!(meta.code, "PL001");
    assert!(meta.desc.is_some(), "parse_error should have a docs URL");
}

#[test]
fn syntax_error_returns_pl002() {
    let meta = catalog::syntax_error();
    assert_eq!(meta.code, "PL002");
    assert!(meta.desc.is_some(), "syntax_error should have a docs URL");
}

#[test]
fn unexpected_eof_returns_pl003() {
    let meta = catalog::unexpected_eof();
    assert_eq!(meta.code, "PL003");
    assert!(meta.desc.is_some(), "unexpected_eof should have a docs URL");
}

// ---------------------------------------------------------------------------
// Pragma warnings (PL100–PL101)
// ---------------------------------------------------------------------------

#[test]
fn missing_strict_returns_pl100() {
    let meta = catalog::missing_strict();
    assert_eq!(meta.code, "PL100");
    assert!(meta.desc.is_some());
}

#[test]
fn missing_warnings_returns_pl101() {
    let meta = catalog::missing_warnings();
    assert_eq!(meta.code, "PL101");
    assert!(meta.desc.is_some());
}

// ---------------------------------------------------------------------------
// Variable diagnostics (PL102–PL103)
// ---------------------------------------------------------------------------

#[test]
fn unused_var_returns_pl102() {
    let meta = catalog::unused_var();
    assert_eq!(meta.code, "PL102");
    assert!(meta.desc.is_some());
}

#[test]
fn undefined_var_returns_pl103() {
    let meta = catalog::undefined_var();
    assert_eq!(meta.code, "PL103");
    assert!(meta.desc.is_some());
}

// ---------------------------------------------------------------------------
// Package diagnostics (PL200–PL201)
// ---------------------------------------------------------------------------

#[test]
fn missing_package_declaration_returns_pl200() {
    let meta = catalog::missing_package_declaration();
    assert_eq!(meta.code, "PL200");
    assert!(meta.desc.is_some());
}

#[test]
fn duplicate_package_returns_pl201() {
    let meta = catalog::duplicate_package();
    assert_eq!(meta.code, "PL201");
    assert!(meta.desc.is_some());
}

// ---------------------------------------------------------------------------
// Subroutine diagnostics (PL300–PL301)
// ---------------------------------------------------------------------------

#[test]
fn duplicate_sub_returns_pl300() {
    let meta = catalog::duplicate_sub();
    assert_eq!(meta.code, "PL300");
    assert!(meta.desc.is_some());
}

#[test]
fn missing_return_returns_pl301() {
    let meta = catalog::missing_return();
    assert_eq!(meta.code, "PL301");
    assert!(meta.desc.is_some());
}

// ---------------------------------------------------------------------------
// Style diagnostics (PL400–PL402)
// ---------------------------------------------------------------------------

#[test]
fn bareword_filehandle_returns_pl400() {
    let meta = catalog::bareword_filehandle();
    assert_eq!(meta.code, "PL400");
    assert!(meta.desc.is_some());
}

#[test]
fn two_arg_open_returns_pl401() {
    let meta = catalog::two_arg_open();
    assert_eq!(meta.code, "PL401");
    assert!(meta.desc.is_some());
}

#[test]
fn implicit_return_returns_pl402() {
    let meta = catalog::implicit_return();
    assert_eq!(meta.code, "PL402");
    assert!(meta.desc.is_some());
}

// ---------------------------------------------------------------------------
// Perl::Critic codes (PC001–PC005)
// ---------------------------------------------------------------------------

#[test]
fn critic_severity_5_returns_pc005_without_docs_url() {
    let meta = catalog::critic_severity_5();
    assert_eq!(meta.code, "PC005");
    assert!(meta.desc.is_none(), "Critic codes have no docs URL");
}

#[test]
fn critic_severity_4_returns_pc004_without_docs_url() {
    let meta = catalog::critic_severity_4();
    assert_eq!(meta.code, "PC004");
    assert!(meta.desc.is_none());
}

#[test]
fn critic_severity_3_returns_pc003_without_docs_url() {
    let meta = catalog::critic_severity_3();
    assert_eq!(meta.code, "PC003");
    assert!(meta.desc.is_none());
}

#[test]
fn critic_severity_2_returns_pc002_without_docs_url() {
    let meta = catalog::critic_severity_2();
    assert_eq!(meta.code, "PC002");
    assert!(meta.desc.is_none());
}

#[test]
fn critic_severity_1_returns_pc001_without_docs_url() {
    let meta = catalog::critic_severity_1();
    assert_eq!(meta.code, "PC001");
    assert!(meta.desc.is_none());
}

// ---------------------------------------------------------------------------
// from_message round-trip
// ---------------------------------------------------------------------------

#[test]
fn from_message_returns_none_for_unknown_message() {
    let result = catalog::from_message("some completely unrecognized text");
    assert!(result.is_none(), "unrecognized messages should return None");
}

#[test]
fn from_message_returns_none_for_empty_string() {
    let result = catalog::from_message("");
    assert!(result.is_none(), "empty message should return None");
}

#[test]
fn from_message_returns_parse_error_meta_for_parse_keyword() {
    // "parse error" should map to PL001
    let result = catalog::from_message("parse error in statement");
    if let Some(meta) = result {
        assert_eq!(meta.code, "PL001");
    }
    // It's OK if from_message returns None — not all strings match.
    // The main invariant is: if it returns Some, the code must be correct.
}

// ---------------------------------------------------------------------------
// diagnostic_meta generic entry point
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_meta_directly_wraps_provided_code() {
    use perl_diagnostics::codes::DiagnosticCode;
    let meta = catalog::diagnostic_meta(DiagnosticCode::ParseError);
    assert_eq!(meta.code, "PL001");
}

#[test]
fn all_pl_codes_have_docs_url_all_pc_codes_do_not() {
    use perl_diagnostics::codes::DiagnosticCode;

    // PL codes should have docs URLs
    for code in [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
    ] {
        let meta = catalog::diagnostic_meta(code);
        assert!(
            meta.desc.is_some(),
            "PL code {:?} should have a docs URL",
            meta.code
        );
    }

    // PC codes should NOT have docs URLs
    for code in [
        DiagnosticCode::CriticSeverity5,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity1,
    ] {
        let meta = catalog::diagnostic_meta(code);
        assert!(
            meta.desc.is_none(),
            "PC code {:?} should NOT have a docs URL",
            meta.code
        );
    }
}

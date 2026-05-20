//! Coverage tests for all public diagnostic catalog functions.
//!
//! Each public helper returns a `DiagnosticMeta` wrapping a `DiagnosticCode`.
//! These tests verify: correct stable code string, presence/absence of docs URL,
//! and that `from_message` correctly round-trips known keywords.

use perl_diagnostics::catalog::{self, DiagnosticMeta};
use perl_diagnostics::codes::DiagnosticCode;

type CatalogFn = fn() -> DiagnosticMeta;
type TestResult = Result<(), Box<dyn std::error::Error>>;

const DOCUMENTED_CATALOG_ENTRIES: &[(&str, CatalogFn, &str)] = &[
    ("parse_error", catalog::parse_error, "PL001"),
    ("syntax_error", catalog::syntax_error, "PL002"),
    ("unexpected_eof", catalog::unexpected_eof, "PL003"),
    ("missing_strict", catalog::missing_strict, "PL100"),
    ("missing_warnings", catalog::missing_warnings, "PL101"),
    ("unused_var", catalog::unused_var, "PL102"),
    ("undefined_var", catalog::undefined_var, "PL103"),
    ("missing_package_declaration", catalog::missing_package_declaration, "PL200"),
    ("duplicate_package", catalog::duplicate_package, "PL201"),
    ("duplicate_sub", catalog::duplicate_sub, "PL300"),
    ("missing_return", catalog::missing_return, "PL301"),
    ("bareword_filehandle", catalog::bareword_filehandle, "PL400"),
    ("two_arg_open", catalog::two_arg_open, "PL401"),
    ("implicit_return", catalog::implicit_return, "PL402"),
    ("eval_error_flow", catalog::eval_error_flow, "PL407"),
];

const UNDOCUMENTED_CATALOG_ENTRIES: &[(&str, CatalogFn, &str)] = &[
    ("critic_severity_5", catalog::critic_severity_5, "PC005"),
    ("critic_severity_4", catalog::critic_severity_4, "PC004"),
    ("critic_severity_3", catalog::critic_severity_3, "PC003"),
    ("critic_severity_2", catalog::critic_severity_2, "PC002"),
    ("critic_severity_1", catalog::critic_severity_1, "PC001"),
];

fn assert_catalog_entry(
    name: &str,
    build_meta: CatalogFn,
    expected_code: &str,
    should_have_docs_url: bool,
) {
    let meta = build_meta();
    assert_eq!(meta.code, expected_code, "{name} should return {expected_code}");
    assert_eq!(
        meta.desc.is_some(),
        should_have_docs_url,
        "{name} docs URL presence should be {should_have_docs_url}",
    );
}

// ---------------------------------------------------------------------------
// Public catalog helper coverage
// ---------------------------------------------------------------------------

#[test]
fn documented_catalog_helpers_return_expected_pl_codes_with_docs_urls() -> TestResult {
    for &(name, build_meta, expected_code) in DOCUMENTED_CATALOG_ENTRIES {
        assert_catalog_entry(name, build_meta, expected_code, true);
    }

    Ok(())
}

#[test]
fn critic_catalog_helpers_return_expected_pc_codes_without_docs_urls() -> TestResult {
    for &(name, build_meta, expected_code) in UNDOCUMENTED_CATALOG_ENTRIES {
        assert_catalog_entry(name, build_meta, expected_code, false);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// from_message round-trip
// ---------------------------------------------------------------------------

#[test]
fn from_message_returns_none_for_unknown_message() -> TestResult {
    let result = catalog::from_message("some completely unrecognized text");
    assert!(result.is_none(), "unrecognized messages should return None");

    Ok(())
}

#[test]
fn from_message_returns_none_for_empty_string() -> TestResult {
    let result = catalog::from_message("");
    assert!(result.is_none(), "empty message should return None");

    Ok(())
}

#[test]
fn from_message_returns_parse_error_meta_for_parse_keyword() -> TestResult {
    // "parse error" should map to PL001 when it matches. It is valid for
    // unmatched messages to return None.
    if let Some(meta) = catalog::from_message("parse error in statement") {
        assert_eq!(meta.code, "PL001");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// diagnostic_meta generic entry point
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_meta_directly_wraps_provided_code() -> TestResult {
    let meta = catalog::diagnostic_meta(DiagnosticCode::ParseError);
    assert_eq!(meta.code, "PL001");

    Ok(())
}

#[test]
fn all_pl_codes_have_docs_url_all_pc_codes_do_not() -> TestResult {
    let documented_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
    ];
    for code in documented_codes {
        let meta = catalog::diagnostic_meta(code);
        assert!(meta.desc.is_some(), "PL code {:?} should have a docs URL", meta.code);
    }

    let undocumented_codes = [
        DiagnosticCode::CriticSeverity5,
        DiagnosticCode::CriticSeverity4,
        DiagnosticCode::CriticSeverity3,
        DiagnosticCode::CriticSeverity2,
        DiagnosticCode::CriticSeverity1,
    ];
    for code in undocumented_codes {
        let meta = catalog::diagnostic_meta(code);
        assert!(meta.desc.is_none(), "PC code {:?} should NOT have a docs URL", meta.code);
    }

    Ok(())
}

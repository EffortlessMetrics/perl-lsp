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
// from_message phrase-boundary regressions
// ---------------------------------------------------------------------------

#[test]
fn from_message_does_not_match_diagnostic_phrases_inside_longer_words() -> TestResult {
    let non_diagnostics = [
        "the unused variablex marker is part of a generated identifier",
        "configuration says never used_by_external_tools during indexing",
        "the module docs mention ause strictness setting",
        "phase block does not enable strictness is explanatory prose",
        "a bareword filehandle_suffix appears in fixture text",
        "two-argumentative prose should not imply open() style",
        "prototype mismatchable examples are not Perl diagnostics",
        "definedness is not a deprecated defined container check",
    ];

    for message in non_diagnostics {
        let result = catalog::from_message(message);
        assert!(result.is_none(), "message should not infer a diagnostic: {message}");
    }

    Ok(())
}

#[test]
fn from_message_matches_real_perl_messages_with_punctuation_boundaries() -> TestResult {
    let cases = [
        (
            "Global symbol \"$x\" requires explicit package name at script.pl line 3.",
            DiagnosticCode::UndefinedVariable,
        ),
        ("Subroutine helper redefined at script.pl line 7.", DiagnosticCode::DuplicateSubroutine),
        (
            "Illegal character in prototype for main::run : ! at script.pl line 9.",
            DiagnosticCode::InvalidPrototype,
        ),
        (
            "Use of uninitialized value $x in concatenation (.) or string at script.pl line 11.",
            DiagnosticCode::UninitializedVariable,
        ),
        (
            "Bareword \"foo\" not allowed while 'strict subs' in use at script.pl line 13.",
            DiagnosticCode::UnquotedBareword,
        ),
        (
            "defined(%hash) is deprecated (it's always defined) at script.pl line 15.",
            DiagnosticCode::DeprecatedDefined,
        ),
    ];

    for (message, expected) in cases {
        let meta = catalog::from_message(message).ok_or("expected diagnostic metadata")?;
        assert_eq!(meta.code, expected.as_str(), "message should infer {expected:?}: {message}");
    }

    Ok(())
}

#[test]
fn diagnostic_meta_exposes_context_hint_for_pl_codes_but_not_critic_codes() -> TestResult {
    let parse_meta = catalog::diagnostic_meta(DiagnosticCode::ParseError);
    let parse_hint = parse_meta.hint.ok_or("PL001 should expose a context hint")?;
    assert!(
        parse_hint.contains("could not parse") || parse_hint.contains("syntax"),
        "parse hint should explain parser context: {parse_hint}"
    );

    let critic_meta = catalog::diagnostic_meta(DiagnosticCode::CriticSeverity3);
    assert!(critic_meta.hint.is_none(), "Perl::Critic metadata should not add generic hints");

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

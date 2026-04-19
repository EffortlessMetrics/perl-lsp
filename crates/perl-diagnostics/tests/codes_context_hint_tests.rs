//! Tests for DiagnosticCode::context_hint() — human-readable error hints.
//!
//! Issue #2316: users see cryptic codes like `unexpected_rbrace_expr` and
//! disable diagnostics thinking the LSP is broken. Each code should return
//! an actionable hint string.

use perl_diagnostics::codes::DiagnosticCode;

// ---------------------------------------------------------------------------
// Parser codes — hints required (PL001-PL003)
// ---------------------------------------------------------------------------

#[test]
fn parse_error_has_context_hint() {
    let hint = DiagnosticCode::ParseError.context_hint();
    assert!(
        hint.is_some(),
        "ParseError (PL001) must have a context hint"
    );
    if let Some(text) = hint {
        assert!(!text.is_empty(), "context hint must not be empty");
    }
}

#[test]
fn syntax_error_has_context_hint() {
    let hint = DiagnosticCode::SyntaxError.context_hint();
    assert!(
        hint.is_some(),
        "SyntaxError (PL002) must have a context hint"
    );
}

#[test]
fn unexpected_eof_has_context_hint() {
    let hint = DiagnosticCode::UnexpectedEof.context_hint();
    assert!(
        hint.is_some(),
        "UnexpectedEof (PL003) must have a context hint"
    );
}

// ---------------------------------------------------------------------------
// Pragma codes — hints required (PL100-PL101)
// ---------------------------------------------------------------------------

#[test]
fn missing_strict_has_context_hint() {
    let hint = DiagnosticCode::MissingStrict.context_hint();
    assert!(
        hint.is_some(),
        "MissingStrict (PL100) must have a context hint"
    );
    if let Some(text) = hint {
        assert!(
            text.to_lowercase().contains("strict"),
            "MissingStrict hint should mention strict, got: {text}"
        );
    }
}

#[test]
fn missing_warnings_has_context_hint() {
    let hint = DiagnosticCode::MissingWarnings.context_hint();
    assert!(
        hint.is_some(),
        "MissingWarnings (PL101) must have a context hint"
    );
    if let Some(text) = hint {
        assert!(
            text.to_lowercase().contains("warn"),
            "MissingWarnings hint should mention warnings, got: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// Variable codes — hints required (PL102-PL103)
// ---------------------------------------------------------------------------

#[test]
fn unused_variable_has_context_hint() {
    let hint = DiagnosticCode::UnusedVariable.context_hint();
    assert!(
        hint.is_some(),
        "UnusedVariable (PL102) must have a context hint"
    );
}

#[test]
fn undefined_variable_has_context_hint() {
    let hint = DiagnosticCode::UndefinedVariable.context_hint();
    assert!(
        hint.is_some(),
        "UndefinedVariable (PL103) must have a context hint"
    );
}

// ---------------------------------------------------------------------------
// Package codes — hints required (PL200-PL201)
// ---------------------------------------------------------------------------

#[test]
fn missing_package_declaration_has_context_hint() {
    let hint = DiagnosticCode::MissingPackageDeclaration.context_hint();
    assert!(
        hint.is_some(),
        "MissingPackageDeclaration (PL200) must have a context hint"
    );
}

#[test]
fn duplicate_package_has_context_hint() {
    let hint = DiagnosticCode::DuplicatePackage.context_hint();
    assert!(
        hint.is_some(),
        "DuplicatePackage (PL201) must have a context hint"
    );
}

// ---------------------------------------------------------------------------
// Subroutine codes — hints required (PL300-PL301)
// ---------------------------------------------------------------------------

#[test]
fn duplicate_subroutine_has_context_hint() {
    let hint = DiagnosticCode::DuplicateSubroutine.context_hint();
    assert!(
        hint.is_some(),
        "DuplicateSubroutine (PL300) must have a context hint"
    );
}

#[test]
fn missing_return_has_context_hint() {
    let hint = DiagnosticCode::MissingReturn.context_hint();
    assert!(
        hint.is_some(),
        "MissingReturn (PL301) must have a context hint"
    );
}

// ---------------------------------------------------------------------------
// Best-practice codes — hints required (PL400-PL402)
// ---------------------------------------------------------------------------

#[test]
fn bareword_filehandle_has_context_hint() {
    let hint = DiagnosticCode::BarewordFilehandle.context_hint();
    assert!(
        hint.is_some(),
        "BarewordFilehandle (PL400) must have a context hint"
    );
}

#[test]
fn two_arg_open_has_context_hint() {
    let hint = DiagnosticCode::TwoArgOpen.context_hint();
    assert!(
        hint.is_some(),
        "TwoArgOpen (PL401) must have a context hint"
    );
}

#[test]
fn implicit_return_has_context_hint() {
    let hint = DiagnosticCode::ImplicitReturn.context_hint();
    assert!(
        hint.is_some(),
        "ImplicitReturn (PL402) must have a context hint"
    );
}

// ---------------------------------------------------------------------------
// Perl::Critic codes — hints intentionally absent (PC001-PC005)
//
// Critic violations already carry per-policy descriptions from perlcritic
// itself; adding generic hints would be redundant and misleading.
// ---------------------------------------------------------------------------

#[test]
fn critic_severity_1_has_no_context_hint() {
    assert!(
        DiagnosticCode::CriticSeverity1.context_hint().is_none(),
        "CriticSeverity1 (PC001) must NOT have a generic context hint"
    );
}

#[test]
fn critic_severity_2_has_no_context_hint() {
    assert!(DiagnosticCode::CriticSeverity2.context_hint().is_none());
}

#[test]
fn critic_severity_3_has_no_context_hint() {
    assert!(DiagnosticCode::CriticSeverity3.context_hint().is_none());
}

#[test]
fn critic_severity_4_has_no_context_hint() {
    assert!(DiagnosticCode::CriticSeverity4.context_hint().is_none());
}

#[test]
fn critic_severity_5_has_no_context_hint() {
    assert!(DiagnosticCode::CriticSeverity5.context_hint().is_none());
}

// ---------------------------------------------------------------------------
// Hint quality: must be actionable (not just a label)
// ---------------------------------------------------------------------------

#[test]
fn all_pl_hints_are_at_least_20_chars() {
    let pl_codes = [
        DiagnosticCode::ParseError,
        DiagnosticCode::SyntaxError,
        DiagnosticCode::UnexpectedEof,
        DiagnosticCode::MissingStrict,
        DiagnosticCode::MissingWarnings,
        DiagnosticCode::UnusedVariable,
        DiagnosticCode::UndefinedVariable,
        DiagnosticCode::MissingPackageDeclaration,
        DiagnosticCode::DuplicatePackage,
        DiagnosticCode::DuplicateSubroutine,
        DiagnosticCode::MissingReturn,
        DiagnosticCode::BarewordFilehandle,
        DiagnosticCode::TwoArgOpen,
        DiagnosticCode::ImplicitReturn,
    ];
    for code in pl_codes {
        let hint = code.context_hint();
        assert!(hint.is_some(), "PL code {code:?} must have a context hint");
        if let Some(text) = hint {
            assert!(
                text.len() >= 20,
                "context hint for {code:?} is too short ({} chars): {text:?}",
                text.len()
            );
        }
    }
}

//! Tests that DiagnosticMeta exposes context hints from DiagnosticCode.
//!
//! Issue #2316: the catalog layer must surface the `context_hint` so that
//! consumers (LSP providers, formatters) can attach it to error messages
//! without pulling in `perl-diagnostics-codes` directly.

use perl_diagnostics::catalog;

#[test]
fn parse_error_meta_has_hint() {
    let meta = catalog::parse_error();
    assert!(
        meta.hint.is_some(),
        "parse_error DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn syntax_error_meta_has_hint() {
    let meta = catalog::syntax_error();
    assert!(
        meta.hint.is_some(),
        "syntax_error DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn unexpected_eof_meta_has_hint() {
    let meta = catalog::unexpected_eof();
    assert!(
        meta.hint.is_some(),
        "unexpected_eof DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn missing_strict_meta_has_hint() {
    let meta = catalog::missing_strict();
    assert!(
        meta.hint.is_some(),
        "missing_strict DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn missing_warnings_meta_has_hint() {
    let meta = catalog::missing_warnings();
    assert!(
        meta.hint.is_some(),
        "missing_warnings DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn unused_var_meta_has_hint() {
    let meta = catalog::unused_var();
    assert!(
        meta.hint.is_some(),
        "unused_var DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn undefined_var_meta_has_hint() {
    let meta = catalog::undefined_var();
    assert!(
        meta.hint.is_some(),
        "undefined_var DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn bareword_filehandle_meta_has_hint() {
    let meta = catalog::bareword_filehandle();
    assert!(
        meta.hint.is_some(),
        "bareword_filehandle DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn two_arg_open_meta_has_hint() {
    let meta = catalog::two_arg_open();
    assert!(
        meta.hint.is_some(),
        "two_arg_open DiagnosticMeta must expose a context hint"
    );
}

#[test]
fn critic_codes_have_no_hint_in_meta() {
    // Critic codes intentionally have no generic hint — the policy description is the hint
    let meta = catalog::critic_severity_1();
    assert!(
        meta.hint.is_none(),
        "critic_severity_1 DiagnosticMeta must NOT expose a generic context hint"
    );
    let meta = catalog::critic_severity_5();
    assert!(
        meta.hint.is_none(),
        "critic_severity_5 must also have no hint"
    );
}

#[test]
fn hint_content_is_non_empty_string_when_present() {
    let meta = catalog::parse_error();
    if let Some(hint) = meta.hint {
        assert!(
            !hint.is_empty(),
            "DiagnosticMeta hint must not be an empty string"
        );
    }
}

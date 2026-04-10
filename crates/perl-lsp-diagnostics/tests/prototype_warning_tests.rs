//! Regression tests for PL302 invalid-prototype warning pipeline (issue #3644)
//!
//! The parser emits `ParseError::SyntaxError` with an "Invalid prototype character(s)"
//! message.  The diagnostics layer must:
//!
//! 1. Promote the code from the generic `PL002` (SyntaxError) to `PL302` (InvalidPrototype)
//!    via `DiagnosticCode::from_message`.
//! 2. Surface the diagnostic with `Warning` severity (not `Error`).
//!
//! These tests are the regression guard for that wiring.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn find_code(diags: &[Diagnostic], code: &str) -> Option<Diagnostic> {
    diags.iter().find(|d| d.code.as_deref() == Some(code)).cloned()
}

// =========================================================================
// PL302 — InvalidPrototype
// =========================================================================

/// PL302 appears when the prototype contains invalid characters.
///
/// `sub foo (XYZ) { }` — X, Y, Z are not valid prototype characters.
#[test]
fn pl302_fires_for_invalid_prototype_chars() {
    let source = "sub foo (XYZ) { }\n";
    let diags = diagnostics_for(source);
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code.as_deref()).collect();
    assert!(
        codes.contains(&"PL302"),
        "Expected PL302 (InvalidPrototype) for 'sub foo (XYZ)'. Got codes: {codes:?}"
    );
}

/// PL302 must not appear for a valid prototype string.
#[test]
fn pl302_absent_for_valid_prototype() {
    let source = "sub foo ($@) { }\n";
    let diags = diagnostics_for(source);
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code.as_deref()).collect();
    assert!(
        !codes.contains(&"PL302"),
        "PL302 should NOT fire for valid prototype '$@'. Got codes: {codes:?}"
    );
}

/// PL302 carries Warning severity, not Error — invalid prototypes do not
/// prevent Perl from running the code, they are merely Perl-level warnings.
#[test]
fn pl302_has_warning_severity() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo (XYZ) { }\n";
    let diags = diagnostics_for(source);
    let pl302 = find_code(&diags, "PL302").ok_or_else(|| {
        format!("Expected PL302 in diagnostics for 'sub foo (XYZ)'. Got: {diags:?}")
    })?;
    assert_eq!(
        pl302.severity,
        DiagnosticSeverity::Warning,
        "PL302 must be Warning severity, got {:?}",
        pl302.severity
    );
    Ok(())
}

/// PL302 must not collide with PL002 (generic SyntaxError) — the code
/// should be promoted to PL302, not left as PL002.
#[test]
fn pl302_not_emitted_as_pl002() {
    let source = "sub foo (XYZ) { }\n";
    let diags = diagnostics_for(source);
    // PL302 present
    let has_302 = diags.iter().any(|d| d.code.as_deref() == Some("PL302"));
    // PL002 must NOT be present for this prototype warning
    let prototype_pl002 = diags.iter().any(|d| {
        d.code.as_deref() == Some("PL002") && d.message.to_lowercase().contains("invalid prototype")
    });
    assert!(has_302, "PL302 must appear for invalid prototype");
    assert!(!prototype_pl002, "The prototype warning must NOT be emitted as PL002");
}

/// Multiple invalid characters all produce exactly one PL302, not one per character.
#[test]
fn pl302_single_diagnostic_for_multiple_invalid_chars() {
    let source = "sub bar (XYZ) { }\n";
    let diags = diagnostics_for(source);
    let count = diags.iter().filter(|d| d.code.as_deref() == Some("PL302")).count();
    assert_eq!(count, 1, "Expected exactly one PL302 for 'XYZ' prototype, got {count}");
}

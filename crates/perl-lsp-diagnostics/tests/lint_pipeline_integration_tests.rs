//! Integration tests: lint checks wired into DiagnosticsProvider::get_diagnostics()
//!
//! These tests verify that the full pipeline — real Perl source → parser → DiagnosticsProvider
//! — emits lint diagnostics (missing-strict, missing-warnings, assignment-in-condition,
//! deprecated-defined).
//!
//! Each test FAILS before the fix (lints not wired) and PASSES after the fix.
//!
//! See: crates/perl-lsp-diagnostics/src/diagnostics.rs::get_diagnostics()
//! Root cause: check_deprecated_syntax, check_strict_warnings, check_common_mistakes
//! are never called from get_diagnostics() — dropped in commit 8e5273dbf.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source)
}

// =========================================================================
// 1. missing-strict fires through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_missing_strict_emits_pl100() {
    // A plain Perl script without 'use strict' -- should get missing-strict advisory
    let source = "my $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    let missing_strict: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).collect();

    assert!(
        !missing_strict.is_empty(),
        "Expected missing-strict (PL100) diagnostic from get_diagnostics(), \
         got {} total diags with codes: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(
        missing_strict[0].severity,
        DiagnosticSeverity::Information,
        "missing-strict should be Information severity"
    );
}

// =========================================================================
// 2. missing-warnings fires through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_missing_warnings_emits_pl101() {
    let source = "use strict;\nmy $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    let missing_warnings: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL101")).collect();

    assert!(
        !missing_warnings.is_empty(),
        "Expected missing-warnings (PL101) diagnostic, \
         got {} total diags: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(missing_warnings[0].severity, DiagnosticSeverity::Information);
}

// =========================================================================
// 3. clean Perl (strict + warnings present) suppresses missing-strict/missing-warnings
// =========================================================================

#[test]
fn lint_pipeline_strict_and_warnings_present_no_pl100_pl101() {
    let source = "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);

    let pragma_diags: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();

    assert!(
        pragma_diags.is_empty(),
        "Should get no missing-strict (PL100)/missing-warnings (PL101) when strict+warnings are present, \
         got: {:?}",
        pragma_diags.iter().map(|d| d.code.as_deref()).collect::<Vec<_>>()
    );
}

// =========================================================================
// 4. deprecated defined(@array) fires through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_deprecated_defined_array_emits_pl500() {
    // defined(@array) is deprecated since Perl 5.6.1 -- parser emits FunctionCall node
    let source = "use strict;\nuse warnings;\nmy @arr = (1, 2, 3);\nif (defined @arr) { }\n";
    let diags = diagnostics_for(source);

    let deprecated: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL500")).collect();

    assert!(
        !deprecated.is_empty(),
        "Expected deprecated-defined (PL500) diagnostic, \
         got {} total diags: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(deprecated[0].severity, DiagnosticSeverity::Warning);
    assert!(
        deprecated[0].tags.contains(&DiagnosticTag::Deprecated),
        "deprecated-defined should carry DiagnosticTag::Deprecated"
    );
}

// =========================================================================
// 5. assignment-in-condition fires through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_assignment_in_if_emits_pl403() {
    // if ($x = 1) -- assignment in condition, likely meant ==
    let source = "use strict;\nuse warnings;\nmy $x;\nif ($x = 1) { print $x; }\n";
    let diags = diagnostics_for(source);

    let assign_warn: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL403")).collect();

    assert!(
        !assign_warn.is_empty(),
        "Expected assignment-in-condition (PL403) diagnostic, \
         got {} total diags: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(assign_warn[0].severity, DiagnosticSeverity::Warning);
    assert!(
        assign_warn[0].suggestion.is_some(),
        "assignment-in-condition should carry a suggestion"
    );
}

// =========================================================================
// 6. Moose (implicit strict) suppresses missing-strict/missing-warnings
// =========================================================================

#[test]
fn lint_pipeline_moose_suppresses_pl100_pl101() {
    // Moose provides implicit strict + warnings -- no missing-strict/missing-warnings expected
    let source = "package MyClass;\nuse Moose;\nhas 'name' => (is => 'ro', isa => 'Str');\n1;\n";
    let diags = diagnostics_for(source);

    let pragma_diags: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();

    assert!(
        pragma_diags.is_empty(),
        "Moose provides implicit strict+warnings, expected no missing-strict (PL100)/missing-warnings (PL101), \
         got: {:?}",
        pragma_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// =========================================================================
// 7. use strict inside BEGIN block suppresses missing-strict (issue #2360)
// =========================================================================

#[test]
fn lint_pipeline_strict_inside_begin_suppresses_pl100() {
    // use strict declared inside BEGIN { } must still suppress the missing-strict advisory.
    // This tests the walker.rs PhaseBlock recursion fix.
    let source = "BEGIN { use strict; }\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let missing_strict: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).collect();
    assert!(
        missing_strict.is_empty(),
        "use strict inside BEGIN should suppress PL100, got {} missing-strict diags",
        missing_strict.len()
    );
}

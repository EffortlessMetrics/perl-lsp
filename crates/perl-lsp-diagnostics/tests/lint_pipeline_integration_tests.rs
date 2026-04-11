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
use tempfile::tempdir;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
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

// =========================================================================
// 8. use warnings inside END block suppresses missing-warnings (issue #2360)
// =========================================================================

#[test]
fn lint_pipeline_warnings_inside_end_suppresses_pl101() {
    // use warnings declared inside END { } must still suppress PL101.
    // All 5 phase keyword bodies are walked by the PhaseBlock fix.
    let source = "use strict;\nEND { use warnings; }\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let missing_warnings: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL101")).collect();
    assert!(
        missing_warnings.is_empty(),
        "use warnings inside END should suppress PL101, got {} missing-warnings diags",
        missing_warnings.len()
    );
}

// =========================================================================
// 9. conditional `use if` pragmas conservatively suppress missing-pragmas
// =========================================================================

#[test]
fn lint_pipeline_use_if_strict_suppresses_pl100() {
    let source = "use if $^O eq 'MSWin32', 'strict';\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let missing_strict: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).collect();
    assert!(
        missing_strict.is_empty(),
        "conditional use-if strict should conservatively suppress PL100, got {} missing-strict diags",
        missing_strict.len()
    );
}

#[test]
fn lint_pipeline_use_if_warnings_suppresses_pl101() {
    let source = "use strict;\nuse if $^O eq 'MSWin32', 'warnings';\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let missing_warnings: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL101")).collect();
    assert!(
        missing_warnings.is_empty(),
        "conditional use-if warnings should conservatively suppress PL101, got {} missing-warnings diags",
        missing_warnings.len()
    );
}

// =========================================================================
// 10. use strict inside non-BEGIN phase block (INIT) suppresses PL100 (#2360)
// =========================================================================

#[test]
fn lint_pipeline_strict_inside_init_suppresses_pl100() {
    // All phase block keywords (not just BEGIN) must be recursed into.
    let source = "INIT { use strict; }\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let missing_strict: Vec<_> =
        diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).collect();
    assert!(
        missing_strict.is_empty(),
        "use strict inside INIT should suppress PL100, got {} missing-strict diags",
        missing_strict.len()
    );
}

// =========================================================================
// 11. Security lint: string eval fires through the full pipeline (#2693)
// =========================================================================

#[test]
fn lint_pipeline_string_eval_emits_pl600() {
    // eval("code") -- string eval is a security risk, should emit PL600
    let source = "use strict;\nuse warnings;\neval(\"system('rm -rf /');\");\n";
    let diags = diagnostics_for(source);

    let security: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL600")).collect();

    assert!(
        !security.is_empty(),
        "Expected security-string-eval (PL600) diagnostic from get_diagnostics(), \
         got {} total diags with codes: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(
        security[0].severity,
        DiagnosticSeverity::Warning,
        "security-string-eval should be Warning severity"
    );
    assert!(security[0].suggestion.is_some(), "security-string-eval should carry a suggestion");
}

// =========================================================================
// 12. Eval error flow lint fires through the full pipeline (#3380)
// =========================================================================

#[test]
fn lint_pipeline_eval_error_flow_emits_pl407() {
    let source = "use v5.40;\neval { risky() };\nmy $marker = 1;\nif ($@) { warn $@; }\n";
    let diags = diagnostics_for(source);

    let flow: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL407")).collect();

    assert!(
        !flow.is_empty(),
        "Expected eval-error-flow (PL407) diagnostic from get_diagnostics(), \
         got {} total diags with codes: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(flow[0].severity, DiagnosticSeverity::Warning);
}

// =========================================================================
// 12. Security lint: global SIG handlers fire through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_global_sig_handler_emits_pl602() {
    let source = "use strict;\nuse warnings;\n$main::SIG{'__WARN__'} = sub { warn \"caught\" };\n";
    let diags = diagnostics_for(source);

    let signal: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL602")).collect();

    assert!(
        !signal.is_empty(),
        "Expected security-signal-handler (PL602) diagnostic from get_diagnostics(), \
         got {} total diags with codes: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(signal[0].severity, DiagnosticSeverity::Warning);
    assert!(signal[0].suggestion.is_some(), "security-signal-handler should carry a suggestion");
}

#[test]
fn lint_pipeline_lexical_sig_shadow_does_not_emit_pl602() {
    let source =
        "use strict;\nuse warnings;\nmy %SIG;\n$SIG{__WARN__} = sub { warn \"caught\" };\n";
    let diags = diagnostics_for(source);

    assert!(
        diags.iter().all(|d| d.code.as_deref() != Some("PL602")),
        "lexical %SIG shadow should not emit PL602 from get_diagnostics(), got {:?}",
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
}

// =========================================================================
// 13. Unused imports lint fires through the full pipeline (#2694)
// =========================================================================

#[test]
fn lint_pipeline_unused_import_emits_pl700() {
    // `use Some::Module;` with no reference to Some::Module elsewhere -- should emit PL700
    let source = "use strict;\nuse warnings;\nuse Some::Module;\nmy $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    let unused: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL700")).collect();

    assert!(
        !unused.is_empty(),
        "Expected unused-import (PL700) diagnostic from get_diagnostics(), \
         got {} total diags with codes: {:?}",
        diags.len(),
        diags.iter().map(|d| d.code.as_deref().unwrap_or("none")).collect::<Vec<_>>()
    );
    assert_eq!(
        unused[0].severity,
        DiagnosticSeverity::Hint,
        "unused-import should be Hint severity"
    );
    assert!(
        unused[0].tags.contains(&DiagnosticTag::Unnecessary),
        "unused-import should carry DiagnosticTag::Unnecessary"
    );
}

// =========================================================================
// 14. Used import does NOT fire PL700 (#2694)
// =========================================================================

#[test]
fn lint_pipeline_used_import_no_pl700() {
    // `use Some::Module;` WITH a reference to Some::Module -- should NOT emit PL700
    let source = "use strict;\nuse warnings;\nuse Some::Module;\nmy $obj = Some::Module->new();\n";
    let diags = diagnostics_for(source);

    let unused: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL700")).collect();

    assert!(
        unused.is_empty(),
        "Should NOT get unused-import (PL700) when module is referenced, \
         got: {:?}",
        unused.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// =========================================================================
// 15. Dedup removes duplicate diagnostics (#2696)
// =========================================================================

#[test]
fn lint_pipeline_dedup_removes_exact_duplicates() {
    // The dedup stage runs at the end of get_diagnostics(). We verify that no
    // two diagnostics in the output share the same (range, severity, code, message).
    let source = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";
    let diags = diagnostics_for(source);

    for (i, a) in diags.iter().enumerate() {
        for b in diags.iter().skip(i + 1) {
            let is_dup = a.range == b.range
                && a.severity == b.severity
                && a.code == b.code
                && a.message == b.message;
            assert!(
                !is_dup,
                "Found duplicate diagnostics after dedup: code={:?}, message={:?}",
                a.code, a.message
            );
        }
    }
}

// =========================================================================
// 16. FFI::CheckLib hints surface through the full pipeline
// =========================================================================

#[test]
fn lint_pipeline_ffi_checklib_missing_library_emits_hint() {
    let tempdir = tempdir().unwrap();
    let source = format!(
        "use FFI::CheckLib;\nfind_lib(lib => 'ffi_checklib_pipeline_missing_3574', libpath => '{}');\n",
        tempdir.path().display()
    );
    let diags = diagnostics_for(&source);

    assert!(
        diags.iter().any(|d| d.message.contains("ffi_checklib_pipeline_missing_3574")),
        "Expected an FFI::CheckLib missing-library diagnostic, got: {:?}",
        diags.iter().map(|d| d.message.as_str()).collect::<Vec<_>>()
    );
}

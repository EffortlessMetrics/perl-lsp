//! Tests for printf/sprintf format specifier arity diagnostic (PL405)
//!
//! These tests cover:
//! - Mismatch cases: too few or too many arguments relative to format specifiers
//! - Correct cases: exact match, %% not counted, variable format strings
//! - IndirectCall form: printf FILEHANDLE FORMAT, LIST

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn printf_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL405"))
        .collect()
}

// --- Mismatch cases (should fire) ---

#[test]
fn sprintf_too_few_args_fires_pl405() {
    let diags = printf_diags("my $s = sprintf \"%s is %d\", $name;\n");
    assert_eq!(diags.len(), 1, "2 specifiers, 1 arg should fire PL405");
    assert!(
        diags[0].message.contains("2 specifier"),
        "message should mention 2 specifiers"
    );
    assert!(
        diags[0].message.contains("1 argument"),
        "message should mention 1 argument"
    );
}

#[test]
fn sprintf_too_many_args_fires_pl405() {
    let diags = printf_diags("my $s = sprintf \"%s\", $a, $b;\n");
    assert_eq!(diags.len(), 1, "1 specifier, 2 args should fire PL405");
}

#[test]
fn printf_too_few_args_fires_pl405() {
    let diags = printf_diags("printf \"%d %d\", $a;\n");
    assert_eq!(diags.len(), 1, "2 specifiers, 1 arg should fire PL405");
}

// --- Correct cases (should NOT fire) ---

#[test]
fn sprintf_exact_match_no_pl405() {
    let diags = printf_diags("my $s = sprintf \"%s: %d\", $name, $age;\n");
    assert!(
        diags.is_empty(),
        "2 specifiers, 2 args should not fire PL405"
    );
}

#[test]
fn printf_double_percent_not_counted() {
    // %% is a literal percent sign — does NOT consume an argument
    let diags = printf_diags("printf \"%s %%\", $name;\n");
    assert!(
        diags.is_empty(),
        "%% should not be counted as a specifier consuming an arg"
    );
}

#[test]
fn sprintf_no_specifiers_no_args_no_pl405() {
    let diags = printf_diags("my $s = sprintf \"hello world\";\n");
    assert!(
        diags.is_empty(),
        "0 specifiers, 0 args should not fire PL405"
    );
}

#[test]
fn variable_format_string_no_pl405() {
    // Variable format — cannot statically validate; must not produce false positives
    let diags = printf_diags("printf $fmt, $a, $b;\n");
    assert!(
        diags.is_empty(),
        "variable format string must not produce false positives"
    );
}

// --- IndirectCall (printf FILEHANDLE FORMAT, LIST) ---

#[test]
fn printf_with_filehandle_mismatch_fires_pl405() {
    // printf STDERR FORMAT, LIST — parser produces IndirectCall form
    let diags = printf_diags("printf STDERR \"%s %d\", $a;\n");
    assert_eq!(
        diags.len(),
        1,
        "printf FILEHANDLE with 2 specifiers and 1 arg should fire PL405"
    );
}

#[test]
fn printf_with_filehandle_exact_match_no_pl405() {
    let diags = printf_diags("printf STDERR \"%s\", $a;\n");
    assert!(
        diags.is_empty(),
        "printf FILEHANDLE with matching specifier count should not fire PL405"
    );
}

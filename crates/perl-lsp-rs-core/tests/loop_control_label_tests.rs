//! Integration tests for PL410 — `next`/`last`/`redo LABEL` validation.
//!
//! These tests drive `DiagnosticsProvider::get_diagnostics` end-to-end to
//! ensure the lint is registered and produces diagnostics with stable
//! wording and a filterable code.

use std::sync::Arc;

use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl410(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter().filter(|d| d.code.as_deref() == Some("PL410")).collect()
}

fn pl410_messages(source: &str) -> Vec<String> {
    pl410(source).into_iter().map(|d| d.message).collect()
}

#[test]
fn next_to_missing_label_warns() {
    let source = r#"use v5.40;
while (1) {
    next MISSING;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`next MISSING`") && m.contains("not defined in this file")),
        "expected PL410 for missing `next LABEL`, got: {messages:?}"
    );
}

#[test]
fn last_to_missing_label_warns() {
    let source = r#"use v5.40;
for my $x (1..10) {
    last NOPE;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("`last NOPE`") && m.contains("not defined in this file")),
        "expected PL410 for missing `last LABEL`, got: {messages:?}"
    );
}

#[test]
fn redo_to_missing_label_warns() {
    let source = r#"use v5.40;
foreach my $item (@items) {
    redo TOP;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages.iter().any(|m| m.contains("`redo TOP`") && m.contains("not defined in this file")),
        "expected PL410 for missing `redo LABEL`, got: {messages:?}"
    );
}

#[test]
fn next_to_existing_label_is_allowed() {
    let source = r#"use v5.40;
OUTER: for my $i (1..10) {
    INNER: for my $j (1..10) {
        next OUTER if $j > 5;
    }
}
"#;
    let messages = pl410_messages(source);
    assert!(messages.is_empty(), "`next` to a defined label should not warn, got: {messages:?}");
}

#[test]
fn last_to_existing_label_is_allowed() {
    let source = r#"use v5.40;
LOOP: while (1) {
    last LOOP;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "`last LOOP` with defined label should not warn, got: {messages:?}"
    );
}

#[test]
fn redo_to_existing_label_is_allowed() {
    let source = r#"use v5.40;
RETRY: for my $x (@attempts) {
    redo RETRY if $x->failed;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "`redo RETRY` with defined label should not warn, got: {messages:?}"
    );
}

#[test]
fn bare_loop_control_is_always_allowed() {
    let source = r#"use v5.40;
while (1) {
    next;
    last;
    redo;
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "bare next/last/redo should never trigger PL410, got: {messages:?}"
    );
}

#[test]
fn only_one_diagnostic_per_bad_loop_control() {
    let source = r#"use v5.40;
while (1) {
    next MISSING;
}
"#;
    let diags = pl410(source);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly one PL410 diagnostic per offending statement, got {}: {diags:?}",
        diags.len()
    );
}

#[test]
fn pl410_carries_actionable_suggestion() {
    let source = r#"use v5.40;
while (1) {
    next MISSING;
}
"#;
    let diags = pl410(source);
    let suggestion = diags.first().and_then(|d| d.suggestion.as_deref()).unwrap_or_default();
    assert!(
        suggestion.contains("MISSING:") && suggestion.contains("next"),
        "expected suggestion mentioning the missing label and a bare form, got: {suggestion:?}"
    );
}

#[test]
fn multiple_bad_loop_controls_each_reported() {
    let source = r#"use v5.40;
while (1) {
    next ALPHA;
    last BETA;
}
"#;
    let messages = pl410_messages(source);
    assert_eq!(messages.len(), 2, "expected one PL410 per offending statement, got: {messages:?}");
    assert!(messages.iter().any(|m| m.contains("ALPHA")));
    assert!(messages.iter().any(|m| m.contains("BETA")));
}

#[test]
fn label_elsewhere_in_file_suppresses_warning() {
    // The conservative rule (matching PL409) is "label exists anywhere in
    // the file" — reachability is not analyzed, to avoid false positives
    // from source filters and conditional code paths.
    let source = r#"use v5.40;
sub outer {
    LATER: for my $x (1..3) { print $x; }
}
sub inner {
    next LATER;  # referenced before the containing loop lexically
}
"#;
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "label defined anywhere in the file should suppress PL410, got: {messages:?}"
    );
}

#[test]
fn pl410_does_not_conflict_with_pl409() {
    // Make sure adding PL410 doesn't break the existing goto-label lint.
    let source = r#"use v5.40;
goto MISSING_GOTO;
while (1) {
    next MISSING_LOOP;
}
"#;
    let all = diagnostics_for(source);
    let pl409s: Vec<_> = all.iter().filter(|d| d.code.as_deref() == Some("PL409")).collect();
    let pl410s: Vec<_> = all.iter().filter(|d| d.code.as_deref() == Some("PL410")).collect();
    assert_eq!(pl409s.len(), 1, "expected 1 PL409, got {pl409s:?}");
    assert_eq!(pl410s.len(), 1, "expected 1 PL410, got {pl410s:?}");
}

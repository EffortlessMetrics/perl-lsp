//! Integration tests for PL403 — assignment inside conditional expressions.
//!
//! These tests ensure our built-in Rust diagnostics catch patterns that users
//! often relied on external perlcritic policies for.

use std::sync::Arc;

use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl403(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter().filter(|d| d.code.as_deref() == Some("PL403")).collect()
}

#[test]
fn detects_assignment_in_elsif_condition() {
    let source = r#"use v5.40;
if ($x == 1) {
    print "one";
} elsif ($x = 2) {
    print "two";
}
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 in elsif condition, got: {diags:?}");
}

#[test]
fn detects_assignment_in_for_condition() {
    let source = r#"use v5.40;
for (my $i = 0; $i = 10; $i++) {
    print $i;
}
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 in for condition, got: {diags:?}");
}

#[test]
fn detects_assignment_in_statement_modifier_condition() {
    let source = r#"use v5.40;
print "ok" if $ready = 1;
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 for statement modifier, got: {diags:?}");
}

#[test]
fn ignores_non_conditional_statement_modifiers() {
    let source = r#"use v5.40;
my $x = 0;
$x += 1 for @items;
"#;

    let diags = pl403(source);
    assert!(diags.is_empty(), "non-conditional modifiers should not trigger PL403, got: {diags:?}");
}

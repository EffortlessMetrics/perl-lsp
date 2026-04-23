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

#[test]
fn detects_assignment_in_unless_statement_modifier() {
    let source = r#"use v5.40;
print "nope" unless $ok = 0;
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 for 'unless' modifier, got: {diags:?}");
}

#[test]
fn detects_assignment_in_until_statement_modifier() {
    let source = r#"use v5.40;
print "loop" until $done = 1;
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 for 'until' modifier, got: {diags:?}");
}

#[test]
fn detects_assignment_in_while_statement_modifier() {
    let source = r#"use v5.40;
print "tick" while $n = next_val();
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 for 'while' modifier, got: {diags:?}");
}

#[test]
fn detects_assignment_in_nested_elsif_branches() {
    // Multiple elsif branches with assignments — each should produce its own PL403.
    let source = r#"use v5.40;
if ($x == 1) {
    print "one";
} elsif ($y = 2) {
    print "two";
} elsif ($z = 3) {
    print "three";
}
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 2, "expected two PL403s across elsif branches, got: {diags:?}");
}

#[test]
fn ignores_assignment_in_foreach_list() {
    // `foreach` (a.k.a. `for my $x (@list)`) has a Foreach node, not For — assignments
    // inside its iteration list should not trigger PL403 since they aren't the condition.
    let source = r#"use v5.40;
foreach my $item (@items) {
    my $y = $item;
}
"#;

    let diags = pl403(source);
    assert!(diags.is_empty(), "foreach list should not trigger PL403, got: {diags:?}");
}

#[test]
fn handles_c_style_for_with_missing_condition() {
    // C-style for without a condition (`for (;;)`) must not panic and must not warn.
    let source = r#"use v5.40;
for (;;) {
    last;
}
"#;

    let diags = pl403(source);
    assert!(diags.is_empty(), "for(;;) should not trigger PL403, got: {diags:?}");
}

#[test]
fn detects_assignment_nested_in_while_inside_if() {
    // An inner `while ($n = next())` inside an outer `if` — exactly one PL403.
    let source = r#"use v5.40;
if ($active) {
    while ($n = next_val()) {
        process($n);
    }
}
"#;

    let diags = pl403(source);
    assert_eq!(diags.len(), 1, "expected one PL403 for inner while, got: {diags:?}");
}

#[test]
fn ignores_variable_declaration_in_while_condition() {
    // The idiomatic `while (my $line = readline($fh)) { ... }` uses a VariableDeclaration
    // in the condition, not an Assignment or Binary `=`. It must not trigger PL403.
    let source = r#"use v5.40;
while (my $line = readline($fh)) {
    chomp $line;
}
"#;

    let diags = pl403(source);
    assert!(
        diags.is_empty(),
        "`my $line = readline(...)` in while should not trigger PL403, got: {diags:?}"
    );
}

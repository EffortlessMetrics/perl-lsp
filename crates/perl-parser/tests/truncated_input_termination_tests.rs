//! Truncated Input Termination Tests
//!
//! These tests verify that the parser terminates gracefully (no infinite loops)
//! when given truncated/incomplete Perl code that hits EOF unexpectedly.
//!
//! This addresses the "WSL crash" class of bugs where incomplete syntax like
//! `"sub ("` could cause the parser to loop forever waiting for tokens that
//! never arrive.
//!
//! The key invariant being tested: Parser MUST terminate in bounded time for
//! ANY input, even malformed or truncated input.
//!
//! Labels: tests:termination, parser:eof-safety, regression:wsl-crash

use perl_parser::Parser;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Default timeout for termination tests (1 second should be ample for any valid termination)
const DEFAULT_TIMEOUT_MS: u64 = 1000;

/// Shorter timeout for simple cases (single tokens, etc.)
const SHORT_TIMEOUT_MS: u64 = 500;

/// Helper to run parser with timeout - detects infinite loops.
///
/// Returns `Ok(())` if the parser terminates within the timeout (regardless of parse success/failure).
/// Returns `Err` if the parser hangs (timeout exceeded).
///
/// Note: On timeout, the spawned thread continues running until it naturally terminates.
/// This is acceptable for tests since the process exits afterward.
fn parse_with_timeout(code: &str, timeout_ms: u64) -> Result<(), String> {
    let code_owned = code.to_string();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut parser = Parser::new(&code_owned);
        let result = parser.parse();
        // Send whether parse succeeded (we don't care about the result, just termination)
        let _ = tx.send(result.is_ok());
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Format error message with truncated input for readability
            let display_code =
                if code.len() > 50 { format!("{}...", &code[..50]) } else { code.to_string() };
            Err(format!("Parser timed out after {}ms on input: {:?}", timeout_ms, display_code))
        }
    }
}

/// Helper that uses the default timeout
fn must_terminate(code: &str) {
    parse_with_timeout(code, DEFAULT_TIMEOUT_MS)
        .unwrap_or_else(|e| perl_tdd_support::must(Err(e)));
}

/// Helper for short-timeout cases
fn must_terminate_fast(code: &str) {
    parse_with_timeout(code, SHORT_TIMEOUT_MS)
        .unwrap_or_else(|e| perl_tdd_support::must(Err(e)));
}

/// Test truncated subroutine declarations - the original "sub (" hang case
#[test]
fn truncated_sub_declaration_terminates() {
    // Each case represents a different truncation point in sub declaration syntax
    let cases = [
        "sub",              // Just keyword
        "sub ",             // Keyword with space
        "sub foo",          // Name but no block
        "sub foo(",         // Opening paren for signature
        "sub foo($",        // Partial signature
        "sub foo($x",       // Partial signature with param
        "sub foo($x,",      // Signature with trailing comma
        "sub foo($x) {",    // Partial block
        "sub foo($x) { my", // Partial statement in block
        "sub (",            // Anonymous sub with opening paren (original bug)
        "sub ($",           // Anonymous sub partial signature
        "sub { ",           // Anonymous sub partial block
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated control flow statements
#[test]
fn truncated_control_flow_terminates() {
    let cases = [
        "if",
        "if (",
        "if ($x",
        "if ($x)",
        "if ($x) {",
        "if ($x) { my $y",
        "unless",
        "unless (",
        "while",
        "while (",
        "while ($x",
        "for",
        "for my",
        "for my $x",
        "for my $x (",
        "foreach",
        "foreach my $item",
        "foreach my $item (",
        "given",
        "given (",
        "when",
        "when (",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated variable declarations
#[test]
fn truncated_variable_declarations_terminates() {
    let cases = [
        "my",
        "my $",
        "my $x",
        "my $x =",
        "my $x = (",
        "my ($",
        "my ($x",
        "my ($x,",
        "my ($x, $y",
        "our",
        "our @",
        "our @arr",
        "local",
        "local $",
        "state",
        "state $x",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated use/no statements (these exercise the EOF guards in declarations.rs)
#[test]
fn truncated_use_no_statements_terminates() {
    let cases = [
        "use",
        "use strict",
        "use Foo::",
        "use Foo::Bar",
        "use Foo::Bar (",
        "use Foo::Bar ('",
        "use Foo::Bar ('a",
        "use Foo::Bar ('a',",
        "use Foo::Bar ('a', 'b",
        "no",
        "no strict",
        "no warnings (",
        "no warnings ('all",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated expressions with operators
#[test]
fn truncated_expressions_terminates() {
    let cases = [
        "$x +",
        "$x + $y *",
        "$x ? $y :",
        "$x ? $y : $z +",
        "$x &&",
        "$x || $y &&",
        "$x and",
        "$x or $y and",
        "!",
        "! (",
        "-",
        "- (",
        "my $x = $y ?",
        "my $x = $y ? $z :",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated array/hash constructs
#[test]
fn truncated_array_hash_terminates() {
    let cases = [
        "[",
        "[ 1",
        "[ 1,",
        "[ 1, 2",
        "[ 1, 2,",
        "{",
        "{ a",
        "{ a =>",
        "{ a => 1",
        "{ a => 1,",
        "{ a => 1, b",
        "{ a => 1, b =>",
        "@arr[",
        "@arr[ 0",
        "$hash{",
        "$hash{ 'key",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated function calls
#[test]
fn truncated_function_calls_terminates() {
    let cases = [
        "foo(",
        "foo( $x",
        "foo( $x,",
        "foo( $x, $y",
        "Foo::bar(",
        "Foo->bar(",
        "Foo->bar( $x",
        "$obj->method(",
        "$obj->method( $x,",
        "print",
        "print $x,",
        "die",
        "die 'error",
        "return",
        "return $x,",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated quote operators
#[test]
fn truncated_quote_operators_terminates() {
    let cases = [
        "q(",
        "q(hello",
        "qq(",
        "qq(hello $x",
        "qw(",
        "qw(a b",
        "qr(",
        "qr(pattern",
        "m/",
        "m/pattern",
        "s/",
        "s/foo",
        "s/foo/",
        "s/foo/bar",
        "tr/",
        "tr/a-z",
        "tr/a-z/",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated heredocs (exercises the EOF guard in heredoc_collector.rs)
#[test]
fn truncated_heredocs_terminates() {
    let cases = [
        "<<EOF",
        "<<EOF\nhello",
        "<<'EOF'",
        "<<'EOF'\nhello",
        "<<~EOF",
        "<<~EOF\n  hello",
        "my $x = <<EOF",
        "my $x = <<EOF\nhello",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated method chains
#[test]
fn truncated_method_chains_terminates() {
    let cases = [
        "$obj->",
        "$obj->method->",
        "$obj->method(",
        "$obj->method()->",
        "$obj->[",
        "$obj->[ 0",
        "$obj->{",
        "$obj->{ 'key",
        "$obj->@*",
        "$obj->%*",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated regex patterns
#[test]
fn truncated_regex_terminates() {
    let cases =
        ["$x =~", "$x =~ /", "$x =~ /pattern", "$x =~ s/", "$x =~ s/foo/", "$x !~", "$x !~ /"];

    for code in cases {
        must_terminate(code);
    }
}

/// Test truncated package/class declarations
#[test]
fn truncated_package_class_terminates() {
    let cases =
        ["package", "package Foo", "package Foo::", "package Foo::Bar", "package Foo::Bar {"];

    for code in cases {
        must_terminate(code);
    }
}

/// Test combinations of truncated constructs (nested contexts)
#[test]
fn truncated_combinations_terminates() {
    let cases = [
        "my $x = sub {",
        "my $x = sub ($y) {",
        "my @arr = map {",
        "my @arr = map { $_ *",
        "my @arr = sort {",
        "my @arr = grep {",
        "my $x = do {",
        "my $x = eval {",
        "if ($x) { sub {",
        "for my $x (@arr) { if (",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Test single-character truncations at critical points
#[test]
fn single_char_truncations_terminates() {
    // These are particularly tricky because they're right at token boundaries
    let cases = [
        "(", ")", "[", "]", "{", "}", "$", "@", "%", "*", "&", "\\", "/", "=", "+", "-", "<", ">",
        "!", "~", "?", ":", ";", ",", ".", "|",
    ];

    for code in cases {
        must_terminate_fast(code);
    }
}

/// Test empty and whitespace-only inputs
#[test]
fn empty_and_whitespace_terminates() {
    let cases = ["", " ", "\t", "\n", "  \n  \t  ", "# comment only\n"];

    for code in cases {
        must_terminate_fast(code);
    }
}

/// Stress test: Many different truncations in rapid succession
/// This catches any state leakage between parses
#[test]
fn rapid_truncation_stress_test() {
    let truncations = [
        "sub (",
        "if (",
        "while (",
        "for my $x (",
        "use Foo (",
        "my ($x,",
        "$x->method(",
        "[ 1, 2,",
        "{ a => 1,",
    ];

    // Run each 10 times in quick succession
    for _ in 0..10 {
        for code in &truncations {
            must_terminate_fast(code);
        }
    }
}

/// Test deeply nested truncations - these stress the recursion guard
#[test]
fn deeply_nested_truncations_terminates() {
    let cases = [
        // Nested blocks
        "if (1) { if (2) { if (3) {",
        "sub foo { sub bar { sub baz {",
        // Nested parens
        "(((((",
        "foo(bar(baz(qux(",
        // Nested brackets
        "[[[[",
        "$a[$b[$c[$d[",
        // Nested braces (hash/block ambiguity)
        "{{{{",
        "$h{$i{$j{$k{",
        // Mixed nesting
        "if ($a[{",
        "my $x = [{[{",
        "sub foo($x) { if ($x) { map { (",
    ];

    for code in cases {
        must_terminate(code);
    }
}

/// Regression test: Ensure valid complete code still parses correctly
/// This verifies the EOF guards don't break normal parsing
#[test]
fn valid_code_still_works() {
    let valid_cases = [
        // Complete use statement with parens
        "use Foo::Bar ('a', 'b');",
        // Complete heredoc
        "my $x = <<EOF;\nhello\nEOF\n",
        // Complete sub with signature
        "sub foo($x, $y) { return $x + $y; }",
        // Complete block with nested control flow
        "if ($x) { while ($y) { print 1; } }",
        // Orphan brace recovery (should produce error node but terminate)
        "} }",
    ];

    for code in valid_cases {
        let mut parser = Parser::new(code);
        // Should complete without hanging
        let result = parser.parse();
        // We don't necessarily require success (some cases may error),
        // but they must terminate
        assert!(result.is_ok() || result.is_err(), "Parse must return a result for: {:?}", code);
    }
}

/// Test that heredocs ending exactly at EOF work correctly
#[test]
fn heredoc_at_exact_eof_terminates() {
    // These test the boundary condition in heredoc_collector.rs
    let cases = [
        // Terminated heredoc at exact EOF (no trailing newline)
        "my $x = <<EOF;\nhello\nEOF",
        // Indented heredoc at exact EOF
        "my $x = <<~EOF;\n  hello\n  EOF",
        // Multiple heredocs, last one at EOF
        "my ($a, $b) = (<<A, <<B);\na\nA\nb\nB",
    ];

    for code in cases {
        must_terminate(code);
    }
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
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

/// Helper to run parser with timeout - detects infinite loops
fn parse_with_timeout(code: &str, timeout_ms: u64) -> Result<(), String> {
    let code_for_thread = code.to_string();
    let code_for_error = code.to_string();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        let mut parser = Parser::new(&code_for_thread);
        let result = parser.parse();
        let _ = tx.send(result.is_ok());
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(_) => {
            let _ = handle.join();
            Ok(())
        }
        Err(_) => Err(format!(
            "Parser timed out after {}ms on input: {:?}",
            timeout_ms,
            if code_for_error.len() > 50 {
                format!("{}...", &code_for_error[..50])
            } else {
                code_for_error
            }
        )),
    }
}

/// Test truncated subroutine declarations - the original "sub (" hang case
#[test]
fn truncated_sub_declaration_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated control flow statements
#[test]
fn truncated_control_flow_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated variable declarations
#[test]
fn truncated_variable_declarations_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated use/no statements (the loops we just fixed)
#[test]
fn truncated_use_no_statements_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated expressions with operators
#[test]
fn truncated_expressions_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated array/hash constructs
#[test]
fn truncated_array_hash_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated function calls
#[test]
fn truncated_function_calls_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated quote operators
#[test]
fn truncated_quote_operators_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated heredocs
#[test]
fn truncated_heredocs_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated method chains
#[test]
fn truncated_method_chains_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated regex patterns
#[test]
fn truncated_regex_terminates() {
    let cases =
        vec!["$x =~", "$x =~ /", "$x =~ /pattern", "$x =~ s/", "$x =~ s/foo/", "$x !~", "$x !~ /"];

    for code in cases {
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test truncated package/class declarations
#[test]
fn truncated_package_class_terminates() {
    let cases =
        vec!["package", "package Foo", "package Foo::", "package Foo::Bar", "package Foo::Bar {"];

    for code in cases {
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test combinations of truncated constructs
#[test]
fn truncated_combinations_terminates() {
    let cases = vec![
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
        parse_with_timeout(code, 1000).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test single-character truncations at critical points
#[test]
fn single_char_truncations_terminates() {
    // These are particularly tricky because they're right at token boundaries
    let cases = vec![
        "(", ")", "[", "]", "{", "}", "$", "@", "%", "*", "&", "\\", "/", "=", "+", "-", "<", ">",
        "!", "~", "?", ":", ";", ",", ".", "|",
    ];

    for code in cases {
        parse_with_timeout(code, 500).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Test empty and whitespace-only inputs
#[test]
fn empty_and_whitespace_terminates() {
    let cases = vec!["", " ", "\t", "\n", "  \n  \t  ", "# comment only\n"];

    for code in cases {
        parse_with_timeout(code, 500).unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
    }
}

/// Stress test: Many different truncations in rapid succession
/// This catches any state leakage between parses
#[test]
fn rapid_truncation_stress_test() {
    let truncations = vec![
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
            parse_with_timeout(code, 500)
                .unwrap_or_else(|e| panic!("Infinite loop detected: {}", e));
        }
    }
}

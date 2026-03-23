//! Snapshot tests for AST structure and error messages.
//!
//! These tests use `insta` to capture baseline snapshots of:
//! - Parser AST structure (s-expression format) for well-formed Perl
//! - Error recovery AST for malformed Perl input
//! - Error message formatting for each ParseError variant
//! - Semantic token legend (token types and modifiers)
//!
//! Run with `cargo test -p perl-parser ast_snapshot` to execute.
//! Update snapshots with `cargo insta review` after intentional changes.

use insta::assert_snapshot;
use perl_parser::{Parser, semantic_tokens};

// ---------------------------------------------------------------------------
// Helper: parse and return sexp from recovery output
// ---------------------------------------------------------------------------

fn parse_sexp(source: &str) -> String {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    output.ast.to_sexp()
}

fn parse_errors(source: &str) -> String {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    // Format errors as a sorted, newline-separated list for stable snapshots
    let mut lines: Vec<String> = output.diagnostics.iter().map(|e| format!("{}", e)).collect();
    lines.sort();
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// 1. Clean Perl AST snapshots (CPAN-style edge cases)
// ---------------------------------------------------------------------------

#[test]
fn snapshot_ast_variable_declaration() {
    assert_snapshot!(parse_sexp("my $x = 42;"));
}

#[test]
fn snapshot_ast_sub_definition() {
    assert_snapshot!(parse_sexp("sub greet { return \"Hello\"; }"));
}

#[test]
fn snapshot_ast_package_declaration() {
    assert_snapshot!(parse_sexp("package My::Module;"));
}

#[test]
fn snapshot_ast_if_elsif_else() {
    assert_snapshot!(parse_sexp(
        "if ($x > 0) { print \"pos\"; } elsif ($x < 0) { print \"neg\"; } else { print \"zero\"; }"
    ));
}

#[test]
fn snapshot_ast_array_operations() {
    assert_snapshot!(parse_sexp("my @arr = (1, 2, 3); push @arr, 4;"));
}

#[test]
fn snapshot_ast_hash_operations() {
    assert_snapshot!(parse_sexp("my %h = (a => 1, b => 2); my $v = $h{a};"));
}

#[test]
fn snapshot_ast_method_call() {
    assert_snapshot!(parse_sexp("$obj->method($arg1, $arg2);"));
}

#[test]
fn snapshot_ast_regex_match() {
    assert_snapshot!(parse_sexp("if ($str =~ /^hello/i) { print \"matched\"; }"));
}

#[test]
fn snapshot_ast_use_strict_warnings() {
    assert_snapshot!(parse_sexp("use strict;\nuse warnings;"));
}

#[test]
fn snapshot_ast_while_loop() {
    assert_snapshot!(parse_sexp("while (my $line = <STDIN>) { chomp $line; print $line; }"));
}

#[test]
fn snapshot_ast_for_loop() {
    assert_snapshot!(parse_sexp("for my $i (1..10) { print \"$i\\n\"; }"));
}

#[test]
fn snapshot_ast_anonymous_sub() {
    assert_snapshot!(parse_sexp("my $code = sub { my ($x) = @_; return $x * 2; };"));
}

#[test]
fn snapshot_ast_string_interpolation() {
    assert_snapshot!(parse_sexp("my $name = \"world\"; my $msg = \"Hello, $name!\";"));
}

#[test]
fn snapshot_ast_chained_method_calls() {
    assert_snapshot!(parse_sexp("$obj->foo->bar->baz;"));
}

#[test]
fn snapshot_ast_ternary_operator() {
    assert_snapshot!(parse_sexp("my $x = $cond ? \"yes\" : \"no\";"));
}

// ---------------------------------------------------------------------------
// 2. Error recovery AST snapshots (malformed input)
// ---------------------------------------------------------------------------

#[test]
fn snapshot_recovery_missing_semicolon() {
    assert_snapshot!(parse_sexp("my $x = 42"));
}

#[test]
fn snapshot_recovery_unclosed_block() {
    assert_snapshot!(parse_sexp("sub foo {"));
}

#[test]
fn snapshot_recovery_missing_rhs() {
    assert_snapshot!(parse_sexp("my $x = ;"));
}

#[test]
fn snapshot_recovery_unclosed_paren() {
    assert_snapshot!(parse_sexp("print(\"hello\";"));
}

#[test]
fn snapshot_recovery_multiple_errors() {
    assert_snapshot!(parse_sexp("my $x = ;\nmy $y = ;"));
}

#[test]
fn snapshot_recovery_truncated_hash() {
    assert_snapshot!(parse_sexp("my %h = (a =>"));
}

#[test]
fn snapshot_recovery_truncated_array() {
    assert_snapshot!(parse_sexp("my @arr = (1, 2,"));
}

#[test]
fn snapshot_recovery_partial_if() {
    assert_snapshot!(parse_sexp("if ($x > 0) {"));
}

#[test]
fn snapshot_recovery_empty_sub_body() {
    assert_snapshot!(parse_sexp("sub foo"));
}

#[test]
fn snapshot_recovery_statement_after_error() {
    // Parser should recover and parse the second statement correctly
    assert_snapshot!(parse_sexp("my $x = ;\nmy $y = 10;"));
}

// ---------------------------------------------------------------------------
// 3. Error message format snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_errors_missing_rhs() {
    assert_snapshot!(parse_errors("my $x = ;"));
}

#[test]
fn snapshot_errors_unclosed_block() {
    assert_snapshot!(parse_errors("sub foo {"));
}

#[test]
fn snapshot_errors_multiple_statements_errors() {
    assert_snapshot!(parse_errors("my $x = ;\nmy $y = ;"));
}

#[test]
fn snapshot_errors_truncated_hash() {
    assert_snapshot!(parse_errors("my %h = (a =>"));
}

// ---------------------------------------------------------------------------
// 4. Semantic token legend snapshot
//    The ordering of token types and modifiers is part of the LSP protocol
//    contract — clients decode by index, so any reordering is a breaking change.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_semantic_token_legend_types() {
    let leg = semantic_tokens::legend();
    let types_str = leg.token_types.join("\n");
    assert_snapshot!(types_str);
}

#[test]
fn snapshot_semantic_token_legend_modifiers() {
    let leg = semantic_tokens::legend();
    let mods_str = leg.modifiers.join("\n");
    assert_snapshot!(mods_str);
}

#[test]
fn snapshot_semantic_token_legend_index_mapping() {
    // Snapshot the full ordered legend as "index: name" pairs
    let leg = semantic_tokens::legend();
    let mut lines = Vec::new();
    lines.push("token_types:".to_string());
    for (i, t) in leg.token_types.iter().enumerate() {
        lines.push(format!("  {}: {}", i, t));
    }
    lines.push("modifiers:".to_string());
    for (i, m) in leg.modifiers.iter().enumerate() {
        lines.push(format!("  {}: {}", i, m));
    }
    assert_snapshot!(lines.join("\n"));
}

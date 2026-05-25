mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

fn assert_missing_rhs_recovery_preserves_statement(code: &str, label: &str) {
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should recover from missing RHS before {label}");
    let ast = must(result);
    let sexp = ast.to_sexp();

    assert!(
        matches!(&ast.kind, NodeKind::Program { .. }),
        "Expected program root while checking {label}; sexp: {sexp}"
    );
    if let NodeKind::Program { statements } = &ast.kind {
        assert!(
            statements.len() >= 2,
            "Should recover into at least 2 statements for {label}; got {}; sexp: {sexp}",
            statements.len()
        );
    }

    assert!(
        !parser.errors().is_empty(),
        "Recovery before {label} should record a missing-operand diagnostic"
    );
}

fn assert_single_statement_without_recovery(code: &str, label: &str) {
    let mut parser = Parser::new(code);
    let result = parser.parse();

    assert!(result.is_ok(), "Parser should accept {label}");
    let ast = must(result);
    let sexp = ast.to_sexp();

    assert!(
        matches!(&ast.kind, NodeKind::Program { .. }),
        "Expected program root while checking {label}; sexp: {sexp}"
    );
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1, "{label} must stay in a single statement; sexp: {sexp}");
    }

    assert!(
        parser.errors().is_empty(),
        "No recovery errors expected for {label}: {:?}",
        parser.errors()
    );
}

// Phaser block recovery.
//
// BEGIN/END/CHECK/INIT/UNITCHECK blocks are compile-time declarations that can
// never be expression operands. When one follows an infix `=`, the parser must
// recover the missing RHS and preserve the phaser block as a separate top-level
// statement.

#[test]
fn test_recovery_missing_rhs_before_begin_block() {
    assert_missing_rhs_recovery_preserves_statement("my $x = BEGIN { print 'hi' }", "BEGIN block");
}

#[test]
fn test_recovery_missing_rhs_before_end_block() {
    assert_missing_rhs_recovery_preserves_statement("my $x = END { cleanup() }", "END block");
}

#[test]
fn test_recovery_missing_rhs_before_check_block() {
    assert_missing_rhs_recovery_preserves_statement("my $x = CHECK { verify() }", "CHECK block");
}

#[test]
fn test_recovery_missing_rhs_before_init_block() {
    assert_missing_rhs_recovery_preserves_statement("my $x = INIT { setup() }", "INIT block");
}

#[test]
fn test_recovery_missing_rhs_before_unitcheck_block() {
    assert_missing_rhs_recovery_preserves_statement(
        "my $x = UNITCHECK { verify() }",
        "UNITCHECK block",
    );
}

// Phaser disambiguation: standalone phasers must still parse cleanly.

#[test]
fn test_standalone_begin_block_parses_cleanly() {
    assert_clean_parse("BEGIN { require 'config.pl' }");
}

#[test]
fn test_standalone_end_block_parses_cleanly() {
    assert_clean_parse("END { close $fh }");
}

#[test]
fn test_standalone_check_block_parses_cleanly() {
    assert_clean_parse("CHECK { validate_config() }");
}

#[test]
fn test_phaser_as_statement_label_parses_cleanly() {
    assert_clean_parse("CHECK: for my $i (1..10) { print $i }");
}

// `given ($x) { ... }` is an experimental compound statement, never an
// expression. The parser must recover when it appears as an infix RHS.

#[test]
fn test_recovery_missing_rhs_before_given_statement() {
    assert_missing_rhs_recovery_preserves_statement(
        "my $x = given ($y) { when (1) { 'one' } }",
        "given statement",
    );
}

// `defer { ... }` (Perl 5.36+) is a block statement, not an expression. The
// parser must recover when it appears as an infix RHS.

#[test]
fn test_recovery_missing_rhs_before_defer_block() {
    assert_missing_rhs_recovery_preserves_statement("my $x = defer { cleanup() }", "defer block");
}

#[test]
fn test_standalone_defer_block_parses_cleanly() {
    assert_clean_parse("defer { cleanup() }");
}

// Guard tests: bareword/hash-key forms must not trigger recovery.

#[test]
fn test_defer_as_hash_key_no_recovery() {
    assert_single_statement_without_recovery("my %h = (defer => 1);", "`defer =>` hash key");
}

#[test]
fn test_begin_as_hash_key_no_recovery() {
    assert_single_statement_without_recovery("my %h = (BEGIN => 'init');", "`BEGIN =>` hash key");
}

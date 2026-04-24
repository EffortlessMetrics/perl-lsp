/// Phase 2 expression continuity recovery tests.
///
/// Tests for two recovery patterns:
/// 1. Missing RHS after infix operator: `$x +` followed by `;` or `}` emits
///    `ParseError::Recovered { site: InfixRhs, kind: MissingOperand }`.
/// 2. Postfix chain truncation: `$obj->` followed by `;` or `}` emits
///    `ParseError::Recovered { site: PostfixChain, kind: TruncatedChain }`.
///
/// All tests verify that:
/// - The parser does not return Err (no catastrophic failure)
/// - The appropriate `ParseError::Recovered` variant is present in `parser.errors()`
/// - Clean Perl does not trigger any Recovered errors
mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::error::{ParseError, RecoveryKind, RecoverySite};
use perl_parser_core::{ParseCloseoutKind, Parser};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

/// Parse source, expect no catastrophic error, return the collected errors.
fn parse_errors(source: &str) -> Vec<ParseError> {
    let mut parser = Parser::new(source);
    let _ = parser.parse(); // ignore Ok/Err — we want errors()
    parser.errors().to_vec()
}

/// Assert at least one `ParseError::Recovered` with the given site+kind.
fn assert_recovered(errors: &[ParseError], site: RecoverySite, kind: RecoveryKind) {
    let found = errors.iter().any(
        |e| matches!(e, ParseError::Recovered { site: s, kind: k, .. } if s == &site && k == &kind),
    );
    assert!(
        found,
        "Expected Recovered {{ site: {:?}, kind: {:?} }} in errors, got: {:?}",
        site, kind, errors,
    );
}

/// Assert that NO `ParseError::Recovered` with the given site+kind exists.
fn assert_not_recovered(errors: &[ParseError], site: RecoverySite, kind: RecoveryKind) {
    let found = errors.iter().any(
        |e| matches!(e, ParseError::Recovered { site: s, kind: k, .. } if s == &site && k == &kind),
    );
    assert!(
        !found,
        "Unexpected Recovered {{ site: {:?}, kind: {:?} }} in errors for clean Perl: {:?}",
        site, kind, errors,
    );
}

// ──────────────────────────────────────────────────────────────
// Pattern 1: Missing RHS after infix operator — before `;`
// ──────────────────────────────────────────────────────────────

#[test]
fn missing_rhs_plus_before_semicolon_emits_recovered() {
    // `my $x = $a +;` — `+` consumes, next is `;`
    let errors = parse_errors("my $x = $a +;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_minus_before_semicolon_emits_recovered() {
    let errors = parse_errors("my $x = $a -;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_dot_concat_before_semicolon_emits_recovered() {
    let errors = parse_errors(r#"my $s = $a .;"#);
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_logical_and_before_semicolon_emits_recovered() {
    let errors = parse_errors("my $x = $a &&;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_logical_or_before_semicolon_emits_recovered() {
    let errors = parse_errors("my $x = $a ||;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_defined_or_before_semicolon_emits_recovered() {
    let errors = parse_errors("my $x = $a //;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

// ──────────────────────────────────────────────────────────────
// Pattern 1: Missing RHS after infix operator — before `}`
// ──────────────────────────────────────────────────────────────

#[test]
fn missing_rhs_plus_before_rbrace_emits_recovered() {
    // Inside a block: `{ $a + }` — `+` then `}`
    let errors = parse_errors("sub foo { return $a + }");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_star_before_rbrace_emits_recovered() {
    let errors = parse_errors("sub foo { my $x = $a * }");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_eq_before_rbrace_emits_recovered() {
    let errors = parse_errors("sub foo { my $x = }");
    // Assignment operator with missing RHS before `}`
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

// ──────────────────────────────────────────────────────────────
// Pattern 1: Parser does not crash — produces usable AST
// ──────────────────────────────────────────────────────────────

#[test]
fn missing_rhs_plus_does_not_catastrophically_fail() {
    // Parser must return Ok (or at least not crash)
    let mut parser = Parser::new("my $x = $a +;");
    let result = parser.parse();
    assert!(result.is_ok(), "Parser must not return catastrophic Err for recoverable input");
}

#[test]
fn missing_rhs_in_if_condition_does_not_crash() {
    let mut parser = Parser::new("if ($x +) { print 1; }");
    let result = parser.parse();
    assert!(result.is_ok(), "Parser must not crash on missing RHS in if condition");
}

// ──────────────────────────────────────────────────────────────
// Pattern 2: Postfix chain truncation — before `;`
// ──────────────────────────────────────────────────────────────

#[test]
fn truncated_arrow_before_semicolon_emits_recovered() {
    // `$obj->;` — `->` consumed, next is `;`
    let errors = parse_errors("my $x = $obj->;");
    assert_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn truncated_arrow_at_eof_emits_recovered() {
    // `$obj->` with no following token
    let errors = parse_errors("$obj->");
    assert_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn truncated_arrow_before_rbrace_emits_recovered() {
    // Inside a block: `sub foo { $obj-> }`
    let errors = parse_errors("sub foo { $obj-> }");
    assert_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

// ──────────────────────────────────────────────────────────────
// Pattern 2: Parser does not crash — produces usable AST
// ──────────────────────────────────────────────────────────────

#[test]
fn truncated_arrow_does_not_catastrophically_fail() {
    let mut parser = Parser::new("my $x = $obj->;");
    let result = parser.parse();
    assert!(result.is_ok(), "Parser must not return catastrophic Err for truncated arrow");
}

#[test]
fn truncated_arrow_chain_at_eof_does_not_crash() {
    let mut parser = Parser::new("$a->b->c->");
    let result = parser.parse();
    assert!(result.is_ok(), "Parser must not crash on truncated method chain at EOF");
}

// ──────────────────────────────────────────────────────────────
// Regression: clean Perl must NOT emit InfixRhs or TruncatedChain
// ──────────────────────────────────────────────────────────────

#[test]
fn clean_addition_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $a + $b;");
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn clean_concatenation_does_not_emit_recovered() {
    let errors = parse_errors(r#"my $s = $a . $b . $c;"#);
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn clean_method_call_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $obj->method();");
    assert_not_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn clean_arrow_subscript_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $aref->[0];");
    assert_not_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn clean_hash_subscript_does_not_emit_recovered() {
    let errors = parse_errors(r#"my $x = $href->{key};"#);
    assert_not_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn clean_chained_methods_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $obj->foo->bar->baz;");
    assert_not_recovered(&errors, RecoverySite::PostfixChain, RecoveryKind::TruncatedChain);
}

#[test]
fn clean_logical_or_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $a || $b || $c;");
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn clean_defined_or_assign_does_not_emit_recovered() {
    let errors = parse_errors("$x //= 'default';");
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn recovery_only_parse_is_classified_as_salvaged() {
    let mut parser = Parser::new("my $x = $a +;");
    let output = parser.parse_with_recovery();
    assert_eq!(
        output.classify_closeout(0, false),
        ParseCloseoutKind::StructuredRecoveryOnly,
        "recoverable missing-RHS input should classify as structured recovery only"
    );
}

// ──────────────────────────────────────────────────────────────
// Pattern 1: Equality and relational operators (deep review addition)
// ──────────────────────────────────────────────────────────────

#[test]
fn missing_rhs_equality_before_semicolon_emits_recovered() {
    // `$x == ;` — equality operator with missing RHS
    let errors = parse_errors("my $x = $a ==;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_less_than_before_semicolon_emits_recovered() {
    // `$x < ;` — relational operator with missing RHS
    let errors = parse_errors("my $x = $a <;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_string_eq_before_semicolon_emits_recovered() {
    // `$s eq ;` — string equality operator with missing RHS
    let errors = parse_errors("my $x = $s eq;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn missing_rhs_power_before_semicolon_emits_recovered() {
    // `$x ** ;` — power operator with missing RHS
    let errors = parse_errors("my $x = $a **;");
    assert_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn clean_equality_does_not_emit_recovered() {
    let errors = parse_errors("my $x = $a == $b;");
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

#[test]
fn clean_relational_does_not_emit_recovered() {
    let errors = parse_errors("if ($a < $b) { print 1; }");
    assert_not_recovered(&errors, RecoverySite::InfixRhs, RecoveryKind::MissingOperand);
}

// ──────────────────────────────────────────────────────────────
// Downstream stability: sibling statements survive recovery
// ──────────────────────────────────────────────────────────────

#[test]
fn sibling_statement_survives_after_missing_rhs() {
    // The statement after the error must still parse
    let source = "my $x = $a +;\nmy $y = 10;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // `my $y = 10` must appear in the tree
    assert!(
        sexp.contains("$y") || sexp.contains("10"),
        "Downstream statement `my $y = 10` not found after recovery. sexp:\n{}",
        sexp
    );
}

#[test]
fn sibling_statement_survives_after_truncated_arrow() {
    let source = "$obj->;\nmy $y = 42;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("$y") || sexp.contains("42"),
        "Downstream statement not found after truncated-arrow recovery. sexp:\n{}",
        sexp
    );
}

//! TDD tests for Phase 1 missing-closer recovery (#2843).
//!
//! These tests verify that `expect_closing_delimiter` emits
//! `ParseError::Recovered { kind: RecoveryKind::InsertedCloser, .. }` when it
//! encounters a strong follower (`;`, `}`, keyword, or EOF) instead of a real
//! closing delimiter, and that parsing continues to produce a usable AST.
//!
//! Invariant: clean-parse tests (no missing closer) must still produce zero
//! errors and zero Recovered diagnostics.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::error::{ParseError, RecoveryKind, RecoverySite};
use perl_parser_core::{NodeKind, ParseCloseoutKind, Parser};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse and return errors.
fn parse_errors(src: &str) -> (perl_parser_core::Node, Vec<ParseError>) {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let errors = parser.errors().to_vec();
    (ast, errors)
}

/// Count `ParseError::Recovered` entries with `InsertedCloser` kind.
fn count_inserted_closer(errors: &[ParseError]) -> usize {
    errors
        .iter()
        .filter(|e| matches!(e, ParseError::Recovered { kind: RecoveryKind::InsertedCloser, .. }))
        .count()
}

/// Count total statements in a Program node.
fn statement_count(ast: &perl_parser_core::Node) -> usize {
    match &ast.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Phase 1 target cases — missing `)` at strong followers
// ---------------------------------------------------------------------------

/// `foo(bar($x)` — missing outer `)` — should parse with Recovered error, not fail.
/// The inner call is complete; the outer call is missing its closer before EOF.
#[test]
fn missing_outer_paren_before_eof_emits_recovered() {
    let src = "foo(bar($x))";
    // First: verify the balanced version parses clean
    assert_clean_parse(src);

    // Now the unbalanced version
    let src = "foo(bar($x)";
    let (ast, errors) = parse_errors(src);

    // Must produce at least one InsertedCloser recovery
    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected at least one InsertedCloser recovery for '{}', got errors: {:?}",
        src,
        errors
    );

    // AST must be a Program (parser did not give up)
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node even with missing closer"
    );
}

/// `my @arr = ($a, $b` — missing `)` before `;` — InsertedCloser recovery at ArgList.
#[test]
fn missing_paren_before_semicolon_emits_recovered() {
    let src = "my @arr = ($a, $b; my $c = 1;";
    let (ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected InsertedCloser for missing ')' before ';' in '{}', got errors: {:?}",
        src,
        errors
    );

    // Downstream statement must still be parsed
    let stmts = statement_count(&ast);
    assert!(
        stmts >= 2,
        "Missing ')' before ';' must not swallow downstream statement, got {} stmts",
        stmts
    );
}

/// `$hash{$key` — missing `}` before `;`.
/// Note: hash subscripts use `expect()` not `expect_closing_delimiter()`, so
/// this path does not emit `ParseError::Recovered { InsertedCloser }` yet
/// (that is a Phase 1 extension beyond this function).  We verify that the
/// parser does not crash and produces a partial AST.
#[test]
fn missing_hash_brace_before_semicolon_does_not_crash() {
    let src = "$hash{$key; my $x = 1;";
    let (ast, _errors) = parse_errors(src);
    // Parser must return a Program node (not panic or produce Err)
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for '{}' (partial parse is OK)",
        src
    );
}

/// Missing `)` before a keyword follower at block-start level.
/// This verifies `expect_closing_delimiter` fires and emits `InsertedCloser`
/// when a keyword follows where `)` was expected.  We use `while` which is a
/// keyword in `is_delimiter_recovery_point`.
#[test]
fn missing_paren_before_keyword_follower_emits_recovered() {
    // `while` keyword is in is_delimiter_recovery_point so recovery fires.
    // `foo(1 while` — missing `)` before `while` keyword.
    let src = "foo(1 while $x > 0) { }";
    // This is a deliberate syntax error: missing `)` before `while`.
    // The `while` is a strong follower → InsertedCloser recovery at ArgList.
    let (_ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected InsertedCloser for missing ')' before 'while' in '{}', got errors: {:?}",
        src,
        errors
    );
}

/// Missing `]` before `;` — array subscript recovery.
#[test]
fn missing_bracket_before_semicolon_emits_recovered() {
    let src = "my $x = $arr[$i; my $y = 2;";
    let (ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected InsertedCloser for missing ']' before ';' in '{}', got errors: {:?}",
        src,
        errors
    );

    // Downstream must survive
    let stmts = statement_count(&ast);
    assert!(
        stmts >= 2,
        "Missing ']' before ';' must not swallow downstream statement, got {} stmts",
        stmts
    );
}

/// The `Recovered` error must record the correct site for `)` recovery.
#[test]
fn recovered_error_has_correct_site_for_arg_list() {
    let src = "foo($x; my $y = 1;";
    let (_ast, errors) = parse_errors(src);

    let has_arg_list_recovery = errors.iter().any(|e| {
        matches!(
            e,
            ParseError::Recovered {
                site: RecoverySite::ArgList,
                kind: RecoveryKind::InsertedCloser,
                ..
            }
        )
    });

    assert!(
        has_arg_list_recovery,
        "Expected Recovered {{ site: ArgList, kind: InsertedCloser }} for '{}', got: {:?}",
        src, errors
    );
}

/// Missing `]` before EOF (not before `;`) — tests the new EOF arm added in this PR.
/// The previous code only had `None` for EOF; `Some(Eof)` was absent and never fired.
#[test]
fn missing_bracket_before_eof_emits_recovered() {
    // Balanced baseline — must parse clean
    assert_clean_parse("my $v = $arr[$i];");

    let src = "my $v = $arr[$i";
    let (ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected InsertedCloser for missing ']' before EOF in '{}', got errors: {:?}",
        src,
        errors
    );

    // AST must not be abandoned
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for '{}' (partial parse is OK)",
        src
    );
}

/// The `Recovered` error for `]` must record `RecoverySite::ArraySubscript`.
#[test]
fn recovered_error_has_correct_site_for_array_subscript() {
    let src = "my $v = $arr[$i;";
    let (_ast, errors) = parse_errors(src);

    let has_array_recovery = errors.iter().any(|e| {
        matches!(
            e,
            ParseError::Recovered {
                site: RecoverySite::ArraySubscript,
                kind: RecoveryKind::InsertedCloser,
                ..
            }
        )
    });

    assert!(
        has_array_recovery,
        "Expected Recovered {{ site: ArraySubscript, kind: InsertedCloser }} for '{}', got: {:?}",
        src, errors
    );
}

/// Both inner and outer parens missing — each level independently emits InsertedCloser.
/// `foo(bar($x` → inner bar's `)` recovered at EOF, then outer foo's `)` also recovered.
/// This tests that nested recovery does not lose the outer call's recovery.
#[test]
fn nested_missing_both_parens_at_eof_emits_two_recovered() {
    // Balanced baseline
    assert_clean_parse("foo(bar($x))");

    let src = "foo(bar($x";
    let (ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 2,
        "Expected at least 2 InsertedCloser recoveries (one per level) for '{}', got {}: {:?}",
        src,
        recovered,
        errors
    );

    // Parser still produces a usable Program
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for nested missing closers in '{}'",
        src
    );
}

/// Paren missing before EOF inside a list assignment — tests truncated file recovery.
/// The most common real-world case: user is still typing when LSP analyses the buffer.
#[test]
fn missing_paren_at_eof_in_list_assignment_emits_recovered() {
    assert_clean_parse("my @a = ($x, $y);");

    let src = "my @a = ($x, $y";
    let (ast, errors) = parse_errors(src);

    let recovered = count_inserted_closer(&errors);
    assert!(
        recovered >= 1,
        "Expected InsertedCloser for truncated list '{}', got errors: {:?}",
        src,
        errors
    );

    // The declaration statement itself must have been produced
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must produce a Program for truncated list assignment '{}'",
        src
    );
}

// ---------------------------------------------------------------------------
// Regression: clean-parse inputs must produce zero Recovered errors
// ---------------------------------------------------------------------------

#[test]
fn clean_function_call_no_recovery() {
    assert_clean_parse("foo($x, $y);");
}

#[test]
fn clean_nested_call_no_recovery() {
    assert_clean_parse("foo(bar($x));");
}

#[test]
fn clean_array_subscript_no_recovery() {
    assert_clean_parse("my $v = $arr[$i];");
}

#[test]
fn clean_hash_subscript_no_recovery() {
    assert_clean_parse("my $v = $hash{$key};");
}

#[test]
fn clean_list_assignment_no_recovery() {
    assert_clean_parse("my @arr = ($a, $b, $c);");
}

#[test]
fn clean_if_condition_no_recovery() {
    assert_clean_parse("if ($x == 1) { print 'yes'; }");
}

#[test]
fn clean_inputs_produce_zero_inserted_closer() {
    let cases = ["my $x = foo(1, 2);", "my @a = (1, 2, 3);", "my $v = $h{k};", "my $v = $a[0];"];
    for src in cases {
        let (_, errors) = parse_errors(src);
        let recovered = count_inserted_closer(&errors);
        assert_eq!(
            recovered, 0,
            "Clean source '{}' should produce zero InsertedCloser recoveries, got {}",
            src, recovered
        );
    }
}

#[test]
fn missing_closer_classifies_as_structured_recovery_only() {
    let mut parser = Parser::new("my @a = ($x, $y");
    let output = parser.parse_with_recovery();
    assert_eq!(output.classify_closeout(0, false), ParseCloseoutKind::StructuredRecoveryOnly);
}

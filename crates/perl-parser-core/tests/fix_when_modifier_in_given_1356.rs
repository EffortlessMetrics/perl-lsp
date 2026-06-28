//! Regression tests for #1356 — `when`/`default` statement modifiers (and
//! ordinary statements) inside a `given` block.
//!
//! Perl 5.10+ allows a `given` block to contain arbitrary statements, not just
//! `when`/`default` block constructs. In particular a statement may carry a
//! `when` (or `default`) postfix modifier, e.g.
//! `print "matched" when $_ == 5;`. The parser previously rejected anything
//! that was not a leading `when`/`default` keyword inside a given block with
//! "Expected 'when' or 'default' in given block".

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_when_modifier_inside_given_block() {
    let source = r#"
given (5) {
    print "When modifier: matched 5\n" when $_ == 5;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_default_modifier_inside_given_block() {
    // `default` has no operand as a modifier; but a plain bareword/postfix mix
    // exercises the same fallback path. Use a `when` modifier with a complex
    // condition to stress the general statement parser inside the block.
    let source = r#"
given ($x) {
    say "low" when $_ < 10;
    say "high" when $_ >= 10;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_plain_statement_inside_given_block() {
    // Ordinary statements are legal inside a given block alongside when/default.
    let source = r#"
given ($x) {
    my $label = "result";
    when (5) { print "$label: five\n"; }
    default { print "$label: other\n"; }
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_when_block_form_still_parses() {
    // Regression guard: the classic when/default block form must keep working.
    let source = r#"
given ($x) {
    when (1) { print "one\n"; }
    when (2) { print "two\n"; }
    default  { print "other\n"; }
}
"#;
    assert_clean_parse(source);
}

// --- Edge cases added by deep review ---

#[test]
fn test_empty_given_block() {
    // An empty given block must not infinite-loop or panic.
    let source = r#"
given ($x) {
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_nested_given_blocks() {
    // A nested `given` falls through the general parse_statement path of the
    // outer given's fallback arm.
    let source = r#"
given ($x) {
    given ($y) {
        when (1) { print "inner one\n"; }
        default  { print "inner other\n"; }
    }
    when (0) { print "outer zero\n"; }
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_when_block_then_trailing_statement() {
    // A `when` block followed by an ordinary statement exercises the
    // transition between the when-block arm and the fallback arm.
    let source = r#"
given ($x) {
    when (1) { print "one\n"; }
    my $done = 1;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_lone_semicolons_in_given_block() {
    // Lone semicolons inside a given block must be silently dropped (they
    // become empty blocks that the filter skips) without corrupting the
    // surrounding when/default arms.
    let source = r#"
given ($x) {
    ;
    when (1) { print "one\n"; }
    ;
    default  { print "other\n"; }
    ;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_statement_modifier_when_complex_condition() {
    // when modifier with a complex boolean condition — exercises the full
    // expression parser from the fallback arm.
    let source = r#"
given ($x) {
    print "in range\n" when $_ >= 1 && $_ <= 10;
    print "out of range\n" when $_ < 1 || $_ > 10;
}
"#;
    assert_clean_parse(source);
}

// --- Error-recovery path of the fallback arm ---
//
// When a statement inside a `given` block fails to parse (parse_statement
// returns Err), the fallback arm records the error, builds a recovery node,
// and calls synchronize() so the rest of the block — including later
// when/default arms — still parses. These tests drive that Err arm directly
// (an "exact error-variant" + "call-observation" of the recovery seam).

#[test]
fn test_malformed_statement_in_given_block_recovers() {
    // A statement starting with `=` cannot parse as an expression, so
    // parse_statement returns Err and the recovery arm runs. The trailing
    // `when` arm must still be parsed after recovery.
    let source = r#"
given ($x) {
    = 5 ;
    when (1) { print "one\n"; }
}
"#;
    // An error is reported for the malformed statement...
    assert_has_error(source, "expected expression");
    // ...and recovery keeps the surrounding given/when structure intact.
    let sexp = parse(source).to_sexp();
    assert!(
        sexp.contains("given") && sexp.contains("when"),
        "expected recovery to preserve given/when structure, got:\n{}",
        sexp
    );
}

#[test]
fn test_malformed_then_default_in_given_block_recovers() {
    // A bare `,` is not a valid statement start; the recovery arm runs and the
    // following `default` block must still be parsed.
    let source = r#"
given ($x) {
    , ;
    when (1) { print "one\n"; }
    default  { print "other\n"; }
}
"#;
    assert_has_error(source, "expected expression");
    let sexp = parse(source).to_sexp();
    assert!(
        sexp.contains("when") && sexp.contains("default"),
        "expected recovery to preserve when/default arms, got:\n{}",
        sexp
    );
}

// --- Extended error-recovery coverage ---
//
// Additional tests to ensure that error recovery paths in parse_given_block
// achieve patch-coverage thresholds. These exercise the error node creation,
// AST integrity preservation, and synchronization logic.

#[test]
fn test_error_node_created_on_malformed_statement() {
    // Verify that a malformed statement produces an explicit Error node in
    // the AST (not just a parser diagnostic).
    let source = r#"
given ($x) {
    = missing_operand;
    when (0) { }
}
"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Confirm an error node exists in the AST
    assert!(
        sexp.to_lowercase().contains("error"),
        "expected error node in AST for malformed statement, got:\n{}",
        sexp
    );

    // Confirm that despite the error, the when block is still present
    assert!(
        sexp.contains("when"),
        "expected when block preserved after error recovery, got:\n{}",
        sexp
    );
}

#[test]
fn test_multiple_errors_in_given_block_with_recovery() {
    // Two separate malformed statements, both should recover and the block
    // should continue parsing subsequent when arms.
    let source = r#"
given ($x) {
    = bad ;
    , also_bad ;
    when (1) { print "survived\n"; }
}
"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Both malformed statements should generate error nodes (the sexp renders
    // them as uppercase `ERROR`, so match case-insensitively).
    let error_count = sexp.to_lowercase().matches("error").count();
    assert!(
        error_count >= 2,
        "expected at least 2 error nodes in AST, found {}, sexp:\n{}",
        error_count,
        sexp
    );

    // The when block must still be present and parseable
    assert!(
        sexp.contains("when"),
        "expected when block to survive multiple error recoveries, got:\n{}",
        sexp
    );
}

#[test]
fn test_error_recovery_preserves_ast_structure() {
    // Ensure that the overall `given` statement structure is preserved
    // even when the block contains malformed statements. The AST should
    // still reflect a proper Given node containing the error nodes.
    let source = r#"
given ($input) {
    bad_stmt;
    when (42) { print "found\n"; }
    default  { print "nope\n"; }
}
"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // The given statement itself must be intact
    assert!(
        sexp.contains("given"),
        "expected given statement in AST after recovery, got:\n{}",
        sexp
    );

    // Both when and default must be present
    assert!(
        sexp.contains("when") && sexp.contains("default"),
        "expected when/default arms intact after recovery, got:\n{}",
        sexp
    );
}

#[test]
fn test_statement_recovery_after_incomplete_expression() {
    // An incomplete expression that starts like a method call but ends
    // prematurely should trigger error recovery in the fallback arm.
    let source = r#"
given ($obj) {
    $obj->;
    when (0) { print "recovered\n"; }
}
"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // The block should contain an error node from recovery
    assert!(
        sexp.to_lowercase().contains("error") || sexp.to_lowercase().contains("missing"),
        "expected error or missing node from incomplete expression, got:\n{}",
        sexp
    );

    // The when block must still be parsed despite the error
    assert!(
        sexp.contains("when"),
        "expected when block after recovery from incomplete expression, got:\n{}",
        sexp
    );
}

#[test]
fn test_error_recovery_with_trailing_stray_tokens() {
    // A malformed statement with trailing stray tokens that don't form a
    // complete statement should still allow synchronization and recovery.
    let source = r#"
given ($x) {
    & * @ $ % ;
    when (1) { print "ok\n"; }
}
"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // An error node should be generated
    assert!(
        sexp.to_lowercase().contains("error") || sexp.to_lowercase().contains("missing"),
        "expected error from stray tokens, got:\n{}",
        sexp
    );

    // Recovery must allow subsequent when to be parsed
    assert!(
        sexp.contains("when"),
        "expected when block parsed after stray-token error, got:\n{}",
        sexp
    );
}

// --- Depth-guard propagation and synchronize-fail branches ---

#[test]
fn test_deeply_nested_given_propagates_depth_error_without_panic() {
    use perl_parser_core::Parser;
    // Deep nesting trips the parser's depth guard. The error must propagate out
    // of the fallback arm's early-return branch (RecursionLimit / NestingTooDeep
    // / Cancelled) gracefully — no panic, no stack overflow.
    let source =
        format!("{}when (1) {{ print 1; }}{}", "given ($x) {\n".repeat(400), "}\n".repeat(400));
    let mut parser = Parser::new(&source);
    let result = parser.parse();
    assert!(
        result.is_err() || !parser.get_errors().is_empty(),
        "expected a graceful depth error for deeply nested given blocks"
    );
}

#[test]
fn test_unterminated_malformed_given_block_breaks_cleanly() {
    use perl_parser_core::Parser;
    // Malformed statements with no recovery boundary before EOF make
    // synchronize() fail, exercising the loop's break-on-failure path. The
    // parser must report an error rather than loop forever or panic.
    let source = "given ($x) {\n= = = =";
    let mut parser = Parser::new(source);
    let result = parser.parse();
    assert!(
        result.is_err() || !parser.get_errors().is_empty(),
        "expected a graceful error for an unterminated malformed given block"
    );
}

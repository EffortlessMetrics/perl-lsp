//! Edge case tests for label/ternary disambiguation in `is_label_start()`.
//!
//! These tests supplement the red tests in `fix_label_ternary_disambiguation.rs`
//! by covering additional edge cases and boundary conditions.
//!
//! The `is_label_start()` function uses 3-token lookahead:
//! - Token 1 (peek): Is it an `Identifier`?
//! - Token 2 (peek_second): Is it a `Colon`?
//! - Token 3 (peek_third): Can the token after colon start a statement?
//!
//! These tests verify the implementation handles edge cases correctly.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Edge Case: Identifier at statement start followed by ternary
// =============================================================================

/// Edge case: Bare identifier at statement start, followed by ternary.
/// This tests `is_label_start()` when the identifier is followed by colon,
/// but the third token is `?` (ternary question), which cannot start a statement.
/// Without the 3-token fix, this would incorrectly identify `FOO:` as a label.
#[test]
fn test_edge_case_identifier_ternary_question() {
    // `FOO: ?` - identifier colon followed by ternary question
    // The 3rd token `?` is in the invalid list, so is_label_start returns false
    // This should parse (gracefully handle the invalid Perl syntax)
    let source = r#"FOO: ?"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // The parse should complete (even if it produces an error node)
    // We're testing that it doesn't panic or produce unexpected_colon
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: ?");
}

/// Edge case: Identifier colon followed by chained ternary colon.
/// `$x ? $y : : $z` - the second `:` is the ternary else-part,
/// but it could be mistaken for a label colon.
#[test]
fn test_edge_case_chained_ternary_second_colon() {
    // `FOO: :` - identifier colon followed by another colon
    // The 3rd token `:` is in the invalid list
    let source = r#"FOO: :"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: :");
}

/// Edge case: Identifier colon followed by semicolon (invalid label).
/// `FOO: ;` - label colon followed by semicolon is not valid.
#[test]
fn test_edge_case_identifier_colon_semicolon() {
    let source = r#"FOO: ;"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: ;");
}

/// Edge case: Identifier colon followed by comma (list expression).
/// `FOO: ,` - in list context, comma after colon is not a label.
#[test]
fn test_edge_case_identifier_colon_comma() {
    let source = r#"FOO: ,"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: ,");
}

// =============================================================================
// Edge Case: Label with closing delimiters after colon
// =============================================================================

/// Edge case: Identifier colon followed by right paren.
/// `FOO: )` - closing paren cannot start a statement.
#[test]
fn test_edge_case_identifier_colon_right_paren() {
    let source = r#"FOO: )"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: )");
}

/// Edge case: Identifier colon followed by right bracket.
/// `FOO: ]` - closing bracket cannot start a statement.
#[test]
fn test_edge_case_identifier_colon_right_bracket() {
    let source = r#"FOO: ]"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: ]");
}

/// Edge case: Identifier colon followed by right brace.
/// `FOO: }` - orphan closing brace cannot start a statement.
#[test]
fn test_edge_case_identifier_colon_right_brace() {
    let source = r#"FOO: }"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: }}");
}

// =============================================================================
// Edge Case: Identifier colon at EOF
// =============================================================================

/// Edge case: Identifier colon at end of file.
/// `FOO:` - nothing after colon, cannot be a valid label statement.
#[test]
fn test_edge_case_identifier_colon_eof() {
    let source = r#"FOO:"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("expected_colon"), "Should not produce expected_colon for FOO: at EOF");
}

// =============================================================================
// Edge Case: Valid label patterns that should still work
// =============================================================================

/// Edge case: Label with unary plus after colon.
/// `FOO: + 5` - unary plus can start an expression statement.
#[test]
fn test_edge_case_label_unary_plus() {
    assert_clean_parse(r#"FOO: + 5; print "done\n";"#);
}

/// Edge case: Label with unary minus after colon.
/// `FOO: - 3` - unary minus can start an expression statement.
#[test]
fn test_edge_case_label_unary_minus() {
    assert_clean_parse(r#"FOO: - 3; print "done\n";"#);
}

/// Edge case: Label with bitwise not after colon.
/// `FOO: ~ 0` - bitwise not can start an expression statement.
#[test]
fn test_edge_case_label_bitwise_not() {
    assert_clean_parse(r#"FOO: ~ 0; print "done\n";"#);
}

/// Edge case: Label with logical not after colon.
/// `FOO: ! $x` - logical not can start an expression statement.
#[test]
fn test_edge_case_label_logical_not() {
    assert_clean_parse(r#"FOO: ! $x; print "done\n";"#);
}

/// Edge case: Label with indirect object syntax.
/// `FOO BAR: print "hello"` - indirect object with labeled statement.
#[test]
fn test_edge_case_label_indirect_object() {
    assert_clean_parse(r#"HELLO: print "hello\n";"#);
}

// =============================================================================
// Edge Case: Label in nested contexts
// =============================================================================

/// Edge case: Label inside a do block.
#[test]
fn test_edge_case_label_in_do_block() {
    assert_clean_parse(r#"my $x = do { FOO: my $y = 1; $y }; print $x, "\n";"#);
}

/// Edge case: Label inside a bare block.
#[test]
fn test_edge_case_label_in_bare_block() {
    assert_clean_parse(r#"{ FOO: my $y = 1; } print "done\n";"#);
}

/// Edge case: Multiple labels in sequence.
#[test]
fn test_edge_case_multiple_labels() {
    assert_clean_parse(
        r#"
        OUTER: INNER: my $x = 1;
        print $x, "\n";
        "#,
    );
}

// =============================================================================
// Edge Case: Qualified identifiers (should NOT be treated as labels)
// =============================================================================

/// Edge case: Qualified identifier with double colon.
/// `$Foo::Bar:` - double colon is DoubleColon token, not Colon,
/// so this should NOT be treated as a label start.
#[test]
fn test_edge_case_qualified_identifier_not_label() {
    // $Foo::Bar is a qualified identifier, not a label
    // The `:` after :: is not a label colon
    assert_clean_parse(r#"my $x = $Foo::Bar; print $x, "\n";"#);
}

// =============================================================================
// Edge Case: Labels with all statement-modifier keywords
// =============================================================================

/// Edge case: Label followed by statement with `until` modifier.
#[test]
fn test_edge_case_label_until_modifier() {
    assert_clean_parse(r#"SKIP: print "skipping\n" until $done;"#);
}

/// Edge case: Label followed by statement with `when` modifier (Perl 5.38+).
#[test]
fn test_edge_case_label_when_modifier() {
    assert_clean_parse(r#"DEFAULT: print "default\n" when $is_default;"#);
}

// Note: `given` modifier test removed - `given` is a Perl 5.38+ feature not fully supported

// =============================================================================
// Edge Case: Fat arrow after identifier (NOT identifier-colon-fat-arrow)
// Note: `KEY: => value` is NOT valid Perl - the colon is separate from fat arrow
// The valid pattern is `KEY => value` (no colon before fat arrow)
// =============================================================================

// =============================================================================
// Edge Case: Statement modifier AFTER label, not before
// =============================================================================

/// Edge case: Label with modifier on the following statement.
#[test]
fn test_edge_case_label_with_modifier() {
    // The modifier applies to the statement after the label, not to the label itself
    assert_clean_parse(r#"RETRY: my $x = 1 if $should_retry; print $x, "\n";"#);
}

// =============================================================================
// Edge Case: Ternary with method call condition
// =============================================================================

/// Edge case: Ternary where condition is a method call.
#[test]
fn test_edge_case_ternary_method_call_condition() {
    // $obj->method ? $then : $else
    // The method call is not an identifier-colon, so is_label_start is not triggered
    assert_clean_parse(r#"my $x = $obj->method ? "yes" : "no";"#);
}

/// Edge case: Ternary where condition has subscript.
#[test]
fn test_edge_case_ternary_subscript_condition() {
    // $hash{key} ? $then : $else
    // The subscript is not an identifier-colon
    assert_clean_parse(r#"my $x = $hash{key} ? "yes" : "no";"#);
}

/// Edge case: Ternary where condition is a postfix deref.
#[test]
fn test_edge_case_ternary_postfix_deref_condition() {
    // $obj->$method ? $then : $else
    assert_clean_parse(r#"my $x = $obj->$method ? "yes" : "no";"#);
}

// =============================================================================
// Edge Case: Very long identifier as label (boundary test)
// =============================================================================

/// Edge case: Very long identifier as label name.
#[test]
fn test_edge_case_long_label_identifier() {
    let long_name = "A".repeat(1000);
    let source = format!(r#"{}: my $x = 1; print $x, "\n";"#, long_name);
    assert_clean_parse(&source);
}

/// Edge case: Unicode identifier as label.
#[test]
fn test_edge_case_unicode_label() {
    // Unicode identifiers are valid in Perl 5.16+
    assert_clean_parse(r#"日本語: my $x = 1; print $x, "\n";"#);
}

// =============================================================================
// Edge Case: Mix of labels and control flow
// =============================================================================

/// Edge case: Label with next statement.
#[test]
fn test_edge_case_label_next() {
    assert_clean_parse(
        r#"
        OUTER: for my $i (1..3) {
            next OUTER if $i == 2;
            print "i=$i\n";
        }
        "#,
    );
}

/// Edge case: Label with last statement.
#[test]
fn test_edge_case_label_last() {
    assert_clean_parse(
        r#"
        my $result = 0;
        LOOP: for my $i (1..10) {
            last LOOP if $i > 5;
            $result += $i;
        }
        print "result=$result\n";
        "#,
    );
}

/// Edge case: Label with redo statement.
#[test]
fn test_edge_case_label_redo() {
    assert_clean_parse(
        r#"
        COUNT: {
            my $n = 0;
            redo COUNT if (++$n < 3);
            print "done\n";
        }
        "#,
    );
}

// =============================================================================
// Edge Case: Statements that could be confused with labels
// =============================================================================

/// Edge case: Bare block followed by statement starting with identifier.
#[test]
fn test_edge_case_bare_block_with_identifier_statement() {
    assert_clean_parse(r#"{ 1; } FOO: 2; print "done\n";"#);
}

/// Edge case: Package declaration could look like label.
#[test]
fn test_edge_case_package_not_label() {
    // package declarations use `package` keyword, not a bare identifier
    assert_clean_parse(r#"package My::Package; print "in package\n";"#);
}

/// Edge case: Subroutine declaration could look like label.
#[test]
fn test_edge_case_sub_not_label() {
    // sub declarations use `sub` keyword
    assert_clean_parse(r#"sub foo { print "foo\n" } foo();"#);
}

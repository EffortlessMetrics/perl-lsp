mod cpan_test_helpers;
use cpan_test_helpers::*;

// POSITIVE TEST CASES (baseline + main bug fix + edge cases)

#[test]
fn test_nested_varlist_single_inner_item() {
    // Baseline: single item in nested list should already work.
    // This confirms we don't break existing behavior.
    let source = "my ($a, ($b)) = (1, 2);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_multiple_inner_items() {
    // MAIN BUG FIX: Multiple items in nested list must parse cleanly.
    // Before the fix, this fails with "expected ')', found ','" at the comma after $b.
    // After the fix, all items ($b, $c) are captured.
    let source = "my ($a, ($b, $c)) = (1, 2, 3);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_deeper_nesting() {
    // Edge case: arbitrary depth of nesting (3-level deep).
    let source = "my ($a, ($b, ($c, $d))) = (1, 2, 3, 4);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_with_assignment() {
    // Reproduction case from the issue: multiple nested items with assignment.
    let source = "my ($outer1, $outer2, ($inner1, $inner2)) = (1, 2, (3, 4));";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_undef_in_nested() {
    // Positive: undef as a placeholder is valid in nested context.
    let source = "my ($a, (undef, $b)) = (1, 2, 3);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_multiple_undef() {
    // Edge case: multiple undef items in nested list.
    let source = "my ($a, (undef, undef, $b)) = (1, 2, 3, 4);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_array_in_nested() {
    // Positive: array variables (@arr, %) in nested context are valid.
    let source = "my (@arr, (@inner)) = @list;";
    assert_clean_parse(source);
}

// ADVERSARIAL TEST CASES (hazard-class defenses)

#[test]
fn test_nested_varlist_comma_in_string() {
    // PARSER-1: Literal/comment blindness
    // A comma inside a string literal must NOT be treated as a list separator.
    // This must parse cleanly (the string is transparent to the parser).
    let source = r#"my ($a, ("string with, comma")) = (1, "x");"#;
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_comma_in_qw_string() {
    // PARSER-1: Another literal blindness case — qw() list-like syntax.
    let source = r#"my ($a, (qw(one, two, three))) = (1, 2, 3);"#;
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_nested_parens() {
    // PARSER-2: Nested parens (triple-nested).
    let source = "my ($a, (($b)))  = (1, 2);";
    assert_clean_parse(source);
}

#[test]
fn test_nested_varlist_unbalanced_parens() {
    // PARSER-2: Missing close paren produces error AST, not panic.
    // Should have an Error node or recovery marker, not crash.
    let source = "my ($a, ($b";
    assert_has_error(source, "paren");
}

#[test]
fn test_nested_varlist_unbalanced_deep() {
    // PARSER-2: Unbalanced at depth 2.
    let source = "my ($a, ($b, ($c))";
    assert_has_error(source, "paren");
}

#[test]
fn test_nested_varlist_missing_comma() {
    // PARSER-4: Missing comma between items in nested list.
    // Must produce error AST node from actual parser recovery, not false positive.
    let source = "my ($a, ($b $c)) = (1, 2, 3);";
    assert_has_error(source, "expected");
}

// ORACLE VALIDATION (PARSER-3)
// These inputs are confirmed valid in Perl via `perl -cw`:
// perl -cw -e 'my ($a, ($b, $c)) = (1, 2, 3);' => OK
// perl -cw -e 'my ($a, ($b, ($c, $d))) = (1, 2, 3, 4);' => OK
// perl -cw -e 'my ($outer1, $outer2, ($inner1, $inner2)) = (1, 2, (3, 4));' => OK
// All positive cases above are valid Perl and should parse cleanly.

#[test]
fn test_nested_varlist_perl_oracle() {
    // Meta-test: confirms the oracle cases all parse cleanly.
    // If any of these fail, the implementation violates the Perl oracle.
    let cases = vec![
        "my ($a, ($b)) = (1, 2);",
        "my ($a, ($b, $c)) = (1, 2, 3);",
        "my ($a, ($b, ($c, $d))) = (1, 2, 3, 4);",
        "my ($outer1, $outer2, ($inner1, $inner2)) = (1, 2, (3, 4));",
        "my ($a, (undef, $b)) = (1, 2, 3);",
        "my ($a, (undef, undef, $b)) = (1, 2, 3, 4);",
        "my (@arr, (@inner)) = @list;",
    ];
    for source in cases {
        assert_clean_parse(source);
    }
}

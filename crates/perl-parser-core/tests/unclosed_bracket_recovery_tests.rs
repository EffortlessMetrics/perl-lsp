mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================
// Regression tests: valid bracket patterns still parse cleanly
// =============================================================

#[test]
fn valid_arrow_array_slice_dereference() {
    // ->@[...] postfix dereference (line 68 path)
    assert_clean_parse("my @slice = $ref->@[0, 1, 2];");
}

#[test]
fn valid_arrow_array_subscript() {
    // ->[index] arrow array dereference (line 252 path)
    assert_clean_parse("my $x = $ref->[0];");
}

#[test]
fn valid_arrow_nested_subscript() {
    assert_clean_parse("my $x = $ref->[0]->[1];");
}

#[test]
fn valid_direct_array_subscript() {
    // Direct array indexing (line 324 path)
    assert_clean_parse("my $x = $array[0];");
}

#[test]
fn valid_array_slice() {
    assert_clean_parse("my @s = @array[0, 1, 2];");
}

#[test]
fn valid_complex_subscript_expression() {
    assert_clean_parse("my $x = $ref->[$i + 1];");
}

// =============================================================
// Recovery tests: missing ] at statement boundary should recover
// gracefully instead of producing a hard parse error.
//
// expect_closing_delimiter records a soft error and continues
// when the parser reaches a delimiter recovery point (;, }, etc).
// The parser should produce a source_file with both the broken
// statement and subsequent recovered statements.
// =============================================================

#[test]
fn recover_missing_bracket_arrow_array_slice() {
    // Missing ] in ->@[...] — semicolon should be a recovery point
    let source = "my @slice = $ref->@[0, 1; my $y = 42;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // Parser should recover and continue parsing the next statement
    assert!(
        sexp.contains("source_file"),
        "Parser should produce a source_file node, got:\n{}",
        sexp
    );
    // The second statement should be recovered
    assert!(
        sexp.contains("my_declaration"),
        "Parser should recover and parse 'my $y = 42' after the broken bracket, got:\n{}",
        sexp
    );
}

#[test]
fn recover_missing_bracket_arrow_subscript() {
    // Missing ] in ->[expr] — semicolon should be a recovery point
    let source = "my $x = $ref->[0; my $y = 42;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("source_file"),
        "Parser should produce a source_file node, got:\n{}",
        sexp
    );
    assert!(
        sexp.contains("my_declaration"),
        "Parser should recover and parse 'my $y = 42' after the broken bracket, got:\n{}",
        sexp
    );
}

#[test]
fn recover_missing_bracket_direct_subscript() {
    // Missing ] in array[expr] — semicolon should be a recovery point
    let source = "my $x = $array[0; my $y = 42;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("source_file"),
        "Parser should produce a source_file node, got:\n{}",
        sexp
    );
    assert!(
        sexp.contains("my_declaration"),
        "Parser should recover and parse 'my $y = 42' after the broken bracket, got:\n{}",
        sexp
    );
}

#[test]
fn recover_missing_bracket_at_brace() {
    // Missing ] followed by } — brace is also a recovery point
    let source = "if (1) { my $x = $ref->[0 }";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("source_file"),
        "Parser should produce a source_file node, got:\n{}",
        sexp
    );
}

#[test]
fn recover_missing_bracket_followed_by_keyword() {
    // Missing ] followed by keyword — keyword is a recovery point
    let source = "my $x = $array[0 my $y = 1;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("source_file"),
        "Parser should produce a source_file node, got:\n{}",
        sexp
    );
}

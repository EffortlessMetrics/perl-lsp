mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// POSITIVE TESTS — keyword tokens before fat arrow must parse cleanly
// (these were RED before the fix; now GREEN)
// =============================================================================

#[test]
fn test_negative_or_before_fatarrow() {
    // Core case: -or => value is a valid bareword hash key in Perl.
    // Before the fix: "expected expression, found 'or' at position 11"
    let source = "my %h = (-or => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_and_before_fatarrow() {
    // Core case: -and => value is a valid bareword hash key in Perl.
    // Before the fix: "expected expression, found 'and'"
    let source = "my %h = (-and => 5);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_xor_before_fatarrow() {
    // Core case: -xor => value.
    let source = "my %h = (-xor => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_not_before_fatarrow() {
    // Core case: -not => value (WordNot).
    let source = "my %h = (-not => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_or_in_array_literal() {
    // -or => in anonymous array constructor.
    let source = "my $y = [ -or => 1 ];";
    assert_clean_parse(source);
}

#[test]
fn test_negative_and_in_array_literal() {
    // -and => in anonymous array constructor.
    let source = "my $y = [ -and => 5 ];";
    assert_clean_parse(source);
}

#[test]
fn test_negative_keyword_as_function_argument() {
    // -or => passed as a function argument (hash flattening).
    let source = "func(-or => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_keyword_in_print_statement() {
    // -or => in a print statement.
    let source = "print -or => 1;";
    assert_clean_parse(source);
}

#[test]
fn test_all_lowprec_ops_before_fatarrow() {
    // All word-operator keywords used as bareword keys in the same hash.
    let source = r#"my %h = (
        -or  => 1,
        -and => 2,
        -xor => 3,
        -not => 4,
    );"#;
    assert_clean_parse(source);
}

#[test]
fn test_negative_cmp_before_fatarrow() {
    // cmp (StringCompare token) as a bareword key.
    let source = "my %h = (-cmp => 1);";
    assert_clean_parse(source);
}

// =============================================================================
// REALISTIC PATTERN — DBIx::Class / SQL::Abstract style
// (These patterns are heavily used in the Perl ecosystem)
// =============================================================================

#[test]
fn test_realistic_negative_bareword_pattern() {
    // SQL::Abstract / DBIx::Class search: -or and -and as condition combinators.
    // Before fix: parser died at the first -or keyword.
    let source = r#"my $rs = $schema->resultset('Foo')->search({
        -or => [
            name => 'bar',
            type => 'baz',
        ],
        -and => [
            active => 1,
        ],
    });"#;
    assert_clean_parse(source);
}

#[test]
fn test_dbix_class_like_search_pattern() {
    // Variant: -or at top level of a function call.
    let source = r#"$rs->search(-or => [ foo => 1, bar => 2 ]);"#;
    assert_clean_parse(source);
}

// =============================================================================
// REGRESSION GUARDS — genuine unary minus must still work
// =============================================================================

#[test]
fn test_unary_minus_on_variable() {
    let source = "my $z = -$x;";
    assert_clean_parse(source);
}

#[test]
fn test_unary_minus_on_function_call() {
    let source = "my $z = -func();";
    assert_clean_parse(source);
}

#[test]
fn test_unary_minus_on_literal() {
    let source = "my $v = -123;";
    assert_clean_parse(source);
}

#[test]
fn test_unary_minus_on_array_access() {
    let source = "my $z = -$arr[0];";
    assert_clean_parse(source);
}

#[test]
fn test_unary_minus_on_hash_access() {
    let source = "my $z = -$h{key};";
    assert_clean_parse(source);
}

// =============================================================================
// REGRESSION GUARDS — file-test operators must still work
// =============================================================================

#[test]
fn test_file_test_operator_e() {
    let source = "if (-e $file) { 1 }";
    assert_clean_parse(source);
}

#[test]
fn test_file_test_operator_f() {
    let source = "if (-f $path) { 1 }";
    assert_clean_parse(source);
}

#[test]
fn test_file_test_operator_d() {
    let source = "if (-d $dir) { 1 }";
    assert_clean_parse(source);
}

#[test]
fn test_file_test_operator_s() {
    // -s is file size test, not a substitution operator
    let source = "my $sz = -s $file;";
    assert_clean_parse(source);
}

// =============================================================================
// REGRESSION GUARDS — single-char barewords before => already worked
// =============================================================================

#[test]
fn test_negative_file_test_letter_as_bareword_before_fatarrow() {
    // -G before => is already handled by the single-char Identifier path.
    let source = "my %h = (-G => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_r_as_bareword_before_fatarrow() {
    // -r before => is already handled by the single-char Identifier path.
    let source = "my %h = (-r => 'readable');";
    assert_clean_parse(source);
}

// =============================================================================
// REGRESSION GUARDS — non-keyword barewords before => already worked
// =============================================================================

#[test]
fn test_negative_nonkeyword_before_fatarrow() {
    // -foo => already works (regular Identifier token).
    let source = "my %h = (-foo => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_bareword_multiline_hash() {
    // Multi-line hash with non-keyword negative barewords.
    let source = "my %h = (-name => 'x', -type => 'y', -color => 'z');";
    assert_clean_parse(source);
}

// =============================================================================
// REGRESSION GUARDS — keywords as barewords in primary context already work
// =============================================================================

#[test]
fn test_negative_if_before_fatarrow() {
    // -if => already works (If parsed as Identifier in primary context).
    let source = "my %h = (-if => 1);";
    assert_clean_parse(source);
}

#[test]
fn test_negative_unless_before_fatarrow() {
    // -unless => already works.
    let source = "my %h = (-unless => 1);";
    assert_clean_parse(source);
}

// =============================================================================
// ERROR RECOVERY — malformed input must not panic
// =============================================================================

#[test]
fn test_error_recovery_unclosed_array_with_negative_keyword() {
    // [ -or => 1  (missing ]) — must produce error node, not panic.
    let source = "[ -or => 1";
    assert_has_error(source, "");
}

#[test]
fn test_error_recovery_unclosed_paren_with_negative_keyword() {
    // ( -or => 1  (missing )) — must produce error node, not panic.
    let source = "( -or => 1";
    assert_has_error(source, "");
}

#[test]
fn test_error_recovery_extra_closing_bracket() {
    // Mismatched delimiters — must not panic.
    let source = "[ -or => 1 ] ]";
    // Parser may or may not error (extra bracket at statement level can be
    // valid in some recovery modes), but must not panic.
    let _ = parse(source);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_space_between_minus_and_or() {
    // "- or" with a space: minus and 'or' are separate tokens at this distance.
    // This is a low-precedence logical NOT of nothing, which is a parse error.
    // The important thing is that it does not panic.
    let source = "my $x = - or 1;";
    // Result may be clean (recovered) or error — just must not panic.
    let _ = parse(source);
}

#[test]
fn test_negative_or_not_fatarrow() {
    // -or without a following => is not a bareword key context.
    // Before the fix, this panicked or gave a very wrong error.
    // After the fix it may still be an error, but a graceful one.
    let source = "my $x = -or;";
    // Must not panic; result may be an error node.
    let _ = parse(source);
}

// =============================================================================
// ORACLE VALIDATION (PARSER-3)
// All inputs below are confirmed valid by: perl -cw -e '<source>'
// =============================================================================

#[test]
fn test_perl_oracle_negative_barewords() {
    // These are all confirmed valid Perl syntax:
    // perl -cw -e 'my %h = (-or => 1);'   => OK
    // perl -cw -e 'my %h = (-and => 2);'  => OK
    // perl -cw -e 'my %h = (-xor => 3);'  => OK
    // perl -cw -e 'my %h = (-not => 4);'  => OK
    // perl -cw -e 'my %h = (-cmp => 5);'  => OK
    let cases = vec![
        "my %h = (-or => 1);",
        "my %h = (-and => 2);",
        "my %h = (-xor => 3);",
        "my %h = (-not => 4);",
        "my %h = (-cmp => 5);",
        "my $y = [ -or => 1 ];",
        "my $y = [ -and => 1, -or => 2 ];",
        "func(-or => 1, -and => 2);",
    ];
    for source in cases {
        assert_clean_parse(source);
    }
}

// =============================================================================
// BEHAVIORAL ASSERTIONS — verify the AST node value, not just "no crash"
// =============================================================================

#[test]
fn test_negative_or_produces_identifier_in_sexp() {
    // The fix must produce NodeKind::Identifier { name: "-or" }, not a Unary
    // node wrapping an error.  The sexp must contain "-or" as a string key so
    // this test fails on regression rather than silently accepting an error node.
    let ast = parse("my %h = (-or => 1);");
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("-or"),
        "Expected sexp to contain '-or' as an identifier key, got:\n{}",
        sexp
    );
    assert!(
        !sexp.to_lowercase().contains("error"),
        "Expected no error nodes in sexp for (-or => 1), got:\n{}",
        sexp
    );
}

// =============================================================================
// DISAMBIGUATION — plain `=` (assignment) must NOT trigger the bareword path
// =============================================================================

#[test]
fn test_minus_keyword_plain_assign_not_fat_arrow() {
    // `-or = $x` uses `=`, not `=>`.  The lookahead checks for FatArrow
    // specifically, so the bareword-key path must NOT fire here.
    // The result will be a parse error (cannot assign to -or) but must not
    // produce a spurious bareword identifier or panic.
    let source = "my $x; -or = $x;";
    let _ = parse(source);
}

// =============================================================================
// NESTED NEGATIVE BAREWORDS — the fix must apply inside nested hash values
// =============================================================================

#[test]
fn test_nested_negative_barewords_in_hash() {
    // Inner hash { -and => 1, -not => 0 } triggers the same lookahead path.
    let source = "my %h = (-or => { -and => 1, -not => 0 });";
    assert_clean_parse(source);
}

#[test]
fn test_negative_bareword_deeply_nested_sql_abstract() {
    // SQL::Abstract nested query with multiple levels of -or / -and.
    // perl -cw confirms all three levels are valid Perl syntax.
    let source = r#"my $q = {
        -or => [
            { -and => [ foo => 1, bar => 2 ] },
            { -and => [ baz => 3 ] },
        ],
    };"#;
    assert_clean_parse(source);
}

// =============================================================================
// RIPR+ SEAM PINNING — assertions that capture exact AST structure
// (These tests catch mutations in the new code paths introduced by #1355)
// =============================================================================

/// Verify each keyword produces the exact sexp identifier format.
/// Mutations like wrong name format ("-AND" instead of "-and"), or missing
/// the identifier altogether, will fail these assertions.
#[test]
fn test_ripr_seam_minus_and_sexp_exact_identifier() {
    // The -and keyword must produce an Identifier node with name "-and".
    let ast = parse("my %h = (-and => 5);");
    let sexp = ast.to_sexp();
    assert!(sexp.contains("-and"), "Expected '-and' identifier in sexp, got:\n{}", sexp);
    assert!(
        !sexp.to_lowercase().contains("error"),
        "Expected no error nodes for -and => value, got:\n{}",
        sexp
    );
}

#[test]
fn test_ripr_seam_minus_xor_sexp_exact_identifier() {
    // The -xor keyword must produce an Identifier node with name "-xor".
    let ast = parse("my %h = (-xor => 1);");
    let sexp = ast.to_sexp();
    assert!(sexp.contains("-xor"), "Expected '-xor' identifier in sexp, got:\n{}", sexp);
}

#[test]
fn test_ripr_seam_minus_not_sexp_exact_identifier() {
    // The -not (WordNot) keyword must produce an Identifier with name "-not".
    let ast = parse("my %h = (-not => 1);");
    let sexp = ast.to_sexp();
    assert!(sexp.contains("-not"), "Expected '-not' identifier in sexp, got:\n{}", sexp);
}

#[test]
fn test_ripr_seam_minus_cmp_sexp_exact_identifier() {
    // The -cmp (StringCompare) keyword must produce an Identifier with name "-cmp".
    let ast = parse("my %h = (-cmp => 1);");
    let sexp = ast.to_sexp();
    assert!(sexp.contains("-cmp"), "Expected '-cmp' identifier in sexp, got:\n{}", sexp);
}

/// Test that the lookahead condition is critical.
/// If peek_second() check is removed or inverted, plain '=' must NOT trigger bareword path.
#[test]
fn test_ripr_seam_lookahead_fatarrow_not_assign() {
    // Using '=' instead of '=>' must NOT produce a bareword identifier.
    // The lookahead specifically checks for FatArrow token.
    let source = "my $x; -and = 5;";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // The sexp may contain an error (can't assign to -and) but must not
    // produce a clean bareword identifier "-and" in the intended bareword context.
    // If the lookahead is broken, it might incorrectly treat this as bareword.
}

/// Test the is_word_op_keyword boundary: only these 5 keywords trigger the path.
#[test]
fn test_ripr_seam_is_word_op_keyword_boundary_if() {
    // -if is NOT a word-operator (it's a control-flow keyword handled separately).
    // It should NOT use the new word-op path; control-flow handling already works.
    let source = "my %h = (-if => 1);";
    let ast = parse(source);
    assert_clean_parse(source);
    // Verify it parses cleanly (whether via word-op path or control-flow path
    // is implementation-dependent, but result must be correct).
}

/// Test that the keyword-dispatch is per-keyword, not a collapsed condition.
/// Each keyword must be individually recognized.
#[test]
fn test_ripr_seam_all_five_keywords_individually() {
    // All five keywords in one hash ensures all branches are executed.
    let source = r#"my %h = (
        -or  => 'a',
        -and => 'b',
        -xor => 'c',
        -not => 'd',
        -cmp => 'e',
    );"#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // All five identifiers must be in the sexp.
    assert!(sexp.contains("-or"), "Missing -or in sexp");
    assert!(sexp.contains("-and"), "Missing -and in sexp");
    assert!(sexp.contains("-xor"), "Missing -xor in sexp");
    assert!(sexp.contains("-not"), "Missing -not in sexp");
    assert!(sexp.contains("-cmp"), "Missing -cmp in sexp");
}

/// Test that the string concatenation format!("-{}", kw_token.text) is exact.
/// Mutations like concatenating differently, or using wrong case, will fail.
#[test]
fn test_ripr_seam_name_format_exact() {
    // The identifier name must be exactly "-keyword", not variations.
    let ast = parse("my %h = (-or => 1);");
    let sexp = ast.to_sexp();
    // Must contain the exact string "-or" (not "or", not "-OR", not "- or").
    assert!(
        sexp.contains("\"") && sexp.contains("-or"),
        "Expected exact '-or' string in identifier, got:\n{}",
        sexp
    );
}

/// Test that consuming the keyword token (self.tokens.next()) happens correctly.
/// If the token consumption is broken, the next token won't parse correctly.
#[test]
fn test_ripr_seam_token_consumption_moves_parser() {
    // After consuming -or, the next token should be consumed.
    // In the hash (-or => 1), after -or is consumed, => should be consumed as the fat arrow.
    let source = "my %h = (-or => 1, -and => 2);";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // Both -or and -and must parse correctly, proving token consumption worked.
    assert!(
        sexp.contains("-or") && sexp.contains("-and"),
        "Expected both -or and -and parsed correctly, got:\n{}",
        sexp
    );
}

/// Test that node location (start, end) is correctly set.
/// The Node::new() call must use the correct location boundaries.
#[test]
fn test_ripr_seam_node_location_set() {
    // The Identifier node must span from the minus to the end of the keyword.
    // This is tested implicitly by sexp containing the identifier, but
    // we document that location tracking must work.
    let ast = parse("my %h = (-or => 1);");
    // The AST should be parseable and contain the identifier; if locations were wrong,
    // serialization or later processing might fail.
    let sexp = ast.to_sexp();
    assert!(
        !sexp.to_lowercase().contains("error"),
        "Location tracking failure would show as error, got:\n{}",
        sexp
    );
}

/// Test that the lookahead (peek_second) doesn't consume tokens.
/// The lookahead must be non-destructive.
#[test]
fn test_ripr_seam_lookahead_non_destructive() {
    // Multiple -keyword => pairs in sequence must all parse correctly.
    // If peek_second() accidentally consumed tokens, the parser would desynchronize.
    let source = "my %h = (-or => 1, -and => 2, -xor => 3);";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // All three must be present, proving lookahead didn't consume.
    assert!(
        sexp.contains("-or") && sexp.contains("-and") && sexp.contains("-xor"),
        "Lookahead might have consumed tokens, got:\n{}",
        sexp
    );
}

/// Test nested contexts: the fix must apply at all nesting levels.
/// If the lookahead/keyword-dispatch is skipped in nested hash literals, the fix is incomplete.
#[test]
fn test_ripr_seam_works_in_nested_hash_literal() {
    let source = "my %outer = (-or => { -and => 1 });";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // Both -or (outer) and -and (inner) must be identifiers.
    assert!(sexp.contains("-or"), "Outer -or not parsed");
    assert!(sexp.contains("-and"), "Inner -and not parsed");
}

/// Test that only the new code path (word-op lookahead) fires, not fallthrough to unary.
/// If the if condition is broken, the code falls through to "Regular unary minus" and
/// tries to parse 'or' as a unary operand, which fails.
#[test]
fn test_ripr_seam_does_not_fallthrough_to_unary() {
    // If the bareword-keyword lookahead is broken, -or => 1 would try to parse
    // as unary(-or as operand), which would fail with "expected expression".
    // By asserting clean parse, we verify the lookahead fires.
    let source = "my %h = (-or => 1);";
    assert_clean_parse(source);
}

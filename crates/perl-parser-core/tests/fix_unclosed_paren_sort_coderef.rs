//! Tests for issue #2750 Pattern C: `sort $coderef LIST` inside parenthesized expressions.
//!
//! Root cause: The postfix parser handles `sort BAREWORD LIST` when the comparator is a
//! lowercase identifier but NOT when the comparator is a scalar variable (`$cmp`). Inside
//! a paren expression, `(sort $cmp @list)` parses `sort`, then `$cmp` as the first list
//! element. When the next token is `(keys %h)` or `@arr`, the parser sees `(` as unexpected
//! (expecting `,` or `)`) and reports an unclosed_paren error.
//!
//! Fix: In the postfix/expression-context sort handler, add a branch for `sort $scalar LIST`
//! (where `$scalar` is a coderef) that mirrors the existing `sort FUNCNAME LIST` branch.
//!
//! Affected corpus files: `JSON/PP.pm`, `JSON/backportPP.pm`

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---- Failing cases: sort $coderef in paren context ----

#[test]
fn test_sort_coderef_array_in_paren() {
    // Primary reproducer: sort $cmp @arr in paren context
    assert_clean_parse(r#"my @s = (sort $cmp @arr);"#);
}

#[test]
fn test_sort_coderef_keys_in_paren() {
    // The exact JSON/PP.pm pattern: sort $keysort (keys %{...})
    assert_clean_parse(r#"my @s = (sort $cmp (keys %h));"#);
}

#[test]
fn test_sort_coderef_ternary_pattern() {
    // The real JSON/PP.pm pattern: ternary with sort $coderef
    assert_clean_parse(r#"defined $keysort ? (sort $keysort (keys %{$_[0]})) : keys %{$_[0]};"#);
}

#[test]
fn test_sort_coderef_with_grep_filter() {
    // sort $cmp with a grep-filtered list
    assert_clean_parse(r#"my @x = (sort $cmp grep { /foo/ } @list);"#);
}

#[test]
fn test_sort_coderef_hash_in_paren() {
    // sort $cmp with a hash (in list context)
    assert_clean_parse(r#"my @s = (sort $cmp %hash);"#);
}

// ---- Regression: existing sort patterns must still work ----

#[test]
fn test_sort_coderef_at_statement_level_regression() {
    // Statement-level sort $cmp @arr — already worked, must still work
    assert_clean_parse(r#"my @s = sort $cmp @arr;"#);
}

#[test]
fn test_sort_block_comparator_regression() {
    // sort { ... } — block comparator in paren context must not regress
    assert_clean_parse(r#"my @s = (sort { $a cmp $b } @arr);"#);
}

#[test]
fn test_sort_named_comparator_regression() {
    // sort by_name @list — named comparator must not regress
    assert_clean_parse(r#"my @s = (sort by_name @arr);"#);
}

#[test]
fn test_sort_no_comparator_regression() {
    // sort @arr — no comparator, must not regress
    assert_clean_parse(r#"my @s = (sort @arr);"#);
}

#[test]
fn test_sort_block_in_func_arg_regression() {
    // sort { ... } in function argument — must not regress
    assert_clean_parse(r#"foo(sort { $a <=> $b } @arr);"#);
}

// ---- Edge cases: less-common but reachable patterns ----

#[test]
fn test_sort_coderef_empty_list() {
    // (sort $cmp) with no list — should parse gracefully without unclosed_paren
    assert_clean_parse(r#"my @s = (sort $cmp);"#);
}

#[test]
fn test_sort_coderef_comma_separated_list() {
    // sort $cmp with an explicit comma-separated list
    assert_clean_parse(r#"my @s = (sort $cmp $a, $b, $c);"#);
}

#[test]
fn test_sort_coderef_map_filtered_list() {
    // sort $cmp with a map-generated list
    assert_clean_parse(r#"my @s = (sort $cmp map { lc } @arr);"#);
}

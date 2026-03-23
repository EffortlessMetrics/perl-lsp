/// Tests for `wantarray ? sort grep { BLOCK } LIST : LIST` ternary patterns.
/// ExtUtils/Installed.pm uses this idiom — the combination of sort+grep with
/// block syntax inside a ternary operator arm causes parser failures.
mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Simple wantarray ternary (should already work — regression guard).
#[test]
fn test_wantarray_simple_ternary() {
    assert_clean_parse(r#"return wantarray ? (1, 2) : 1;"#);
}

/// wantarray with sort in true branch.
#[test]
fn test_wantarray_sort_branch() {
    assert_clean_parse(r#"return wantarray ? sort @list : scalar @list;"#);
}

/// wantarray ? sort grep { BLOCK } LIST : LIST
/// This is the ExtUtils::Installed.pm pattern.
#[test]
fn test_wantarray_sort_grep_block() {
    assert_clean_parse(
        r#"return wantarray ? sort grep { not /^:private:$/ } keys %$self : grep { not /^:private:$/ } keys %$self;"#,
    );
}

/// Shorter form: sort grep { BLOCK } LIST in ternary.
#[test]
fn test_sort_grep_block_in_ternary() {
    assert_clean_parse(r#"return wantarray ? sort grep { /foo/ } @list : @list;"#);
}

/// grep with regex in true branch of ternary.
#[test]
fn test_grep_regex_in_ternary_branch() {
    assert_clean_parse(r#"my @x = $cond ? grep { /pat/ } @arr : @arr;"#);
}

/// sort grep pattern without wantarray (standalone expression).
#[test]
fn test_sort_grep_block_standalone() {
    assert_clean_parse(r#"my @x = sort grep { /foo/ } @list;"#);
}

/// sort map should not be treated as a named comparator either.
/// `sort map { ... } @list` = sort the result of map, not `sort map_func @list`.
#[test]
fn test_sort_map_not_comparator() {
    assert_clean_parse(r#"my @x = sort map { uc($_) } @list;"#);
}

/// sort sort (legal Perl): sort the result of an inner sort.
#[test]
fn test_sort_sort_not_comparator() {
    assert_clean_parse(r#"my @x = sort sort @list;"#);
}

/// Custom comparator names that are NOT block-list functions must still work.
/// `sort by_name @list` should use `by_name` as a comparator, not a sub-expression.
#[test]
fn test_sort_custom_comparator_still_works() {
    assert_clean_parse(r#"my @sorted = sort by_name @list;"#);
}

/// sort with a block comparator: sort { $a cmp $b } @list.
#[test]
fn test_sort_block_comparator_still_works() {
    assert_clean_parse(r#"my @sorted = sort { $a cmp $b } @list;"#);
}

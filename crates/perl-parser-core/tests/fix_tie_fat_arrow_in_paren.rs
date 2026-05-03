// Regression coverage for the `tie EXPR` builtin used inside an expression
// context (parentheses) with `=>` as the separator instead of `,`.
//
// Perl treats `=>` as a synonym for `,`, so all of these forms are valid:
//   tie %h, 'X', $arg;
//   tie %h => 'X', $arg;
//   my $rc = (tie %h => 'X', $arg);
//
// Before the fix, the expression-context `tie` parser only accepted a comma
// between the variable and the package name, which caused parses of the
// real-world idiom `(tie %cache => $module, @opts)` (Memoize/Expire.pm) to
// drive the parser into an unbounded recovery loop and OOM.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_tie_fat_arrow_in_parens() {
    assert_clean_parse(r#"(tie %h => $m, $n);"#);
}

#[test]
fn test_tie_fat_arrow_inside_my_assignment() {
    // From Memoize/Expire.pm:32 — was OOMing the parser.
    assert_clean_parse(r#"my $rc = (tie %cache => $module, @opts);"#);
}

#[test]
fn test_tie_fat_arrow_single_pair_in_parens() {
    assert_clean_parse(r#"(tie %h => $m);"#);
}

#[test]
fn test_tie_fat_arrow_with_my_in_parens() {
    assert_clean_parse(r#"(tie my %h => 'Pkg', $arg);"#);
}

#[test]
fn test_tie_comma_in_parens_still_works() {
    assert_clean_parse(r#"my $rc = (tie %h, $m, @opts);"#);
}

#[test]
fn test_tie_mixed_comma_and_fat_arrow_in_parens() {
    assert_clean_parse(r#"(tie %h => 'Pkg', LIFETIME => 60, NUM_USES => 10);"#);
}

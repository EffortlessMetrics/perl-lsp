mod cpan_test_helpers;
use cpan_test_helpers::*;

// fix for issue #2187: if/unless accept paren-less conditions.
// Primary acceptance criterion: test_grep_block_file_test in block_list_in_parens_tests.rs.

#[test]
fn if_grep_block_no_parens() {
    // Basic no-paren if with grep block — primary corpus pattern
    assert_clean_parse(r#"if grep {-r $_} @files { print "found"; }"#);
}

#[test]
fn if_grep_block_method_call_no_parens() {
    // Exact Catmandu::Env pattern that triggered the corpus failure
    assert_clean_parse(r#"if grep {-r File::Spec->catfile($path, $_)} @files { }"#);
}

#[test]
fn unless_grep_block_no_parens() {
    // unless should get the same fix
    assert_clean_parse(r#"unless grep { defined $_ } @list { die "empty"; }"#);
}

#[test]
fn if_with_parens_still_works() {
    // Regression: paren form must still parse correctly
    assert_clean_parse(r#"if (grep { $_ eq $target } @items) { }"#);
}

#[test]
fn if_simple_scalar_no_parens() {
    // Simple scalar condition without parens
    assert_clean_parse(r#"if $ok { print "yes"; }"#);
}

#[test]
fn if_any_block_no_parens() {
    // List::Util's any() — also is_block_list_func
    assert_clean_parse(r#"if any { $_ > 0 } @values { return 1; }"#);
}

#[test]
fn if_no_parens_with_else() {
    // else branch must still parse after no-paren condition
    assert_clean_parse(r#"if grep { /pattern/ } @lines { found(); } else { not_found(); }"#);
}

#[test]
fn nested_if_grep_no_parens() {
    // Nested no-paren ifs — stop_before_bare_brace must reset correctly
    assert_clean_parse(r#"if grep { $_ } @a { if grep { $_ } @b { } }"#);
}

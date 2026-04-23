mod cpan_test_helpers;
use cpan_test_helpers::*;

// Pattern A from #2149: C-style foreach loop
//
// In Perl, `for` and `foreach` are fully interchangeable.
// `foreach (my $i=0; $i<10; $i++) { ... }` is valid C-style syntax.
// The parser's parse_foreach_statement() only handled list-style foreach,
// not C-style. This fix delegates to C-style for loop parsing when
// semicolons are detected inside the parens.

#[test]
fn test_foreach_c_style_basic() {
    // Basic C-style foreach loop
    assert_clean_parse(r#"foreach (my $i=0; $i<10; $i++) { print $i; }"#);
}

#[test]
fn test_foreach_c_style_array_length() {
    // C-style foreach with array-length condition
    assert_clean_parse(r#"foreach (my $i=0; $i<@arr; $i+=2) { push @out, $arr[$i]; }"#);
}

#[test]
fn test_foreach_c_style_decrement() {
    // C-style foreach decrementing
    assert_clean_parse(r#"foreach (my $i = $n-1; $i >= 0; $i--) { next; }"#);
}

#[test]
fn test_foreach_c_style_complex_condition() {
    // C-style foreach with complex condition (from CPAN: Biber/Entry/Names.pm)
    assert_clean_parse(r#"foreach (my $x=0; $x <= $#choices; $x++) { push @ret, $choices[$x]; }"#);
}

#[test]
fn test_foreach_c_style_expression_init() {
    // C-style foreach where init is a plain expression (no my)
    assert_clean_parse(r#"foreach ($i = 0; $i < 10; $i++) { print $i; }"#);
}

#[test]
fn test_foreach_c_style_empty_parts() {
    // C-style foreach with empty init
    assert_clean_parse(r#"foreach (; $i < 10; $i++) { print $i; }"#);
}

#[test]
fn test_foreach_c_style_all_empty() {
    // C-style foreach with all parts empty (infinite loop)
    assert_clean_parse(r#"foreach (;;) { last; }"#);
}

#[test]
fn test_foreach_c_style_nested() {
    // Nested C-style foreach loops
    assert_clean_parse(
        r#"foreach (my $i = 0; $i < 10; $i++) {
        foreach (my $j = 0; $j < 10; $j++) {
            print "$i $j\n";
        }
    }"#,
    );
}

// Regression tests — list-style foreach must still work

#[test]
fn test_foreach_list_style_regression() {
    // Standard list-style foreach
    assert_clean_parse(r#"foreach my $x (@list) { print $x; }"#);
}

#[test]
fn test_foreach_list_style_bare_var_regression() {
    // Bare variable in list-style foreach
    assert_clean_parse(r#"foreach $item (@items) { print $item; }"#);
}

#[test]
fn test_foreach_list_style_implicit_topic_regression() {
    // Implicit $_ topic variable
    assert_clean_parse(r#"foreach (@list) { print; }"#);
}

#[test]
fn test_for_c_style_still_works_regression() {
    // C-style `for` must still work (not just `foreach`)
    assert_clean_parse(r#"for (my $i=0; $i<10; $i++) { print $i; }"#);
}

#[test]
fn test_for_list_style_still_works_regression() {
    // List-style `for` must still work
    assert_clean_parse(r#"for my $x (@list) { print $x; }"#);
}

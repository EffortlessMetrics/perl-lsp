mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for fat-arrow pairs as ternary branch expressions — issue #2402
// Root cause: parse_ternary uses parse_assignment for the then-branch,
// but parse_assignment returns only the bare identifier (e.g. `a`) before `=>`,
// leaving `=>` as the next token, which fails the expect(Colon) call.
// Fix: collect_fat_arrow_ternary_branch helper called after parse_assignment.

// Primary failing pattern: bare fat-arrow pair as ternary branch
#[test]
fn ternary_fat_arrow_single_pair_both_branches() {
    assert_clean_parse(r#"my $x = $cond ? a => 1 : b => 2;"#);
}

#[test]
fn ternary_fat_arrow_true_branch_only() {
    assert_clean_parse(r#"my $x = $cond ? a => 1 : 42;"#);
}

#[test]
fn ternary_fat_arrow_false_branch_only() {
    // false branch is an else-branch parse, same code path
    assert_clean_parse(r#"my $x = $cond ? 42 : b => 2;"#);
}

#[test]
fn ternary_fat_arrow_multi_pair_then_branch() {
    // multiple key=>value pairs as then-branch inside parens
    assert_clean_parse(r#"my $x = $cond ? (a => 1, c => 2) : b => 2;"#);
}

#[test]
fn ternary_fat_arrow_in_function_call_args() {
    assert_clean_parse(r#"foo($cond ? a => 1 : b => 2);"#);
}

#[test]
fn ternary_fat_arrow_as_return_value() {
    assert_clean_parse(r#"sub test { return $cond ? a => 1 : b => 2; }"#);
}

#[test]
fn nested_ternary_with_fat_arrow_branches() {
    assert_clean_parse(r#"my $r = $a ? $b ? c => 1 : d => 2 : e => 3;"#);
}

#[test]
fn ternary_fat_arrow_inside_hash_constructor() {
    // Common Catalyst/Moose pattern
    assert_clean_parse(r#"my %h = (key => $cond ? a => 1 : b => 2);"#);
}

#[test]
fn ternary_fat_arrow_inside_method_call_no_parens() {
    assert_clean_parse(r#"$self->render($cond ? status => 200 : status => 404);"#);
}

// Regression: paren forms still work (from fix_nested_ternary_2393.rs)
#[test]
fn ternary_fat_arrow_paren_form_unchanged() {
    assert_clean_parse(r#"my $x = $cond ? (a => 1) : (b => 2);"#);
}

// Regression: simple ternary without fat arrow still works
#[test]
fn ternary_without_fat_arrow_unchanged() {
    assert_clean_parse(r#"my $x = $cond ? 1 : 2;"#);
}

// Regression: chained ternary without fat arrow still works
#[test]
fn chained_ternary_no_fat_arrow_unchanged() {
    assert_clean_parse(r#"my $x = $a ? 1 : $b ? 2 : 3;"#);
}

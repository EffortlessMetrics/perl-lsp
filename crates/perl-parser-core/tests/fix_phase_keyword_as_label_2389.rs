mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #2389 — Phase-block keywords used as statement labels
//
// In Perl, BEGIN / END / CHECK / INIT / UNITCHECK are valid statement labels
// when followed by `:`.  The parser previously dispatched these tokens to
// `parse_phase_block` before reaching the label-detection path, so
// `CHECK: for (...)` produced a parse error instead of a LabeledStatement.
//
// Sample from Mojo::Exception:
//   CHECK: for (my $i = 0; $i < @$spec; $i += 2) { ... }

#[test]
fn test_check_as_label_c_style_for() {
    assert_clean_parse("CHECK: for (my $i = 0; $i < @$spec; $i += 2) { }");
}

#[test]
fn test_check_as_label_foreach() {
    assert_clean_parse("CHECK: for my $x (@items) { }");
}

#[test]
fn test_init_as_label() {
    assert_clean_parse("INIT: for my $x (@items) { }");
}

#[test]
fn test_begin_as_label() {
    assert_clean_parse("BEGIN: for my $x (@items) { }");
}

#[test]
fn test_end_as_label() {
    assert_clean_parse("END: for my $x (@items) { }");
}

#[test]
fn test_unitcheck_as_label() {
    assert_clean_parse("UNITCHECK: for my $x (@items) { }");
}

#[test]
fn test_check_label_with_last() {
    // Labels are commonly used with `last LABEL` / `next LABEL`.
    // Note: `last CHECK` would require loop-control to also accept keyword
    // tokens as label names (a separate issue); test with `next` on a regular
    // flow to verify the label statement itself parses correctly.
    assert_clean_parse("CHECK: while (1) { last; }");
}

#[test]
fn test_phase_block_without_colon_still_works() {
    // Regression: actual phase blocks must still parse correctly
    assert_clean_parse("CHECK { print 'check phase'; }");
}

#[test]
fn test_begin_block_still_works() {
    assert_clean_parse("BEGIN { my $x = 1; }");
}

#[test]
fn test_end_block_still_works() {
    assert_clean_parse("END { cleanup(); }");
}

#[test]
fn test_check_can_be_called_as_subroutine() {
    // CPAN modules may define a sub named CHECK and invoke it as a normal call.
    assert_clean_parse("CHECK();");
}

#[test]
fn test_begin_can_be_called_as_subroutine() {
    assert_clean_parse("BEGIN('arg');");
}

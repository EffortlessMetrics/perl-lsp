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
    assert_clean_parse("CHECK: while (1) { last; }");
}

#[test]
fn test_check_label_targeted_by_last_keyword_label() {
    // Loop control can target the CHECK label as a bareword.
    assert_clean_parse("CHECK: while (1) { last CHECK; }");
}

#[test]
fn test_check_label_targeted_by_next_keyword_label() {
    // `next LABEL` must also resolve phase-keyword labels — same parse_loop_control path.
    assert_clean_parse("CHECK: while (1) { next CHECK; }");
}

#[test]
fn test_check_label_targeted_by_redo_keyword_label() {
    // `redo LABEL` must also resolve phase-keyword labels — same parse_loop_control path.
    assert_clean_parse("CHECK: while (1) { redo CHECK; }");
}

#[test]
fn test_begin_label_targeted_by_last() {
    // BEGIN is a valid label — exercise the full set to avoid regressions.
    assert_clean_parse("BEGIN: while (1) { last BEGIN; }");
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

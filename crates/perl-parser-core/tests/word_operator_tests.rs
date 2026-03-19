//! Tests for Perl word operators in expression contexts.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ─── Low-precedence logical operators ───────────────────────────────────────

#[test]
fn test_word_op_and() {
    assert_clean_parse("$a and $b;");
}

#[test]
fn test_word_op_or() {
    assert_clean_parse("$a or $b;");
}

#[test]
fn test_word_op_not() {
    assert_clean_parse("not $x;");
}

#[test]
fn test_word_op_xor() {
    assert_clean_parse("$a xor $b;");
}

// ─── String comparison operators ────────────────────────────────────────────

#[test]
fn test_word_op_eq() {
    assert_clean_parse("$a eq $b;");
}

#[test]
fn test_word_op_ne() {
    assert_clean_parse("$a ne $b;");
}

#[test]
fn test_word_op_lt() {
    assert_clean_parse("$a lt $b;");
}

#[test]
fn test_word_op_gt() {
    assert_clean_parse("$a gt $b;");
}

#[test]
fn test_word_op_le() {
    assert_clean_parse("$a le $b;");
}

#[test]
fn test_word_op_ge() {
    assert_clean_parse("$a ge $b;");
}

#[test]
fn test_word_op_cmp() {
    assert_clean_parse("$a cmp $b;");
}

// ─── Repetition operator ───────────────────────────────────────────────────

#[test]
fn test_word_op_x_string_repetition() {
    assert_clean_parse("'x' x 5;");
}

#[test]
fn test_word_op_x_array_repetition() {
    assert_clean_parse("@list x 3;");
}

// ─── Complex expressions combining word operators ──────────────────────────

#[test]
fn test_word_op_return_with_eq_and() {
    assert_clean_parse("return 1 if $a eq 'test' and $b ne 'other';");
}

#[test]
fn test_word_op_ternary_with_gt() {
    assert_clean_parse("my $result = $x gt $y ? 'greater' : 'less';");
}

#[test]
fn test_word_op_print_with_not() {
    assert_clean_parse("print 'yes' if not $disabled;");
}

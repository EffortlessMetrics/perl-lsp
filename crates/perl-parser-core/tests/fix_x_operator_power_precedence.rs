mod cpan_test_helpers;
use cpan_test_helpers::*;

// === x operator with ** on RHS ===

#[test]
fn x_operator_power_rhs_variable() {
    // ("x") x $n**2 should parse as ("x") x ($n**2)
    assert_clean_parse(r#"my @a = ("x") x $n**2;"#);
}

#[test]
fn x_operator_power_rhs_literal() {
    // "a" x 2**3 should parse as "a" x (2**3) = "a" x 8
    assert_clean_parse(r#"my $s = "a" x 2**3;"#);
}

#[test]
fn x_operator_power_rhs_complex() {
    // $s x ($a**$b) — variable power expression
    assert_clean_parse(r#"$format .= "0" x 2**$bits;"#);
}

// === Regression guards ===

#[test]
fn x_operator_simple_still_works() {
    // Basic x operator must still work
    assert_clean_parse(r#"my $line = "-" x 80;"#);
}

#[test]
fn x_operator_additive_rhs() {
    // x with additive expression on RHS
    assert_clean_parse(r#"my $s = "ab" x $count;"#);
}

//! Tests for issue #2730: fix unexpected_eq_expr — optional-arg unary builtins
//! followed by binary operators.
//!
//! `length`, `ref`, `pos`, `defined`, and similar optional-arg builtins can operate
//! on `$_` implicitly (0-arg form).  When immediately followed by a binary operator
//! such as `==`, the parser was falling into the "parse arguments" branch and trying
//! to parse `==` as an argument expression — which fails with
//! "expected expression, found '=='".
//!
//! Fix: extend `is_nullary_without_args` in `postfix.rs` to also cover
//! `is_optional_arg_unary_builtin`, mirroring the existing `is_str_op_terminated`
//! guard that handles `ref eq 'CODE'`.
//!
//! Primary CPAN evidence: Dpkg::Conf, Dpkg::Source::BinaryFiles,
//! Dpkg::Control::HashCore, Dpkg::Shlibs::Objdump::Object, Date::Calc::PP.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── Primary CPAN pattern (Dpkg corpus) ────────────────────────────────

#[test]
fn test_length_eq_zero_statement_modifier() {
    // Dpkg::Conf, Dpkg::Source::BinaryFiles, Dpkg::Shlibs::Objdump::Object
    assert_clean_parse(r#"next if /^#/ or length == 0;"#);
}

#[test]
fn test_length_eq_zero_plain_modifier() {
    assert_clean_parse(r#"next if length == 0;"#);
}

#[test]
fn test_length_eq_zero_in_if() {
    // Dpkg::Control::HashCore: next if length == 0 and $paraborder;
    assert_clean_parse(r#"next if length == 0 and $paraborder;"#);
}

#[test]
fn test_length_eq_zero_in_elsif() {
    // Dpkg::Control::HashCore: } elsif (length == 0 ||
    assert_clean_parse(r#"if ($x) { 1; } elsif (length == 0 || /^\.+$/) { 2; }"#);
}

// ── Operator variants ─────────────────────────────────────────────────

#[test]
fn test_length_ne_zero() {
    assert_clean_parse(r#"die if length != 0;"#);
}

#[test]
fn test_length_gt_zero() {
    assert_clean_parse(r#"die if length > 0;"#);
}

#[test]
fn test_length_in_assignment() {
    assert_clean_parse(r#"my $x = length == 0;"#);
}

#[test]
fn test_length_in_ternary() {
    assert_clean_parse(r#"my $x = length == 0 ? "empty" : $str;"#);
}

// ── Other optional-arg builtins ───────────────────────────────────────

#[test]
fn test_ref_eq_zero() {
    // ref on $_ compared numerically (unusual but valid Perl)
    assert_clean_parse(r#"my $is_ref = ref == 0;"#);
}

#[test]
fn test_defined_eq_one() {
    assert_clean_parse(r#"die if defined == 1;"#);
}

#[test]
fn test_ord_eq_value() {
    // ord without args uses $_ — `ord == 65` means ord($_) == 65
    assert_clean_parse(r#"next if ord == 65;"#);
}

#[test]
fn test_ord_with_explicit_arg_unchanged() {
    // ord with explicit arg must still parse the arg
    assert_clean_parse(r#"my $n = ord $c;"#);
}

// ── Edge cases: operator variety and qualification ────────────────────

#[test]
fn test_core_qualified_length_eq_zero() {
    // CORE::length strips to "length" via core_qualified_builtin_name
    assert_clean_parse(r#"next if CORE::length == 0;"#);
}

#[test]
fn test_length_ternary() {
    // Question token is in is_binary_operator; length() used as ternary condition
    assert_clean_parse(r#"my $x = length ? "has" : "empty";"#);
}

#[test]
fn test_abs_dot_concat() {
    // Dot (concatenation) is in is_binary_operator; abs($_) . "px"
    assert_clean_parse(r#"my $s = abs . "px";"#);
}

#[test]
fn test_chained_optional_arg_builtins() {
    // Two optional-arg builtins both implicitly 0-arg in same expression
    assert_clean_parse(r#"die if length == 0 || defined == 1;"#);
}

// ── Must NOT regress ──────────────────────────────────────────────────

#[test]
fn test_length_with_explicit_arg_unchanged() {
    // Explicit arg form must still work
    assert_clean_parse(r#"if (length($str) == 0) { 1; }"#);
}

#[test]
fn test_length_with_bare_arg_unchanged() {
    // length $str still passes $str as arg (next token is sigil, not binary op)
    assert_clean_parse(r#"my $n = length $str;"#);
}

#[test]
fn test_ref_with_arg_unchanged() {
    assert_clean_parse(r#"if (ref $obj eq "ARRAY") { 1; }"#);
}

#[test]
fn test_defined_with_arg_unchanged() {
    assert_clean_parse(r#"die unless defined $x;"#);
}

//! Tests for issue #2622: defined/ref in blocks followed by word operators.
//!
//! When `defined` or `ref` appears at statement level (e.g. inside grep/map/sort
//! blocks) followed by a word operator (`and`, `or`, `xor`), the parser was
//! requiring an argument because `allow_no_args=false`. Fix: pass `true` so the
//! existing word-operator guard in `parse_named_unary_statement_call` can fire.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === defined + word operator in blocks ===

#[test]
fn test_defined_and_length_in_grep() {
    assert_clean_parse(r#"grep { defined and length } @list;"#);
}

#[test]
fn test_defined_or_next_in_map() {
    assert_clean_parse(r#"map { defined or next } @items;"#);
}

// === real-world CPAN case ===

#[test]
fn test_locale_maketext_real_case() {
    assert_clean_parse(
        r#"my $pkg = join('::', grep { defined and length } $args{Class}, $args{Subclass});"#,
    );
}

// === standalone (non-block) cases ===

#[test]
fn test_defined_and_at_statement_level() {
    assert_clean_parse(r#"defined and length;"#);
}

// === chained word operators ===

#[test]
fn test_defined_and_length_and_defined() {
    assert_clean_parse(r#"grep { defined and length and defined } @list;"#);
}

// === ref + word operator ===

#[test]
fn test_ref_and_in_grep() {
    // no-arg ref followed by word operator
    assert_clean_parse(r#"grep { ref and something } @list;"#);
}

// === defined not $x — WordNot is NOT a binary op, so defined takes it as arg ===

#[test]
fn test_defined_not_in_grep() {
    // `not` is NOT in is_binary_operator, so defined takes "not $x" as argument
    assert_clean_parse(r#"grep { defined not $x } @list;"#);
}

// === regression guards: defined/ref WITH explicit argument must still work ===

#[test]
fn test_defined_with_argument_still_works() {
    assert_clean_parse(r#"grep { defined $_ } @list;"#);
}

#[test]
fn test_ref_with_argument_still_works() {
    assert_clean_parse(r#"map { ref $_ eq 'ARRAY' } @items;"#);
}

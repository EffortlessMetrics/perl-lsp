//! Tests for issue #2622: defined/ref in blocks followed by word operators.
//!
//! When `defined` or `ref` appears at statement level (e.g. inside grep/map/sort
//! blocks) followed by a word operator (`and`, `or`, `xor`), the parser was
//! requiring an argument because `allow_no_args=false`. Fix: pass `true` so the
//! existing word-operator guard in `parse_named_unary_statement_call` can fire.
//!
//! The fix is applied ONLY when there are no parentheses (line 779 of statements.rs:
//! `if self.peek_kind() != Some(TokenKind::LeftParen)`). Thus `defined($x)` and
//! `ref($obj)` with explicit parens are unaffected — they always fall through to
//! normal argument parsing. The change only affects the no-paren no-arg path.

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

// === Additional regression guards: parenthesized versions ===

#[test]
fn test_defined_with_parens_and_arg() {
    // Parens should always work—they delimit, so `allow_no_args=false` doesn't matter
    assert_clean_parse(r#"my $x = defined($y);"#);
}

#[test]
fn test_ref_with_parens_and_arg() {
    // Parens should always work
    assert_clean_parse(r#"my $r = ref($obj);"#);
}

#[test]
fn test_defined_no_parens_with_arg_after_if() {
    // No parens, but has explicit argument
    assert_clean_parse(r#"if (defined $x) { }"#);
}

#[test]
fn test_ref_cmp_eq_regression() {
    // With parens and string comparison operator
    assert_clean_parse(r#"if (ref($x) eq 'HASH') { }"#);
}

// === Stress tests for specific edge cases ===

#[test]
fn test_defined_hash_subscript() {
    // defined $hash{key} — subscript after defined, no word op.
    // With allow_no_args=true, omit_optional_arg fires only for binary ops;
    // Dollar sigil is not a binary op, so defined still takes $hash{key} as arg.
    assert_clean_parse(r#"if (defined $hash{key}) { }"#);
}

#[test]
fn test_defined_hash_subscript_no_parens() {
    // Same but at statement level without outer parens
    assert_clean_parse(r#"defined $hash{key} or die;"#);
}

#[test]
fn test_ref_backslash_ref() {
    // ref \$x — backslash is not a binary op, so ref takes \$x as argument.
    assert_clean_parse(r#"my $r = ref \$x;"#);
}

#[test]
fn test_defined_or_die() {
    // defined or die — zero-arg defined with word or
    assert_clean_parse(r#"defined or die "undef";"#);
}

#[test]
fn test_defined_paren_arg_then_and() {
    // defined($x) and — parens path, unaffected by this change
    assert_clean_parse(r#"defined($x) and length($x);"#);
}

// === Operator variety: symbolic || and // at STATEMENT level (not RHS of assignment) ===
//
// Note: `defined` without args inside an assignment RHS (e.g. `my $x = defined || ...`)
// is a pre-existing parser limitation — `defined` in expression context goes through a
// different code path than `parse_simple_statement`. The fix here is ONLY for the
// statement-level dispatch. Tests below use statement-level forms only.

#[test]
fn test_defined_symbolic_or_statement_level() {
    // defined || fallback at statement level (not inside an assignment RHS)
    // || (TokenKind::Or) is in is_binary_operator, so omit_optional_arg fires
    assert_clean_parse(r#"defined || length;"#);
}

#[test]
fn test_defined_symbolic_and_statement_level() {
    // defined && x at statement level
    // && (TokenKind::And) is in is_binary_operator
    assert_clean_parse(r#"defined && length;"#);
}

#[test]
fn test_defined_xor_operator() {
    // defined xor something — WordXor is in is_binary_operator
    assert_clean_parse(r#"grep { defined xor something } @list;"#);
}

// === Nested: defined and ref combined ===

#[test]
fn test_nested_defined_in_grep() {
    // defined and ref $_ eq 'HASH' — combination of no-arg and with-arg
    assert_clean_parse(r#"grep { defined and ref $_ eq 'HASH' } @list;"#);
}

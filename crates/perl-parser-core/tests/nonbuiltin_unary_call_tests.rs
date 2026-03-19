mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #1943: non-builtin unary function calls without parens
// in parenthesized context (e.g., `if (blessed $self)`)
//
// The `looks_like_bare_call` heuristic in postfix.rs handles these cases
// by detecting that an unknown identifier is followed by a sigil-bearing
// argument and parsing it as a function call.

#[test]
fn test_blessed_in_if_condition() {
    let source = r#"if (blessed $self) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_in_unless_condition() {
    let source = r#"unless (blessed $ref) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_with_and_operator() {
    let source = r#"if (blessed $err and $err->isa("Foo")) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_reftype_with_eq() {
    let source = r#"if (reftype $x eq 'ARRAY') { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_in_elsif() {
    let source = r#"if (0) { 1; } elsif (blessed $element && $element->isa("Foo")) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_looks_like_number_in_condition() {
    let source = r#"if (looks_like_number $val) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_weaken_as_statement() {
    let source = r#"weaken $self;"#;
    assert_clean_parse(source);
}

#[test]
fn test_croak_with_variable() {
    let source = r#"croak $msg;"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_in_ternary() {
    let source = r#"my $x = blessed $obj ? 1 : 0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_double_ampersand() {
    let source = r#"if (blessed $self && $self->isa("Foo")) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_scalar_util_blessed_qualified() {
    let source = r#"if (Scalar::Util::blessed($self)) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_nested_paren_blessed() {
    let source = r#"my $x = (blessed $obj) ? 1 : 0;"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_or_operator() {
    let source = r#"if (blessed $self or $fallback) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_multiple_nonbuiltin_unary_in_expression() {
    let source = r#"if (blessed $a && blessed $b) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_confess_with_variable() {
    let source = r#"confess $error;"#;
    assert_clean_parse(source);
}

#[test]
fn test_carp_with_variable() {
    let source = r#"carp $warning;"#;
    assert_clean_parse(source);
}

#[test]
fn test_nonbuiltin_unary_in_while() {
    let source = r#"while (defined $line && chomp $line) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_blessed_with_deref() {
    let source = r#"if (blessed $self->{obj}) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_nonbuiltin_in_return() {
    let source = r#"return blessed $obj;"#;
    assert_clean_parse(source);
}

#[test]
fn test_nonbuiltin_in_assignment() {
    let source = r#"my $type = blessed $obj;"#;
    assert_clean_parse(source);
}

#[test]
fn test_nonbuiltin_unary_negated() {
    let source = r#"if (!blessed $obj) { 1; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_croak_with_string_literal() {
    let source = r#"croak "something went wrong";"#;
    assert_clean_parse(source);
}

#[test]
fn test_confess_with_string_in_condition() {
    let source = r#"confess "error: $msg" if $bad;"#;
    assert_clean_parse(source);
}

#[test]
fn test_nonbuiltin_unary_with_array_arg() {
    let source = r#"my @sorted = shuffle @items;"#;
    assert_clean_parse(source);
}

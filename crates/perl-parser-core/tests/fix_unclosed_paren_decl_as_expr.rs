//! Tests for issue #2750 Patterns D and E: `our`/`my` declaration as binary-expression operand.
//!
//! Pattern D root cause: `parse_declaration_arg()` returns a `VariableDeclaration` node
//! immediately when there is no `=` initializer. The paren-expression parser then passes
//! the result to `parse_word_or_expr()`, which only handles `or`/`and`/`xor`/`not`.
//! Tokens like `&&`, `||`, `=~`, `!~`, `==`, etc. are silently left unconsumed, causing
//! the outer paren to appear unclosed.
//!
//! Pattern E is the same root cause: `(our $AUTOLOAD =~ /pattern/)` — the declaration
//! is returned, `=~` is not handled, so the `qr` is treated as a new expression and the
//! `)` looks unclosed.
//!
//! Fix: In `parse_declaration_arg()`, after returning a declaration with no initializer,
//! continue into `parse_ternary()` with the declaration as the left-hand side so that
//! all binary operators at ternary-and-below precedence are properly consumed.
//!
//! Affected corpus files: `Method/Generate/Accessor.pm`, `Moo/HandleMoose/FakeMetaClass.pm`

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---- Pattern D: declaration + `&&`/`||` in condition ----

#[test]
fn test_our_and_and_in_if_condition() {
    // The primary reproducer from Method/Generate/Accessor.pm
    assert_clean_parse(r#"if (our $CAN_HAZ_XS && $self->is_simple_get()) { 1; }"#);
}

#[test]
fn test_our_or_or_in_if_condition() {
    assert_clean_parse(r#"if (our $X || $y) { 1; }"#);
}

#[test]
fn test_my_and_and_in_if_condition() {
    assert_clean_parse(r#"if (my $x && defined($y)) { 1; }"#);
}

#[test]
fn test_local_and_and_in_if_condition() {
    assert_clean_parse(r#"if (local $/ && $x) { 1; }"#);
}

#[test]
fn test_state_and_and_in_if_condition() {
    assert_clean_parse(r#"if (state $x && $y) { 1; }"#);
}

#[test]
fn test_our_and_and_chained_in_paren() {
    // Multiple && in a single paren expression
    assert_clean_parse(r#"(our $X && $y && $z);"#);
}

#[test]
fn test_our_and_and_with_method_call() {
    // Real-world pattern: `our $CAN_HAZ_XS && $self->method(...)`
    assert_clean_parse(r#"(our $CAN_HAZ_XS && $self->make_reader_subs($name));"#);
}

#[test]
fn test_my_or_or_in_paren() {
    assert_clean_parse(r#"(my $x || "default");"#);
}

// ---- Pattern E: declaration + `=~` binding operator ----

#[test]
fn test_our_regex_bind_in_paren() {
    // Primary reproducer from Moo/HandleMoose/FakeMetaClass.pm
    assert_clean_parse(r#"my ($meth) = (our $AUTOLOAD =~ /([^:]+)$/);"#);
}

#[test]
fn test_my_regex_bind_in_paren() {
    assert_clean_parse(r#"(my $copy =~ s/foo/bar/);"#);
}

#[test]
fn test_my_str_regex_capture() {
    assert_clean_parse(r#"my ($m) = (my $str =~ /(pattern)/);"#);
}

#[test]
fn test_our_regex_bind_rhs_extraction() {
    // Test that the rhs is actually captured
    assert_clean_parse(r#"my ($m) = (our $AUTOLOAD =~ /([^:]+)$/);"#);
}

// ---- Regression: existing valid patterns must still work ----

#[test]
fn test_our_without_operator_regression() {
    // `our $X` with no following operator — must still work
    assert_clean_parse(r#"if (our $X) { 1; }"#);
}

#[test]
fn test_my_list_decl_regression() {
    // `my ($x, $y) = @_` — list declaration must not be affected
    assert_clean_parse(r#"my ($x, $y) = @_;"#);
}

#[test]
fn test_for_my_loop_regression() {
    // `for my $x (@arr)` — loop variable must not be affected
    assert_clean_parse(r#"for my $x (@arr) { }"#);
}

#[test]
fn test_my_with_initializer_regression() {
    // `(our $X = 1 && $y)` — operator is inside initializer, must still work
    assert_clean_parse(r#"if (our $X = 1 && $y) { }"#);
}

#[test]
fn test_our_comma_list_regression() {
    // `(our $X, our $Y)` — comma-separated declarations must not be affected
    assert_clean_parse(r#"(our $X, our $Y);"#);
}

#[test]
fn test_my_decl_in_paren_regression() {
    // `my $x` with no operator in paren context — must still work
    assert_clean_parse(r#"my $x = (my $y);"#);
}

// ---- Edge cases: less-common operators after declaration ----

#[test]
fn test_our_equality_in_if_condition() {
    // Equality operator after declaration (== handled by parse_equality_with)
    assert_clean_parse(r#"if (our $X == 1) { 1; }"#);
}

#[test]
fn test_our_relational_in_paren() {
    // Relational operator after declaration
    assert_clean_parse(r#"(our $count > 0);"#);
}

#[test]
fn test_our_defined_or_in_paren() {
    // Defined-or operator after declaration (// is TokenKind::DefinedOr, handled by parse_or_with)
    assert_clean_parse(r#"(our $X // "default");"#);
}

#[test]
fn test_elsif_decl_with_binary_op() {
    // elsif condition with declaration + binary op (control_flow.rs elsif branch)
    assert_clean_parse(r#"if (0) { } elsif (our $X && $y) { 1; }"#);
}

#[test]
fn test_while_initializer_regression() {
    // while (my $line = <$fh>) — initializer form must not regress
    assert_clean_parse(r#"while (my $line = <$fh>) { print $line; }"#);
}

#[test]
fn test_our_ternary_after_decl() {
    // Ternary operator after declaration (handled by parse_ternary_with)
    assert_clean_parse(r#"(our $X ? "yes" : "no");"#);
}

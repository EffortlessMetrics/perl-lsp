//! Tests for issue #2396: word operators (or/and) after return, loop control,
//! indirect calls, and inside paren lists.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === return + word operator ===

#[test]
fn test_return_or_die() {
    // `return or die` means `(return) or (die)` — no return value
    assert_clean_parse("return or die;");
}

#[test]
fn test_return_and_die() {
    assert_clean_parse("return and die;");
}

#[test]
fn test_return_xor_die() {
    assert_clean_parse("return xor die;");
}

#[test]
fn test_return_value_or_die() {
    // return with a value, then or die — value parsed, or at stmt level
    assert_clean_parse(r#"return $x or die "no value";"#);
}

#[test]
fn test_return_or_in_sub() {
    assert_clean_parse("sub f { return or die; }");
}

// === loop control + word operator ===

#[test]
fn test_last_and_die() {
    // `last and die` — last terminates the loop, then die executes
    assert_clean_parse("last and die;");
}

#[test]
fn test_next_or_die() {
    assert_clean_parse("next or die;");
}

#[test]
fn test_last_or_warn() {
    assert_clean_parse(r#"last or warn "loop ended";"#);
}

#[test]
fn test_next_and_log() {
    assert_clean_parse(r#"next and print "skipped\n";"#);
}

// === indirect call (print/say/warn) + word operator ===

#[test]
fn test_print_fh_or_die() {
    // print with filehandle followed by or die
    assert_clean_parse(r#"print $fh "data" or die "write failed";"#);
}

#[test]
fn test_print_stderr_or_die() {
    assert_clean_parse(r#"print STDERR "error\n" or die "write failed";"#);
}

#[test]
fn test_say_fh_or_die() {
    assert_clean_parse(r#"say $fh "data" or die "write failed";"#);
}

#[test]
fn test_warn_or_die() {
    // warn is also indirect; it stops at word op
    assert_clean_parse(r#"warn "something" or die;"#);
}

// === word op inside paren list (hash value) ===

#[test]
fn test_hash_value_or_default() {
    // The `or` applies to the value in the pair
    assert_clean_parse(r#"my %h = (key => $val or "default");"#);
}

#[test]
fn test_hash_multiple_pairs_with_or() {
    assert_clean_parse(r#"my %h = (a => $x or 1, b => $y or 2);"#);
}

// === CPAN patterns from the issue ===

#[test]
fn test_hash_element_or_return() {
    // $hash{key} or return — already works, regression guard
    assert_clean_parse(r#"$hash{key} or return;"#);
}

#[test]
fn test_method_call_or_croak() {
    // $obj->method() or croak — already works, regression guard
    assert_clean_parse(r#"$obj->method() or croak "method failed";"#);
}

#[test]
fn test_return_value_or_croak() {
    assert_clean_parse(r#"return $self->{attr} or croak "no attr";"#);
}

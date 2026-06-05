//! Tests for issue #752 finding #2: unknown-function + builtin-call(args) + concat.
//!
//! An unknown (imported, non-builtin) function called WITHOUT parens, whose
//! argument is a builtin-call followed by a binary operator, was dropping the
//! argument binding.  Root cause: `looks_like_bare_call` in helpers.rs
//! early-returned `false` when the argument was a builtin function followed by
//! `(` instead of a sigil.
//!
//! Real-world manifestations: Catalyst::Component, DB_File, Dpkg, Debconf.
//! Examples:
//!   croak ref(shift) . " is not a valid class"
//!   croak ref($self) . "::new() is an abstract method"
//!   confess ref($x) . " cannot be used here"

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Parser;
use perl_tdd_support::must;

/// Parse source and return its sexp, asserting no ERROR nodes.
fn sexp_of(source: &str) -> String {
    assert_clean_parse(source);
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    ast.to_sexp()
}

// === Primary bug cases (ref+concat as unknown-function arg) ===

#[test]
fn test_unknown_fn_ref_shift_concat() {
    // croak ref(shift) . "x" — the original failing case.
    // The concat (binary_.) must appear INSIDE the function call argument,
    // not as a separate top-level error node.
    let sexp = sexp_of(r#"croak ref(shift) . "x";"#);
    assert!(sexp.contains("binary_."), "Expected binary_. inside croak arg, got: {sexp}");
    // The whole concat must be inside the call, not a top-level ERROR
    assert!(!sexp.contains("ERROR"), "Expected no ERROR node, got: {sexp}");
}

#[test]
fn test_unknown_fn_ref_var_concat() {
    // croak ref($x) . "y" — variable arg to ref.
    // The concat must be inside the call argument.
    let sexp = sexp_of(r#"croak ref($x) . "y";"#);
    assert!(sexp.contains("binary_."), "Expected binary_. inside croak arg, got: {sexp}");
}

#[test]
fn test_unknown_fn_ref_self_concat() {
    // croak ref($self) . "::new() is abstract" — common OO idiom
    assert_clean_parse(r#"croak ref($self) . "::new() is abstract";"#);
}

#[test]
fn test_confess_ref_var_concat() {
    // confess ref($x) . " not allowed" — different unknown function
    assert_clean_parse(r#"confess ref($x) . " not allowed";"#);
}

#[test]
fn test_carp_ref_var_concat() {
    // carp ref($x) . " warning" — another Carp variant
    assert_clean_parse(r#"carp ref($x) . " warning";"#);
}

// === ref without concat (also previously broken) ===

#[test]
fn test_unknown_fn_ref_var_no_concat() {
    // croak ref($x) — no trailing binary op; arg is just ref($x)
    assert_clean_parse(r#"croak ref($x);"#);
}

#[test]
fn test_unknown_fn_ref_shift_no_concat() {
    // croak ref(shift) — no trailing binary op
    assert_clean_parse(r#"croak ref(shift);"#);
}

// === length builtin with concat ===

#[test]
fn test_unknown_fn_length_var_concat() {
    // foo length($x) . " chars" — length is an optional-arg builtin
    assert_clean_parse(r#"foo length($x) . " chars";"#);
}

// === warn (known builtin) still works — regression guard ===

#[test]
fn test_warn_ref_var_concat_still_works() {
    // warn ref($x) . "y" — already worked, must not regress
    assert_clean_parse(r#"warn ref($x) . "y";"#);
}

// === Simple arg cases still work — regression guards ===

#[test]
fn test_croak_string_still_works() {
    // croak "msg" — worked before, must not regress
    assert_clean_parse(r#"croak "msg";"#);
}

#[test]
fn test_croak_var_still_works() {
    // croak $err — sigil arg, also worked (separate code path)
    assert_clean_parse(r#"croak $err;"#);
}

#[test]
fn test_croak_ref_paren_still_works() {
    // croak ref($x) — should now work with fix (formerly broken)
    assert_clean_parse(r#"croak ref($x);"#);
}

// === Statement separation still correct ===

#[test]
fn test_two_separate_calls_still_separate() {
    // foo(); bar(); — must remain two separate statements
    assert_clean_parse(r#"foo(); bar();"#);
}

#[test]
fn test_func_list_args_still_work() {
    // croak $x, $y — two comma-separated args, must still work
    assert_clean_parse(r#"croak $x, $y;"#);
}

// === Assignment context unaffected ===

#[test]
fn test_assign_ref_var_concat_unaffected() {
    // my $r = ref($x) . "y" — assignment context, must not regress
    assert_clean_parse(r#"my $r = ref($x) . "y";"#);
}

// === print with indirect filehandle — must not regress ===

#[test]
fn test_print_fh_string_unaffected() {
    // print $fh "x" — indirect filehandle idiom, must not regress
    assert_clean_parse(r#"print $fh "x";"#);
}

//! Tests for issue #3357: Subroutine attribute validation.
//!
//! Valid built-in Perl attributes (`:lvalue`, `:method`, `:prototype(...)`, `:const`)
//! must parse cleanly. Custom or unknown attribute names must also parse cleanly with
//! NO errors — Perl allows arbitrary attributes via `MODIFY_CODE_ATTRIBUTES` (see
//! `perldoc attributes`). Only genuine syntax errors (unterminated parens, missing
//! name) should produce diagnostics.
//!
//! Updated by #1361: the false-positive "unknown attribute" error was removed.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Valid built-in attributes — must parse cleanly with no errors
// ---------------------------------------------------------------------------

#[test]
fn valid_lvalue_no_warning() {
    let src = "sub valid :lvalue { }";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected no errors for :lvalue, got: {:?}",
        parser.errors()
    );
}

#[test]
fn valid_method_no_warning() {
    let src = "sub also_valid :method { }";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected no errors for :method, got: {:?}",
        parser.errors()
    );
}

#[test]
fn valid_prototype_no_warning() {
    // Note: `sub proto :prototype($) { }` hits a pre-existing lexer bug where
    // the lexer tokenises `$)` as the special process-group variable rather than
    // `$` + `)`.  Use `\@` instead — that is an equally valid prototype character
    // that avoids the issue.  The lexer-context bug is tracked separately.
    let src = "sub proto :prototype(\\@) { }";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected no errors for :prototype(\\@), got: {:?}",
        parser.errors()
    );
}

#[test]
fn valid_const_no_warning() {
    let src = "sub c :const { 42 }";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected no errors for :const, got: {:?}",
        parser.errors()
    );
}

#[test]
fn valid_lvalue_method_combined_no_warning() {
    let src = "sub combo :lvalue :method { }";
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected no errors for :lvalue :method, got: {:?}",
        parser.errors()
    );
}

// ---------------------------------------------------------------------------
// Custom / misspelled attributes — must parse cleanly with NO errors.
// Per #1361: Perl allows arbitrary attributes via MODIFY_CODE_ATTRIBUTES;
// emitting false-positive errors for unknown names was a parser bug.
// ---------------------------------------------------------------------------

#[test]
fn custom_lvalue_misspelled_no_error() {
    // :lvaluE is technically a custom attribute name (Perl attributes are
    // case-sensitive), not a built-in.  It should parse cleanly.
    let src = "sub invalid :lvaluE { }";
    assert_clean_parse(src);
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for custom attribute :lvaluE (#1361 fix), but got: {:?}",
        errors
    );
}

#[test]
fn custom_foobar_attr_no_error() {
    // :foobar is a valid custom attribute name — should parse cleanly.
    let src = "sub unknown :foobar { }";
    assert_clean_parse(src);
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for custom attribute :foobar (#1361 fix), but got: {:?}",
        errors
    );
}

#[test]
fn attribute_handlers_custom_attribute_is_recognized() {
    let src = r#"
use Attribute::Handlers;
sub MyAttr :ATTR(CODE) { }
sub foo :MyAttr(foo) { }
"#;
    assert_clean_parse(src);

    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "Expected Attribute::Handlers custom attribute support to avoid warnings, got: {:?}",
        parser.errors()
    );
}

#[test]
fn custom_attr_does_not_produce_error_ast_node() {
    // Parsing should succeed — custom attributes are valid in Perl (#1361)
    let src = "sub unknown :foobar { }";
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Anonymous subs with custom attributes parse cleanly (no false-positive error).
// ---------------------------------------------------------------------------

#[test]
fn anon_sub_custom_attr_no_error() {
    // :notreal is a valid custom attribute — anonymous subs may also carry them.
    let src = "my $f = sub :notreal { 1 };";
    assert_clean_parse(src);
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        errors.is_empty(),
        "Expected no errors for anonymous sub :notreal (#1361 fix), but got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Regression guard: existing clean-parse tests still pass
// ---------------------------------------------------------------------------

#[test]
fn named_sub_clean_parse_regression() {
    let src = "sub foo :lvalue { return 1; }";
    assert_clean_parse(src);
}

#[test]
fn method_lvalue_clean_parse_regression() {
    let src = "sub limit : lvalue { my $self = shift; $self->{LIMIT}; }";
    assert_clean_parse(src);
}

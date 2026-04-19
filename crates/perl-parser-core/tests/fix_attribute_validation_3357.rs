//! Tests for issue #3357: Subroutine attribute validation.
//!
//! Valid built-in Perl attributes (`:lvalue`, `:method`, `:prototype(...)`, `:const`)
//! must parse cleanly. Unknown or misspelled attributes should produce a warning
//! (pushed to `parser.errors()`) but still parse successfully — users may define
//! custom attributes via the `attributes` module.

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
// Invalid / misspelled attributes — must parse (no Error AST node) but
// must emit a warning in parser.errors()
// ---------------------------------------------------------------------------

#[test]
fn invalid_lvalue_misspelled_warns() {
    // :lvaluE is not a known attribute (case-sensitive)
    let src = "sub invalid :lvaluE { }";
    // Parses without an Error AST node
    assert_clean_parse(src);
    // But should have a warning in parser.errors()
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        !errors.is_empty(),
        "Expected a warning for unknown attribute :lvaluE, but errors was empty"
    );
    let has_attr_warning = errors
        .iter()
        .any(|e| format!("{e}").to_lowercase().contains("lvalue"));
    assert!(
        has_attr_warning,
        "Expected warning mentioning 'lvalue' for :lvaluE, got: {:?}",
        errors
    );
}

#[test]
fn unknown_foobar_attr_warns() {
    let src = "sub unknown :foobar { }";
    // Parses without an Error AST node
    assert_clean_parse(src);
    // But should have a warning
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        !errors.is_empty(),
        "Expected a warning for unknown attribute :foobar, but errors was empty"
    );
    let has_attr_warning = errors
        .iter()
        .any(|e| format!("{e}").to_lowercase().contains("foobar"));
    assert!(
        has_attr_warning,
        "Expected warning mentioning 'foobar' for :foobar, got: {:?}",
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
fn unknown_attr_does_not_produce_error_ast_node() {
    // Parsing should succeed — unknown attributes produce warnings, not hard errors
    let src = "sub unknown :foobar { }";
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Anonymous subs with unknown attributes also warn
// ---------------------------------------------------------------------------

#[test]
fn anon_sub_unknown_attr_warns() {
    let src = "my $f = sub :notreal { 1 };";
    assert_clean_parse(src);
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    let errors = parser.errors();
    assert!(
        !errors.is_empty(),
        "Expected a warning for anonymous sub :notreal, but errors was empty"
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

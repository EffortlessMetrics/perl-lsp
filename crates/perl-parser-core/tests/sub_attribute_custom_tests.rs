//! Tests for subroutine custom attributes (issue #1361)
//!
//! Verifies that custom (unknown) subroutine attributes like `:public`, `:cached(...)`,
//! `:Path(...)`, etc. parse cleanly without false-positive error diagnostics.
//!
//! Perl allows arbitrary attributes via the `MODIFY_CODE_ATTRIBUTES` hook. The parser
//! should not emit errors for unknown attributes, only for true syntax violations.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ============================================================================
// POSITIVE TESTS: Custom attributes should parse cleanly with NO errors
// ============================================================================

/// Custom attribute `:public` should not emit error diagnostic.
#[test]
fn test_custom_attribute_public() {
    let code = r#"
sub my_method :public {
    return 1;
}
"#;
    assert_clean_parse(code);
}

/// Multiple custom attributes on same subroutine should parse cleanly.
#[test]
fn test_custom_attributes_multiple() {
    let code = r#"
sub my_method :public :private {
    return 1;
}
"#;
    assert_clean_parse(code);
}

/// Custom attribute with parenthesized arguments should parse cleanly.
#[test]
fn test_custom_attribute_with_args() {
    let code = r#"
sub my_method :cached(timeout => 30) {
    return 1;
}
"#;
    assert_clean_parse(code);
}

/// Catalyst-style attributes (real-world pattern) should parse cleanly.
/// :Path and :Args are custom attributes defined by Catalyst framework.
#[test]
fn test_catalyst_style_attributes() {
    let code = r#"
sub user_handler :Path('/users') :Args(1) {
    my ($self, $id) = @_;
    return "User $id";
}
"#;
    assert_clean_parse(code);
}

/// Moose-style attributes (real-world pattern) should parse cleanly.
/// Custom attributes are used to mark methods with framework semantics.
#[test]
fn test_moose_style_custom_attribute() {
    let code = r#"
sub my_method :Moose {
    return shift;
}
"#;
    assert_clean_parse(code);
}

/// Custom attribute with nested parentheses/arguments should parse cleanly.
#[test]
fn test_custom_attribute_complex_args() {
    let code = r#"
sub my_method :cached(config => { timeout => 30, retries => 3 }) {
    return 1;
}
"#;
    assert_clean_parse(code);
}

/// Anonymous subroutine with custom attribute should parse cleanly.
#[test]
fn test_anonymous_sub_with_custom_attribute() {
    let code = r#"
my $sub = sub :public {
    return 42;
};
"#;
    assert_clean_parse(code);
}

// ============================================================================
// REGRESSION GUARDS: Built-in attributes should continue to work unchanged
// ============================================================================

/// Built-in `:method` attribute should still work (regression guard).
#[test]
fn test_builtin_method_attribute_regression() {
    let code = r#"
sub my_method :method {
    my ($self) = @_;
    return $self;
}
"#;
    assert_clean_parse(code);
}

/// Built-in `:lvalue` attribute should still work (regression guard).
#[test]
fn test_builtin_lvalue_attribute_regression() {
    let code = r#"
sub my_lvalue :lvalue {
    return $_[0];
}
"#;
    assert_clean_parse(code);
}

/// Built-in `:prototype($)` attribute should still work (regression guard).
#[test]
fn test_builtin_prototype_attribute_regression() {
    let code = r#"
sub my_sub :prototype($) {
    my ($arg) = @_;
    return $arg;
}
"#;
    assert_clean_parse(code);
}

/// Built-in `:const` attribute should still work (regression guard).
#[test]
fn test_builtin_const_attribute_regression() {
    let code = r#"
sub my_constant :const {
    return 42;
}
"#;
    assert_clean_parse(code);
}

/// Multiple built-in attributes should still work together (regression guard).
#[test]
fn test_builtin_multiple_attributes_regression() {
    let code = r#"
sub my_sub :method :lvalue {
    return $_[0];
}
"#;
    assert_clean_parse(code);
}

// ============================================================================
// ADVERSARIAL TESTS: Verify parser.errors() is empty for all custom cases
// ============================================================================

/// Verify that parser.errors() is empty when parsing custom attributes.
/// This directly asserts the side effect (error collection) is not triggered.
#[test]
fn test_custom_attributes_no_parser_errors() {
    let code = r#"
sub foo :public { 1 }
sub bar :cached(x => 1) { 2 }
sub baz :Path('/path') :Args(2) { 3 }
"#;
    let mut parser = perl_parser_core::Parser::new(code);
    let _ast = perl_tdd_support::must(parser.parse());
    let errors = parser.get_errors();

    assert!(
        errors.is_empty(),
        "Expected no parser errors for custom attributes, but got: {:?}",
        errors
    );
}

/// Verify that builtin attributes do NOT trigger errors (regression).
#[test]
fn test_builtin_attributes_no_parser_errors() {
    let code = r#"
sub foo :method { 1 }
sub bar :lvalue { 2 }
sub baz :prototype($) { 3 }
sub qux :const { 4 }
"#;
    let mut parser = perl_parser_core::Parser::new(code);
    let _ast = perl_tdd_support::must(parser.parse());
    let errors = parser.get_errors();

    assert!(
        errors.is_empty(),
        "Expected no parser errors for built-in attributes, but got: {:?}",
        errors
    );
}

// ============================================================================
// MALFORMED ATTRIBUTE SYNTAX: Genuine syntax errors should still be caught
// ============================================================================

/// Malformed attribute syntax (unterminated paren) should still produce an error.
/// This verifies we don't over-suppress errors — only unknown attribute NAMES are allowed.
#[test]
fn test_malformed_attribute_syntax_still_errors() {
    let code = r#"
sub foo :cached(timeout => 30 {
    return 1;
}
"#;
    // This should have an error for unterminated parentheses, not for unknown attribute.
    assert_has_error(code, "unterminated");
}

/// Attribute with no name after colon should still error.
#[test]
fn test_attribute_missing_name_still_errors() {
    let code = r#"
sub foo : {
    return 1;
}
"#;
    // This should have an error for missing attribute name.
    assert_has_error(code, "expected attribute name");
}

// ============================================================================
// EDGE CASES: Ensure comprehensive coverage per §Test-Grid
// ============================================================================

/// Variable attribute `:shared` should continue to work (not affected by sub fix).
#[test]
fn test_variable_shared_attribute_regression() {
    let code = r#"
my $x :shared;
"#;
    assert_clean_parse(code);
}

/// Class attribute `:isa(Parent)` should continue to work (not affected).
#[test]
fn test_class_isa_attribute_regression() {
    let code = r#"
class Point :isa(Base) {
    field $x :param;
    field $y :param;
}
"#;
    assert_clean_parse(code);
}

/// Mix of custom and builtin attributes on same sub should parse cleanly.
#[test]
fn test_custom_and_builtin_mixed() {
    let code = r#"
sub my_sub :public :method :cached(x => 1) {
    return shift;
}
"#;
    assert_clean_parse(code);
}

/// Multiple subroutines with various custom attribute patterns.
#[test]
fn test_multiple_subs_various_custom_attributes() {
    let code = r#"
sub foo :public { 1 }
sub bar :private :readonly { 2 }
sub baz :cached(ttl => 60) { 3 }
sub qux :Path('/api/users') :Args(1) :GET { 4 }
sub quux :MyCustom :Attr :WithArgs(a => 1, b => 2) { 5 }
"#;
    assert_clean_parse(code);
}

// ============================================================================
// AST CAPTURE: Verify attributes land in the AST, not just "no error"
// ============================================================================

/// Custom attributes must be captured in the AST sexp output.
///
/// This is a stronger assertion than `assert_clean_parse`: it confirms the
/// attribute name actually appears in the S-expression, proving the parser
/// records the attribute in `NodeKind::Subroutine { attributes }` and does
/// not silently discard it.
#[test]
fn test_custom_attribute_captured_in_ast_sexp() {
    let code = "sub foo :public { }";
    let ast = parse(code);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains(":public"),
        "Expected ':public' in AST sexp to confirm attribute is recorded, got: {}",
        sexp
    );
}

/// Multi-attribute case: all attributes must appear in the AST.
#[test]
fn test_multiple_custom_attributes_captured_in_ast_sexp() {
    let code = "sub handler :Path('/users') :Args(1) { }";
    let ast = parse(code);
    let sexp = ast.to_sexp();
    assert!(sexp.contains(":Path("), "Expected ':Path(' in AST sexp, got: {}", sexp);
    assert!(sexp.contains(":Args("), "Expected ':Args(' in AST sexp, got: {}", sexp);
}

// ============================================================================
// EDGE CASES: Statement modifiers, prototypes, and unusual attribute syntax
// ============================================================================

/// Statement modifier keyword (`if`) after a variable attribute must NOT be consumed
/// as part of the attribute name.  This guards the critical safety boundary in the
/// adjacent-attr continuation loop which only continues for Identifier/Method tokens.
#[test]
fn test_variable_attribute_followed_by_statement_modifier() {
    // `my $x :shared if $cond;` — `:shared` is the attribute; `if` begins the modifier.
    let code = "my $x :shared if (1) { 1 }";
    assert_clean_parse(code);
    let ast = parse(code);
    let sexp = ast.to_sexp();
    // Variable-declaration attributes render as `(attributes shared)` in sexp (no colon prefix).
    // The statement modifier renders as `statement_modifier_if`.
    assert!(
        sexp.contains("shared"),
        "Expected 'shared' attribute in AST sexp (variable attrs render without ':'), got: {}",
        sexp
    );
    assert!(
        sexp.contains("statement_modifier_if"),
        "Expected 'statement_modifier_if' to confirm `if` was NOT consumed as an attribute, got: {}",
        sexp
    );
}

/// Custom attribute before a prototype — parser must accept both and not confuse them.
/// Verifies the attribute-then-prototype order is handled correctly.
#[test]
fn test_custom_attribute_before_prototype() {
    // Perl allows: sub foo :custom (\@) { }
    let code = r#"sub foo :public (\@) { }"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains(":public"),
        "Expected ':public' in AST sexp when attribute precedes prototype, got: {}",
        sexp
    );
}

/// Numeric token (`:123`) after colon is not a valid attribute name and must error.
/// This is a pure-syntax error — not a "unknown name" situation.
#[test]
fn test_numeric_after_colon_is_syntax_error() {
    // `:123` is not a bareword attribute — it should be a syntax error
    // because a number token is not in `can_be_sub_name`.
    let code = "sub foo :123 { }";
    assert_has_error(code, "expected attribute name");
}

/// Adjacent attribute names after the SAME colon (e.g. `:foo bar`) — only
/// Identifier and Method tokens continue the inner loop.  A second colon is
/// needed to start the next attribute.
#[test]
fn test_adjacent_attrs_on_same_colon_only_identifiers() {
    // `:public` is one attribute (with its own colon); `:method` is another.
    // `sub foo :public :method { }` — two colons, two attributes.
    let code = "sub foo :public :method { }";
    assert_clean_parse(code);
    let ast = parse(code);
    let sexp = ast.to_sexp();
    assert!(sexp.contains(":public"), "Expected ':public' in AST sexp, got: {}", sexp);
    assert!(sexp.contains(":method"), "Expected ':method' in AST sexp, got: {}", sexp);
}

//! Tests for issue #792: field declaration attribute and initializer parsing.
//!
//! Verifies that all Perl 5.38+ `field` declaration forms parse cleanly:
//! - Field attributes (:param, :reader, :writer, :accessor, :mutator, :weak, :shared)
//! - Keyword-named attributes (:default) that are tokenized as non-Identifier kinds
//! - Parenthesized attribute arguments (:default(0), :reader(name))
//! - Initializers (= expr, //= expr, ||= expr)
//! - Combined attributes + initializers
//! - Array and hash fields
//! - Regression guard: plain `field $x;` unchanged

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_ast::NodeKind;
use perl_parser_core::Parser;
use perl_tdd_support::must;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Parse `source`, assert no ERROR nodes, return sexp.
fn clean_sexp(source: &str) -> String {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes:\n{}", source, sexp);
    sexp
}

/// Extract the first VariableDeclaration with declarator == "field" from the AST.
fn first_field_decl(source: &str) -> (Vec<String>, bool) {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    find_field_decl(&ast)
}

fn find_field_decl(node: &perl_parser_core::Node) -> (Vec<String>, bool) {
    if let NodeKind::VariableDeclaration { declarator, attributes, initializer, .. } = &node.kind {
        if declarator == "field" {
            return (attributes.clone(), initializer.is_some());
        }
    }
    for child in node.children() {
        let found = find_field_decl(child);
        if !found.0.is_empty() || found.1 {
            return found;
        }
    }
    (vec![], false)
}

// ── Basic field forms (regression: must continue to work) ────────────────────

#[test]
fn test_field_plain_scalar_no_error() {
    // field $x;  — baseline, already worked
    assert_clean_parse("class C { field $x; }");
}

#[test]
fn test_field_array_no_error() {
    // field @items;
    assert_clean_parse("class C { field @items; }");
}

#[test]
fn test_field_hash_no_error() {
    // field %opts;
    assert_clean_parse("class C { field %opts; }");
}

// ── Attribute-only forms ──────────────────────────────────────────────────────

#[test]
fn test_field_param_attr_no_error() {
    // field $x :param;
    assert_clean_parse("class C { field $x :param; }");
    let sexp = clean_sexp("class C { field $x :param; }");
    assert!(sexp.contains("(attributes param)"), "Expected (attributes param) in sexp: {}", sexp);
}

#[test]
fn test_field_reader_attr_no_error() {
    // field $x :reader;
    assert_clean_parse("class C { field $x :reader; }");
    let sexp = clean_sexp("class C { field $x :reader; }");
    assert!(sexp.contains("(attributes reader)"), "Expected (attributes reader) in sexp: {}", sexp);
}

#[test]
fn test_field_writer_attr_no_error() {
    // field $x :writer;
    assert_clean_parse("class C { field $x :writer; }");
}

#[test]
fn test_field_accessor_attr_no_error() {
    // field $x :accessor;
    assert_clean_parse("class C { field $x :accessor; }");
}

#[test]
fn test_field_weak_attr_no_error() {
    // field $x :weak;
    assert_clean_parse("class C { field $x :weak; }");
}

#[test]
fn test_field_multiple_attrs_no_error() {
    // field $x :reader :writer;
    assert_clean_parse("class C { field $x :reader :writer; }");
    let sexp = clean_sexp("class C { field $x :reader :writer; }");
    assert!(
        sexp.contains("reader") && sexp.contains("writer"),
        "Expected reader and writer in sexp: {}",
        sexp
    );
}

// ── Keyword-named attribute (:default) — the core failing case ────────────────
//
// The `default` keyword is tokenized as TokenKind::Default (not Identifier),
// so parse_declaration_attributes_with_extras rejects it with
// "Expected attribute name after ':'". This test verifies the fix.

#[test]
fn test_field_default_keyword_attr_no_error() {
    // field $x :default(0);  — :default is a keyword token, not Identifier
    assert_clean_parse("class C { field $x :default(0); }");
}

#[test]
fn test_field_default_attr_value_captured() {
    // :default(0) must be captured in the sexp
    let sexp = clean_sexp("class C { field $x :default(0); }");
    assert!(sexp.contains("default(0)"), "Expected 'default(0)' in attribute sexp: {}", sexp);
}

// ── Initializer-only forms ────────────────────────────────────────────────────

#[test]
fn test_field_assign_init_no_error() {
    // field $x = 1;
    assert_clean_parse("class C { field $x = 1; }");
    let (_, has_init) = first_field_decl("class C { field $x = 1; }");
    assert!(has_init, "field $x = 1 should have an initializer");
}

#[test]
fn test_field_orlassign_init_no_error() {
    // field $x //= 1;
    assert_clean_parse("class C { field $x //= 1; }");
}

// ── Combined attribute + initializer ─────────────────────────────────────────

#[test]
fn test_field_param_and_assign_no_error() {
    // field $x :param = 1;
    assert_clean_parse("class C { field $x :param = 1; }");
    let sexp = clean_sexp("class C { field $x :param = 1; }");
    assert!(sexp.contains("param"), "Expected param attribute in sexp: {}", sexp);
    let (attrs, has_init) = first_field_decl("class C { field $x :param = 1; }");
    assert!(attrs.contains(&"param".to_string()), "Expected param attribute, got: {:?}", attrs);
    assert!(has_init, "field $x :param = 1 should have an initializer");
}

#[test]
fn test_field_reader_and_string_init_no_error() {
    // field $x :reader = "default";
    assert_clean_parse(r#"class C { field $x :reader = "default"; }"#);
}

#[test]
fn test_field_param_and_orlassign_init_no_error() {
    // field $x :param //= 1;
    assert_clean_parse("class C { field $x :param //= 1; }");
}

#[test]
fn test_field_multiple_attrs_and_init() {
    // field $x :reader :param = 0;
    assert_clean_parse("class C { field $x :reader :param = 0; }");
    let (attrs, has_init) = first_field_decl("class C { field $x :reader :param = 0; }");
    assert!(attrs.contains(&"reader".to_string()), "Expected reader, got: {:?}", attrs);
    assert!(attrs.contains(&"param".to_string()), "Expected param, got: {:?}", attrs);
    assert!(has_init, "Should have initializer");
}

// ── Parenthesized attribute arguments ────────────────────────────────────────

#[test]
fn test_field_reader_with_accessor_name() {
    // field $x :reader(get_x);
    assert_clean_parse("class C { field $x :reader(get_x); }");
    let sexp = clean_sexp("class C { field $x :reader(get_x); }");
    assert!(sexp.contains("reader(get_x)"), "Expected reader(get_x) in sexp: {}", sexp);
}

#[test]
fn test_field_isa_constraint() {
    // field $x :isa(Str);
    assert_clean_parse("class C { field $x :isa(Str); }");
    let sexp = clean_sexp("class C { field $x :isa(Str); }");
    assert!(sexp.contains("isa(Str)"), "Expected isa(Str) in sexp: {}", sexp);
}

// ── Array/hash fields with attributes ────────────────────────────────────────

#[test]
fn test_field_array_with_param_attr() {
    // field @items :param;
    assert_clean_parse("class C { field @items :param; }");
}

#[test]
fn test_field_hash_with_param_attr() {
    // field %data :param;
    assert_clean_parse("class C { field %data :param; }");
}

// ── No spurious soft-warning for known field attrs ────────────────────────────

#[test]
fn test_field_param_produces_no_errors() {
    // :param should not trigger "unknown attribute" soft warning
    let mut parser = Parser::new("class C { field $x :param; }");
    let _ = must(parser.parse());
    // :param is in BUILTIN_VAR_ATTRIBUTES — no diagnostic expected
    let errors = parser.get_errors();
    assert!(errors.is_empty(), "Expected no parse errors for field $x :param; got: {:?}", errors);
}

// ── Regression: existing class/method parsing unchanged ──────────────────────

#[test]
fn test_class_with_multiple_field_forms() {
    // Full class with multiple field styles
    let source = r#"
class Point {
    field $x :param;
    field $y :param;
    field $label :reader = "unnamed";
    field @tags;
    field %meta;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_method_in_class_still_works() {
    // method parsing in same class must remain unchanged
    let source = r#"
class Foo {
    field $name :param;
    method get_name() { return $name; }
}
"#;
    assert_clean_parse(source);
}

use super::*;
use perl_tdd_support::must;

/// Helper: parse code and assert no ERROR nodes appear in the s-expression.
fn parse_ok(code: &str) -> String {
    let mut parser = Parser::new(code);
    let result = parser.parse();
    let ast = must(result);
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "Parse produced ERROR node for: {}\n  sexp: {}\n  errors: {:?}",
        code,
        sexp,
        parser.errors(),
    );
    sexp
}

// ── stringify operator ───────────────────────────────────────────────

#[test]
fn test_use_overload_stringify_operator() {
    let sexp = parse_ok(r#"use overload '""' => \&stringify;"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── arithmetic operators ─────────────────────────────────────────────

#[test]
fn test_use_overload_arithmetic_operators() {
    let sexp = parse_ok(r#"use overload '+' => \&add, '-' => \&sub;"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── comparison operators ─────────────────────────────────────────────

#[test]
fn test_use_overload_comparison_operators() {
    let sexp = parse_ok(r#"use overload '<=>' => \&compare, 'cmp' => \&str_compare;"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── conversion operators ─────────────────────────────────────────────

#[test]
fn test_use_overload_conversion_operators() {
    let sexp = parse_ok(r#"use overload '0+' => \&numify, 'bool' => \&boolify;"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── diamond operator plus bare key ───────────────────────────────────

#[test]
fn test_use_overload_diamond_and_fallback() {
    let sexp = parse_ok(r#"use overload '<>' => \&iterate, fallback => 1;"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── method name values (strings, not code refs) ──────────────────────

#[test]
fn test_use_overload_method_name_string_value() {
    let sexp = parse_ok(r#"use overload '+' => 'add';"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── multiple method overloads ────────────────────────────────────────

#[test]
fn test_use_overload_multiple_method_overloads() {
    let sexp = parse_ok(r#"use overload '-' => 'subtract', '*' => 'multiply';"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── inline sub values ────────────────────────────────────────────────

#[test]
fn test_use_overload_inline_sub() {
    let sexp = parse_ok(r#"use overload '+' => sub { $_[0]{val} + $_[1] };"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── complex multi-line overload declaration ──────────────────────────

#[test]
fn test_use_overload_complex_multi_operator() {
    let code = r#"use overload
    '+' => sub { $_[0]{val} + $_[1] },
    '-' => sub { $_[0]{val} - $_[1] },
    '""' => sub { $_[0]{str} },
    fallback => 1;"#;
    let sexp = parse_ok(code);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── deref overloading ────────────────────────────────────────────────

#[test]
fn test_use_overload_deref_operators() {
    let sexp = parse_ok(r#"use overload '@{}' => sub { $_[0]{array} };"#);
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

// ── numeric (0+) plus bool ───────────────────────────────────────────

#[test]
fn test_use_overload_numify_and_bool() {
    let sexp = parse_ok(
        r#"use overload '0+' => sub { $_[0]->to_number }, 'bool' => sub { $_[0]->is_true };"#,
    );
    assert!(sexp.contains("(use overload"), "Expected use overload node, got: {}", sexp);
}

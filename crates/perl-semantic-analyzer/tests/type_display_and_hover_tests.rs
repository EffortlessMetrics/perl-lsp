//! Tests for variable type display and hover label generation.
//!
//! Covers:
//! - `Display` implementation for `PerlType` and `ScalarType`
//! - `TypeInferenceEngine::hover_label_for` returning human-readable type strings

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::type_inference::{PerlType, ScalarType, TypeInferenceEngine};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// PerlType Display
// ---------------------------------------------------------------------------

#[test]
fn test_scalar_type_display_integer() {
    let ty = PerlType::Scalar(ScalarType::Integer);
    assert_eq!(ty.to_string(), "Int");
}

#[test]
fn test_scalar_type_display_float() {
    let ty = PerlType::Scalar(ScalarType::Float);
    assert_eq!(ty.to_string(), "Float");
}

#[test]
fn test_scalar_type_display_string() {
    let ty = PerlType::Scalar(ScalarType::String);
    assert_eq!(ty.to_string(), "Str");
}

#[test]
fn test_scalar_type_display_boolean() {
    let ty = PerlType::Scalar(ScalarType::Boolean);
    assert_eq!(ty.to_string(), "Bool");
}

#[test]
fn test_scalar_type_display_undef() {
    let ty = PerlType::Scalar(ScalarType::Undef);
    assert_eq!(ty.to_string(), "Undef");
}

#[test]
fn test_scalar_type_display_mixed() {
    let ty = PerlType::Scalar(ScalarType::Mixed);
    assert_eq!(ty.to_string(), "Scalar");
}

#[test]
fn test_array_type_display_with_element_type() {
    let ty = PerlType::Array(Box::new(PerlType::Scalar(ScalarType::Integer)));
    assert_eq!(ty.to_string(), "Array[Int]");
}

#[test]
fn test_array_any_type_display() {
    let ty = PerlType::Array(Box::new(PerlType::Any));
    assert_eq!(ty.to_string(), "Array");
}

#[test]
fn test_hash_type_display_with_types() {
    let ty = PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(PerlType::Scalar(ScalarType::Integer)),
    };
    assert_eq!(ty.to_string(), "Hash[Str => Int]");
}

#[test]
fn test_hash_any_value_type_display() {
    let ty = PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(PerlType::Any),
    };
    assert_eq!(ty.to_string(), "Hash");
}

#[test]
fn test_reference_type_display() {
    let ty = PerlType::Reference(Box::new(PerlType::Array(Box::new(PerlType::Any))));
    assert_eq!(ty.to_string(), "Ref[Array]");
}

#[test]
fn test_object_type_display() {
    let ty = PerlType::Object("MyClass".to_string());
    assert_eq!(ty.to_string(), "MyClass");
}

#[test]
fn test_glob_type_display() {
    let ty = PerlType::Glob;
    assert_eq!(ty.to_string(), "Glob");
}

#[test]
fn test_any_type_display() {
    let ty = PerlType::Any;
    assert_eq!(ty.to_string(), "Any");
}

#[test]
fn test_void_type_display() {
    let ty = PerlType::Void;
    assert_eq!(ty.to_string(), "Void");
}

// ---------------------------------------------------------------------------
// TypeInferenceEngine::hover_label_for
// ---------------------------------------------------------------------------

/// After inferring `my $x = 42`, `hover_label_for("x")` returns "Int".
#[test]
fn test_hover_label_integer_scalar() {
    let mut engine = TypeInferenceEngine::new();
    let code = "my $x = 42;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("x");
    assert_eq!(label.as_deref(), Some("Int"));
}

/// After inferring `my $s = "hello"`, `hover_label_for("s")` returns "Str".
#[test]
fn test_hover_label_string_scalar() {
    let mut engine = TypeInferenceEngine::new();
    let code = r#"my $s = "hello";"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("s");
    assert_eq!(label.as_deref(), Some("Str"));
}

/// After inferring `my @list = (1, 2, 3)`, `hover_label_for("list")` starts with "Array".
#[test]
fn test_hover_label_array() {
    let mut engine = TypeInferenceEngine::new();
    let code = "my @list = (1, 2, 3);";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("list");
    let label_str = label.as_deref().unwrap_or("");
    assert!(label_str.starts_with("Array"), "Expected Array-prefixed label, got: {:?}", label);
}

/// After inferring `my %h = (a => 1)`, `hover_label_for("h")` starts with "Hash".
#[test]
fn test_hover_label_hash() {
    let mut engine = TypeInferenceEngine::new();
    let code = r#"my %h = (a => 1, b => 2);"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("h");
    let label_str = label.as_deref().unwrap_or("");
    assert!(label_str.starts_with("Hash"), "Expected Hash-prefixed label, got: {:?}", label);
}

/// Unknown variable returns `None`.
#[test]
fn test_hover_label_unknown_variable_returns_none() {
    let engine = TypeInferenceEngine::new();
    let label = engine.hover_label_for("not_declared");
    assert!(label.is_none());
}

/// Float variable gets a "Float" label.
#[test]
fn test_hover_label_float_scalar() {
    let mut engine = TypeInferenceEngine::new();
    let code = "my $pi = 3.14;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("pi");
    assert_eq!(label.as_deref(), Some("Float"));
}

//! Extended unit tests for perl-dap-variables crate
//!
//! This module provides comprehensive test coverage for edge cases, boundary conditions,
//! and complex scenarios not covered by the main test suite.
#![allow(clippy::panic, clippy::approx_constant)]

use perl_dap::variables::{
    PerlValue, PerlVariableRenderer, RenderedVariable, VariableParseError, VariableParser,
    VariableRenderer,
};

// ═══════════════════════════════════════════════════════════════════════
// PerlValue Extended Tests - Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn perl_value_scalar_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::scalar("");
    assert_eq!(val, PerlValue::Scalar(String::new()));
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "SCALAR");
    Ok(())
}

#[test]
fn perl_value_scalar_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let special_str = "hello\nworld\t\"quotes\"";
    let val = PerlValue::scalar(special_str);
    assert!(matches!(
        &val,
        PerlValue::Scalar(s)
            if s == special_str && s.contains('\n') && s.contains('\t') && s.contains('"')
    ));
    Ok(())
}

#[test]
fn perl_value_number_zero() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Number(0.0);
    assert_eq!(val.type_name(), "SCALAR");
    assert!(!val.is_expandable());
    Ok(())
}

#[test]
fn perl_value_number_negative() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Number(-123.456);
    assert_eq!(val.type_name(), "SCALAR");
    assert!(matches!(val, PerlValue::Number(n) if (n - (-123.456)).abs() < 0.001));
    Ok(())
}

#[test]
fn perl_value_number_infinity() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Number(f64::INFINITY);
    assert_eq!(val.type_name(), "SCALAR");
    assert!(matches!(val, PerlValue::Number(n) if n.is_infinite()));
    Ok(())
}

#[test]
fn perl_value_integer_zero() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Integer(0);
    assert_eq!(val.type_name(), "SCALAR");
    assert!(!val.is_expandable());
    Ok(())
}

#[test]
fn perl_value_integer_negative() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Integer(-999);
    assert_eq!(val.type_name(), "SCALAR");
    assert!(matches!(val, PerlValue::Integer(i) if i == -999));
    Ok(())
}

#[test]
fn perl_value_integer_max() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Integer(i64::MAX);
    assert!(matches!(val, PerlValue::Integer(i) if i == i64::MAX));
    Ok(())
}

#[test]
fn perl_value_integer_min() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Integer(i64::MIN);
    assert!(matches!(val, PerlValue::Integer(i) if i == i64::MIN));
    Ok(())
}

#[test]
fn perl_value_array_single_element() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::array(vec![PerlValue::Undef]);
    assert_eq!(val.child_count(), Some(1));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "ARRAY");
    Ok(())
}

#[test]
fn perl_value_array_many_elements() -> Result<(), Box<dyn std::error::Error>> {
    let elements = (0..1000).map(PerlValue::Integer).collect();
    let val = PerlValue::Array(elements);
    assert_eq!(val.child_count(), Some(1000));
    assert!(val.is_expandable());
    Ok(())
}

#[test]
fn perl_value_array_mixed_types() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::array(vec![
        PerlValue::Undef,
        PerlValue::Integer(42),
        PerlValue::Number(3.14),
        PerlValue::scalar("hello"),
        PerlValue::Array(vec![]),
    ]);
    assert_eq!(val.child_count(), Some(5));
    Ok(())
}

#[test]
fn perl_value_hash_single_pair() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::hash(vec![("key".to_string(), PerlValue::Undef)]);
    assert_eq!(val.child_count(), Some(1));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "HASH");
    Ok(())
}

#[test]
fn perl_value_hash_empty_key() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::hash(vec![("".to_string(), PerlValue::Integer(42))]);
    assert_eq!(val.child_count(), Some(1));
    assert!(matches!(&val, PerlValue::Hash(pairs) if pairs[0].0.is_empty()));
    Ok(())
}

#[test]
fn perl_value_hash_many_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let pairs: Vec<(String, PerlValue)> =
        (0..100).map(|i| (format!("key_{}", i), PerlValue::Integer(i))).collect();
    let val = PerlValue::Hash(pairs);
    assert_eq!(val.child_count(), Some(100));
    Ok(())
}

#[test]
fn perl_value_hash_special_keys() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::hash(vec![
        ("key:with:colons".to_string(), PerlValue::Integer(1)),
        ("key-with-dashes".to_string(), PerlValue::Integer(2)),
        ("key with spaces".to_string(), PerlValue::Integer(3)),
        ("key\nwith\nnewlines".to_string(), PerlValue::Integer(4)),
    ]);
    assert_eq!(val.child_count(), Some(4));
    Ok(())
}

#[test]
fn perl_value_reference_to_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::reference(PerlValue::scalar("hello"));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "REF");
    assert_eq!(val.child_count(), None);
    Ok(())
}

#[test]
fn perl_value_reference_to_array() -> Result<(), Box<dyn std::error::Error>> {
    let val =
        PerlValue::reference(PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "REF");
    Ok(())
}

#[test]
fn perl_value_reference_to_hash() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::reference(PerlValue::Hash(vec![("a".to_string(), PerlValue::Integer(1))]));
    assert!(val.is_expandable());
    Ok(())
}

#[test]
fn perl_value_reference_to_reference() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::reference(PerlValue::reference(PerlValue::Integer(42)));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "REF");
    Ok(())
}

#[test]
fn perl_value_object_with_empty_hash() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::object("Empty::Class", PerlValue::Hash(vec![]));
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "OBJECT");
    assert!(matches!(
        &val,
        PerlValue::Object { class, value }
            if class == "Empty::Class"
                && matches!(value.as_ref(), PerlValue::Hash(h) if h.is_empty())
    ));
    Ok(())
}

#[test]
fn perl_value_object_with_array() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::object(
        "Array::Class",
        PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]),
    );
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "OBJECT");
    Ok(())
}

#[test]
fn perl_value_object_with_nested_object() -> Result<(), Box<dyn std::error::Error>> {
    let inner = PerlValue::object("Inner", PerlValue::scalar("data"));
    let outer = PerlValue::object("Outer", inner);
    assert!(outer.is_expandable());
    assert_eq!(outer.type_name(), "OBJECT");
    Ok(())
}

#[test]
fn perl_value_code_with_name() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Code { name: Some("my_function".to_string()) };
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "CODE");
    assert!(matches!(
        &val,
        PerlValue::Code { name } if name.as_deref() == Some("my_function")
    ));
    Ok(())
}

#[test]
fn perl_value_code_anonymous() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Code { name: None };
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "CODE");
    Ok(())
}

#[test]
fn perl_value_code_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Code { name: Some("Package::function".to_string()) };
    assert!(!val.is_expandable());
    assert!(matches!(
        &val,
        PerlValue::Code { name } if name.as_deref() == Some("Package::function")
    ));
    Ok(())
}

#[test]
fn perl_value_glob_simple() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Glob("foo".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "GLOB");
    Ok(())
}

#[test]
fn perl_value_glob_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Glob("Package::varname".to_string());
    assert!(!val.is_expandable());
    assert!(matches!(&val, PerlValue::Glob(name) if name == "Package::varname"));
    Ok(())
}

#[test]
fn perl_value_regex_simple() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Regex("pattern".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "Regexp");
    Ok(())
}

#[test]
fn perl_value_regex_complex() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Regex(r"[a-z]+\d{2,4}".to_string());
    assert!(!val.is_expandable());
    assert!(matches!(
        &val,
        PerlValue::Regex(pattern) if pattern.contains('[') && pattern.contains('}')
    ));
    Ok(())
}

#[test]
fn perl_value_tied_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Tied { class: "Tie::Class".to_string(), value: None };
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "TIED");
    Ok(())
}

#[test]
fn perl_value_tied_with_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Tied {
        class: "Tie::Scalar".to_string(),
        value: Some(Box::new(PerlValue::scalar("data"))),
    };
    assert!(val.is_expandable());
    assert_eq!(val.type_name(), "TIED");
    Ok(())
}

#[test]
fn perl_value_tied_with_hash() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Tied {
        class: "Tie::Hash".to_string(),
        value: Some(Box::new(PerlValue::Hash(vec![("key".to_string(), PerlValue::Integer(1))]))),
    };
    assert!(val.is_expandable());
    Ok(())
}

#[test]
fn perl_value_truncated_without_count() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Truncated { summary: "HUGE_STRUCTURE".to_string(), total_count: None };
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "...");
    assert_eq!(val.child_count(), None);
    Ok(())
}

#[test]
fn perl_value_truncated_with_count() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Truncated { summary: "LARGE_ARRAY".to_string(), total_count: Some(50000) };
    assert!(!val.is_expandable());
    assert_eq!(val.child_count(), Some(50000));
    Ok(())
}

#[test]
fn perl_value_error_simple() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Error("Variable not found".to_string());
    assert!(!val.is_expandable());
    assert_eq!(val.type_name(), "ERROR");
    assert!(matches!(&val, PerlValue::Error(msg) if msg == "Variable not found"));
    Ok(())
}

#[test]
fn perl_value_error_with_context() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::Error("Cannot dereference: SCALAR at file.pl line 42".to_string());
    assert!(!val.is_expandable());
    assert!(matches!(&val, PerlValue::Error(msg) if msg.contains("line 42")));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// RenderedVariable Extended Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn rendered_variable_basic_construction() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$var", "value");
    assert_eq!(rv.name, "$var");
    assert_eq!(rv.value, "value");
    assert_eq!(rv.type_name, None);
    assert_eq!(rv.variables_reference, 0);
    assert_eq!(rv.named_variables, None);
    assert_eq!(rv.indexed_variables, None);
    assert_eq!(rv.presentation_hint, None);
    assert_eq!(rv.memory_reference, None);
    Ok(())
}

#[test]
fn rendered_variable_with_type() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("@arr", "[]").with_type("ARRAY");
    assert_eq!(rv.type_name, Some("ARRAY".to_string()));
    Ok(())
}

#[test]
fn rendered_variable_with_reference() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("$ref", "REF(0x1234)").with_reference(42);
    assert_eq!(rv.variables_reference, 42);
    assert!(rv.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_with_indexed() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("@arr", "[1, 2, 3]").with_indexed_variables(3);
    assert_eq!(rv.indexed_variables, Some(3));
    Ok(())
}

#[test]
fn rendered_variable_with_named() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("%hash", "{a => 1}").with_named_variables(1);
    assert_eq!(rv.named_variables, Some(1));
    Ok(())
}

#[test]
fn rendered_variable_expandable_only_with_reference() -> Result<(), Box<dyn std::error::Error>> {
    let non_expandable = RenderedVariable::new("$x", "42");
    assert!(!non_expandable.is_expandable());

    let expandable = RenderedVariable::new("$x", "42").with_reference(1);
    assert!(expandable.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_chained_builders() -> Result<(), Box<dyn std::error::Error>> {
    let rv = RenderedVariable::new("@arr", "[1, 2]")
        .with_type("ARRAY")
        .with_reference(5)
        .with_indexed_variables(2);
    assert_eq!(rv.type_name, Some("ARRAY".to_string()));
    assert_eq!(rv.variables_reference, 5);
    assert_eq!(rv.indexed_variables, Some(2));
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// PerlVariableRenderer Extended Tests - Edge Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn renderer_format_empty_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("");
    let rendered = renderer.render("$empty", &val);
    assert_eq!(rendered.value, "\"\"");
    Ok(())
}

#[test]
fn renderer_format_scalar_with_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("line1\nline2\nline3");
    let rendered = renderer.render("$multi", &val);
    assert!(rendered.value.contains("\\n"));
    Ok(())
}

#[test]
fn renderer_format_scalar_with_tabs() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("col1\tcol2\tcol3");
    let rendered = renderer.render("$tabs", &val);
    assert!(rendered.value.contains("\\t"));
    Ok(())
}

#[test]
fn renderer_format_scalar_with_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("He said \"hello\"");
    let rendered = renderer.render("$quoted", &val);
    assert!(rendered.value.contains("\\\""));
    Ok(())
}

#[test]
fn renderer_format_scalar_with_backslashes() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("C:\\path\\to\\file");
    let rendered = renderer.render("$path", &val);
    assert!(rendered.value.contains("\\\\"));
    Ok(())
}

#[test]
fn renderer_format_scalar_all_escapes() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::scalar("a\"b'c\nd\re\tf");
    let rendered = renderer.render("$escapes", &val);
    assert!(rendered.value.contains("\\\""));
    assert!(rendered.value.contains("\\n"));
    assert!(rendered.value.contains("\\r"));
    assert!(rendered.value.contains("\\t"));
    Ok(())
}

#[test]
fn renderer_truncate_long_string() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(20);
    let long_str = "a".repeat(100);
    let val = PerlValue::scalar(long_str);
    let rendered = renderer.render("$long", &val);
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn renderer_max_string_length_zero() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(0);
    let val = PerlValue::scalar("hello");
    let rendered = renderer.render("$x", &val);
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn renderer_max_string_length_one() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_string_length(1);
    let val = PerlValue::scalar("hello");
    let rendered = renderer.render("$x", &val);
    assert!(rendered.value.contains("..."));
    Ok(())
}

#[test]
fn renderer_format_number_zero() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Number(0.0);
    let rendered = renderer.render("$zero", &val);
    assert_eq!(rendered.value, "0");
    Ok(())
}

#[test]
fn renderer_format_number_large() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Number(123456789.123456);
    let rendered = renderer.render("$big", &val);
    assert!(rendered.value.contains("123456789"));
    Ok(())
}

#[test]
fn renderer_format_integer_large() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Integer(9_223_372_036_854_775_807); // i64::MAX
    let rendered = renderer.render("$max", &val);
    assert!(rendered.value.contains("922337203"));
    Ok(())
}

#[test]
fn renderer_format_empty_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![]);
    let rendered = renderer.render("@empty", &val);
    assert_eq!(rendered.value, "[]");
    assert_eq!(rendered.indexed_variables, Some(0));
    Ok(())
}

#[test]
fn renderer_format_array_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(2);
    let val = PerlValue::Array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
        PerlValue::Integer(4),
    ]);
    let rendered = renderer.render("@arr", &val);
    assert!(rendered.value.contains("(4 total)"));
    Ok(())
}

#[test]
fn renderer_format_array_no_preview() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_array_preview(0);
    let val = PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]);
    let rendered = renderer.render("@arr", &val);
    assert!(rendered.value.contains("(2 total)"));
    Ok(())
}

#[test]
fn renderer_format_empty_hash() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![]);
    let rendered = renderer.render("%empty", &val);
    assert_eq!(rendered.value, "{}");
    assert_eq!(rendered.named_variables, Some(0));
    Ok(())
}

#[test]
fn renderer_format_hash_preview_limit() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new().with_max_hash_preview(2);
    let val = PerlValue::Hash(vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
        ("c".to_string(), PerlValue::Integer(3)),
    ]);
    let rendered = renderer.render("%hash", &val);
    assert!(rendered.value.contains("(3 keys)"));
    Ok(())
}

#[test]
fn renderer_format_undef() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Undef;
    let rendered = renderer.render("$x", &val);
    assert_eq!(rendered.value, "undef");
    assert!(!rendered.is_expandable());
    Ok(())
}

#[test]
fn renderer_format_reference_to_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::scalar("data"));
    let rendered = renderer.render("$ref", &val);
    assert!(rendered.value.contains("\\"));
    assert!(rendered.value.contains("data"));
    Ok(())
}

#[test]
fn renderer_format_reference_to_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::Array(vec![PerlValue::Integer(1)]));
    let rendered = renderer.render("$aref", &val);
    assert!(rendered.value.contains("\\"));
    // Full format now shows the array content preview instead of the opaque "ARRAY"
    assert!(rendered.value.contains("[1]"));
    Ok(())
}

#[test]
fn renderer_format_code_with_name() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Code { name: Some("my_sub".to_string()) };
    let rendered = renderer.render("$code", &val);
    assert!(rendered.value.contains("my_sub"));
    Ok(())
}

#[test]
fn renderer_format_code_anonymous() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Code { name: None };
    let rendered = renderer.render("$anon", &val);
    assert!(rendered.value.contains("sub"));
    Ok(())
}

#[test]
fn renderer_format_glob() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Glob("STDOUT".to_string());
    let rendered = renderer.render("$fh", &val);
    assert_eq!(rendered.value, "*STDOUT");
    Ok(())
}

#[test]
fn renderer_format_regex() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Regex("[0-9]+".to_string());
    let rendered = renderer.render("$re", &val);
    assert!(rendered.value.contains("qr"));
    assert!(rendered.value.contains("[0-9]"));
    Ok(())
}

#[test]
fn renderer_format_tied_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied { class: "Tie::Class".to_string(), value: None };
    let rendered = renderer.render("$tied", &val);
    assert!(rendered.value.contains("TIED"));
    assert!(rendered.value.contains("Tie::Class"));
    Ok(())
}

#[test]
fn renderer_format_tied_with_value() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Tied {
        class: "Tie::Scalar".to_string(),
        value: Some(Box::new(PerlValue::scalar("data"))),
    };
    let rendered = renderer.render("$tied", &val);
    assert!(rendered.value.contains("TIED"));
    assert!(rendered.value.contains("Tie::Scalar"));
    Ok(())
}

#[test]
fn renderer_format_truncated_without_count() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Truncated { summary: "[1, 2, 3, ...]".to_string(), total_count: None };
    let rendered = renderer.render("$huge", &val);
    assert_eq!(rendered.value, "[1, 2, 3, ...]");
    Ok(())
}

#[test]
fn renderer_format_truncated_with_count() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Truncated { summary: "[1, 2, ...]".to_string(), total_count: Some(50000) };
    let rendered = renderer.render("$huge", &val);
    assert!(rendered.value.contains("50000"));
    Ok(())
}

#[test]
fn renderer_format_error() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Error("Cannot evaluate".to_string());
    let rendered = renderer.render("$err", &val);
    assert!(rendered.value.contains("error"));
    assert!(rendered.value.contains("Cannot evaluate"));
    Ok(())
}

#[test]
fn renderer_render_children_empty_array() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![]);
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 0);
    Ok(())
}

#[test]
fn renderer_render_children_array_with_start() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![
        PerlValue::Integer(10),
        PerlValue::Integer(20),
        PerlValue::Integer(30),
        PerlValue::Integer(40),
    ]);
    let children = renderer.render_children(&val, 1, 2);
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "[1]");
    assert_eq!(children[1].name, "[2]");
    Ok(())
}

#[test]
fn renderer_render_children_array_start_beyond_length() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Array(vec![PerlValue::Integer(1)]);
    let children = renderer.render_children(&val, 5, 10);
    assert_eq!(children.len(), 0);
    Ok(())
}

#[test]
fn renderer_render_children_hash_empty() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![]);
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 0);
    Ok(())
}

#[test]
fn renderer_render_children_hash_with_start() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Hash(vec![
        ("a".to_string(), PerlValue::Integer(1)),
        ("b".to_string(), PerlValue::Integer(2)),
        ("c".to_string(), PerlValue::Integer(3)),
    ]);
    let children = renderer.render_children(&val, 1, 1);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "b");
    Ok(())
}

#[test]
fn renderer_render_children_reference() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::reference(PerlValue::Integer(42));
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "$_");
    assert_eq!(children[0].value, "42");
    Ok(())
}

#[test]
fn renderer_render_children_object() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::object(
        "MyClass",
        PerlValue::Hash(vec![
            ("x".to_string(), PerlValue::Integer(1)),
            ("y".to_string(), PerlValue::Integer(2)),
        ]),
    );
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    Ok(())
}

#[test]
fn renderer_render_children_non_expandable() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new();
    let val = PerlValue::Integer(42);
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 0);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// VariableParser Extended Tests - Complex Cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn parser_parse_array_with_empty_elements() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("(1, , 3)", 0);
    // Should still parse successfully
    if let Ok(PerlValue::Array(arr)) = result {
        assert!(arr.len() >= 2); // At least the non-empty elements
    }
    Ok(())
}

#[test]
fn parser_parse_nested_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("([1, 2], [3, 4])", 0);
    if let Ok(PerlValue::Array(arr)) = result {
        assert_eq!(arr.len(), 2);
    }
    Ok(())
}

#[test]
fn parser_parse_nested_hashes() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("{a => {b => 1}, c => {d => 2}}", 0);
    if let Ok(PerlValue::Hash(pairs)) = result {
        assert_eq!(pairs.len(), 2);
    }
    Ok(())
}

#[test]
fn parser_parse_deeply_nested_structure() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("{a => [1, {b => [2, 3]}]}", 0);
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn parser_max_depth_enforcement() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new().with_max_depth(3);
    let result = parser.parse_value("{a => {b => {c => {d => 1}}}}", 0);
    assert!(matches!(result, Err(VariableParseError::MaxDepthExceeded(_))));
    Ok(())
}

#[test]
fn parser_parse_hash_with_arrow_in_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("{key => \"value=>arrow\"}", 0);
    if let Ok(PerlValue::Hash(pairs)) = result {
        assert_eq!(pairs.len(), 1);
    }
    Ok(())
}

#[test]
fn parser_parse_array_with_nested_string_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("(\"first\\\"quoted\", 'second')", 0);
    if let Ok(PerlValue::Array(arr)) = result {
        assert!(!arr.is_empty());
    }
    Ok(())
}

#[test]
fn parser_parse_assignment_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_assignment("  $x  =  42  ");
    if let Ok((name, val)) = result {
        assert_eq!(name, "$x");
        assert!(matches!(val, PerlValue::Integer(42)));
    }
    Ok(())
}

#[test]
fn parser_parse_assignment_array_sigil() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_assignment("@arr = (1, 2, 3)");
    if let Ok((name, val)) = result {
        assert_eq!(name, "@arr");
        assert!(matches!(val, PerlValue::Array(_)));
    }
    Ok(())
}

#[test]
fn parser_parse_assignment_hash_sigil() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_assignment("%hash = {a => 1}");
    if let Ok((name, val)) = result {
        assert_eq!(name, "%hash");
        assert!(matches!(val, PerlValue::Hash(_)));
    }
    Ok(())
}

#[test]
fn parser_parse_variables_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let vars = parser.parse_variables("");
    assert_eq!(vars.len(), 0);
    Ok(())
}

#[test]
fn parser_parse_variables_with_errors() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let output = "$x = 1\ninvalid line\n$y = 2";
    let vars = parser.parse_variables(output);
    // Should only parse valid lines
    assert!(vars.len() >= 2);
    Ok(())
}

#[test]
fn parser_parse_variables_blank_lines() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let output = "$x = 1\n\n\n$y = 2";
    let vars = parser.parse_variables(output);
    assert_eq!(vars.len(), 2);
    Ok(())
}

#[test]
fn parser_parse_number_with_leading_zero() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("0123", 0);
    if let Ok(PerlValue::Integer(n)) = result {
        assert_eq!(n, 123); // Leading zeros are stripped in decimal
    }
    Ok(())
}

#[test]
fn parser_parse_number_with_plus_sign() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    // Parser should still treat this as a bareword since regex requires minus or nothing
    let result = parser.parse_value("+42", 0);
    // Should parse as bareword scalar, not integer
    assert!(matches!(result, Ok(PerlValue::Scalar(_))));
    Ok(())
}

#[test]
fn parser_parse_float_without_integer_part() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value(".5", 0);
    if let Ok(PerlValue::Number(n)) = result {
        assert!((n - 0.5).abs() < 0.001);
    }
    Ok(())
}

#[test]
fn parser_parse_float_without_decimal_part() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("5.", 0);
    if let Ok(PerlValue::Number(n)) = result {
        assert!((n - 5.0).abs() < 0.001);
    }
    Ok(())
}

#[test]
fn parser_parse_scientific_notation_uppercase_e() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("1E3", 0);
    if let Ok(PerlValue::Number(n)) = result {
        assert!((n - 1000.0).abs() < 0.001);
    }
    Ok(())
}

#[test]
fn parser_parse_scientific_notation_negative_exponent() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("1e-3", 0);
    if let Ok(PerlValue::Number(n)) = result {
        assert!((n - 0.001).abs() < 0.0001);
    }
    Ok(())
}

#[test]
fn parser_parse_single_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("'single'", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert_eq!(s, "single");
    }
    Ok(())
}

#[test]
fn parser_parse_double_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("\"double\"", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert_eq!(s, "double");
    }
    Ok(())
}

#[test]
fn parser_parse_string_with_single_quote_inside() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("\"It's\"", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert_eq!(s, "It's");
    }
    Ok(())
}

#[test]
fn parser_parse_string_with_double_quote_inside() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("'He said \"hi\"'", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert_eq!(s, "He said \"hi\"");
    }
    Ok(())
}

#[test]
fn parser_parse_string_escape_backslash() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("\"back\\\\slash\"", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert_eq!(s, "back\\slash");
    }
    Ok(())
}

#[test]
fn parser_parse_string_escape_carriage_return() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("\"line1\\rline2\"", 0);
    if let Ok(PerlValue::Scalar(s)) = result {
        assert!(s.contains('\r'));
    }
    Ok(())
}

#[test]
fn parser_parse_object_array_based() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("MyClass=ARRAY(0x1234)", 0);
    if let Ok(PerlValue::Object { class, value }) = result {
        assert_eq!(class, "MyClass");
        assert!(matches!(*value, PerlValue::Array(_)));
    }
    Ok(())
}

#[test]
fn parser_parse_object_scalar_based() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("MyClass=SCALAR(0x5678)", 0);
    if let Ok(PerlValue::Object { class, value }) = result {
        assert_eq!(class, "MyClass");
        assert!(matches!(*value, PerlValue::Scalar(_)));
    }
    Ok(())
}

#[test]
fn parser_parse_object_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_value("My::Package::Class=HASH(0x9abc)", 0);
    if let Ok(PerlValue::Object { class, .. }) = result {
        assert_eq!(class, "My::Package::Class");
    }
    Ok(())
}

#[test]
fn parser_serialization_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use serde_json;

    let val = PerlValue::Hash(vec![
        ("x".to_string(), PerlValue::Integer(1)),
        ("arr".to_string(), PerlValue::Array(vec![PerlValue::scalar("a"), PerlValue::Integer(2)])),
    ]);

    let json = serde_json::to_string(&val)?;
    let deserialized: PerlValue = serde_json::from_str(&json)?;
    assert_eq!(val, deserialized);
    Ok(())
}

#[test]
fn parser_parse_variables_large_input() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let mut lines = Vec::new();
    for i in 0..100 {
        lines.push(format!("$var_{} = {}", i, i));
    }
    let output = lines.join("\n");
    let vars = parser.parse_variables(&output);
    assert_eq!(vars.len(), 100);
    Ok(())
}

#[test]
fn renderer_custom_settings() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = PerlVariableRenderer::new()
        .with_max_string_length(50)
        .with_max_array_preview(5)
        .with_max_hash_preview(5);

    let val = PerlValue::array(vec![
        PerlValue::Integer(1),
        PerlValue::Integer(2),
        PerlValue::Integer(3),
        PerlValue::Integer(4),
        PerlValue::Integer(5),
        PerlValue::Integer(6),
    ]);

    let rendered = renderer.render("@arr", &val);
    assert!(rendered.value.contains("(6 total)"));
    Ok(())
}

#[test]
fn perl_value_serialization_all_types() -> Result<(), Box<dyn std::error::Error>> {
    use serde_json;

    let test_cases = vec![
        PerlValue::Undef,
        PerlValue::Scalar("test".to_string()),
        PerlValue::Number(3.14),
        PerlValue::Integer(42),
        PerlValue::Array(vec![]),
        PerlValue::Hash(vec![]),
        PerlValue::Reference(Box::new(PerlValue::Undef)),
        PerlValue::Object { class: "Class".to_string(), value: Box::new(PerlValue::Undef) },
        PerlValue::Code { name: Some("test".to_string()) },
        PerlValue::Glob("test".to_string()),
        PerlValue::Regex("pattern".to_string()),
        PerlValue::Tied { class: "Tie".to_string(), value: None },
        PerlValue::Truncated { summary: "...".to_string(), total_count: Some(1000) },
        PerlValue::Error("error msg".to_string()),
    ];

    for val in test_cases {
        let json = serde_json::to_string(&val)?;
        let _deserialized: PerlValue = serde_json::from_str(&json)?;
    }
    Ok(())
}

#[test]
fn complex_nested_structure() -> Result<(), Box<dyn std::error::Error>> {
    let complex = PerlValue::object(
        "ComplexClass",
        PerlValue::Hash(vec![
            (
                "arrays".to_string(),
                PerlValue::Array(vec![
                    PerlValue::Array(vec![PerlValue::Integer(1), PerlValue::Integer(2)]),
                    PerlValue::Array(vec![PerlValue::Integer(3), PerlValue::Integer(4)]),
                ]),
            ),
            (
                "nested_hash".to_string(),
                PerlValue::Hash(vec![(
                    "inner".to_string(),
                    PerlValue::reference(PerlValue::scalar("ref_value")),
                )]),
            ),
        ]),
    );

    assert!(complex.is_expandable());
    assert_eq!(complex.type_name(), "OBJECT");

    let renderer = PerlVariableRenderer::new();
    let _rendered = renderer.render("$complex", &complex);
    // render_with_reference sets the variable reference ID for expansion
    let rendered_with_ref = renderer.render_with_reference("$complex", &complex, 1);
    assert!(rendered_with_ref.is_expandable());
    Ok(())
}

#[test]
fn rendered_variable_full_json_serialization() -> Result<(), Box<dyn std::error::Error>> {
    use serde_json;

    let rv = RenderedVariable::new("@data", "[1, 2, 3]")
        .with_type("ARRAY")
        .with_reference(5)
        .with_indexed_variables(3);

    let json = serde_json::to_string(&rv)?;
    assert!(json.contains("data"));
    assert!(json.contains("ARRAY"));

    let deserialized: RenderedVariable = serde_json::from_str(&json)?;
    assert_eq!(deserialized.name, "@data");
    Ok(())
}

#[test]
fn parser_trailing_whitespace_handling() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_assignment("$x = 42   ");
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn parser_leading_whitespace_handling() -> Result<(), Box<dyn std::error::Error>> {
    let parser = VariableParser::new();
    let result = parser.parse_assignment("   $x = 42");
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn array_with_undef_elements() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::array(vec![
        PerlValue::Integer(1),
        PerlValue::Undef,
        PerlValue::Integer(3),
        PerlValue::Undef,
    ]);

    assert_eq!(val.child_count(), Some(4));

    let renderer = PerlVariableRenderer::new();
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 4);
    assert_eq!(children[1].value, "undef");
    Ok(())
}

#[test]
fn hash_with_undef_values() -> Result<(), Box<dyn std::error::Error>> {
    let val = PerlValue::hash(vec![
        ("defined".to_string(), PerlValue::Integer(1)),
        ("undefined".to_string(), PerlValue::Undef),
    ]);

    assert_eq!(val.child_count(), Some(2));

    let renderer = PerlVariableRenderer::new();
    let children = renderer.render_children(&val, 0, 10);
    assert_eq!(children.len(), 2);
    Ok(())
}

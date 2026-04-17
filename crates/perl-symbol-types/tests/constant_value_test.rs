//! Tests for ConstantValue enum and SymbolKind::Constant with value support.
//!
//! These tests verify that:
//! 1. ConstantValue enum exists with all required variants
//! 2. SymbolKind::Constant carries an optional ConstantValue
//! 3. LSP kind mappings work correctly for the new Constant variant
//!
//! Phase 1 scope: Integer arithmetic only

use perl_symbol_types::{ConstantValue, SymbolKind};

// ============================================================================
// ConstantValue enum tests
// ============================================================================

#[test]
fn test_constant_value_integer_variant_exists() {
    // Should be able to construct Integer variant
    let val = ConstantValue::Integer(42);
    assert!(matches!(val, ConstantValue::Integer(42)));
}

#[test]
fn test_constant_value_float_variant_exists() {
    let val = ConstantValue::Float(3.14);
    assert!(matches!(val, ConstantValue::Float(f) if (f - 3.14).abs() < 0.001));
}

#[test]
fn test_constant_value_string_variant_exists() {
    let val = ConstantValue::String("hello".to_string());
    assert!(matches!(val, ConstantValue::String(s) if s == "hello"));
}

#[test]
fn test_constant_value_bool_variant_exists() {
    let val_true = ConstantValue::Bool(true);
    let val_false = ConstantValue::Bool(false);
    assert!(matches!(val_true, ConstantValue::Bool(true)));
    assert!(matches!(val_false, ConstantValue::Bool(false)));
}

#[test]
fn test_constant_value_undef_variant_exists() {
    let val = ConstantValue::Undef;
    assert!(matches!(val, ConstantValue::Undef));
}

#[test]
fn test_constant_value_array_variant_exists() {
    let val = ConstantValue::Array(vec![ConstantValue::Integer(1), ConstantValue::Integer(2)]);
    assert!(matches!(val, ConstantValue::Array(arr) if arr.len() == 2));
}

#[test]
fn test_constant_value_hash_variant_exists() {
    let val = ConstantValue::Hash(vec![(
        ConstantValue::String("key".to_string()),
        ConstantValue::Integer(1),
    )]);
    assert!(matches!(val, ConstantValue::Hash(h) if h.len() == 1));
}

#[test]
fn test_constant_value_non_constant_variant_exists() {
    // NonConstant is a sentinel for "cannot fold"
    let val = ConstantValue::NonConstant;
    assert!(matches!(val, ConstantValue::NonConstant));
}

// ============================================================================
// SymbolKind::Constant with Optional Value tests
// ============================================================================

#[test]
fn test_symbol_kind_constant_with_value() {
    // SymbolKind::Constant should carry an optional ConstantValue
    let const_with_value = SymbolKind::Constant(Some(ConstantValue::Integer(100)));
    assert!(matches!(const_with_value, SymbolKind::Constant(Some(ConstantValue::Integer(100)))));
}

#[test]
fn test_symbol_kind_constant_without_value() {
    // SymbolKind::Constant can also be None (for constants where value is unknown)
    let const_without_value = SymbolKind::Constant(None);
    assert!(matches!(const_without_value, SymbolKind::Constant(None)));
}

#[test]
fn test_symbol_kind_constant_lsp_kind_with_value() {
    // Constant with a value should still map to LSP kind 14 (Constant)
    let const_kind = SymbolKind::Constant(Some(ConstantValue::Integer(42)));
    assert_eq!(const_kind.to_lsp_kind(), 14);
}

#[test]
fn test_symbol_kind_constant_lsp_kind_without_value() {
    // Constant without a value should also map to LSP kind 14
    let const_kind = SymbolKind::Constant(None);
    assert_eq!(const_kind.to_lsp_kind(), 14);
}

#[test]
fn test_symbol_kind_constant_lsp_kind_document_symbol_with_value() {
    // to_lsp_kind_document_symbol should also return 14 for Constant
    let const_kind = SymbolKind::Constant(Some(ConstantValue::String("test".to_string())));
    assert_eq!(const_kind.to_lsp_kind_document_symbol(), 14);
}

#[test]
fn test_symbol_kind_constant_lsp_kind_document_symbol_without_value() {
    let const_kind = SymbolKind::Constant(None);
    assert_eq!(const_kind.to_lsp_kind_document_symbol(), 14);
}

// ============================================================================
// ConstantValue equality tests
// ============================================================================

#[test]
fn test_constant_value_integer_equality() {
    assert_eq!(ConstantValue::Integer(42), ConstantValue::Integer(42));
    assert_ne!(ConstantValue::Integer(42), ConstantValue::Integer(43));
}

#[test]
fn test_constant_value_float_equality() {
    assert_eq!(ConstantValue::Float(3.14), ConstantValue::Float(3.14));
    assert_ne!(ConstantValue::Float(3.14), ConstantValue::Float(3.15));
}

#[test]
fn test_constant_value_string_equality() {
    assert_eq!(
        ConstantValue::String("hello".to_string()),
        ConstantValue::String("hello".to_string())
    );
    assert_ne!(
        ConstantValue::String("hello".to_string()),
        ConstantValue::String("world".to_string())
    );
}

#[test]
fn test_constant_value_bool_equality() {
    assert_eq!(ConstantValue::Bool(true), ConstantValue::Bool(true));
    assert_ne!(ConstantValue::Bool(true), ConstantValue::Bool(false));
}

#[test]
fn test_constant_value_undef_equality() {
    assert_eq!(ConstantValue::Undef, ConstantValue::Undef);
}

#[test]
fn test_constant_value_non_constant_equality() {
    assert_eq!(ConstantValue::NonConstant, ConstantValue::NonConstant);
}

#[test]
fn test_constant_value_array_equality() {
    let arr1 = ConstantValue::Array(vec![ConstantValue::Integer(1), ConstantValue::Integer(2)]);
    let arr2 = ConstantValue::Array(vec![ConstantValue::Integer(1), ConstantValue::Integer(2)]);
    let arr3 = ConstantValue::Array(vec![ConstantValue::Integer(1), ConstantValue::Integer(3)]);
    assert_eq!(arr1, arr2);
    assert_ne!(arr1, arr3);
}

// ============================================================================
// ConstantValue Debug tests
// ============================================================================

#[test]
fn test_constant_value_has_debug_representation() {
    let val = ConstantValue::Integer(42);
    let debug_str = format!("{:?}", val);
    assert!(debug_str.contains("42"));
}

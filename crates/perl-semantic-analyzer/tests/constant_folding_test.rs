//! Tests for constant_folding module - evaluates constant expressions at analysis time.
//!
//! Phase 1 scope: Integer arithmetic only (+, -, *, /, %, **, unary -)
//!
//! These tests verify that the ConstantFolder correctly folds expressions
//! and propagates NonConstant when folding cannot be performed.

use perl_semantic_analyzer::analysis::constant_folding::{ConstantFolder, ConstantValue};

// ============================================================================
// Basic binary operations
// ============================================================================

#[test]
fn test_folder_addition() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::Integer(100), ConstantValue::Integer(50));
    assert_eq!(result, ConstantValue::Integer(150));
}

#[test]
fn test_folder_subtraction() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("-", ConstantValue::Integer(100), ConstantValue::Integer(50));
    assert_eq!(result, ConstantValue::Integer(50));
}

#[test]
fn test_folder_multiplication() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("*", ConstantValue::Integer(100), ConstantValue::Integer(2));
    assert_eq!(result, ConstantValue::Integer(200));
}

#[test]
fn test_folder_division() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("/", ConstantValue::Integer(100), ConstantValue::Integer(4));
    assert_eq!(result, ConstantValue::Integer(25));
}

#[test]
fn test_folder_modulo() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("%", ConstantValue::Integer(100), ConstantValue::Integer(7));
    assert_eq!(result, ConstantValue::Integer(2));
}

#[test]
fn test_folder_power() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("**", ConstantValue::Integer(2), ConstantValue::Integer(8));
    assert_eq!(result, ConstantValue::Integer(256));
}

// ============================================================================
// Unary operations
// ============================================================================

#[test]
fn test_folder_unary_minus() {
    let folder = ConstantFolder::new();
    let result = folder.fold_unary("-", ConstantValue::Integer(100));
    assert_eq!(result, ConstantValue::Integer(-100));
}

#[test]
fn test_folder_unary_plus() {
    let folder = ConstantFolder::new();
    let result = folder.fold_unary("+", ConstantValue::Integer(100));
    assert_eq!(result, ConstantValue::Integer(100));
}

// ============================================================================
// NonConstant propagation - AC4
// ============================================================================

#[test]
fn test_folder_left_operand_non_constant() {
    // When left is NonConstant, result is NonConstant
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::NonConstant, ConstantValue::Integer(50));
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_right_operand_non_constant() {
    // When right is NonConstant, result is NonConstant
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::Integer(100), ConstantValue::NonConstant);
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_both_operands_non_constant() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("*", ConstantValue::NonConstant, ConstantValue::NonConstant);
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_non_constant_unary() {
    let folder = ConstantFolder::new();
    let result = folder.fold_unary("-", ConstantValue::NonConstant);
    assert_eq!(result, ConstantValue::NonConstant);
}

// ============================================================================
// Division by zero - AC5
// ============================================================================

#[test]
fn test_folder_division_by_zero() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("/", ConstantValue::Integer(100), ConstantValue::Integer(0));
    assert_eq!(result, ConstantValue::NonConstant, "Division by zero should return NonConstant");
}

#[test]
fn test_folder_modulo_by_zero() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("%", ConstantValue::Integer(100), ConstantValue::Integer(0));
    assert_eq!(result, ConstantValue::NonConstant, "Modulo by zero should return NonConstant");
}

// ============================================================================
// Unsupported operators
// ============================================================================

#[test]
fn test_folder_unsupported_binary_operator() {
    // String concatenation is not yet supported
    let folder = ConstantFolder::new();
    let result = folder.fold_binary(".", ConstantValue::Integer(1), ConstantValue::Integer(2));
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_unsupported_unary_operator() {
    // Logical not is not yet supported in Phase 1
    let folder = ConstantFolder::new();
    let result = folder.fold_unary("!", ConstantValue::Integer(1));
    assert_eq!(result, ConstantValue::NonConstant);
}

// ============================================================================
// Unsupported operand types (non-integer)
// ============================================================================

#[test]
fn test_folder_float_operands_return_non_constant() {
    // Phase 1 only supports integer arithmetic
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::Float(3.14), ConstantValue::Integer(1));
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_string_operands_return_non_constant() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary(
        "+",
        ConstantValue::String("hello".to_string()),
        ConstantValue::Integer(1),
    );
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_bool_operands_return_non_constant() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::Bool(true), ConstantValue::Integer(1));
    assert_eq!(result, ConstantValue::NonConstant);
}

#[test]
fn test_folder_undef_operands_return_non_constant() {
    let folder = ConstantFolder::new();
    let result = folder.fold_binary("+", ConstantValue::Undef, ConstantValue::Integer(1));
    assert_eq!(result, ConstantValue::NonConstant);
}

// ============================================================================
// Overflow handling (i64 wrapping)
// ============================================================================

#[test]
fn test_folder_large_integer_addition() {
    let folder = ConstantFolder::new();
    // i64::MAX + 1 wraps
    let max = i64::MAX;
    let result = folder.fold_binary("+", ConstantValue::Integer(max), ConstantValue::Integer(1));
    // Should wrap, not panic
    assert!(matches!(result, ConstantValue::Integer(_)));
}

#[test]
fn test_folder_large_integer_multiplication() {
    let folder = ConstantFolder::new();
    // Large multiplication
    let result = folder.fold_binary(
        "*",
        ConstantValue::Integer(1_000_000_000),
        ConstantValue::Integer(1_000_000_000),
    );
    assert!(matches!(result, ConstantValue::Integer(1_000_000_000_000_000_000)));
}

// ============================================================================
// Nested folding via chaining
// ============================================================================

#[test]
fn test_folder_nested_expression_manual() {
    // Manually fold 1 + 2 * 3
    // First: 2 * 3 = 6
    let folder = ConstantFolder::new();
    let step1 = folder.fold_binary("*", ConstantValue::Integer(2), ConstantValue::Integer(3));
    assert_eq!(step1, ConstantValue::Integer(6));

    // Then: 1 + 6 = 7
    let step2 = folder.fold_binary("+", ConstantValue::Integer(1), step1);
    assert_eq!(step2, ConstantValue::Integer(7));
}

#[test]
fn test_folder_nested_expression_with_unary() {
    // Manually fold -(1 + 2)
    let folder = ConstantFolder::new();
    let inner = folder.fold_binary("+", ConstantValue::Integer(1), ConstantValue::Integer(2));
    assert_eq!(inner, ConstantValue::Integer(3));
    let result = folder.fold_unary("-", inner);
    assert_eq!(result, ConstantValue::Integer(-3));
}

// ============================================================================
// Complex expressions
// ============================================================================

#[test]
fn test_folder_complex_expression_1() {
    // 2 ** 3 + 4 * 5 - 6 / 2 -> 8 + 20 - 3 = 25
    let folder = ConstantFolder::new();

    let p1 = folder.fold_binary("**", ConstantValue::Integer(2), ConstantValue::Integer(3)); // 8
    let p2 = folder.fold_binary("*", ConstantValue::Integer(4), ConstantValue::Integer(5)); // 20
    let p3 = folder.fold_binary("/", ConstantValue::Integer(6), ConstantValue::Integer(2)); // 3

    let s1 = folder.fold_binary("+", p1, p2); // 28
    let result = folder.fold_binary("-", s1, p3); // 25

    assert_eq!(result, ConstantValue::Integer(25));
}

#[test]
fn test_folder_negative_number_folding() {
    // -5 + -3 -> -8
    let folder = ConstantFolder::new();
    let neg5 = folder.fold_unary("-", ConstantValue::Integer(5));
    let neg3 = folder.fold_unary("-", ConstantValue::Integer(3));
    let result = folder.fold_binary("+", neg5, neg3);
    assert_eq!(result, ConstantValue::Integer(-8));
}

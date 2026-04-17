//! Tests for constant_expression_parser - a recursive-descent expression parser
//! for evaluating integer constant expressions at compile time.
//!
//! Phase 1 scope: Integer arithmetic only (+, -, *, /, %, **, unary -)
//! Operator precedence: ** > * / % > + -
//!
//! These tests verify that expressions like "1 + 2 * 3" fold to 7 (not 9).

use perl_symbol_surface::constant_expression_parser::{
    ConstantValue, ParseError, parse_expression,
};

// ============================================================================
// Basic integer arithmetic - AC1
// ============================================================================

#[test]
fn test_simple_integer_addition() {
    // 100 + 50 -> 150
    let tokens = vec!["100".to_string(), "+".to_string(), "50".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), ConstantValue::Integer(150));
}

#[test]
fn test_simple_integer_subtraction() {
    // 100 - 50 -> 50
    let tokens = vec!["100".to_string(), "-".to_string(), "50".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(50));
}

#[test]
fn test_simple_integer_multiplication() {
    // 100 * 2 -> 200
    let tokens = vec!["100".to_string(), "*".to_string(), "2".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(200));
}

#[test]
fn test_simple_integer_division() {
    // 100 / 4 -> 25
    let tokens = vec!["100".to_string(), "/".to_string(), "4".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(25));
}

#[test]
fn test_simple_integer_modulo() {
    // 100 % 7 -> 2
    let tokens = vec!["100".to_string(), "%".to_string(), "7".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(2));
}

#[test]
fn test_simple_integer_power() {
    // 2 ** 8 -> 256
    let tokens = vec!["2".to_string(), "**".to_string(), "8".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(256));
}

#[test]
fn test_unary_minus() {
    // -100 -> -100
    let tokens = vec!["-".to_string(), "100".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(-100));
}

// ============================================================================
// Operator precedence - AC2
// ============================================================================

#[test]
fn test_addition_and_multiplication_precedence() {
    // 1 + 2 * 3 -> 7 (not 9)
    // Multiplication has higher precedence than addition
    let tokens =
        vec!["1".to_string(), "+".to_string(), "2".to_string(), "*".to_string(), "3".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(7), "1 + 2 * 3 should be 7, not 9");
}

#[test]
fn test_subtraction_and_multiplication_precedence() {
    // 10 - 2 * 4 -> 2 (not 32)
    let tokens =
        vec!["10".to_string(), "-".to_string(), "2".to_string(), "*".to_string(), "4".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(2), "10 - 2 * 4 should be 2");
}

#[test]
fn test_division_and_addition_precedence() {
    // 1 + 6 / 2 -> 4 (not 3.5)
    let tokens =
        vec!["1".to_string(), "+".to_string(), "6".to_string(), "/".to_string(), "2".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(4), "1 + 6 / 2 should be 4");
}

#[test]
fn test_power_has_higher_precedence_than_multiplication() {
    // 2 * 3 ** 2 -> 18 (not 36)
    // ** binds tighter than *
    let tokens =
        vec!["2".to_string(), "*".to_string(), "3".to_string(), "**".to_string(), "2".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(18), "2 * 3 ** 2 should be 18");
}

#[test]
fn test_left_associativity_of_addition() {
    // 10 - 2 - 1 -> 7 (not 9)
    // Subtraction is left-associative
    let tokens =
        vec!["10".to_string(), "-".to_string(), "2".to_string(), "-".to_string(), "1".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(7), "10 - 2 - 1 should be 7");
}

#[test]
fn test_left_associativity_of_multiplication() {
    // 2 * 3 * 4 -> 24
    let tokens =
        vec!["2".to_string(), "*".to_string(), "3".to_string(), "*".to_string(), "4".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(24));
}

#[test]
fn test_multiple_operations_same_precedence() {
    // 2 * 3 + 4 * 5 -> 26 (not 50)
    // (2*3) + (4*5) = 6 + 20 = 26
    let tokens = vec![
        "2".to_string(),
        "*".to_string(),
        "3".to_string(),
        "+".to_string(),
        "4".to_string(),
        "*".to_string(),
        "5".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(26), "2 * 3 + 4 * 5 should be 26");
}

#[test]
fn test_parenthesized_expression() {
    // (1 + 2) * 3 -> 9
    let tokens = vec![
        "(".to_string(),
        "1".to_string(),
        "+".to_string(),
        "2".to_string(),
        ")".to_string(),
        "*".to_string(),
        "3".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(9), "(1 + 2) * 3 should be 9");
}

#[test]
fn test_nested_parentheses() {
    // ((1 + 2)) -> 3
    let tokens = vec![
        "(".to_string(),
        "(".to_string(),
        "1".to_string(),
        "+".to_string(),
        "2".to_string(),
        ")".to_string(),
        ")".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(3));
}

#[test]
fn test_parentheses_affect_precedence() {
    // (1 + 2) * (3 + 4) -> 21
    let tokens = vec![
        "(".to_string(),
        "1".to_string(),
        "+".to_string(),
        "2".to_string(),
        ")".to_string(),
        "*".to_string(),
        "(".to_string(),
        "3".to_string(),
        "+".to_string(),
        "4".to_string(),
        ")".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(21));
}

// ============================================================================
// Hex and Octal literals - AC1
// ============================================================================

#[test]
fn test_hex_literal() {
    // 0xFF -> 255
    let tokens = vec!["0xFF".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(255));
}

#[test]
fn test_octal_literal() {
    // 0777 -> 511
    let tokens = vec!["0777".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(511));
}

#[test]
fn test_binary_literal() {
    // 0b1010 -> 10
    let tokens = vec!["0b1010".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(10));
}

#[test]
fn test_hex_literal_in_expression() {
    // 0x10 + 1 -> 17
    let tokens = vec!["0x10".to_string(), "+".to_string(), "1".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(17));
}

// ============================================================================
// Division by zero - AC5
// ============================================================================

#[test]
fn test_division_by_zero_returns_non_constant() {
    // 100 / 0 -> NonConstant
    let tokens = vec!["100".to_string(), "/".to_string(), "0".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok(), "Parse should succeed");
    assert_eq!(
        result.unwrap(),
        ConstantValue::NonConstant,
        "Division by zero should return NonConstant"
    );
}

#[test]
fn test_modulo_by_zero_returns_non_constant() {
    // 100 % 0 -> NonConstant
    let tokens = vec!["100".to_string(), "%".to_string(), "0".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        ConstantValue::NonConstant,
        "Modulo by zero should return NonConstant"
    );
}

// ============================================================================
// Unary minus edge cases
// ============================================================================

#[test]
fn test_double_negation() {
    // --5 -> 5
    let tokens = vec!["-".to_string(), "-".to_string(), "5".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(5));
}

#[test]
fn test_unary_minus_with_expression() {
    // -(1 + 2) -> -3
    let tokens = vec![
        "-".to_string(),
        "(".to_string(),
        "1".to_string(),
        "+".to_string(),
        "2".to_string(),
        ")".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(-3));
}

// ============================================================================
// Complex expressions
// ============================================================================

#[test]
fn test_complex_expression_1() {
    // 2 ** 3 + 4 * 5 - 6 / 2 -> 8 + 20 - 3 = 25
    let tokens = vec![
        "2".to_string(),
        "**".to_string(),
        "3".to_string(),
        "+".to_string(),
        "4".to_string(),
        "*".to_string(),
        "5".to_string(),
        "-".to_string(),
        "6".to_string(),
        "/".to_string(),
        "2".to_string(),
    ];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(25));
}

#[test]
fn test_negative_numbers_in_expression() {
    // -5 + -3 -> -8
    let tokens =
        vec!["-".to_string(), "5".to_string(), "+".to_string(), "-".to_string(), "3".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(-8));
}

// ============================================================================
// Single value expressions
// ============================================================================

#[test]
fn test_single_integer() {
    // "42" -> 42
    let tokens = vec!["42".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(42));
}

#[test]
fn test_single_negative_integer() {
    // "-42" -> -42
    let tokens = vec!["-42".to_string()];
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ConstantValue::Integer(-42));
}

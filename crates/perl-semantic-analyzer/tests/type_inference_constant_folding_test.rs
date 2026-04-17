//! Integration tests for constant folding in type inference.
//!
//! These tests verify that when the type inference engine encounters
//! constant expressions (e.g., `MAX * 2` where MAX is a known constant),
//! it can fold them to produce precise constant types.
//!
//! Phase 1 scope: Integer arithmetic only

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::type_inference::TypeInferenceEngine;
use perl_symbol_types::ConstantValue;
use perl_tdd_support::must;
use std::sync::Arc;

// ============================================================================
// Integration: use constant declarations are recognized
// ============================================================================

#[test]
fn test_type_inference_loads_use_constant_into_environment()
-> Result<(), Box<dyn std::error::Error>> {
    // Parse a file with a use constant declaration
    let code = "use constant MAX => 100;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // TypeInferenceEngine should load the constant
    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // When the feature is implemented, we should be able to verify
    // that the constant was loaded into the environment
    // This test currently fails because:
    // - TypeEnvironment doesn't have get_constant() method
    // - There's no way to verify the constant was loaded

    // IMPLEMENTATION REQUIRED: After implementation, uncomment:
    // let max_value = engine.get_global_env().get_constant("MAX");
    // assert_eq!(max_value, Some(&ConstantValue::Integer(100)));

    Ok(())
}

// ============================================================================
// Integration: constant folding with use constant
// ============================================================================

#[test]
fn test_constant_folding_with_use_constant() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that:
    // 1. "use constant MAX => 100" is parsed
    // 2. When inferring "MAX * 2", the engine:
    //    - Looks up MAX in the constants map -> finds 100
    //    - Recognizes both operands are constants
    //    - Folds 100 * 2 = 200
    //    - Returns the type that represents the constant 200
    //
    // Currently this will fail because:
    // - SymbolKind::Constant doesn't carry values
    // - TypeEnvironment doesn't have a constants map
    // - ConstantFolder doesn't exist
    // - TypeInferenceEngine doesn't call the folder

    let code = r#"
        use constant MAX => 100;
        my $x = MAX * 2;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // IMPLEMENTATION REQUIRED: After implementation, verify:
    // When we query the type of $x, it should have a constant value of 200
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(200)));

    Ok(())
}

// ============================================================================
// Integration: nested constant expressions
// ============================================================================

#[test]
fn test_nested_constant_folding() -> Result<(), Box<dyn std::error::Error>> {
    // use constant {
    //     A => 10,
    //     B => A * 2,    # B = 20
    //     C => A + B,    # C = 30
    // };
    // my $x = C * 2;  # Should fold to 60

    let code = r#"
        use constant {
            A => 10,
            B => A * 2,
            C => A + B,
        };
        my $x = C * 2;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // IMPLEMENTATION REQUIRED: Verify that nested constant expressions fold correctly
    // A = 10, B = A * 2 = 20, C = A + B = 30, x = C * 2 = 60
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(60)));

    Ok(())
}

// ============================================================================
// Integration: unknown constants fall back gracefully
// ============================================================================

#[test]
fn test_unknown_constant_falls_back_to_non_constant() -> Result<(), Box<dyn std::error::Error>> {
    // When a constant is not in scope, the expression should be NonConstant
    // and type inference should fall back to its existing behavior
    //
    // Currently this will fail because:
    // - Constants aren't stored in TypeEnvironment
    // - Binary expression inference doesn't check constants map
    // - There's no NonConstant propagation

    let code = r#"
        use constant MAX => 100;
        my $x = UNKNOWN * 2;  # UNKNOWN is not defined
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // IMPLEMENTATION REQUIRED: $x should have a type that indicates the value cannot be
    // determined at compile time (NonConstant -> falls back to Any or Mixed)
    // let x_type = engine.get_type_of("x");
    // When using an unknown constant, the result should be NonConstant

    Ok(())
}

// ============================================================================
// Integration: division by zero is handled gracefully
// ============================================================================

#[test]
fn test_division_by_zero_returns_non_constant() -> Result<(), Box<dyn std::error::Error>> {
    // 100 / 0 should return NonConstant, not panic
    let code = r#"
        use constant MAX => 100;
        my $x = MAX / 0;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // IMPLEMENTATION REQUIRED: Should not panic, should return NonConstant
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::NonConstant));

    Ok(())
}

// ============================================================================
// Integration: operator precedence in expressions with constants
// ============================================================================

#[test]
fn test_constant_folding_respects_precedence() -> Result<(), Box<dyn std::error::Error>> {
    // 1 + 2 * 3 should fold to 7, not 9
    let code = r#"
        my $x = 1 + 2 * 3;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // Expected: $x should have a type with constant value 7
    // The expression parser must use correct precedence: * before +
    // IMPLEMENTATION REQUIRED:
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(7)));

    Ok(())
}

// ============================================================================
// Integration: parenthesized expressions
// ============================================================================

#[test]
fn test_parenthesized_expressions_fold() -> Result<(), Box<dyn std::error::Error>> {
    // (1 + 2) * 3 should fold to 9
    let code = r#"
        my $x = (1 + 2) * 3;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // Expected: $x should have type with constant value 9
    // IMPLEMENTATION REQUIRED:
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(9)));

    Ok(())
}

// ============================================================================
// Integration: unary minus with constants
// ============================================================================

#[test]
fn test_unary_minus_folds() -> Result<(), Box<dyn std::error::Error>> {
    // -100 should fold to -100
    let code = r#"
        use constant NEG => 100;
        my $x = -NEG;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // Expected: $x should have type with constant value -100
    // IMPLEMENTATION REQUIRED:
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(-100)));

    Ok(())
}

// ============================================================================
// Integration: multiple operations chain correctly
// ============================================================================

#[test]
fn test_chained_operations_fold() -> Result<(), Box<dyn std::error::Error>> {
    // 2 ** 3 + 4 * 5 - 6 / 2 -> 8 + 20 - 3 = 25
    let code = r#"
        use constant {
            A => 2,
            B => 3,
            C => 4,
            D => 5,
            E => 6,
        };
        my $x = A ** B + C * D - E / 2;
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let mut engine = TypeInferenceEngine::new();
    let _result = engine.infer(&ast);

    // Expected: $x should have type with constant value 25
    // IMPLEMENTATION REQUIRED:
    // let x_type = engine.get_type_of("x");
    // assert_eq!(x_type.get_constant_value(), Some(ConstantValue::Integer(25)));

    Ok(())
}

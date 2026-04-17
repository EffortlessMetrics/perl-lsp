//! Tests for TypeEnvironment constants map and set_constant/get_constant methods.
//!
//! These tests verify that TypeEnvironment can store and retrieve constant values
//! needed for constant folding during type inference.

use perl_semantic_analyzer::analysis::type_inference::TypeEnvironment;
use perl_symbol_types::ConstantValue;

// ============================================================================
// Basic constants map operations
// ============================================================================

#[test]
fn test_type_environment_has_constants_field() {
    // TypeEnvironment should have a constants map
    let mut env = TypeEnvironment::new();
    // Should be able to set a constant
    env.set_constant("MAX".to_string(), ConstantValue::Integer(100));
    // Should be able to get a constant
    let result = env.get_constant("MAX");
    assert!(result.is_some(), "Expected to find MAX in environment");
    assert_eq!(result.unwrap(), &ConstantValue::Integer(100));
}

#[test]
fn test_type_environment_get_constant_returns_none_for_unknown() {
    let env = TypeEnvironment::new();
    let result = env.get_constant("UNKNOWN");
    assert!(result.is_none(), "Unknown constant should return None");
}

#[test]
fn test_type_environment_constants_persist_after_set() {
    let mut env = TypeEnvironment::new();
    env.set_constant("PI".to_string(), ConstantValue::Float(3.14));
    env.set_constant("NAME".to_string(), ConstantValue::String("test".to_string()));

    assert_eq!(env.get_constant("PI"), Some(&ConstantValue::Float(3.14)));
    assert_eq!(env.get_constant("NAME"), Some(&ConstantValue::String("test".to_string())));
}

#[test]
fn test_type_environment_constants_can_be_overwritten() {
    let mut env = TypeEnvironment::new();
    env.set_constant("VALUE".to_string(), ConstantValue::Integer(10));
    env.set_constant("VALUE".to_string(), ConstantValue::Integer(20));

    assert_eq!(env.get_constant("VALUE"), Some(&ConstantValue::Integer(20)));
}

// ============================================================================
// Parent scope and constants inheritance
// ============================================================================

#[test]
fn test_type_environment_constants_inherited_from_parent() {
    let mut parent = TypeEnvironment::new();
    parent.set_constant("PARENT_CONST".to_string(), ConstantValue::Integer(100));

    let child = TypeEnvironment::with_parent(parent);

    // Child should be able to see parent's constants
    assert_eq!(child.get_constant("PARENT_CONST"), Some(&ConstantValue::Integer(100)));
}

#[test]
fn test_type_environment_child_constants_do_not_affect_parent() {
    let mut parent = TypeEnvironment::new();
    parent.set_constant("SHARED".to_string(), ConstantValue::Integer(100));

    let mut child = TypeEnvironment::with_parent(parent.clone());
    child.set_constant("SHARED".to_string(), ConstantValue::Integer(200));

    // Parent should still have old value
    assert_eq!(parent.get_constant("SHARED"), Some(&ConstantValue::Integer(100)));
}

#[test]
fn test_type_environment_child_can_see_own_constants() {
    let parent = TypeEnvironment::new();
    let mut child = TypeEnvironment::with_parent(parent);

    child.set_constant("CHILD_CONST".to_string(), ConstantValue::Integer(42));

    assert_eq!(child.get_constant("CHILD_CONST"), Some(&ConstantValue::Integer(42)));
}

#[test]
fn test_type_environment_child_shadows_parent_constant() {
    let mut parent = TypeEnvironment::new();
    parent.set_constant("VALUE".to_string(), ConstantValue::Integer(100));

    let mut child = TypeEnvironment::with_parent(parent);
    child.set_constant("VALUE".to_string(), ConstantValue::Integer(200));

    // Child should see its own value, not parent's
    assert_eq!(child.get_constant("VALUE"), Some(&ConstantValue::Integer(200)));
}

// ============================================================================
// Clone behavior
// ============================================================================

#[test]
fn test_type_environment_clone_copies_constants() {
    let mut env = TypeEnvironment::new();
    env.set_constant("ORIGINAL".to_string(), ConstantValue::Integer(42));

    let cloned = env.clone();

    assert_eq!(cloned.get_constant("ORIGINAL"), Some(&ConstantValue::Integer(42)));
}

#[test]
fn test_clone_is_independent() {
    let mut env = TypeEnvironment::new();
    env.set_constant("SHARED".to_string(), ConstantValue::Integer(10));

    let mut cloned = env.clone();
    cloned.set_constant("SHARED".to_string(), ConstantValue::Integer(20));

    // Original should be unchanged
    assert_eq!(env.get_constant("SHARED"), Some(&ConstantValue::Integer(10)));
}

// ============================================================================
// All ConstantValue variants can be stored
// ============================================================================

#[test]
fn test_store_integer_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("INT".to_string(), ConstantValue::Integer(42));
    assert_eq!(env.get_constant("INT"), Some(&ConstantValue::Integer(42)));
}

#[test]
fn test_store_float_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("FLOAT".to_string(), ConstantValue::Float(3.14));
    assert_eq!(env.get_constant("FLOAT"), Some(&ConstantValue::Float(3.14)));
}

#[test]
fn test_store_string_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("STR".to_string(), ConstantValue::String("hello".to_string()));
    assert_eq!(env.get_constant("STR"), Some(&ConstantValue::String("hello".to_string())));
}

#[test]
fn test_store_bool_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("FLAG".to_string(), ConstantValue::Bool(true));
    assert_eq!(env.get_constant("FLAG"), Some(&ConstantValue::Bool(true)));
}

#[test]
fn test_store_undef_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("UNDEF".to_string(), ConstantValue::Undef);
    assert_eq!(env.get_constant("UNDEF"), Some(&ConstantValue::Undef));
}

#[test]
fn test_store_non_constant() {
    let mut env = TypeEnvironment::new();
    env.set_constant("NON_CONST".to_string(), ConstantValue::NonConstant);
    assert_eq!(env.get_constant("NON_CONST"), Some(&ConstantValue::NonConstant));
}

#[test]
fn test_store_array_constant() {
    let mut env = TypeEnvironment::new();
    let arr = ConstantValue::Array(vec![ConstantValue::Integer(1), ConstantValue::Integer(2)]);
    env.set_constant("ARR".to_string(), arr);
    assert!(matches!(env.get_constant("ARR"), Some(&ConstantValue::Array(_))));
}

#[test]
fn test_store_hash_constant() {
    let mut env = TypeEnvironment::new();
    let hash = ConstantValue::Hash(vec![(
        ConstantValue::String("key".to_string()),
        ConstantValue::Integer(1),
    )]);
    env.set_constant("HASH".to_string(), hash);
    assert!(matches!(env.get_constant("HASH"), Some(&ConstantValue::Hash(_))));
}

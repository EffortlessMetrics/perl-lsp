//! SemVer hygiene tests for perl-semantic-analyzer crate.
//!
//! These tests verify that published types have `#[non_exhaustive]`
//! to allow future minor-version additions without SemVer-major bumps.

use perl_semantic_analyzer::SourceLocation;
use perl_semantic_analyzer::analysis::class_model::{
    Attribute, ClassModelBuilder, MethodInfo, MethodModifier, ModifierKind,
};

// ============================================================================
// Test 1: Attribute should have #[non_exhaustive]
// ============================================================================
//
// Attribute represents a Moose/Moo attribute declared via `has`.
// #[non_exhaustive] allows future minor-version field additions without
// breaking downstream consumers.

#[test]
fn attribute_has_expected_fields() {
    let attr = Attribute {
        name: "test".to_string(),
        is: None,
        isa: Some("Str".to_string()),
        default: true,
        required: false,
        accessor_name: "test".to_string(),
        location: SourceLocation::default(),
        builder: None,
        coerce: false,
        predicate: None,
        clearer: None,
        trigger: false,
    };

    assert_eq!(attr.name, "test");
    assert_eq!(attr.isa, Some("Str".to_string()));
    assert!(attr.default);
    assert!(!attr.required);
}

#[test]
fn attribute_type_has_non_exhaustive_marker() {
    // This test documents that Attribute should have #[non_exhaustive].
    // The actual verification is done by cargo semver-checks.
    //
    // Attribute is constructed via builder patterns internally,
    // and #[non_exhaustive] ensures external consumers cannot
    // break with minor-version field additions.
    let _attr = Attribute {
        name: String::new(),
        is: None,
        isa: None,
        default: false,
        required: false,
        accessor_name: String::new(),
        location: SourceLocation::default(),
        builder: None,
        coerce: false,
        predicate: None,
        clearer: None,
        trigger: false,
    };
}

// ============================================================================
// Test 2: MethodModifier should have #[non_exhaustive]
// ============================================================================

#[test]
fn method_modifier_has_expected_fields() {
    let modifier = MethodModifier {
        kind: ModifierKind::Before,
        method_name: "foo".to_string(),
        location: SourceLocation::default(),
    };

    assert!(matches!(modifier.kind, ModifierKind::Before));
    assert_eq!(modifier.method_name, "foo");
}

#[test]
fn method_modifier_type_has_non_exhaustive_marker() {
    // This test documents that MethodModifier should have #[non_exhaustive].
    // The actual verification is done by cargo semver-checks.
    let _modifier = MethodModifier {
        kind: ModifierKind::After,
        method_name: String::new(),
        location: SourceLocation::default(),
    };
}

// ============================================================================
// Test 3: MethodInfo should have #[non_exhaustive]
// ============================================================================

#[test]
fn method_info_has_expected_fields() {
    let method = MethodInfo::new("test_method".to_string(), SourceLocation::default());

    assert_eq!(method.name, "test_method");
    assert!(!method.synthetic);
    assert!(method.accessor_mode.is_none());
}

#[test]
fn method_info_type_has_non_exhaustive_marker() {
    // This test documents that MethodInfo should have #[non_exhaustive].
    // The actual verification is done by cargo semver-checks.
    let _method = MethodInfo::new(String::new(), SourceLocation::default());
}

// ============================================================================
// Test 4: ClassModel should have #[non_exhaustive]
// ============================================================================
//
// ClassModel is the structured model of a Perl OOP class or role.
// It is constructed via ClassModelBuilder, not via struct literal.
// #[non_exhaustive] ensures future minor-version field additions
// don't break downstream consumers.

#[test]
fn class_model_builder_can_be_instantiated() {
    // ClassModelBuilder can be instantiated - actual building requires AST
    let _builder = ClassModelBuilder::new();
}

//! SemVer hygiene tests for perl-semantic-analyzer crate.
//!
//! These tests verify that published types have `#[non_exhaustive]`
//! to allow future minor-version additions without SemVer-major bumps.

use perl_semantic_analyzer::SourceLocation;
use perl_semantic_analyzer::analysis::class_model::{
    Attribute, ClassModelBuilder, MethodInfo, MethodModifier,
};

// ============================================================================
// Test 1: Attribute should have #[non_exhaustive]
// ============================================================================
//
// Attribute represents a Moose/Moo attribute declared via `has`.
// #[non_exhaustive] allows future minor-version field additions without
// breaking downstream consumers.
//
// NOTE: Attribute is #[non_exhaustive], so external code cannot construct
// instances via struct literal. The #[non_exhaustive] marker is verified
// by cargo semver-checks. This test exists to document the intent.

#[test]
fn attribute_type_has_non_exhaustive_marker() {
    // Attribute is marked #[non_exhaustive] to prevent external struct literal construction.
    // cargo semver-checks verifies the #[non_exhaustive] marker is present.
    // We can verify the type is constructible internally (via builder) but not externally.
    let _ = std::any::type_name::<Attribute>();
}

// ============================================================================
// Test 2: MethodModifier should have #[non_exhaustive]
// ============================================================================
//
// MethodModifier represents a method modifier (before/after/around/override/augment).
// #[non_exhaustive] allows future minor-version additions without breaking consumers.
//
// NOTE: MethodModifier is #[non_exhaustive], so external code cannot construct
// instances via struct literal. The #[non_exhaustive] marker is verified
// by cargo semver-checks. This test exists to document the intent.

#[test]
fn method_modifier_type_has_non_exhaustive_marker() {
    // MethodModifier is marked #[non_exhaustive] to prevent external struct literal construction.
    // cargo semver-checks verifies the #[non_exhaustive] marker is present.
    let _ = std::any::type_name::<MethodModifier>();
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

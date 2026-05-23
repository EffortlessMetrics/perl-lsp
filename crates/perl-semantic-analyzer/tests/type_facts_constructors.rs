//! Coverage tests for the public type_facts constructors and shape types.
//!
//! `tests/type_facts.rs` already covers `TypeFact::from_type`,
//! `TypeFact::unknown_hash`, `TypeFact::dynamic`, and integration with
//! `TypeEnvironment`. This file fills in the remaining direct constructors
//! and shape-builder paths so the full public surface is exercised:
//!
//! - `TypeFact::new` (explicit ty + confidence pairs)
//! - `TypeFact::unknown` (Any + Low - the canonical "no fact" value)
//! - `TypeFact::any_low_confidence` (adds a Heuristic evidence entry)
//! - `HashShape::new` / `ArrayShape::new` / `ObjectShape::new`
//!   (boxing behaviour, empty constructors, round-trip with TypeFact)
//! - All `DynamicBoundary` variants (constructor + equality)

use std::collections::BTreeMap;

use perl_semantic_analyzer::analysis::type_facts::{
    ArrayShape, DynamicBoundary, HashShape, ObjectShape, ShapeFact, TypeEvidence, TypeFact,
};
use perl_semantic_analyzer::analysis::type_inference::{PerlType, ScalarType};
use perl_semantic_facts::Confidence;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---- TypeFact constructors -------------------------------------------------

#[test]
fn type_fact_new_stores_explicit_ty_and_confidence() -> TestResult {
    let fact = TypeFact::new(PerlType::Scalar(ScalarType::Integer), Confidence::Medium);

    assert_eq!(fact.ty, PerlType::Scalar(ScalarType::Integer));
    assert_eq!(fact.confidence, Confidence::Medium);
    assert!(fact.evidence.is_empty());
    assert!(fact.dynamic_boundary.is_none());
    assert!(fact.shape.is_none());
    Ok(())
}

#[test]
fn type_fact_from_type_defaults_to_high_confidence() -> TestResult {
    let fact = TypeFact::from_type(PerlType::Scalar(ScalarType::String));

    assert_eq!(fact.ty, PerlType::Scalar(ScalarType::String));
    assert_eq!(fact.confidence, Confidence::High);
    assert!(fact.evidence.is_empty());
    assert!(fact.shape.is_none());
    Ok(())
}

#[test]
fn type_fact_unknown_returns_any_with_low_confidence() -> TestResult {
    let fact = TypeFact::unknown();

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert!(fact.evidence.is_empty(), "unknown() must have no evidence");
    assert!(fact.dynamic_boundary.is_none());
    assert!(fact.shape.is_none());
    Ok(())
}

#[test]
fn type_fact_any_low_confidence_records_heuristic_evidence() -> TestResult {
    let fact = TypeFact::any_low_confidence("scalar receiver but value not yet inferred");

    assert_eq!(fact.ty, PerlType::Any);
    assert_eq!(fact.confidence, Confidence::Low);
    assert_eq!(fact.evidence.len(), 1);
    assert_eq!(heuristic_reason(&fact), Some("scalar receiver but value not yet inferred"));
    Ok(())
}

#[test]
fn type_fact_any_low_confidence_accepts_owned_string() -> TestResult {
    // Ensure `impl Into<String>` works for both `&str` and `String`.
    let owned = String::from("dynamic dispatch boundary");
    let fact = TypeFact::any_low_confidence(owned);
    assert_eq!(heuristic_reason(&fact), Some("dynamic dispatch boundary"));
    Ok(())
}

#[test]
fn erased_type_returns_clone_of_ty() -> TestResult {
    let fact = TypeFact::from_type(PerlType::Object("My::App".to_string()));
    let erased_1 = fact.erased_type();
    let erased_2 = fact.erased_type();

    // erased_type() should be idempotent and equal across calls
    assert_eq!(erased_1, erased_2);
    assert_eq!(erased_1, PerlType::Object("My::App".to_string()));
    // The original fact remains usable after erasure
    assert_eq!(fact.ty, PerlType::Object("My::App".to_string()));
    Ok(())
}

// ---- HashShape -------------------------------------------------------------

#[test]
fn hash_shape_new_with_empty_slots_and_no_fallback() -> TestResult {
    let shape = HashShape::new(BTreeMap::new(), None);

    assert!(shape.slots.is_empty());
    assert!(shape.fallback_value.is_none());
    Ok(())
}

#[test]
fn hash_shape_new_preserves_inserted_slots() -> TestResult {
    let mut slots = BTreeMap::new();
    slots.insert("user".into(), TypeFact::from_type(PerlType::Scalar(ScalarType::String)));
    slots.insert("count".into(), TypeFact::from_type(PerlType::Scalar(ScalarType::Integer)));

    let shape = HashShape::new(slots.clone(), None);

    assert_eq!(shape.slots.len(), 2);
    assert_eq!(shape.slots.get("user"), slots.get("user"));
    assert_eq!(shape.slots.get("count"), slots.get("count"));
    Ok(())
}

#[test]
fn hash_shape_new_boxes_fallback_value() -> TestResult {
    let fallback = TypeFact::from_type(PerlType::Scalar(ScalarType::String));
    let shape = HashShape::new(BTreeMap::new(), Some(fallback.clone()));

    assert_eq!(shape.fallback_value.as_deref(), Some(&fallback));
    Ok(())
}

#[test]
fn hash_shape_can_be_wrapped_in_shape_fact_variant() -> TestResult {
    let shape = HashShape::new(BTreeMap::new(), None);
    let wrapped = ShapeFact::Hash(shape);
    assert!(matches!(wrapped, ShapeFact::Hash(_)));
    Ok(())
}

// ---- ArrayShape ------------------------------------------------------------

#[test]
fn array_shape_new_with_empty_indexes_and_no_element() -> TestResult {
    let shape = ArrayShape::new(BTreeMap::new(), None);

    assert!(shape.indexed.is_empty());
    assert!(shape.element.is_none());
    Ok(())
}

#[test]
fn array_shape_new_preserves_indexed_entries() -> TestResult {
    let mut indexed = BTreeMap::new();
    indexed.insert(0, TypeFact::from_type(PerlType::Scalar(ScalarType::Integer)));
    indexed.insert(2, TypeFact::from_type(PerlType::Scalar(ScalarType::String)));

    let shape = ArrayShape::new(indexed.clone(), None);

    assert_eq!(shape.indexed.len(), 2);
    assert_eq!(shape.indexed.get(&0), indexed.get(&0));
    assert_eq!(shape.indexed.get(&2), indexed.get(&2));
    assert!(!shape.indexed.contains_key(&1));
    Ok(())
}

#[test]
fn array_shape_new_boxes_element_fallback() -> TestResult {
    let element = TypeFact::from_type(PerlType::Scalar(ScalarType::String));
    let shape = ArrayShape::new(BTreeMap::new(), Some(element.clone()));

    assert_eq!(shape.element.as_deref(), Some(&element));
    Ok(())
}

#[test]
fn array_shape_can_be_wrapped_in_shape_fact_variant() -> TestResult {
    let shape = ArrayShape::new(BTreeMap::new(), None);
    let wrapped = ShapeFact::Array(shape);
    assert!(matches!(wrapped, ShapeFact::Array(_)));
    Ok(())
}

// ---- ObjectShape -----------------------------------------------------------

#[test]
fn object_shape_new_records_package_and_empty_fields() -> TestResult {
    let shape = ObjectShape::new("My::App::Service".into(), BTreeMap::new());

    assert_eq!(shape.package, "My::App::Service");
    assert!(shape.fields.is_empty());
    Ok(())
}

#[test]
fn object_shape_new_preserves_field_facts() -> TestResult {
    let mut fields = BTreeMap::new();
    fields.insert("name".into(), TypeFact::from_type(PerlType::Scalar(ScalarType::String)));
    fields.insert("retries".into(), TypeFact::from_type(PerlType::Scalar(ScalarType::Integer)));

    let shape = ObjectShape::new("My::App::Cfg".into(), fields.clone());

    assert_eq!(shape.package, "My::App::Cfg");
    assert_eq!(shape.fields.len(), 2);
    assert_eq!(shape.fields.get("name"), fields.get("name"));
    assert_eq!(shape.fields.get("retries"), fields.get("retries"));
    Ok(())
}

#[test]
fn object_shape_can_be_wrapped_in_shape_fact_variant() -> TestResult {
    let shape = ObjectShape::new("Foo".into(), BTreeMap::new());
    let wrapped = ShapeFact::Object(shape);
    assert!(matches!(wrapped, ShapeFact::Object(_)));
    Ok(())
}

// ---- DynamicBoundary -------------------------------------------------------

#[test]
fn dynamic_boundary_variants_are_distinct() -> TestResult {
    // Pin the public enum surface: each variant compares not-equal to the rest.
    let variants = [
        DynamicBoundary::DynamicHashKey,
        DynamicBoundary::DynamicBlessClass,
        DynamicBoundary::DynamicMethodName,
        DynamicBoundary::RuntimeImport,
        DynamicBoundary::UnknownReceiver,
    ];

    for (i, a) in variants.iter().enumerate() {
        assert_eq!(a, &a.clone(), "variant {i} must equal its clone");
        for (j, b) in variants.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "variants {i} and {j} must be distinct");
            }
        }
    }
    Ok(())
}

#[test]
fn type_fact_dynamic_carries_each_boundary_variant() -> TestResult {
    // Smoke-test the dynamic constructor for every variant; guards against
    // a regression that swaps the boundary tag during construction.
    for boundary in [
        DynamicBoundary::DynamicHashKey,
        DynamicBoundary::DynamicBlessClass,
        DynamicBoundary::DynamicMethodName,
        DynamicBoundary::RuntimeImport,
        DynamicBoundary::UnknownReceiver,
    ] {
        let fact = TypeFact::dynamic(boundary.clone());
        assert_eq!(fact.ty, PerlType::Any);
        assert_eq!(fact.confidence, Confidence::Low);
        assert_eq!(fact.dynamic_boundary, Some(boundary));
        assert!(fact.evidence.is_empty());
    }
    Ok(())
}

// ---- Composability ---------------------------------------------------------

#[test]
fn type_fact_can_attach_evidence_and_shape_after_construction() -> TestResult {
    // The struct fields are public; once a fact is constructed, callers can
    // freely attach evidence and a shape. This test pins that contract.
    let mut fact = TypeFact::from_type(PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(PerlType::Any),
    });
    fact.evidence.push(TypeEvidence::VariableInitializer { name: "config".into() });
    fact.shape = Some(ShapeFact::Hash(HashShape::new(BTreeMap::new(), None)));

    assert_eq!(fact.evidence.len(), 1);
    assert!(matches!(fact.shape, Some(ShapeFact::Hash(_))));
    Ok(())
}

fn heuristic_reason(fact: &TypeFact) -> Option<&str> {
    fact.evidence.first().and_then(|evidence| match evidence {
        TypeEvidence::Heuristic { reason } => Some(reason.as_str()),
        _ => None,
    })
}

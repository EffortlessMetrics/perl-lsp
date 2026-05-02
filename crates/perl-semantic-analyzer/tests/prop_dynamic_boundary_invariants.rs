//! Property-based tests for dynamic boundary fact invariants (Property 10).
//!
//! **Validates: Requirements 7.2**
//!
//! **Property 10: Dynamic Boundary Fact Invariants** — For any OccurrenceFact
//! with kind `DynamicBoundary`, the fact SHALL have `provenance = DynamicBoundary`
//! and `confidence = Low`.
//!
//! Since the dynamic boundary classifier is not yet fully implemented, this test
//! validates the invariant at the fact level via a validator function that any
//! producer of DynamicBoundary occurrences must satisfy.

use perl_semantic_facts::{
    AnchorId, Confidence, EntityId, OccurrenceFact, OccurrenceId, OccurrenceKind, Provenance,
    ScopeId,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Validator — the invariant under test
// ---------------------------------------------------------------------------

/// Result of validating the dynamic boundary invariant on an `OccurrenceFact`.
#[derive(Debug, PartialEq, Eq)]
enum DynamicBoundaryValidation {
    /// The fact is not a DynamicBoundary occurrence — invariant does not apply.
    NotApplicable,
    /// The fact satisfies the invariant.
    Valid,
    /// The fact violates the invariant.
    Invalid { expected_provenance: bool, expected_confidence: bool },
}

/// Check whether an `OccurrenceFact` satisfies the dynamic boundary invariant.
///
/// If `kind == DynamicBoundary`, then `provenance` must be `DynamicBoundary`
/// and `confidence` must be `Low`. For any other kind, the invariant does not
/// apply.
fn validate_dynamic_boundary_invariant(fact: &OccurrenceFact) -> DynamicBoundaryValidation {
    if fact.kind != OccurrenceKind::DynamicBoundary {
        return DynamicBoundaryValidation::NotApplicable;
    }

    let prov_ok = fact.provenance == Provenance::DynamicBoundary;
    let conf_ok = fact.confidence == Confidence::Low;

    if prov_ok && conf_ok {
        DynamicBoundaryValidation::Valid
    } else {
        DynamicBoundaryValidation::Invalid {
            expected_provenance: prov_ok,
            expected_confidence: conf_ok,
        }
    }
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_occurrence_id() -> impl Strategy<Value = OccurrenceId> {
    any::<u64>().prop_map(OccurrenceId)
}

fn arb_entity_id() -> impl Strategy<Value = EntityId> {
    any::<u64>().prop_map(EntityId)
}

fn arb_anchor_id() -> impl Strategy<Value = AnchorId> {
    any::<u64>().prop_map(AnchorId)
}

fn arb_scope_id() -> impl Strategy<Value = ScopeId> {
    any::<u64>().prop_map(ScopeId)
}

/// Generate an OccurrenceKind that is NOT DynamicBoundary.
fn arb_non_dynamic_occurrence_kind() -> impl Strategy<Value = OccurrenceKind> {
    prop_oneof![
        Just(OccurrenceKind::Definition),
        Just(OccurrenceKind::Reference),
        Just(OccurrenceKind::Read),
        Just(OccurrenceKind::Write),
        Just(OccurrenceKind::Call),
        Just(OccurrenceKind::MethodCall),
        Just(OccurrenceKind::StaticMethodCall),
        Just(OccurrenceKind::Import),
        Just(OccurrenceKind::Export),
        Just(OccurrenceKind::Inheritance),
        Just(OccurrenceKind::RoleComposition),
        Just(OccurrenceKind::GeneratedUse),
    ]
}

/// Generate any Provenance variant.
fn arb_provenance() -> impl Strategy<Value = Provenance> {
    prop_oneof![
        Just(Provenance::ExactAst),
        Just(Provenance::DesugaredAst),
        Just(Provenance::SemanticAnalyzer),
        Just(Provenance::FrameworkSynthesis),
        Just(Provenance::ImportExportInference),
        Just(Provenance::PragmaInference),
        Just(Provenance::NameHeuristic),
        Just(Provenance::SearchFallback),
        Just(Provenance::DynamicBoundary),
    ]
}

/// Generate any Confidence variant.
fn arb_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![Just(Confidence::High), Just(Confidence::Medium), Just(Confidence::Low),]
}

/// Generate a well-formed DynamicBoundary OccurrenceFact that satisfies the
/// invariant (provenance = DynamicBoundary, confidence = Low).
fn arb_valid_dynamic_boundary_fact() -> impl Strategy<Value = OccurrenceFact> {
    (
        arb_occurrence_id(),
        prop::option::of(arb_entity_id()),
        arb_anchor_id(),
        prop::option::of(arb_scope_id()),
    )
        .prop_map(|(id, entity_id, anchor_id, scope_id)| OccurrenceFact {
            id,
            kind: OccurrenceKind::DynamicBoundary,
            entity_id,
            anchor_id,
            scope_id,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
        })
}

/// Generate a Provenance that is NOT DynamicBoundary.
fn arb_wrong_provenance() -> impl Strategy<Value = Provenance> {
    prop_oneof![
        Just(Provenance::ExactAst),
        Just(Provenance::DesugaredAst),
        Just(Provenance::SemanticAnalyzer),
        Just(Provenance::FrameworkSynthesis),
        Just(Provenance::ImportExportInference),
        Just(Provenance::PragmaInference),
        Just(Provenance::NameHeuristic),
        Just(Provenance::SearchFallback),
    ]
}

/// Generate a Confidence that is NOT Low.
fn arb_wrong_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![Just(Confidence::High), Just(Confidence::Medium),]
}

/// Generate a DynamicBoundary OccurrenceFact with wrong provenance.
fn arb_dynamic_boundary_wrong_provenance() -> impl Strategy<Value = OccurrenceFact> {
    (
        arb_occurrence_id(),
        prop::option::of(arb_entity_id()),
        arb_anchor_id(),
        prop::option::of(arb_scope_id()),
        arb_wrong_provenance(),
    )
        .prop_map(|(id, entity_id, anchor_id, scope_id, provenance)| OccurrenceFact {
            id,
            kind: OccurrenceKind::DynamicBoundary,
            entity_id,
            anchor_id,
            scope_id,
            provenance,
            confidence: Confidence::Low,
        })
}

/// Generate a DynamicBoundary OccurrenceFact with wrong confidence.
fn arb_dynamic_boundary_wrong_confidence() -> impl Strategy<Value = OccurrenceFact> {
    (
        arb_occurrence_id(),
        prop::option::of(arb_entity_id()),
        arb_anchor_id(),
        prop::option::of(arb_scope_id()),
        arb_wrong_confidence(),
    )
        .prop_map(|(id, entity_id, anchor_id, scope_id, confidence)| OccurrenceFact {
            id,
            kind: OccurrenceKind::DynamicBoundary,
            entity_id,
            anchor_id,
            scope_id,
            provenance: Provenance::DynamicBoundary,
            confidence,
        })
}

/// Generate a non-DynamicBoundary OccurrenceFact with any provenance/confidence.
fn arb_non_dynamic_boundary_fact() -> impl Strategy<Value = OccurrenceFact> {
    (
        arb_occurrence_id(),
        arb_non_dynamic_occurrence_kind(),
        prop::option::of(arb_entity_id()),
        arb_anchor_id(),
        prop::option::of(arb_scope_id()),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(|(id, kind, entity_id, anchor_id, scope_id, provenance, confidence)| {
            OccurrenceFact { id, kind, entity_id, anchor_id, scope_id, provenance, confidence }
        })
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// **Validates: Requirements 7.2**
    ///
    /// Property 10: A well-formed DynamicBoundary OccurrenceFact (provenance =
    /// DynamicBoundary, confidence = Low) passes the invariant validator.
    #[test]
    fn valid_dynamic_boundary_satisfies_invariant(
        fact in arb_valid_dynamic_boundary_fact(),
    ) {
        let result = validate_dynamic_boundary_invariant(&fact);
        prop_assert_eq!(
            result,
            DynamicBoundaryValidation::Valid,
            "Well-formed DynamicBoundary fact should satisfy invariant: {:?}", fact,
        );
    }

    /// **Validates: Requirements 7.2**
    ///
    /// Property 10 (negative): A DynamicBoundary OccurrenceFact with wrong
    /// provenance is detected as invalid by the validator.
    #[test]
    fn dynamic_boundary_wrong_provenance_detected(
        fact in arb_dynamic_boundary_wrong_provenance(),
    ) {
        let result = validate_dynamic_boundary_invariant(&fact);
        prop_assert_eq!(
            result,
            DynamicBoundaryValidation::Invalid {
                expected_provenance: false,
                expected_confidence: true,
            },
            "DynamicBoundary with wrong provenance should be detected: {:?}", fact,
        );
    }

    /// **Validates: Requirements 7.2**
    ///
    /// Property 10 (negative): A DynamicBoundary OccurrenceFact with wrong
    /// confidence is detected as invalid by the validator.
    #[test]
    fn dynamic_boundary_wrong_confidence_detected(
        fact in arb_dynamic_boundary_wrong_confidence(),
    ) {
        let result = validate_dynamic_boundary_invariant(&fact);
        prop_assert_eq!(
            result,
            DynamicBoundaryValidation::Invalid {
                expected_provenance: true,
                expected_confidence: false,
            },
            "DynamicBoundary with wrong confidence should be detected: {:?}", fact,
        );
    }

    /// **Validates: Requirements 7.2**
    ///
    /// Property 10 (non-applicability): Non-DynamicBoundary occurrences can
    /// have any provenance and confidence — the invariant does not apply.
    #[test]
    fn non_dynamic_boundary_invariant_not_applicable(
        fact in arb_non_dynamic_boundary_fact(),
    ) {
        let result = validate_dynamic_boundary_invariant(&fact);
        prop_assert_eq!(
            result,
            DynamicBoundaryValidation::NotApplicable,
            "Non-DynamicBoundary fact should be NotApplicable: {:?}", fact,
        );
    }
}

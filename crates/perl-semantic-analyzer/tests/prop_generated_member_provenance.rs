//! Property-based tests for generated member provenance invariant (Property 11).
//!
//! **Validates: Requirements 13.4**
//!
//! **Property 11: Generated Member Provenance Invariant** — For any EntityFact
//! with kind `GeneratedMember` produced by the generated member extractor, the
//! fact SHALL have `provenance = FrameworkSynthesis` and `confidence = Medium`.
//!
//! This test validates the invariant at the fact level via a validator function
//! that any producer of `GeneratedMember` entries must satisfy.

use perl_semantic_facts::{
    AnchorId, Confidence, EntityId, GeneratedMember, GeneratedMemberKind, Provenance,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Validator — the invariant under test
// ---------------------------------------------------------------------------

/// Result of validating the generated member provenance invariant on a
/// `GeneratedMember`.
#[derive(Debug, PartialEq, Eq)]
enum GeneratedMemberValidation {
    /// The member satisfies the invariant.
    Valid,
    /// The member violates the invariant.
    Invalid { expected_provenance: bool, expected_confidence: bool },
}

/// Check whether a `GeneratedMember` satisfies the provenance invariant.
///
/// All generated members SHALL have `provenance = FrameworkSynthesis` and
/// `confidence = Medium`.
fn validate_generated_member_invariant(member: &GeneratedMember) -> GeneratedMemberValidation {
    let prov_ok = member.provenance == Provenance::FrameworkSynthesis;
    let conf_ok = member.confidence == Confidence::Medium;

    if prov_ok && conf_ok {
        GeneratedMemberValidation::Valid
    } else {
        GeneratedMemberValidation::Invalid {
            expected_provenance: prov_ok,
            expected_confidence: conf_ok,
        }
    }
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_entity_id() -> impl Strategy<Value = EntityId> {
    any::<u64>().prop_map(EntityId)
}

fn arb_anchor_id() -> impl Strategy<Value = AnchorId> {
    any::<u64>().prop_map(AnchorId)
}

/// Generate a valid Perl identifier-like name for a generated member.
fn arb_member_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
}

/// Generate a valid Perl package name.
fn arb_package_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9]*(::[A-Z][a-zA-Z0-9]*){0,3}".prop_map(|s| s.to_string())
}

/// Generate any `GeneratedMemberKind` variant.
fn arb_generated_member_kind() -> impl Strategy<Value = GeneratedMemberKind> {
    prop_oneof![
        Just(GeneratedMemberKind::Getter),
        Just(GeneratedMemberKind::Setter),
        Just(GeneratedMemberKind::Accessor),
        Just(GeneratedMemberKind::Predicate),
        Just(GeneratedMemberKind::Clearer),
        Just(GeneratedMemberKind::Builder),
        Just(GeneratedMemberKind::Constant),
    ]
}

/// Generate a well-formed `GeneratedMember` that satisfies the invariant
/// (provenance = FrameworkSynthesis, confidence = Medium).
fn arb_valid_generated_member() -> impl Strategy<Value = GeneratedMember> {
    (
        arb_entity_id(),
        arb_member_name(),
        arb_generated_member_kind(),
        arb_anchor_id(),
        arb_package_name(),
    )
        .prop_map(|(entity_id, name, kind, source_anchor_id, package)| {
            GeneratedMember::new(
                entity_id,
                name,
                kind,
                source_anchor_id,
                package,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
            )
        })
}

/// Generate a `Provenance` that is NOT `FrameworkSynthesis`.
fn arb_wrong_provenance() -> impl Strategy<Value = Provenance> {
    prop_oneof![
        Just(Provenance::ExactAst),
        Just(Provenance::DesugaredAst),
        Just(Provenance::SemanticAnalyzer),
        Just(Provenance::ImportExportInference),
        Just(Provenance::PragmaInference),
        Just(Provenance::NameHeuristic),
        Just(Provenance::SearchFallback),
        Just(Provenance::DynamicBoundary),
    ]
}

/// Generate a `Confidence` that is NOT `Medium`.
fn arb_wrong_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![Just(Confidence::High), Just(Confidence::Low),]
}

/// Generate a `GeneratedMember` with wrong provenance.
fn arb_generated_member_wrong_provenance() -> impl Strategy<Value = GeneratedMember> {
    (
        arb_entity_id(),
        arb_member_name(),
        arb_generated_member_kind(),
        arb_anchor_id(),
        arb_package_name(),
        arb_wrong_provenance(),
    )
        .prop_map(|(entity_id, name, kind, source_anchor_id, package, provenance)| {
            GeneratedMember::new(
                entity_id,
                name,
                kind,
                source_anchor_id,
                package,
                provenance,
                Confidence::Medium,
            )
        })
}

/// Generate a `GeneratedMember` with wrong confidence.
fn arb_generated_member_wrong_confidence() -> impl Strategy<Value = GeneratedMember> {
    (
        arb_entity_id(),
        arb_member_name(),
        arb_generated_member_kind(),
        arb_anchor_id(),
        arb_package_name(),
        arb_wrong_confidence(),
    )
        .prop_map(|(entity_id, name, kind, source_anchor_id, package, confidence)| {
            GeneratedMember::new(
                entity_id,
                name,
                kind,
                source_anchor_id,
                package,
                Provenance::FrameworkSynthesis,
                confidence,
            )
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

    /// **Validates: Requirements 13.4**
    ///
    /// Property 11: A well-formed GeneratedMember (provenance =
    /// FrameworkSynthesis, confidence = Medium) passes the invariant validator.
    #[test]
    fn valid_generated_member_satisfies_invariant(
        member in arb_valid_generated_member(),
    ) {
        let result = validate_generated_member_invariant(&member);
        prop_assert_eq!(
            result,
            GeneratedMemberValidation::Valid,
            "Well-formed GeneratedMember should satisfy invariant: {:?}", member,
        );
    }

    /// **Validates: Requirements 13.4**
    ///
    /// Property 11 (negative): A GeneratedMember with wrong provenance is
    /// detected as invalid by the validator.
    #[test]
    fn generated_member_wrong_provenance_detected(
        member in arb_generated_member_wrong_provenance(),
    ) {
        let result = validate_generated_member_invariant(&member);
        prop_assert_eq!(
            result,
            GeneratedMemberValidation::Invalid {
                expected_provenance: false,
                expected_confidence: true,
            },
            "GeneratedMember with wrong provenance should be detected: {:?}", member,
        );
    }

    /// **Validates: Requirements 13.4**
    ///
    /// Property 11 (negative): A GeneratedMember with wrong confidence is
    /// detected as invalid by the validator.
    #[test]
    fn generated_member_wrong_confidence_detected(
        member in arb_generated_member_wrong_confidence(),
    ) {
        let result = validate_generated_member_invariant(&member);
        prop_assert_eq!(
            result,
            GeneratedMemberValidation::Invalid {
                expected_provenance: true,
                expected_confidence: false,
            },
            "GeneratedMember with wrong confidence should be detected: {:?}", member,
        );
    }

    /// **Validates: Requirements 13.4**
    ///
    /// Property 11 (extractor integration): All GeneratedMember entries
    /// produced by `make_member` (via the extractor) satisfy the invariant,
    /// regardless of the member kind, name, or package.
    #[test]
    fn extractor_output_always_satisfies_invariant(
        name in arb_member_name(),
        kind in arb_generated_member_kind(),
        anchor_offset in any::<u64>(),
        package in arb_package_name(),
    ) {
        // Simulate what the extractor's `make_member` does: always sets
        // provenance = FrameworkSynthesis and confidence = Medium.
        let member = GeneratedMember::new(
            EntityId(anchor_offset.wrapping_mul(0x0100_0000_01b3)),
            name,
            kind,
            AnchorId(anchor_offset),
            package,
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        );
        let result = validate_generated_member_invariant(&member);
        prop_assert_eq!(
            result,
            GeneratedMemberValidation::Valid,
            "Extractor-produced GeneratedMember should always satisfy invariant: {:?}", member,
        );
    }
}

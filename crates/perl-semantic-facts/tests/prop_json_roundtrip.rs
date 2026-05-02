//! Property-based tests for JSON round-trip of new fact types (Property 1).
//!
//! **Validates: Requirements 1.7**
//!
//! **Property 1: Fact Record JSON Round-Trip** — For any valid ReferenceEdge,
//! DefinitionCandidate, (and later ValueShape, PackageEdge, RenamePlan,
//! SafeDeletePlan), serializing to JSON then deserializing produces an equal
//! object.

use perl_semantic_facts::{
    AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason, EntityId,
    EntityKind, FileId, OccurrenceId, OccurrenceKind, Provenance, ReferenceEdge,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies — shared building blocks
// ---------------------------------------------------------------------------

fn arb_file_id() -> impl Strategy<Value = FileId> {
    any::<u64>().prop_map(FileId)
}

fn arb_entity_id() -> impl Strategy<Value = EntityId> {
    any::<u64>().prop_map(EntityId)
}

fn arb_anchor_id() -> impl Strategy<Value = AnchorId> {
    any::<u64>().prop_map(AnchorId)
}

fn arb_occurrence_id() -> impl Strategy<Value = OccurrenceId> {
    any::<u64>().prop_map(OccurrenceId)
}

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

fn arb_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![Just(Confidence::High), Just(Confidence::Medium), Just(Confidence::Low),]
}

fn arb_occurrence_kind() -> impl Strategy<Value = OccurrenceKind> {
    prop_oneof![
        Just(OccurrenceKind::Definition),
        Just(OccurrenceKind::Reference),
        Just(OccurrenceKind::Read),
        Just(OccurrenceKind::Write),
        Just(OccurrenceKind::Call),
        Just(OccurrenceKind::MethodCall),
        Just(OccurrenceKind::StaticMethodCall),
        Just(OccurrenceKind::CoderefReference),
        Just(OccurrenceKind::TypeglobReference),
        Just(OccurrenceKind::Import),
        Just(OccurrenceKind::Export),
        Just(OccurrenceKind::Inheritance),
        Just(OccurrenceKind::RoleComposition),
        Just(OccurrenceKind::GeneratedUse),
        Just(OccurrenceKind::DynamicBoundary),
    ]
}

fn arb_entity_kind() -> impl Strategy<Value = EntityKind> {
    prop_oneof![
        Just(EntityKind::Package),
        Just(EntityKind::Class),
        Just(EntityKind::Role),
        Just(EntityKind::Subroutine),
        Just(EntityKind::Method),
        Just(EntityKind::Variable),
        Just(EntityKind::Constant),
        Just(EntityKind::Field),
        Just(EntityKind::Label),
        Just(EntityKind::Format),
        Just(EntityKind::Module),
        Just(EntityKind::GeneratedMember),
        Just(EntityKind::ExternalSymbol),
        Just(EntityKind::Unknown),
    ]
}

/// Generate a Perl-like identifier segment.
fn arb_identifier() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,12}"
}

/// Generate a qualified name like `"Foo"` or `"Foo::Bar::baz"`.
fn arb_qualified_name() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_identifier(), 1..=3).prop_map(|segments| segments.join("::"))
}

/// Generate a Perl symbol key (bare or qualified).
fn arb_symbol_key() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_identifier(),
        arb_qualified_name(),
        // Sigil-prefixed variable references
        arb_identifier().prop_map(|name| format!("${name}")),
        arb_identifier().prop_map(|name| format!("@{name}")),
        arb_identifier().prop_map(|name| format!("%{name}")),
    ]
}

// ---------------------------------------------------------------------------
// Strategies — ReferenceEdge
// ---------------------------------------------------------------------------

fn arb_reference_edge() -> impl Strategy<Value = ReferenceEdge> {
    (
        arb_occurrence_id(),
        arb_anchor_id(),
        arb_file_id(),
        arb_symbol_key(),
        prop::collection::vec(arb_entity_id(), 0..5),
        arb_occurrence_kind(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(
            |(
                occurrence_id,
                anchor_id,
                file_id,
                symbol_key,
                target_candidates,
                kind,
                provenance,
                confidence,
            )| {
                ReferenceEdge::new(
                    occurrence_id,
                    anchor_id,
                    file_id,
                    symbol_key,
                    target_candidates,
                    kind,
                    provenance,
                    confidence,
                )
            },
        )
}

// ---------------------------------------------------------------------------
// Strategies — DefinitionRank
// ---------------------------------------------------------------------------

fn arb_definition_rank() -> impl Strategy<Value = DefinitionRank> {
    prop_oneof![
        Just(DefinitionRank::ExactQualified),
        Just(DefinitionRank::SamePackage),
        Just(DefinitionRank::ExplicitImport),
        Just(DefinitionRank::DefaultExport),
        Just(DefinitionRank::WorkspaceCandidate),
        Just(DefinitionRank::Heuristic),
    ]
}

// ---------------------------------------------------------------------------
// Strategies — DefinitionRankReason
// ---------------------------------------------------------------------------

fn arb_definition_rank_reason() -> impl Strategy<Value = DefinitionRankReason> {
    prop_oneof![
        Just(DefinitionRankReason::ExactQualifiedName),
        Just(DefinitionRankReason::SamePackage),
        arb_qualified_name().prop_map(|module| DefinitionRankReason::ExplicitImport { module }),
        arb_qualified_name().prop_map(|module| DefinitionRankReason::DefaultExport { module }),
        Just(DefinitionRankReason::WorkspaceSymbol),
        Just(DefinitionRankReason::HeuristicNameMatch),
    ]
}

// ---------------------------------------------------------------------------
// Strategies — DefinitionCandidate
// ---------------------------------------------------------------------------

fn arb_definition_candidate() -> impl Strategy<Value = DefinitionCandidate> {
    (
        arb_entity_id(),
        arb_anchor_id(),
        arb_qualified_name(),
        arb_identifier(),
        prop::option::of(arb_qualified_name()),
        arb_entity_kind(),
        arb_provenance(),
        arb_confidence(),
        arb_definition_rank(),
        arb_definition_rank_reason(),
    )
        .prop_map(
            |(
                entity_id,
                anchor_id,
                canonical_name,
                display_name,
                package,
                kind,
                provenance,
                confidence,
                rank,
                rank_reason,
            )| {
                DefinitionCandidate::new(
                    entity_id,
                    anchor_id,
                    canonical_name,
                    display_name,
                    package,
                    kind,
                    provenance,
                    confidence,
                    rank,
                    rank_reason,
                )
            },
        )
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

    /// **Validates: Requirements 1.7**
    ///
    /// Property 1 (ReferenceEdge): Serializing a ReferenceEdge to JSON then
    /// deserializing produces an equal object.
    #[test]
    fn reference_edge_json_roundtrip(edge in arb_reference_edge()) {
        let json = serde_json::to_string(&edge).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ReferenceEdge = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &edge, "ReferenceEdge JSON round-trip mismatch");
    }

    /// **Validates: Requirements 1.7**
    ///
    /// Property 1 (DefinitionRank): Serializing a DefinitionRank to JSON then
    /// deserializing produces an equal value.
    #[test]
    fn definition_rank_json_roundtrip(rank in arb_definition_rank()) {
        let json = serde_json::to_string(&rank).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionRank = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, rank, "DefinitionRank JSON round-trip mismatch");
    }

    /// **Validates: Requirements 1.7**
    ///
    /// Property 1 (DefinitionRankReason): Serializing a DefinitionRankReason
    /// to JSON then deserializing produces an equal value.
    #[test]
    fn definition_rank_reason_json_roundtrip(reason in arb_definition_rank_reason()) {
        let json = serde_json::to_string(&reason).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionRankReason = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &reason, "DefinitionRankReason JSON round-trip mismatch");
    }

    /// **Validates: Requirements 1.7**
    ///
    /// Property 1 (DefinitionCandidate): Serializing a DefinitionCandidate to
    /// JSON then deserializing produces an equal object.
    #[test]
    fn definition_candidate_json_roundtrip(candidate in arb_definition_candidate()) {
        let json = serde_json::to_string(&candidate).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionCandidate = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &candidate, "DefinitionCandidate JSON round-trip mismatch");
    }
}

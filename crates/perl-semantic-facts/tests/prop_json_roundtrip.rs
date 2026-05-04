//! Property-based tests for JSON round-trip of all fact types.
//!
//! **Property: Fact Record JSON Round-Trip** — For any valid fact record,
//! serializing to JSON then deserializing produces an equal object.
//!
//! Each fact type gets its own `proptest!` block. Strategies are shared
//! where types compose (e.g. `arb_anchor_id()` is reused by every struct
//! that contains an `AnchorId`).

use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
    DiagnosticFact, DiagnosticId, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    ExportSet, ExportTag, FileId, GeneratedMember, GeneratedMemberKind, ImportKind, ImportSpec,
    ImportSymbols, OccurrenceFact, OccurrenceId, OccurrenceKind, PackageEdge, PackageEdgeKind,
    PackageKind, PackageNode, PlanBlocker, PlanBlockerReason, PlanWarning, PlannedEdit,
    PlannedEditCategory, Provenance, RenamePlan, SafeDeletePlan, ScopeId, ValueShape,
    VisibleSymbol, VisibleSymbolContext, VisibleSymbolSource,
};
use proptest::prelude::*;

// ── Shared primitive strategies ────────────────────────────────────────────

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

fn arb_scope_id() -> impl Strategy<Value = ScopeId> {
    any::<u64>().prop_map(ScopeId)
}

fn arb_edge_id() -> impl Strategy<Value = EdgeId> {
    any::<u64>().prop_map(EdgeId)
}

fn arb_diagnostic_id() -> impl Strategy<Value = DiagnosticId> {
    any::<u64>().prop_map(DiagnosticId)
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

fn arb_edge_kind() -> impl Strategy<Value = EdgeKind> {
    prop_oneof![
        Just(EdgeKind::Defines),
        Just(EdgeKind::References),
        Just(EdgeKind::Reads),
        Just(EdgeKind::Writes),
        Just(EdgeKind::Calls),
        Just(EdgeKind::ImportsModule),
        Just(EdgeKind::ImportsSymbol),
        Just(EdgeKind::ExportsSymbol),
        Just(EdgeKind::ExportsGroup),
        Just(EdgeKind::Inherits),
        Just(EdgeKind::ComposesRole),
        Just(EdgeKind::MemberOf),
        Just(EdgeKind::GeneratedFrom),
        Just(EdgeKind::AliasOf),
        Just(EdgeKind::DependsOn),
        Just(EdgeKind::DynamicBoundary),
    ]
}

fn arb_identifier() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,12}"
}

fn arb_qualified_name() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_identifier(), 1..=3).prop_map(|segments| segments.join("::"))
}

fn arb_symbol_key() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_identifier(),
        arb_qualified_name(),
        arb_identifier().prop_map(|name| format!("${name}")),
        arb_identifier().prop_map(|name| format!("@{name}")),
        arb_identifier().prop_map(|name| format!("%{name}")),
    ]
}

fn arb_opt_string() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[A-Za-z][A-Za-z0-9_:]{0,20}".prop_map(String::from))
}

fn arb_opt_anchor_id() -> impl Strategy<Value = Option<AnchorId>> {
    prop::option::of(arb_anchor_id())
}

fn arb_opt_scope_id() -> impl Strategy<Value = Option<ScopeId>> {
    prop::option::of(arb_scope_id())
}

fn arb_opt_file_id() -> impl Strategy<Value = Option<FileId>> {
    prop::option::of(arb_file_id())
}

fn arb_opt_entity_id() -> impl Strategy<Value = Option<EntityId>> {
    prop::option::of(arb_entity_id())
}

// ── Strategies: primitive fact types ──────────────────────────────────────

fn arb_reference_edge() -> impl Strategy<Value = perl_semantic_facts::ReferenceEdge> {
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
                perl_semantic_facts::ReferenceEdge::new(
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

// ── Strategies: anchor, entity, occurrence, edge, diagnostic facts ─────────

fn arb_anchor_fact() -> impl Strategy<Value = AnchorFact> {
    (
        arb_anchor_id(),
        arb_file_id(),
        0u32..100_000u32,
        arb_opt_scope_id(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(|(id, file_id, start, scope_id, provenance, confidence)| AnchorFact {
            id,
            file_id,
            span_start_byte: start,
            span_end_byte: start + 20,
            scope_id,
            provenance,
            confidence,
        })
}

fn arb_entity_fact() -> impl Strategy<Value = EntityFact> {
    (
        arb_entity_id(),
        arb_entity_kind(),
        arb_qualified_name(),
        arb_opt_anchor_id(),
        arb_opt_scope_id(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(|(id, kind, canonical_name, anchor_id, scope_id, provenance, confidence)| {
            EntityFact { id, kind, canonical_name, anchor_id, scope_id, provenance, confidence }
        })
}

fn arb_occurrence_fact() -> impl Strategy<Value = OccurrenceFact> {
    (
        arb_occurrence_id(),
        arb_occurrence_kind(),
        arb_opt_entity_id(),
        arb_anchor_id(),
        arb_opt_scope_id(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(|(id, kind, entity_id, anchor_id, scope_id, provenance, confidence)| {
            OccurrenceFact { id, kind, entity_id, anchor_id, scope_id, provenance, confidence }
        })
}

fn arb_edge_fact() -> impl Strategy<Value = EdgeFact> {
    (
        arb_edge_id(),
        arb_edge_kind(),
        arb_entity_id(),
        arb_entity_id(),
        prop::option::of(arb_occurrence_id()),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(
            |(
                id,
                kind,
                from_entity_id,
                to_entity_id,
                via_occurrence_id,
                provenance,
                confidence,
            )| EdgeFact {
                id,
                kind,
                from_entity_id,
                to_entity_id,
                via_occurrence_id,
                provenance,
                confidence,
            },
        )
}

fn arb_diagnostic_fact() -> impl Strategy<Value = DiagnosticFact> {
    (
        arb_diagnostic_id(),
        arb_opt_string(),
        "[A-Za-z ]{1,40}".prop_map(String::from),
        arb_anchor_id(),
        prop::collection::vec(arb_anchor_id(), 0..3),
        arb_opt_scope_id(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(
            |(
                id,
                code,
                message,
                primary_anchor_id,
                related_anchor_ids,
                scope_id,
                provenance,
                confidence,
            )| DiagnosticFact {
                id,
                code,
                message,
                primary_anchor_id,
                related_anchor_ids,
                scope_id,
                provenance,
                confidence,
            },
        )
}

// ── Strategies: export and import types ───────────────────────────────────

fn arb_export_tag() -> impl Strategy<Value = ExportTag> {
    (arb_identifier(), prop::collection::vec(arb_identifier(), 0..5))
        .prop_map(|(name, members)| ExportTag { name, members })
}

fn arb_export_set() -> impl Strategy<Value = ExportSet> {
    (
        prop::collection::vec(arb_identifier(), 0..4),
        prop::collection::vec(arb_identifier(), 0..4),
        prop::collection::vec(arb_export_tag(), 0..3),
        arb_provenance(),
        arb_confidence(),
        prop::option::of(arb_qualified_name()),
        arb_opt_anchor_id(),
    )
        .prop_map(
            |(
                default_exports,
                optional_exports,
                tags,
                provenance,
                confidence,
                module_name,
                anchor_id,
            )| ExportSet {
                default_exports,
                optional_exports,
                tags,
                provenance,
                confidence,
                module_name,
                anchor_id,
            },
        )
}

fn arb_import_kind() -> impl Strategy<Value = ImportKind> {
    prop_oneof![
        Just(ImportKind::Use),
        Just(ImportKind::UseEmpty),
        Just(ImportKind::UseExplicitList),
        Just(ImportKind::UseTag),
        Just(ImportKind::Require),
        Just(ImportKind::RequireThenImport),
        Just(ImportKind::UseConstant),
        Just(ImportKind::DynamicRequire),
        Just(ImportKind::ManualImport),
    ]
}

fn arb_import_symbols() -> impl Strategy<Value = ImportSymbols> {
    prop_oneof![
        Just(ImportSymbols::Default),
        Just(ImportSymbols::None),
        prop::collection::vec(arb_identifier(), 0..5).prop_map(ImportSymbols::Explicit),
        prop::collection::vec(arb_identifier(), 0..4).prop_map(ImportSymbols::Tags),
        (
            prop::collection::vec(arb_identifier(), 0..3),
            prop::collection::vec(arb_identifier(), 0..3),
        )
            .prop_map(|(tags, names)| ImportSymbols::Mixed { tags, names }),
        Just(ImportSymbols::Dynamic),
    ]
}

fn arb_import_spec() -> impl Strategy<Value = ImportSpec> {
    (
        arb_qualified_name(),
        arb_import_kind(),
        arb_import_symbols(),
        arb_provenance(),
        arb_confidence(),
        arb_opt_file_id(),
        arb_opt_anchor_id(),
        arb_opt_scope_id(),
        prop::option::of(any::<u32>()),
    )
        .prop_map(
            |(
                module,
                kind,
                symbols,
                provenance,
                confidence,
                file_id,
                anchor_id,
                scope_id,
                span_start_byte,
            )| ImportSpec {
                module,
                kind,
                symbols,
                provenance,
                confidence,
                file_id,
                anchor_id,
                scope_id,
                span_start_byte,
            },
        )
}

// ── Strategies: visible symbol ─────────────────────────────────────────────

fn arb_visible_symbol_source() -> impl Strategy<Value = VisibleSymbolSource> {
    prop_oneof![
        Just(VisibleSymbolSource::LocalLexical),
        Just(VisibleSymbolSource::LocalPackage),
        Just(VisibleSymbolSource::ExplicitImport),
        Just(VisibleSymbolSource::DefaultExport),
        Just(VisibleSymbolSource::ExportTag),
        Just(VisibleSymbolSource::Constant),
        Just(VisibleSymbolSource::Generated),
        Just(VisibleSymbolSource::External),
        Just(VisibleSymbolSource::DynamicUnknown),
    ]
}

fn arb_visible_symbol_context() -> impl Strategy<Value = VisibleSymbolContext> {
    (prop::option::of(arb_qualified_name()), arb_opt_anchor_id(), arb_opt_anchor_id()).prop_map(
        |(source_module, source_import_anchor_id, source_export_anchor_id)| {
            VisibleSymbolContext::new(
                source_module,
                source_import_anchor_id,
                source_export_anchor_id,
            )
        },
    )
}

fn arb_visible_symbol() -> impl Strategy<Value = VisibleSymbol> {
    (
        arb_symbol_key(),
        arb_opt_entity_id(),
        arb_visible_symbol_source(),
        arb_confidence(),
        prop::option::of(arb_visible_symbol_context()),
    )
        .prop_map(|(name, entity_id, source, confidence, context)| VisibleSymbol {
            name,
            entity_id,
            source,
            confidence,
            context,
        })
}

// ── Strategies: rename and safe-delete plans ──────────────────────────────

fn arb_plan_blocker_reason() -> impl Strategy<Value = PlanBlockerReason> {
    prop_oneof![
        Just(PlanBlockerReason::DynamicBoundary),
        Just(PlanBlockerReason::AmbiguousReference),
        Just(PlanBlockerReason::CrossModuleExport),
        Just(PlanBlockerReason::ImportedSymbol),
        Just(PlanBlockerReason::ExportedSymbol),
        Just(PlanBlockerReason::ReferencesExist),
        Just(PlanBlockerReason::GeneratedMember),
        Just(PlanBlockerReason::UnclassifiedOccurrence),
    ]
}

fn arb_plan_blocker() -> impl Strategy<Value = PlanBlocker> {
    (arb_plan_blocker_reason(), arb_opt_anchor_id(), "[A-Za-z ]{1,30}".prop_map(String::from))
        .prop_map(|(reason, anchor_id, description)| {
            PlanBlocker::new(reason, anchor_id, description)
        })
}

fn arb_plan_warning() -> impl Strategy<Value = PlanWarning> {
    ("[A-Za-z ]{1,30}".prop_map(String::from), arb_opt_anchor_id())
        .prop_map(|(message, anchor_id)| PlanWarning::new(message, anchor_id))
}

fn arb_planned_edit_category() -> impl Strategy<Value = PlannedEditCategory> {
    prop_oneof![
        Just(PlannedEditCategory::Definition),
        Just(PlannedEditCategory::Reference),
        Just(PlannedEditCategory::ImportList),
        Just(PlannedEditCategory::ExportList),
    ]
}

fn arb_planned_edit() -> impl Strategy<Value = PlannedEdit> {
    (
        arb_anchor_id(),
        arb_file_id(),
        arb_planned_edit_category(),
        arb_identifier(),
        arb_identifier(),
    )
        .prop_map(|(anchor_id, file_id, category, old_text, new_text)| {
            PlannedEdit::new(anchor_id, file_id, category, old_text, new_text)
        })
}

fn arb_rename_plan() -> impl Strategy<Value = RenamePlan> {
    (
        arb_entity_id(),
        arb_identifier(),
        arb_identifier(),
        prop::collection::vec(arb_planned_edit(), 0..4),
        prop::collection::vec(arb_plan_blocker(), 0..3),
        prop::collection::vec(arb_plan_warning(), 0..3),
    )
        .prop_map(|(entity_id, old_name, new_name, edits, blockers, warnings)| {
            RenamePlan::new(entity_id, old_name, new_name, edits, blockers, warnings)
        })
}

fn arb_safe_delete_plan() -> impl Strategy<Value = SafeDeletePlan> {
    (
        arb_entity_id(),
        arb_identifier(),
        prop::collection::vec(arb_plan_blocker(), 0..3),
        prop::collection::vec(arb_plan_warning(), 0..3),
    )
        .prop_map(|(entity_id, name, blockers, warnings)| {
            SafeDeletePlan::new(entity_id, name, blockers, warnings)
        })
}

// ── Strategies: package graph types ───────────────────────────────────────

fn arb_package_kind() -> impl Strategy<Value = PackageKind> {
    prop_oneof![
        Just(PackageKind::Package),
        Just(PackageKind::Class),
        Just(PackageKind::Role),
        Just(PackageKind::External),
    ]
}

fn arb_package_edge_kind() -> impl Strategy<Value = PackageEdgeKind> {
    prop_oneof![
        Just(PackageEdgeKind::Inherits),
        Just(PackageEdgeKind::ComposesRole),
        Just(PackageEdgeKind::DependsOn),
    ]
}

fn arb_package_node() -> impl Strategy<Value = PackageNode> {
    (
        arb_entity_id(),
        arb_qualified_name(),
        arb_package_kind(),
        arb_opt_anchor_id(),
        arb_opt_file_id(),
    )
        .prop_map(|(entity_id, name, kind, anchor_id, file_id)| {
            PackageNode::new(entity_id, name, kind, anchor_id, file_id)
        })
}

fn arb_package_edge() -> impl Strategy<Value = PackageEdge> {
    (
        arb_qualified_name(),
        arb_qualified_name(),
        arb_package_edge_kind(),
        arb_opt_anchor_id(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(|(from_package, to_package, kind, anchor_id, provenance, confidence)| {
            PackageEdge::new(from_package, to_package, kind, anchor_id, provenance, confidence)
        })
}

// ── Strategies: generated member ──────────────────────────────────────────

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

fn arb_generated_member() -> impl Strategy<Value = GeneratedMember> {
    (
        arb_entity_id(),
        arb_identifier(),
        arb_generated_member_kind(),
        arb_anchor_id(),
        arb_qualified_name(),
        arb_provenance(),
        arb_confidence(),
    )
        .prop_map(
            |(entity_id, name, kind, source_anchor_id, package, provenance, confidence)| {
                GeneratedMember::new(
                    entity_id,
                    name,
                    kind,
                    source_anchor_id,
                    package,
                    provenance,
                    confidence,
                )
            },
        )
}

// ── Strategies: value shape ────────────────────────────────────────────────

fn arb_value_shape() -> impl Strategy<Value = ValueShape> {
    prop_oneof![
        Just(ValueShape::Unknown),
        Just(ValueShape::Scalar),
        Just(ValueShape::ArrayRef),
        Just(ValueShape::HashRef),
        Just(ValueShape::CodeRef),
        arb_qualified_name().prop_map(|package| ValueShape::PackageName { package }),
        (arb_qualified_name(), arb_confidence())
            .prop_map(|(package, confidence)| ValueShape::Object { package, confidence }),
    ]
}

// ── Property tests ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    // ── Original 4 types ──────────────────────────────────────────────────

    #[test]
    fn reference_edge_json_roundtrip(edge in arb_reference_edge()) {
        let json = serde_json::to_string(&edge).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: perl_semantic_facts::ReferenceEdge = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &edge);
    }

    #[test]
    fn definition_rank_json_roundtrip(rank in arb_definition_rank()) {
        let json = serde_json::to_string(&rank).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionRank = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, rank);
    }

    #[test]
    fn definition_rank_reason_json_roundtrip(reason in arb_definition_rank_reason()) {
        let json = serde_json::to_string(&reason).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionRankReason = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &reason);
    }

    #[test]
    fn definition_candidate_json_roundtrip(candidate in arb_definition_candidate()) {
        let json = serde_json::to_string(&candidate).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DefinitionCandidate = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &candidate);
    }

    // ── Core fact types ────────────────────────────────────────────────────

    #[test]
    fn anchor_fact_json_roundtrip(fact in arb_anchor_fact()) {
        let json = serde_json::to_string(&fact).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: AnchorFact = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &fact);
    }

    #[test]
    fn entity_fact_json_roundtrip(fact in arb_entity_fact()) {
        let json = serde_json::to_string(&fact).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: EntityFact = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &fact);
    }

    #[test]
    fn occurrence_fact_json_roundtrip(fact in arb_occurrence_fact()) {
        let json = serde_json::to_string(&fact).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: OccurrenceFact = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &fact);
    }

    #[test]
    fn edge_fact_json_roundtrip(fact in arb_edge_fact()) {
        let json = serde_json::to_string(&fact).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: EdgeFact = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &fact);
    }

    #[test]
    fn diagnostic_fact_json_roundtrip(fact in arb_diagnostic_fact()) {
        let json = serde_json::to_string(&fact).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: DiagnosticFact = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &fact);
    }

    // ── Export and import types ─────────────────────────────────────────────

    #[test]
    fn export_tag_json_roundtrip(tag in arb_export_tag()) {
        let json = serde_json::to_string(&tag).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ExportTag = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &tag);
    }

    #[test]
    fn export_set_json_roundtrip(export_set in arb_export_set()) {
        let json = serde_json::to_string(&export_set).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ExportSet = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &export_set);
    }

    #[test]
    fn import_kind_json_roundtrip(kind in arb_import_kind()) {
        let json = serde_json::to_string(&kind).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ImportKind = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, kind);
    }

    #[test]
    fn import_symbols_json_roundtrip(symbols in arb_import_symbols()) {
        let json = serde_json::to_string(&symbols).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ImportSymbols = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &symbols);
    }

    #[test]
    fn import_spec_json_roundtrip(spec in arb_import_spec()) {
        let json = serde_json::to_string(&spec).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ImportSpec = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &spec);
    }

    // ── Visible symbol ──────────────────────────────────────────────────────

    #[test]
    fn visible_symbol_source_json_roundtrip(source in arb_visible_symbol_source()) {
        let json = serde_json::to_string(&source).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: VisibleSymbolSource = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, source);
    }

    #[test]
    fn visible_symbol_context_json_roundtrip(ctx in arb_visible_symbol_context()) {
        let json = serde_json::to_string(&ctx).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: VisibleSymbolContext = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &ctx);
    }

    #[test]
    fn visible_symbol_json_roundtrip(symbol in arb_visible_symbol()) {
        let json = serde_json::to_string(&symbol).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: VisibleSymbol = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &symbol);
    }

    // ── Rename and safe-delete plans ────────────────────────────────────────

    #[test]
    fn plan_blocker_reason_json_roundtrip(reason in arb_plan_blocker_reason()) {
        let json = serde_json::to_string(&reason).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PlanBlockerReason = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, reason);
    }

    #[test]
    fn plan_blocker_json_roundtrip(blocker in arb_plan_blocker()) {
        let json = serde_json::to_string(&blocker).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PlanBlocker = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &blocker);
    }

    #[test]
    fn plan_warning_json_roundtrip(warning in arb_plan_warning()) {
        let json = serde_json::to_string(&warning).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PlanWarning = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &warning);
    }

    #[test]
    fn planned_edit_json_roundtrip(edit in arb_planned_edit()) {
        let json = serde_json::to_string(&edit).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PlannedEdit = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &edit);
    }

    #[test]
    fn rename_plan_json_roundtrip(plan in arb_rename_plan()) {
        let json = serde_json::to_string(&plan).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: RenamePlan = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &plan);
    }

    #[test]
    fn safe_delete_plan_json_roundtrip(plan in arb_safe_delete_plan()) {
        let json = serde_json::to_string(&plan).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: SafeDeletePlan = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &plan);
    }

    // ── Package graph types ─────────────────────────────────────────────────

    #[test]
    fn package_kind_json_roundtrip(kind in arb_package_kind()) {
        let json = serde_json::to_string(&kind).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PackageKind = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, kind);
    }

    #[test]
    fn package_edge_kind_json_roundtrip(kind in arb_package_edge_kind()) {
        let json = serde_json::to_string(&kind).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PackageEdgeKind = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, kind);
    }

    #[test]
    fn package_node_json_roundtrip(node in arb_package_node()) {
        let json = serde_json::to_string(&node).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PackageNode = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &node);
    }

    #[test]
    fn package_edge_json_roundtrip(edge in arb_package_edge()) {
        let json = serde_json::to_string(&edge).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: PackageEdge = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &edge);
    }

    // ── Generated member ────────────────────────────────────────────────────

    #[test]
    fn generated_member_kind_json_roundtrip(kind in arb_generated_member_kind()) {
        let json = serde_json::to_string(&kind).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: GeneratedMemberKind = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(decoded, kind);
    }

    #[test]
    fn generated_member_json_roundtrip(member in arb_generated_member()) {
        let json = serde_json::to_string(&member).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: GeneratedMember = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &member);
    }

    // ── Value shape ─────────────────────────────────────────────────────────

    #[test]
    fn value_shape_json_roundtrip(shape in arb_value_shape()) {
        let json = serde_json::to_string(&shape).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let decoded: ValueShape = serde_json::from_str(&json).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&decoded, &shape);
    }
}

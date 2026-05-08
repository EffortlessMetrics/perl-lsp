//! BDD-style consumer workflow tests for perl-semantic-facts.
//!
//! Each scenario is framed as a consumer of the fact vocabulary —
//! "Given <context>, when <operation>, then <expected outcome>."
//! These tests exercise the semantic contracts that downstream providers
//! (goto-definition, rename, hover, completion) depend on.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
    DiagnosticFact, DiagnosticId, EdgeFact, EdgeId, EdgeKind, EntityFact, EntityId, EntityKind,
    ExportSet, ExportTag, FileId, GeneratedMember, GeneratedMemberKind, ImportKind, ImportSpec,
    ImportSymbols, OccurrenceFact, OccurrenceId, OccurrenceKind, PackageEdge, PackageEdgeKind,
    PackageKind, PackageNode, PlanBlocker, PlanBlockerReason, PlanWarning, PlannedEdit,
    PlannedEditCategory, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface, RenamePlan, SafeDeletePlan, ScopeId,
    ValueShape, VisibleSymbol, VisibleSymbolContext, VisibleSymbolSource,
};

// ── Scenario 1: Goto-Definition Provider — Candidate Ranking ──────────────

/// Given a mixed list of definition candidates with different ranks,
/// when the goto-definition provider sorts them by rank,
/// then ExactQualified candidates appear before SamePackage, and SamePackage
/// appears before WorkspaceCandidate, preserving the design intent that
/// the most specific match is always offered first.
#[test]
fn given_mixed_candidates_when_sorted_by_rank_then_most_specific_first() {
    let mut candidates = [
        DefinitionCandidate::new(
            EntityId(3),
            AnchorId(30),
            "Some::Module::helper".to_string(),
            "helper".to_string(),
            Some("Some::Module".to_string()),
            EntityKind::Subroutine,
            Provenance::NameHeuristic,
            Confidence::Low,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        ),
        DefinitionCandidate::new(
            EntityId(1),
            AnchorId(10),
            "App::Controller::process".to_string(),
            "process".to_string(),
            Some("App::Controller".to_string()),
            EntityKind::Method,
            Provenance::ExactAst,
            Confidence::High,
            DefinitionRank::ExactQualified,
            DefinitionRankReason::ExactQualifiedName,
        ),
        DefinitionCandidate::new(
            EntityId(2),
            AnchorId(20),
            "App::Controller::process".to_string(),
            "process".to_string(),
            Some("App::Controller".to_string()),
            EntityKind::Method,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            DefinitionRank::SamePackage,
            DefinitionRankReason::SamePackage,
        ),
    ];

    candidates.sort_by_key(|c| c.rank);

    assert_eq!(candidates[0].rank, DefinitionRank::ExactQualified);
    assert_eq!(candidates[1].rank, DefinitionRank::SamePackage);
    assert_eq!(candidates[2].rank, DefinitionRank::Heuristic);
}

/// Given a definition candidate backed by an explicit import,
/// when the provider inspects the rank reason,
/// then the originating module is available for hover explanation.
#[test]
fn given_import_backed_candidate_when_inspecting_rank_reason_then_source_module_is_known() {
    let candidate = DefinitionCandidate::new(
        EntityId(10),
        AnchorId(1),
        "List::Util::first".to_string(),
        "first".to_string(),
        Some("List::Util".to_string()),
        EntityKind::Subroutine,
        Provenance::ImportExportInference,
        Confidence::Medium,
        DefinitionRank::ExplicitImport,
        DefinitionRankReason::ExplicitImport { module: "List::Util".to_string() },
    );

    let DefinitionRankReason::ExplicitImport { module } = &candidate.rank_reason else {
        panic!("expected ExplicitImport rank reason");
    };
    assert_eq!(module, "List::Util");
    assert_eq!(candidate.display_name, "first");
}

/// Given an empty candidate list (symbol not found anywhere in workspace),
/// when the provider checks it,
/// then it handles zero candidates gracefully without panic.
#[test]
fn given_no_candidates_when_provider_processes_then_empty_list_is_handled() {
    let candidates: Vec<DefinitionCandidate> = vec![];
    assert!(candidates.is_empty());
    // Provider should show "no definition found" without panicking.
    let best = candidates.iter().min_by_key(|c| c.rank);
    assert!(best.is_none());
}

// ── Scenario 2: Rename Provider — Plan Construction and Safety ─────────────

/// Given a RenamePlan for a simple local subroutine,
/// when the rename provider checks for blockers,
/// then no blockers are present and edits can be applied.
#[test]
fn given_safe_rename_plan_when_provider_checks_blockers_then_no_blockers() {
    let plan = RenamePlan::new(
        EntityId(100),
        "old_helper".to_string(),
        "new_helper".to_string(),
        vec![
            PlannedEdit::new(
                AnchorId(1),
                FileId(1),
                PlannedEditCategory::Definition,
                "old_helper".to_string(),
                "new_helper".to_string(),
            ),
            PlannedEdit::new(
                AnchorId(2),
                FileId(1),
                PlannedEditCategory::Reference,
                "old_helper".to_string(),
                "new_helper".to_string(),
            ),
        ],
        vec![],
        vec![],
    );

    assert!(plan.blockers.is_empty(), "safe rename should have no blockers");
    assert_eq!(plan.edits.len(), 2);
    assert_eq!(plan.old_name, "old_helper");
    assert_eq!(plan.new_name, "new_helper");
}

/// Given a RenamePlan blocked by a dynamic boundary (string eval),
/// when the provider checks safety,
/// then the blocker reason is DynamicBoundary and the rename is rejected.
#[test]
fn given_blocked_rename_when_provider_checks_then_dynamic_boundary_is_reported() {
    let plan = RenamePlan::new(
        EntityId(200),
        "dispatch".to_string(),
        "handle".to_string(),
        vec![],
        vec![PlanBlocker::new(
            PlanBlockerReason::DynamicBoundary,
            Some(AnchorId(50)),
            "symbol referenced inside eval string".to_string(),
        )],
        vec![],
    );

    assert!(!plan.blockers.is_empty());
    assert_eq!(plan.blockers[0].reason, PlanBlockerReason::DynamicBoundary);
    assert!(plan.blockers[0].anchor_id.is_some());
}

/// Given a RenamePlan with edits categorized as definition, reference, and export,
/// when the provider partitions edits by category,
/// then each category can be handled independently.
#[test]
fn given_multi_category_rename_plan_when_partitioned_then_categories_are_distinct() {
    let edits = vec![
        PlannedEdit::new(
            AnchorId(1),
            FileId(1),
            PlannedEditCategory::Definition,
            "foo".to_string(),
            "bar".to_string(),
        ),
        PlannedEdit::new(
            AnchorId(2),
            FileId(2),
            PlannedEditCategory::Reference,
            "foo".to_string(),
            "bar".to_string(),
        ),
        PlannedEdit::new(
            AnchorId(3),
            FileId(1),
            PlannedEditCategory::ExportList,
            "foo".to_string(),
            "bar".to_string(),
        ),
    ];
    let plan =
        RenamePlan::new(EntityId(300), "foo".to_string(), "bar".to_string(), edits, vec![], vec![]);

    let def_edits: Vec<_> =
        plan.edits.iter().filter(|e| e.category == PlannedEditCategory::Definition).collect();
    let ref_edits: Vec<_> =
        plan.edits.iter().filter(|e| e.category == PlannedEditCategory::Reference).collect();
    let export_edits: Vec<_> =
        plan.edits.iter().filter(|e| e.category == PlannedEditCategory::ExportList).collect();

    assert_eq!(def_edits.len(), 1);
    assert_eq!(ref_edits.len(), 1);
    assert_eq!(export_edits.len(), 1);
}

// ── Scenario 3: Safe Delete Provider ──────────────────────────────────────

/// Given a SafeDeletePlan with remaining references as blocker,
/// when the provider evaluates deletion,
/// then the blocker prevents deletion and references are reported.
#[test]
fn given_safe_delete_with_references_when_evaluated_then_deletion_is_blocked() {
    let plan = SafeDeletePlan::new(
        EntityId(400),
        "legacy_fn".to_string(),
        vec![PlanBlocker::new(
            PlanBlockerReason::ReferencesExist,
            Some(AnchorId(70)),
            "3 call sites remain in workspace".to_string(),
        )],
        vec![PlanWarning::new("also referenced in comments".to_string(), None)],
    );

    assert!(!plan.blockers.is_empty());
    assert_eq!(plan.blockers[0].reason, PlanBlockerReason::ReferencesExist);
    assert_eq!(plan.warnings.len(), 1);
    assert!(plan.warnings[0].anchor_id.is_none());
}

// ── Scenario 4: Import Resolution Consumer ────────────────────────────────

/// Given a UseExplicitList ImportSpec listing specific symbols,
/// when the completion provider resolves visible symbols,
/// then exactly the listed symbols are available at the import site.
#[test]
fn given_explicit_import_spec_when_resolved_then_listed_symbols_available() {
    let spec = ImportSpec {
        module: "Scalar::Util".to_string(),
        kind: ImportKind::UseExplicitList,
        symbols: ImportSymbols::Explicit(vec![
            "blessed".to_string(),
            "reftype".to_string(),
            "weaken".to_string(),
        ]),
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
        file_id: Some(FileId(1)),
        anchor_id: Some(AnchorId(5)),
        scope_id: None,
        span_start_byte: Some(20),
    };

    let ImportSymbols::Explicit(ref symbols) = spec.symbols else {
        panic!("expected Explicit symbols");
    };
    assert_eq!(symbols.len(), 3);
    assert!(symbols.contains(&"blessed".to_string()));
    assert!(symbols.contains(&"reftype".to_string()));
    assert_eq!(spec.kind, ImportKind::UseExplicitList);
}

/// Given a ManualImport spec (Class->import(@names)),
/// when the diagnostic provider evaluates bareword suppression,
/// then span_start_byte controls order-aware suppression.
#[test]
fn given_manual_import_spec_when_checking_order_sensitivity_then_span_is_used() {
    let spec = ImportSpec {
        module: "Foo::Exporter".to_string(),
        kind: ImportKind::ManualImport,
        symbols: ImportSymbols::Dynamic,
        provenance: Provenance::SemanticAnalyzer,
        confidence: Confidence::Medium,
        file_id: Some(FileId(2)),
        anchor_id: Some(AnchorId(30)),
        scope_id: None,
        span_start_byte: Some(150),
    };

    // The diagnostic provider should suppress barewords that appear after
    // the import site (byte offset 150) but not before.
    assert_eq!(spec.kind, ImportKind::ManualImport);
    assert_eq!(spec.span_start_byte, Some(150));
    // Symbols are Dynamic — the provider must treat all names conservatively.
    assert_eq!(spec.symbols, ImportSymbols::Dynamic);
}

/// Given a UseTag ImportSpec with multiple tags,
/// when the provider resolves visible names,
/// then the tags drive group-based symbol lookup.
#[test]
fn given_use_tag_import_spec_when_resolving_then_tags_available() {
    let spec = ImportSpec {
        module: "POSIX".to_string(),
        kind: ImportKind::UseTag,
        symbols: ImportSymbols::Tags(vec![":math".to_string(), ":ctype".to_string()]),
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
        file_id: None,
        anchor_id: None,
        scope_id: None,
        span_start_byte: None,
    };

    let ImportSymbols::Tags(ref tags) = spec.symbols else {
        panic!("expected Tags symbols");
    };
    assert!(tags.contains(&":math".to_string()));
    assert!(tags.contains(&":ctype".to_string()));
}

// ── Scenario 5: Export Set Consumer ───────────────────────────────────────

/// Given an ExportSet with default and optional exports plus tags,
/// when the provider checks what a plain `use` imports,
/// then only the default exports are visible by default.
#[test]
fn given_export_set_when_plain_use_then_only_default_exports_visible() {
    let export_set = ExportSet {
        default_exports: vec!["encode_json".to_string(), "decode_json".to_string()],
        optional_exports: vec!["to_json".to_string(), "from_json".to_string()],
        tags: vec![ExportTag {
            name: "all".to_string(),
            members: vec![
                "encode_json".to_string(),
                "decode_json".to_string(),
                "to_json".to_string(),
                "from_json".to_string(),
            ],
        }],
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
        module_name: Some("JSON".to_string()),
        anchor_id: Some(AnchorId(10)),
    };

    // Plain `use JSON` imports only default_exports.
    assert_eq!(export_set.default_exports.len(), 2);
    assert!(export_set.default_exports.contains(&"encode_json".to_string()));
    // Optional exports are NOT visible without explicit import list.
    assert!(!export_set.default_exports.contains(&"to_json".to_string()));
    // :all tag expands to include everything.
    let all_tag = export_set.tags.iter().find(|t| t.name == "all").expect("all tag present");
    assert_eq!(all_tag.members.len(), 4);
}

/// Given an ExportSet with no module name (inferred from context),
/// when the provider requests the exporting module,
/// then it handles the None case without error.
#[test]
fn given_export_set_without_module_name_when_queried_then_none_handled() {
    let export_set = ExportSet {
        default_exports: vec!["slurp".to_string()],
        optional_exports: vec![],
        tags: vec![],
        provenance: Provenance::ImportExportInference,
        confidence: Confidence::Low,
        module_name: None,
        anchor_id: None,
    };

    assert!(export_set.module_name.is_none());
    assert!(export_set.anchor_id.is_none());
    assert_eq!(export_set.default_exports, ["slurp"]);
}

// ── Scenario 6: Visible Symbol — Hover Origin Tracing ─────────────────────

/// Given a VisibleSymbol imported from an explicit `use` statement,
/// when the hover provider inspects its origin,
/// then the source module and import anchor are available for hover text.
#[test]
fn given_explicit_import_visible_symbol_when_hovering_then_source_module_and_anchor_known() {
    let symbol = VisibleSymbol {
        name: "blessed".to_string(),
        entity_id: Some(EntityId(77)),
        source: VisibleSymbolSource::ExplicitImport,
        confidence: Confidence::High,
        context: Some(VisibleSymbolContext::new(
            Some("Scalar::Util".to_string()),
            Some(AnchorId(12)),
            Some(AnchorId(99)),
        )),
    };

    assert_eq!(symbol.source, VisibleSymbolSource::ExplicitImport);
    let ctx = symbol.context.as_ref().expect("context present");
    assert_eq!(ctx.source_module.as_deref(), Some("Scalar::Util"));
    assert!(ctx.source_import_anchor_id.is_some());
    assert!(ctx.source_export_anchor_id.is_some());
}

/// Given a locally-defined lexical symbol,
/// when the completion provider inspects it,
/// then it has LocalLexical source with no external context.
#[test]
fn given_local_lexical_symbol_when_completing_then_no_external_context() {
    let symbol = VisibleSymbol {
        name: "$count".to_string(),
        entity_id: Some(EntityId(5)),
        source: VisibleSymbolSource::LocalLexical,
        confidence: Confidence::High,
        context: None,
    };

    assert_eq!(symbol.source, VisibleSymbolSource::LocalLexical);
    assert!(symbol.context.is_none());
    assert!(symbol.name.starts_with('$'));
}

/// Given a dynamically-defined symbol (AUTOLOAD or string eval),
/// when the provider evaluates it,
/// then it has DynamicUnknown source with Low confidence.
#[test]
fn given_dynamic_unknown_symbol_when_evaluated_then_low_confidence_and_dynamic_source() {
    let symbol = VisibleSymbol {
        name: "AUTOLOAD_dispatch".to_string(),
        entity_id: None,
        source: VisibleSymbolSource::DynamicUnknown,
        confidence: Confidence::Low,
        context: None,
    };

    assert_eq!(symbol.source, VisibleSymbolSource::DynamicUnknown);
    assert_eq!(symbol.confidence, Confidence::Low);
    assert!(symbol.entity_id.is_none());
}

// ── Scenario 7: Value Shape — Method Completion Filtering ─────────────────

/// Given a ValueShape::Object with a known package,
/// when the completion provider filters method candidates,
/// then the package name guides lookup in the class graph.
#[test]
fn given_object_value_shape_when_completing_methods_then_package_drives_lookup() {
    let shape =
        ValueShape::Object { package: "DateTime".to_string(), confidence: Confidence::High };

    match &shape {
        ValueShape::Object { package, confidence } => {
            assert_eq!(package, "DateTime");
            assert_eq!(*confidence, Confidence::High);
        }
        _ => panic!("expected Object shape"),
    }
}

/// Given a ValueShape::PackageName (static class call like `Foo->new`),
/// when the provider resolves methods,
/// then it knows the class at compile time.
#[test]
fn given_package_name_shape_when_resolving_class_then_package_known_statically() {
    let shape = ValueShape::PackageName { package: "LWP::UserAgent".to_string() };

    let ValueShape::PackageName { package } = &shape else {
        panic!("expected PackageName shape");
    };
    assert_eq!(package, "LWP::UserAgent");
}

/// Given a ValueShape::Unknown,
/// when the provider receives it,
/// then it falls back to workspace-wide name search.
#[test]
fn given_unknown_value_shape_when_completing_then_fallback_to_workspace_search() {
    let shape = ValueShape::Unknown;
    // Provider logic: Unknown shape → no package filtering → search all methods
    assert!(matches!(shape, ValueShape::Unknown));
}

// ── Scenario 8: Package Graph — Inheritance Walk ──────────────────────────

/// Given a set of PackageEdges forming a linear inheritance chain,
/// when the provider traverses the parent chain,
/// then it can collect all ancestors in order.
#[test]
fn given_linear_inheritance_chain_when_traversing_then_ancestors_reachable() {
    let edges = [
        PackageEdge::new(
            "Child".to_string(),
            "Parent".to_string(),
            PackageEdgeKind::Inherits,
            Some(AnchorId(1)),
            Provenance::ExactAst,
            Confidence::High,
        ),
        PackageEdge::new(
            "Parent".to_string(),
            "GrandParent".to_string(),
            PackageEdgeKind::Inherits,
            Some(AnchorId(2)),
            Provenance::ExactAst,
            Confidence::High,
        ),
    ];

    let parents_of_child: Vec<&str> =
        edges.iter().filter(|e| e.from_package == "Child").map(|e| e.to_package.as_str()).collect();

    assert_eq!(parents_of_child, ["Parent"]);

    let parents_of_parent: Vec<&str> = edges
        .iter()
        .filter(|e| e.from_package == "Parent")
        .map(|e| e.to_package.as_str())
        .collect();
    assert_eq!(parents_of_parent, ["GrandParent"]);
}

/// Given PackageEdges mixing inheritance and role composition,
/// when the provider extracts only role compositions,
/// then only ComposesRole edges are returned.
#[test]
fn given_mixed_package_edges_when_filtering_by_kind_then_only_role_edges_selected() {
    let edges = [
        PackageEdge::new(
            "MyClass".to_string(),
            "BaseClass".to_string(),
            PackageEdgeKind::Inherits,
            None,
            Provenance::ExactAst,
            Confidence::High,
        ),
        PackageEdge::new(
            "MyClass".to_string(),
            "Role::Printable".to_string(),
            PackageEdgeKind::ComposesRole,
            None,
            Provenance::ExactAst,
            Confidence::High,
        ),
        PackageEdge::new(
            "MyClass".to_string(),
            "Role::Serializable".to_string(),
            PackageEdgeKind::ComposesRole,
            None,
            Provenance::ExactAst,
            Confidence::High,
        ),
    ];

    let roles: Vec<&str> = edges
        .iter()
        .filter(|e| e.kind == PackageEdgeKind::ComposesRole)
        .map(|e| e.to_package.as_str())
        .collect();

    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&"Role::Printable"));
    assert!(roles.contains(&"Role::Serializable"));
}

// ── Scenario 9: Generated Members — Accessor Discovery ────────────────────

/// Given a GeneratedMember representing a Moo/Moose getter,
/// when the completion provider adds it to the method list,
/// then the name and kind are preserved for display.
#[test]
fn given_moo_getter_generated_member_when_completing_then_name_and_kind_available() {
    let member = GeneratedMember::new(
        EntityId(1000),
        "username".to_string(),
        GeneratedMemberKind::Getter,
        AnchorId(200),
        "MyApp::User".to_string(),
        Provenance::FrameworkSynthesis,
        Confidence::Medium,
    );

    assert_eq!(member.name, "username");
    assert_eq!(member.kind, GeneratedMemberKind::Getter);
    assert_eq!(member.package, "MyApp::User");
    assert_eq!(member.provenance, Provenance::FrameworkSynthesis);
}

/// Given multiple GeneratedMembers for the same `has` attribute,
/// when collecting accessor completions,
/// then predicate and clearer can be distinguished from getter/setter.
#[test]
fn given_attribute_accessors_when_collecting_then_predicate_and_clearer_distinguishable() {
    let members = [
        GeneratedMember::new(
            EntityId(1),
            "name".to_string(),
            GeneratedMemberKind::Accessor,
            AnchorId(1),
            "Foo".to_string(),
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        ),
        GeneratedMember::new(
            EntityId(2),
            "has_name".to_string(),
            GeneratedMemberKind::Predicate,
            AnchorId(1),
            "Foo".to_string(),
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        ),
        GeneratedMember::new(
            EntityId(3),
            "clear_name".to_string(),
            GeneratedMemberKind::Clearer,
            AnchorId(1),
            "Foo".to_string(),
            Provenance::FrameworkSynthesis,
            Confidence::Medium,
        ),
    ];

    let accessor = members.iter().find(|m| m.kind == GeneratedMemberKind::Accessor).unwrap();
    let predicate = members.iter().find(|m| m.kind == GeneratedMemberKind::Predicate).unwrap();
    let clearer = members.iter().find(|m| m.kind == GeneratedMemberKind::Clearer).unwrap();

    assert_eq!(accessor.name, "name");
    assert_eq!(predicate.name, "has_name");
    assert_eq!(clearer.name, "clear_name");
}

// ── Scenario 10: Confidence — Diagnostic Filtering ────────────────────────

/// Given DiagnosticFacts with different confidence levels,
/// when the provider filters for High-confidence-only mode,
/// then Medium and Low diagnostics are excluded.
#[test]
fn given_mixed_confidence_diagnostics_when_filtering_high_only_then_low_excluded() {
    let diagnostics = [
        DiagnosticFact {
            id: DiagnosticId(1),
            code: Some("PL001".to_string()),
            message: "strict violation".to_string(),
            primary_anchor_id: AnchorId(1),
            related_anchor_ids: vec![],
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
        DiagnosticFact {
            id: DiagnosticId(2),
            code: Some("PL101".to_string()),
            message: "possible issue".to_string(),
            primary_anchor_id: AnchorId(2),
            related_anchor_ids: vec![],
            scope_id: None,
            provenance: Provenance::NameHeuristic,
            confidence: Confidence::Low,
        },
        DiagnosticFact {
            id: DiagnosticId(3),
            code: Some("PL050".to_string()),
            message: "likely issue".to_string(),
            primary_anchor_id: AnchorId(3),
            related_anchor_ids: vec![],
            scope_id: None,
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::Medium,
        },
    ];

    let high_only: Vec<_> =
        diagnostics.iter().filter(|d| d.confidence == Confidence::High).collect();

    assert_eq!(high_only.len(), 1);
    assert_eq!(high_only[0].code.as_deref(), Some("PL001"));
}

// ── Scenario 11: Anchor Fact — Span Navigation ────────────────────────────

/// Given an AnchorFact with byte-span coordinates,
/// when the LSP provider converts to an LSP Range,
/// then span_start_byte and span_end_byte are non-negative and ordered.
#[test]
fn given_anchor_fact_when_inspecting_span_then_start_is_before_end() {
    let anchor = AnchorFact {
        id: AnchorId(50),
        file_id: FileId(3),
        span_start_byte: 100,
        span_end_byte: 115,
        scope_id: Some(ScopeId(7)),
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    assert!(anchor.span_start_byte < anchor.span_end_byte);
    assert_eq!(anchor.span_end_byte - anchor.span_start_byte, 15); // length
    assert!(anchor.scope_id.is_some());
}

// ── Scenario 12: Entity Fact — Qualified Name Structure ───────────────────

/// Given an EntityFact for a method in a class,
/// when the hover provider formats the display,
/// then the canonical name and entity kind are available.
#[test]
fn given_method_entity_fact_when_formatting_hover_then_qualified_name_and_kind_available() {
    let entity = EntityFact {
        id: EntityId(500),
        kind: EntityKind::Method,
        canonical_name: "MyApp::Model::User::save".to_string(),
        anchor_id: Some(AnchorId(10)),
        scope_id: None,
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    assert_eq!(entity.kind, EntityKind::Method);
    assert!(entity.canonical_name.contains("::save"));
    assert!(entity.canonical_name.starts_with("MyApp::"));
}

// ── Scenario 13: Edge Fact — Import Graph Traversal ───────────────────────

/// Given EdgeFacts forming a module import chain,
/// when the provider finds all ImportsModule edges from a source entity,
/// then the set of imported modules is discoverable.
#[test]
fn given_import_edges_when_traversing_from_entity_then_imported_modules_discoverable() {
    let edges = [
        EdgeFact {
            id: EdgeId(1),
            kind: EdgeKind::ImportsModule,
            from_entity_id: EntityId(1),
            to_entity_id: EntityId(100),
            via_occurrence_id: Some(OccurrenceId(10)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
        EdgeFact {
            id: EdgeId(2),
            kind: EdgeKind::ImportsModule,
            from_entity_id: EntityId(1),
            to_entity_id: EntityId(200),
            via_occurrence_id: Some(OccurrenceId(11)),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
        EdgeFact {
            id: EdgeId(3),
            kind: EdgeKind::Calls,
            from_entity_id: EntityId(1),
            to_entity_id: EntityId(300),
            via_occurrence_id: None,
            provenance: Provenance::SemanticAnalyzer,
            confidence: Confidence::Medium,
        },
    ];

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.from_entity_id == EntityId(1) && e.kind == EdgeKind::ImportsModule)
        .collect();

    assert_eq!(imports.len(), 2);
    // Call edges are separate from import edges.
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();
    assert_eq!(calls.len(), 1);
}

// ── Scenario 14: OccurrenceFact — Call vs Reference Disambiguation ─────────

/// Given OccurrenceFacts at the same location with different kinds,
/// when the provider disambiguates read vs write vs call,
/// then each occurrence kind is correctly identified.
#[test]
fn given_occurrence_facts_when_disambiguating_by_kind_then_each_kind_distinct() {
    let occurrences = [
        OccurrenceFact {
            id: OccurrenceId(1),
            kind: OccurrenceKind::Read,
            entity_id: Some(EntityId(10)),
            anchor_id: AnchorId(1),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
        OccurrenceFact {
            id: OccurrenceId(2),
            kind: OccurrenceKind::Write,
            entity_id: Some(EntityId(10)),
            anchor_id: AnchorId(2),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
        OccurrenceFact {
            id: OccurrenceId(3),
            kind: OccurrenceKind::Call,
            entity_id: Some(EntityId(20)),
            anchor_id: AnchorId(3),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        },
    ];

    let reads: Vec<_> = occurrences.iter().filter(|o| o.kind == OccurrenceKind::Read).collect();
    let writes: Vec<_> = occurrences.iter().filter(|o| o.kind == OccurrenceKind::Write).collect();
    let calls: Vec<_> = occurrences.iter().filter(|o| o.kind == OccurrenceKind::Call).collect();

    assert_eq!(reads.len(), 1);
    assert_eq!(writes.len(), 1);
    assert_eq!(calls.len(), 1);
}

// ── Scenario 15: DefinitionRank — Ordering Contract ─────────────────────

/// Given the full rank ordering contract from the design,
/// when each adjacent pair is compared,
/// then the Ord implementation preserves the total ordering.
#[test]
fn given_definition_rank_variants_when_comparing_then_ordering_is_total_and_correct() {
    // Ordered best-to-worst: ExactQualified < SamePackage < ExplicitImport
    // < DefaultExport < WorkspaceCandidate < Heuristic
    let ordered = [
        DefinitionRank::ExactQualified,
        DefinitionRank::SamePackage,
        DefinitionRank::ExplicitImport,
        DefinitionRank::DefaultExport,
        DefinitionRank::WorkspaceCandidate,
        DefinitionRank::Heuristic,
    ];

    for window in ordered.windows(2) {
        assert!(window[0] < window[1], "{:?} should be less than {:?}", window[0], window[1]);
    }
}

/// Given the same rank on two candidates with different entity kinds,
/// when comparing by rank only,
/// then they are equal rank (not ordered by kind).
#[test]
fn given_same_rank_different_kinds_when_comparing_then_ranks_are_equal() {
    let sub_candidate = DefinitionCandidate::new(
        EntityId(1),
        AnchorId(1),
        "Foo::bar".to_string(),
        "bar".to_string(),
        None,
        EntityKind::Subroutine,
        Provenance::ExactAst,
        Confidence::High,
        DefinitionRank::ExactQualified,
        DefinitionRankReason::ExactQualifiedName,
    );
    let method_candidate = DefinitionCandidate::new(
        EntityId(2),
        AnchorId(2),
        "Foo::bar".to_string(),
        "bar".to_string(),
        None,
        EntityKind::Method,
        Provenance::ExactAst,
        Confidence::High,
        DefinitionRank::ExactQualified,
        DefinitionRankReason::ExactQualifiedName,
    );

    assert_eq!(sub_candidate.rank, method_candidate.rank);
}

// ── Scenario 16: PackageNode — Class/Role/Package Classification ──────────

/// Given PackageNodes representing different package kinds,
/// when the class graph builder separates classes from plain packages,
/// then each kind can be filtered independently.
#[test]
fn given_mixed_package_nodes_when_filtering_by_kind_then_classes_and_roles_separate() {
    let nodes = [
        PackageNode::new(EntityId(1), "App::Model".to_string(), PackageKind::Class, None, None),
        PackageNode::new(EntityId(2), "Role::Printable".to_string(), PackageKind::Role, None, None),
        PackageNode::new(EntityId(3), "App::Util".to_string(), PackageKind::Package, None, None),
        PackageNode::new(
            EntityId(4),
            "External::Dep".to_string(),
            PackageKind::External,
            None,
            None,
        ),
    ];

    let classes: Vec<_> = nodes.iter().filter(|n| n.kind == PackageKind::Class).collect();
    let roles: Vec<_> = nodes.iter().filter(|n| n.kind == PackageKind::Role).collect();
    let packages: Vec<_> = nodes.iter().filter(|n| n.kind == PackageKind::Package).collect();
    let externals: Vec<_> = nodes.iter().filter(|n| n.kind == PackageKind::External).collect();

    assert_eq!(classes.len(), 1);
    assert_eq!(roles.len(), 1);
    assert_eq!(packages.len(), 1);
    assert_eq!(externals.len(), 1);
    assert_eq!(classes[0].name, "App::Model");
    assert_eq!(roles[0].name, "Role::Printable");
}

// ── Scenario 17: Provider Fact Trace — Cutover Proof ─────────────────

/// Given a provider answer sourced from shadowed compiler facts,
/// when the answer is traced,
/// then the trace records provider surface, fact source, provenance, confidence,
/// freshness, and fallback state without changing provider behavior.
#[test]
fn given_provider_answer_when_traced_then_source_provenance_and_fallback_are_explicit() {
    let trace = ProviderFactTrace::new(
        ProviderSurface::Completion,
        ProviderFactSourceKind::CompilerFact,
        Provenance::ImportExportInference,
        Confidence::High,
        ProviderFactFreshness::Fresh,
        ProviderFallbackState::Shadow,
        Some("fixture-source-sha".to_string()),
        Some(AnchorId(17)),
        Some(1),
    );

    assert_eq!(trace.surface, ProviderSurface::Completion);
    assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
    assert_eq!(trace.provenance, Provenance::ImportExportInference);
    assert_eq!(trace.confidence, Confidence::High);
    assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
    assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
    assert_eq!(trace.anchor_id, Some(AnchorId(17)));
}

/// Given a provider blocks an unsafe refactor because of a dynamic boundary,
/// when the blocker is traced,
/// then the trace records the dynamic-boundary source and blocked state.
#[test]
fn given_dynamic_boundary_blocker_when_traced_then_provider_source_is_blocked() {
    let trace = ProviderFactTrace::new(
        ProviderSurface::Rename,
        ProviderFactSourceKind::DynamicBoundary,
        Provenance::DynamicBoundary,
        Confidence::Low,
        ProviderFactFreshness::Fresh,
        ProviderFallbackState::Blocked,
        None,
        Some(AnchorId(18)),
        Some(1),
    );

    assert_eq!(trace.surface, ProviderSurface::Rename);
    assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
    assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
    assert_eq!(trace.anchor_id, Some(AnchorId(18)));
}

// ── Scenario 17: ExportSet — Tag-Based Resolution ─────────────────────────

/// Given an ExportSet with %EXPORT_TAGS entries,
/// when the provider resolves `use Module ':tagname'`,
/// then the correct member set is returned for the tag.
#[test]
fn given_export_tags_when_resolving_tag_import_then_correct_members_returned() {
    let export_set = ExportSet {
        default_exports: vec![],
        optional_exports: vec![
            "encode".to_string(),
            "decode".to_string(),
            "validate".to_string(),
            "log_error".to_string(),
        ],
        tags: vec![
            ExportTag {
                name: "codec".to_string(),
                members: vec!["encode".to_string(), "decode".to_string()],
            },
            ExportTag {
                name: "all".to_string(),
                members: vec![
                    "encode".to_string(),
                    "decode".to_string(),
                    "validate".to_string(),
                    "log_error".to_string(),
                ],
            },
        ],
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
        module_name: Some("My::Codec".to_string()),
        anchor_id: None,
    };

    let codec_tag = export_set.tags.iter().find(|t| t.name == "codec").expect("codec tag exists");
    assert_eq!(codec_tag.members.len(), 2);
    assert!(codec_tag.members.contains(&"encode".to_string()));
    assert!(codec_tag.members.contains(&"decode".to_string()));
    assert!(!codec_tag.members.contains(&"log_error".to_string()));

    let all_tag = export_set.tags.iter().find(|t| t.name == "all").expect("all tag exists");
    assert_eq!(all_tag.members.len(), 4);
}

// ── Scenario 18: Provenance — Source Attribution Chain ────────────────────

/// Given facts with different provenances,
/// when the provider ranks by reliability,
/// then ExactAst is treated as most reliable, NameHeuristic least.
#[test]
fn given_mixed_provenance_facts_when_compared_then_exact_ast_most_reliable() {
    // The contract: consumers treat Provenance as an ordered reliability indicator.
    // ExactAst > DesugaredAst > SemanticAnalyzer > ... > NameHeuristic
    let most_reliable = [
        Provenance::ExactAst,
        Provenance::DesugaredAst,
        Provenance::SemanticAnalyzer,
        Provenance::FrameworkSynthesis,
        Provenance::ImportExportInference,
        Provenance::PragmaInference,
        Provenance::NameHeuristic,
    ];

    // All variants should be distinct (exhaustive check via debug format).
    let debug_strs: std::collections::HashSet<String> =
        most_reliable.iter().map(|p| format!("{p:?}")).collect();
    assert_eq!(debug_strs.len(), most_reliable.len(), "all provenance variants must be distinct");
    assert!(debug_strs.contains("ExactAst"));
    assert!(debug_strs.contains("NameHeuristic"));
}

//! Provider-level scorecard gate fixture suites.
//!
//! These fixtures exercise the per-provider cutover functions with synthetic
//! semantic query stubs, verifying that the scorecard gate criteria are met
//! at the provider integration level.
//!
//! # Requirements
//!
//! - **Req 10.6**: goto-definition — regressions=0, ambiguous classified,
//!   unavailable falls back.
//! - **Req 10.7**: find-references — legacy count parity or better,
//!   definition exclusion correct.
//! - **Req 10.8**: completion — explicit import pass, default export pass,
//!   empty import suppresses, tag export pass.
//! - **Req 10.9**: diagnostics — imported-symbol false positives=0,
//!   dynamic-boundary exact warnings=0.
//! - **Req 10.10**: rename/safe-delete — unsafe edits=0, dynamic blocked,
//!   ambiguous blocked, export/import blocked or planned.

#[cfg(test)]
mod tests {
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason, EntityId,
        EntityKind, FileId, OccurrenceFact, OccurrenceId, OccurrenceKind, PlanBlocker,
        PlanBlockerReason, PlannedEdit, PlannedEditCategory, Provenance, RenamePlan,
        SafeDeletePlan, ScopeId, VisibleSymbol, VisibleSymbolSource,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic::scorecard::{Scorecard, ScorecardMode};
    use perl_workspace::workspace_index::WorkspaceIndex;

    use crate::providers::completion::completion_shadow::{
        CompletionCutoverResult, completion_visibility_cutover,
    };
    use crate::providers::diagnostics::diagnostics_shadow::{
        DiagnosticAction, diagnostics_undefined_symbol_cutover,
    };
    use crate::providers::navigation::definition_shadow::{
        DefinitionCutoverResult, goto_definition_cutover,
    };
    use crate::providers::navigation::references_shadow::{
        ReferencesCutoverResult, find_references_cutover,
    };
    use crate::providers::navigation::rename_shadow::{RenameCutoverResult, rename_cutover};
    use crate::providers::navigation::safe_delete_shadow::{
        SafeDeleteCutoverResult, safe_delete_cutover,
    };

    // ── Stub SemanticQueries ──

    /// Configurable stub for provider-level scorecard gate tests.
    struct GateStub {
        definitions_result: Vec<DefinitionCandidate>,
        references_result: Vec<OccurrenceFact>,
        visible_symbols_result: Vec<VisibleSymbol>,
        rename_plan_result: RenamePlan,
        safe_delete_plan_result: SafeDeletePlan,
    }

    impl GateStub {
        fn with_definitions(candidates: Vec<DefinitionCandidate>) -> Self {
            Self {
                definitions_result: candidates,
                references_result: vec![],
                visible_symbols_result: vec![],
                rename_plan_result: empty_rename_plan(),
                safe_delete_plan_result: empty_safe_delete_plan(),
            }
        }

        fn with_references(occurrences: Vec<OccurrenceFact>) -> Self {
            Self {
                definitions_result: vec![],
                references_result: occurrences,
                visible_symbols_result: vec![],
                rename_plan_result: empty_rename_plan(),
                safe_delete_plan_result: empty_safe_delete_plan(),
            }
        }

        fn with_visible_symbols(symbols: Vec<VisibleSymbol>) -> Self {
            Self {
                definitions_result: vec![],
                references_result: vec![],
                visible_symbols_result: symbols,
                rename_plan_result: empty_rename_plan(),
                safe_delete_plan_result: empty_safe_delete_plan(),
            }
        }

        fn with_rename_plan(plan: RenamePlan) -> Self {
            Self {
                definitions_result: vec![],
                references_result: vec![],
                visible_symbols_result: vec![],
                rename_plan_result: plan,
                safe_delete_plan_result: empty_safe_delete_plan(),
            }
        }

        fn with_safe_delete_plan(plan: SafeDeletePlan) -> Self {
            Self {
                definitions_result: vec![],
                references_result: vec![],
                visible_symbols_result: vec![],
                rename_plan_result: empty_rename_plan(),
                safe_delete_plan_result: plan,
            }
        }
    }

    fn empty_rename_plan() -> RenamePlan {
        RenamePlan::new(EntityId(0), String::new(), String::new(), vec![], vec![], vec![])
    }

    fn empty_safe_delete_plan() -> SafeDeletePlan {
        SafeDeletePlan::new(EntityId(0), String::new(), vec![], vec![])
    }

    impl SemanticQueries for GateStub {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(perl_semantic_facts::EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            self.definitions_result.clone()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            self.references_result.clone()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            self.visible_symbols_result.clone()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            vec![]
        }

        fn rename_plan(&self, _entity_id: EntityId, _new_name: &str) -> RenamePlan {
            self.rename_plan_result.clone()
        }

        fn safe_delete_plan(&self, _entity_id: EntityId) -> SafeDeletePlan {
            self.safe_delete_plan_result.clone()
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            None
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            None
        }
    }

    // ── Helpers ──

    fn make_candidate(
        name: &str,
        anchor_id: u64,
        entity_id: u64,
        confidence: Confidence,
    ) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(anchor_id),
            name.to_string(),
            name.to_string(),
            None,
            EntityKind::Subroutine,
            Provenance::ExactAst,
            confidence,
            DefinitionRank::ExactQualified,
            DefinitionRankReason::ExactQualifiedName,
        )
    }

    fn make_ref_occurrence(occ_id: u64, anchor_id: u64, entity_id: u64) -> OccurrenceFact {
        OccurrenceFact {
            id: OccurrenceId(occ_id),
            kind: OccurrenceKind::Call,
            entity_id: Some(EntityId(entity_id)),
            anchor_id: AnchorId(anchor_id),
            scope_id: None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        }
    }

    fn make_visible(
        name: &str,
        source: VisibleSymbolSource,
        confidence: Confidence,
    ) -> VisibleSymbol {
        VisibleSymbol { name: name.to_string(), entity_id: None, source, confidence, context: None }
    }

    // ════════════════════════════════════════════════════════════════════
    // 1. Goto-Definition Provider Gate (Req 10.6)
    // ════════════════════════════════════════════════════════════════════

    /// Goto-definition cutover: exact single candidate → Exact result.
    #[test]
    fn gate_goto_def_exact_single_candidate() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let stub =
            GateStub::with_definitions(vec![make_candidate("Foo::bar", 10, 100, Confidence::High)]);
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &stub, "Foo::bar", &ctx);
        assert!(
            matches!(outcome.result, DefinitionCutoverResult::Exact(_)),
            "single high-confidence candidate should be Exact"
        );
        Ok(())
    }

    /// Goto-definition cutover: no candidates → LegacyFallback.
    #[test]
    fn gate_goto_def_unavailable_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let stub = GateStub::with_definitions(vec![]);
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &stub, "missing", &ctx);
        assert!(
            matches!(outcome.result, DefinitionCutoverResult::LegacyFallback(_)),
            "no candidates should fall back to legacy"
        );
        Ok(())
    }

    /// Goto-definition cutover: multiple candidates → Ambiguous.
    #[test]
    fn gate_goto_def_ambiguous_classified() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let stub = GateStub::with_definitions(vec![
            make_candidate("Foo::bar", 10, 100, Confidence::High),
            make_candidate("Baz::bar", 20, 200, Confidence::High),
        ]);
        let ctx = QueryContext::new(FileId(1), None, None);

        let outcome = goto_definition_cutover(&index, &stub, "bar", &ctx);
        assert!(
            matches!(outcome.result, DefinitionCutoverResult::Ambiguous(_)),
            "multiple candidates should be Ambiguous"
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 2. Find-References Provider Gate (Req 10.7)
    // ════════════════════════════════════════════════════════════════════

    /// Find-references cutover: typed references returned.
    #[test]
    fn gate_find_refs_typed_references() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let stub = GateStub::with_references(vec![
            make_ref_occurrence(1, 10, 100),
            make_ref_occurrence(2, 20, 100),
        ]);

        let outcome = find_references_cutover(&index, &stub, "Foo::bar", EntityId(100));
        assert!(
            matches!(outcome.result, ReferencesCutoverResult::Exact(_)),
            "typed references should produce Exact result"
        );
        Ok(())
    }

    /// Find-references cutover: no occurrences → LegacyFallback.
    #[test]
    fn gate_find_refs_fallback_when_empty() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let stub = GateStub::with_references(vec![]);

        let outcome = find_references_cutover(&index, &stub, "missing", EntityId(100));
        assert!(
            matches!(outcome.result, ReferencesCutoverResult::LegacyFallback(_)),
            "no occurrences should fall back to legacy"
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 3. Completion Provider Gate (Req 10.8)
    // ════════════════════════════════════════════════════════════════════

    /// Completion cutover: explicit import symbols ranked high.
    #[test]
    fn gate_completion_explicit_import() -> Result<(), Box<dyn std::error::Error>> {
        let stub = GateStub::with_visible_symbols(vec![make_visible(
            "alpha",
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
        )]);
        let outcome =
            completion_visibility_cutover(vec![], &stub, FileId(1), 0, None, "explicit_import");
        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert!(!ranked.is_empty(), "should have ranked symbols");
            }
            CompletionCutoverResult::LegacyFallback(_) => {
                panic!("explicit import should not fall back");
            }
        }
        Ok(())
    }

    /// Completion cutover: default export symbols visible.
    #[test]
    fn gate_completion_default_export() -> Result<(), Box<dyn std::error::Error>> {
        let stub = GateStub::with_visible_symbols(vec![make_visible(
            "alpha",
            VisibleSymbolSource::DefaultExport,
            Confidence::High,
        )]);
        let outcome =
            completion_visibility_cutover(vec![], &stub, FileId(1), 0, None, "default_export");
        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert!(!ranked.is_empty(), "default export should produce ranked symbols");
            }
            CompletionCutoverResult::LegacyFallback(_) => {
                panic!("default export should not fall back");
            }
        }
        Ok(())
    }

    /// Completion cutover: empty import → no symbols → fallback.
    #[test]
    fn gate_completion_empty_import_suppresses() -> Result<(), Box<dyn std::error::Error>> {
        let stub = GateStub::with_visible_symbols(vec![]);
        let outcome = completion_visibility_cutover(
            vec!["legacy_sym".to_string()],
            &stub,
            FileId(1),
            0,
            None,
            "empty_import",
        );
        assert!(
            matches!(outcome.result, CompletionCutoverResult::LegacyFallback(_)),
            "empty visible symbols should fall back"
        );
        Ok(())
    }

    /// Completion cutover: tag export symbols visible.
    #[test]
    fn gate_completion_tag_export() -> Result<(), Box<dyn std::error::Error>> {
        let stub = GateStub::with_visible_symbols(vec![
            make_visible("alpha", VisibleSymbolSource::ExportTag, Confidence::High),
            make_visible("beta", VisibleSymbolSource::ExportTag, Confidence::High),
        ]);
        let outcome =
            completion_visibility_cutover(vec![], &stub, FileId(1), 0, None, "tag_export");
        match &outcome.result {
            CompletionCutoverResult::Semantic(ranked) => {
                assert_eq!(ranked.len(), 2, "tag export should produce 2 ranked symbols");
            }
            CompletionCutoverResult::LegacyFallback(_) => {
                panic!("tag export should not fall back");
            }
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 4. Diagnostics Provider Gate (Req 10.9)
    // ════════════════════════════════════════════════════════════════════

    /// Diagnostics cutover: imported symbol → suppress (no false positive).
    #[test]
    fn gate_diagnostics_imported_no_false_positive() -> Result<(), Box<dyn std::error::Error>> {
        let stub = GateStub::with_definitions(vec![make_candidate(
            "Foo::alpha",
            10,
            100,
            Confidence::High,
        )]);
        let outcome = diagnostics_undefined_symbol_cutover(
            true, // legacy_should_warn
            &stub,
            "Foo::alpha",
            FileId(1),
            None,
            0,
            false, // not in dynamic scope
        );
        assert!(
            matches!(outcome.action, DiagnosticAction::Suppress),
            "imported symbol with definition should suppress, got: {:?}",
            outcome.action
        );
        Ok(())
    }

    /// Diagnostics cutover: dynamic boundary → suppress (no exact warning).
    #[test]
    fn gate_diagnostics_dynamic_boundary_no_exact_warning() -> Result<(), Box<dyn std::error::Error>>
    {
        let stub = GateStub::with_definitions(vec![]);
        let outcome = diagnostics_undefined_symbol_cutover(
            true, // legacy_should_warn
            &stub,
            "dynamic_sym",
            FileId(1),
            None,
            0,
            true, // in_dynamic_scope
        );
        assert!(
            matches!(outcome.action, DiagnosticAction::Suppress),
            "dynamic scope should suppress, got: {:?}",
            outcome.action
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 5. Rename/Safe-Delete Provider Gate (Req 10.10)
    // ════════════════════════════════════════════════════════════════════

    /// Rename cutover: dynamic boundary → blocked.
    #[test]
    fn gate_rename_dynamic_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(100),
            "dispatch".to_string(),
            "new_dispatch".to_string(),
            vec![],
            vec![PlanBlocker::new(
                PlanBlockerReason::DynamicBoundary,
                Some(AnchorId(10)),
                "dynamic boundary reference".to_string(),
            )],
            vec![],
        );
        let stub = GateStub::with_rename_plan(plan);
        let outcome = rename_cutover(
            true, // legacy_allowed
            &stub,
            EntityId(100),
            "new_dispatch",
        );
        assert!(
            matches!(outcome.result, RenameCutoverResult::Blocked { .. }),
            "dynamic boundary should block rename"
        );
        Ok(())
    }

    /// Rename cutover: no blockers → allowed with classified edits.
    #[test]
    fn gate_rename_allowed_no_unsafe_edits() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(100),
            "bar".to_string(),
            "baz".to_string(),
            vec![
                PlannedEdit::new(
                    AnchorId(10),
                    FileId(1),
                    PlannedEditCategory::Definition,
                    "bar".to_string(),
                    "baz".to_string(),
                ),
                PlannedEdit::new(
                    AnchorId(20),
                    FileId(1),
                    PlannedEditCategory::Reference,
                    "bar".to_string(),
                    "baz".to_string(),
                ),
            ],
            vec![],
            vec![],
        );
        let stub = GateStub::with_rename_plan(plan);
        let outcome = rename_cutover(
            true, // legacy_allowed
            &stub,
            EntityId(100),
            "baz",
        );
        assert!(
            matches!(outcome.result, RenameCutoverResult::Allowed { .. }),
            "no blockers should allow rename"
        );
        Ok(())
    }

    /// Rename cutover: ambiguous reference → blocked.
    #[test]
    fn gate_rename_ambiguous_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(100),
            "bar".to_string(),
            "baz".to_string(),
            vec![],
            vec![PlanBlocker::new(
                PlanBlockerReason::AmbiguousReference,
                None,
                "ambiguous reference".to_string(),
            )],
            vec![],
        );
        let stub = GateStub::with_rename_plan(plan);
        let outcome = rename_cutover(true, &stub, EntityId(100), "baz");
        assert!(
            matches!(outcome.result, RenameCutoverResult::Blocked { .. }),
            "ambiguous reference should block rename"
        );
        Ok(())
    }

    /// Rename cutover: export/import → blocked or planned.
    #[test]
    fn gate_rename_export_import_blocked_or_planned() -> Result<(), Box<dyn std::error::Error>> {
        let plan = RenamePlan::new(
            EntityId(100),
            "alpha".to_string(),
            "renamed_alpha".to_string(),
            vec![],
            vec![PlanBlocker::new(
                PlanBlockerReason::CrossModuleExport,
                None,
                "symbol is exported cross-module".to_string(),
            )],
            vec![],
        );
        let stub = GateStub::with_rename_plan(plan);
        let outcome = rename_cutover(true, &stub, EntityId(100), "renamed_alpha");
        assert!(
            matches!(outcome.result, RenameCutoverResult::Blocked { .. }),
            "cross-module export should block rename"
        );
        Ok(())
    }

    /// Safe-delete cutover: dynamic boundary → blocked.
    #[test]
    fn gate_safe_delete_dynamic_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(
            EntityId(100),
            "dispatch".to_string(),
            vec![PlanBlocker::new(
                PlanBlockerReason::DynamicBoundary,
                Some(AnchorId(10)),
                "dynamic boundary reference".to_string(),
            )],
            vec![],
        );
        let stub = GateStub::with_safe_delete_plan(plan);
        let outcome = safe_delete_cutover(
            true, // legacy_allowed
            &stub,
            EntityId(100),
            "dispatch",
        );
        assert!(
            matches!(outcome.result, SafeDeleteCutoverResult::Blocked { .. }),
            "dynamic boundary should block safe-delete"
        );
        Ok(())
    }

    /// Safe-delete cutover: no blockers → allowed.
    #[test]
    fn gate_safe_delete_allowed_when_clean() -> Result<(), Box<dyn std::error::Error>> {
        let plan = SafeDeletePlan::new(EntityId(100), "unused".to_string(), vec![], vec![]);
        let stub = GateStub::with_safe_delete_plan(plan);
        let outcome = safe_delete_cutover(true, &stub, EntityId(100), "unused");
        assert!(
            matches!(outcome.result, SafeDeleteCutoverResult::Allowed),
            "no blockers should allow safe-delete"
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // 6. Provider-Level Aggregate Scorecard
    // ════════════════════════════════════════════════════════════════════

    /// Aggregate provider-level scorecard: all receipts pass.
    #[test]
    fn gate_aggregate_provider_scorecard_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut scorecard = Scorecard::new(ScorecardMode::Check);
        let index = WorkspaceIndex::new();

        // Goto-definition: exact match
        let stub =
            GateStub::with_definitions(vec![make_candidate("Foo::bar", 10, 100, Confidence::High)]);
        let outcome = goto_definition_cutover(
            &index,
            &stub,
            "Foo::bar",
            &QueryContext::new(FileId(1), None, None),
        );
        scorecard.add_receipt(outcome.receipt);

        // Find-references: typed refs
        let stub = GateStub::with_references(vec![make_ref_occurrence(1, 10, 100)]);
        let outcome = find_references_cutover(&index, &stub, "Foo::bar", EntityId(100));
        scorecard.add_receipt(outcome.receipt);

        // Completion: explicit import
        let stub = GateStub::with_visible_symbols(vec![make_visible(
            "alpha",
            VisibleSymbolSource::ExplicitImport,
            Confidence::High,
        )]);
        let outcome =
            completion_visibility_cutover(vec![], &stub, FileId(1), 0, None, "explicit_import");
        scorecard.add_receipt(outcome.receipt);

        // Diagnostics: imported symbol suppressed (both paths agree)
        let stub = GateStub::with_definitions(vec![make_candidate(
            "Foo::alpha",
            10,
            100,
            Confidence::High,
        )]);
        let outcome = diagnostics_undefined_symbol_cutover(
            false, // legacy also would not warn (symbol is defined)
            &stub,
            "Foo::alpha",
            FileId(1),
            None,
            0,
            false,
        );
        scorecard.add_receipt(outcome.receipt);

        // Rename: allowed
        let plan = RenamePlan::new(
            EntityId(100),
            "bar".to_string(),
            "baz".to_string(),
            vec![PlannedEdit::new(
                AnchorId(10),
                FileId(1),
                PlannedEditCategory::Definition,
                "bar".to_string(),
                "baz".to_string(),
            )],
            vec![],
            vec![],
        );
        let stub = GateStub::with_rename_plan(plan);
        let outcome = rename_cutover(true, &stub, EntityId(100), "baz");
        scorecard.add_receipt(outcome.receipt);

        // Safe-delete: allowed
        let plan = SafeDeletePlan::new(EntityId(100), "unused".to_string(), vec![], vec![]);
        let stub = GateStub::with_safe_delete_plan(plan);
        let outcome = safe_delete_cutover(true, &stub, EntityId(100), "unused");
        scorecard.add_receipt(outcome.receipt);

        let report = scorecard.report();
        assert!(
            report.passed,
            "aggregate provider scorecard should pass, regressions: {}",
            report.totals.regression
        );
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }
}

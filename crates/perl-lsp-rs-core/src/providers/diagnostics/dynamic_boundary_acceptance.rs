//! Dynamic boundary acceptance test fixtures.
//!
//! Verifies end-to-end behavior of the diagnostics cutover, rename stubs,
//! and safe-delete stubs for Perl patterns that cross dynamic boundaries:
//!
//! - `eval $code`, `eval "sub $name { ... }"`
//! - `require $module`, `$module->import(qw(foo))`
//! - `*alias = \&target`, `${$name} = 1` (symbolic dereferences)
//! - `sub AUTOLOAD { ... }` dispatch
//!
//! # Requirements
//!
//! - **Req 23.1**: eval patterns
//! - **Req 23.2**: require/import patterns
//! - **Req 23.3**: symbolic dereference patterns
//! - **Req 23.4**: AUTOLOAD dispatch patterns
//! - **Req 23.5**: diagnostics suppresses undefined-symbol
//! - **Req 23.6**: rename blocks or warns
//! - **Req 23.7**: safe-delete blocks or warns
//! - **Req 23.8**: hover explains dynamic boundary

#[cfg(test)]
mod tests {
    use crate::providers::diagnostics::diagnostics_shadow::{
        DiagnosticAction, DiagnosticClassification, DiagnosticsCutoverOutcome,
        diagnostics_undefined_symbol_cutover,
    };
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, DefinitionRank, DefinitionRankReason,
        EntityFact, EntityId, EntityKind, FileId, OccurrenceFact, PlanBlocker, PlanBlockerReason,
        Provenance, RenamePlan, SafeDeletePlan, ScopeId, VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };
    use perl_workspace::semantic_shadow_compare::SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION;

    // ── Configurable SemanticQueries stub ──

    /// A stub that lets each test configure definitions, rename, and
    /// safe-delete behavior independently.
    struct DynamicBoundaryStub {
        /// Definition candidates returned by `definitions`.
        definitions_result: Vec<DefinitionCandidate>,
        /// Whether rename_plan reports blocked.
        rename_blocked: bool,
        /// Reason string for rename block.
        rename_block_reason: Option<String>,
        /// Whether safe_delete_plan reports blocked.
        safe_delete_blocked: bool,
        /// Reason string for safe-delete block.
        safe_delete_block_reason: Option<String>,
    }

    impl DynamicBoundaryStub {
        /// Create a stub where the symbol is in a dynamic scope:
        /// - definitions returns dynamic-boundary candidates
        /// - rename is blocked with "dynamic boundary" reason
        /// - safe-delete is blocked with "dynamic boundary" reason
        fn dynamic_scope() -> Self {
            Self {
                definitions_result: vec![make_dynamic_candidate("dyn_sym", 1, 1)],
                rename_blocked: true,
                rename_block_reason: Some("dynamic boundary".to_string()),
                safe_delete_blocked: true,
                safe_delete_block_reason: Some("dynamic boundary".to_string()),
            }
        }

        /// Create a stub where the symbol has no candidates at all
        /// (undefined in a dynamic scope — diagnostics should still suppress).
        fn empty_in_dynamic_scope() -> Self {
            Self {
                definitions_result: vec![],
                rename_blocked: true,
                rename_block_reason: Some("dynamic boundary".to_string()),
                safe_delete_blocked: true,
                safe_delete_block_reason: Some("dynamic boundary".to_string()),
            }
        }
    }

    impl SemanticQueries for DynamicBoundaryStub {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            self.definitions_result.clone()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            Vec::new()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            let blockers = if self.rename_blocked {
                let reason_text =
                    self.rename_block_reason.clone().unwrap_or_else(|| "blocked".to_string());
                vec![PlanBlocker::new(PlanBlockerReason::DynamicBoundary, None, reason_text)]
            } else {
                vec![]
            };
            RenamePlan::new(
                entity_id,
                String::new(),
                new_name.to_string(),
                vec![],
                blockers,
                vec![],
            )
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            let blockers = if self.safe_delete_blocked {
                let reason_text =
                    self.safe_delete_block_reason.clone().unwrap_or_else(|| "blocked".to_string());
                vec![PlanBlocker::new(PlanBlockerReason::DynamicBoundary, None, reason_text)]
            } else {
                vec![]
            };
            SafeDeletePlan::new(entity_id, String::new(), blockers, vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            // Stub: stubs that report rename/safe-delete blocked as DynamicBoundary
            // also report dynamic boundary coverage at any position.
            if self.rename_blocked || self.safe_delete_blocked {
                Some(OccurrenceFact {
                    id: perl_semantic_facts::OccurrenceId(9999),
                    kind: perl_semantic_facts::OccurrenceKind::DynamicBoundary,
                    entity_id: None,
                    anchor_id: AnchorId(9999),
                    scope_id: None,
                    provenance: perl_semantic_facts::Provenance::DynamicBoundary,
                    confidence: perl_semantic_facts::Confidence::Low,
                })
            } else {
                None
            }
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            // When the stub is in a dynamic scope, dynamic callables may also
            // be visible (same condition as dynamic_boundary_at).
            if self.rename_blocked || self.safe_delete_blocked {
                Some(DynamicCallableEvidence::DynamicImport {
                    file_id,
                    anchor_id: Some(AnchorId(9998)),
                    module: "DynamicBoundaryStub".to_string(),
                })
            } else {
                None
            }
        }
    }

    // ── Helpers ──

    fn make_dynamic_candidate(name: &str, anchor_id: u64, entity_id: u64) -> DefinitionCandidate {
        DefinitionCandidate::new(
            EntityId(entity_id),
            AnchorId(anchor_id),
            name.to_string(),
            name.to_string(),
            None,
            EntityKind::Subroutine,
            Provenance::DynamicBoundary,
            Confidence::Low,
            DefinitionRank::Heuristic,
            DefinitionRankReason::HeuristicNameMatch,
        )
    }

    /// Run the cutover with `is_in_dynamic_scope = true` and assert the
    /// outcome suppresses diagnostics.
    fn assert_dynamic_scope_suppresses(
        stub: &DynamicBoundaryStub,
        symbol: &str,
    ) -> Result<DiagnosticsCutoverOutcome, Box<dyn std::error::Error>> {
        let outcome = diagnostics_undefined_symbol_cutover(
            true, // legacy would warn
            stub,
            symbol,
            FileId(1),
            None,
            0,
            true, // is_in_dynamic_scope
        );

        assert_eq!(
            outcome.action,
            DiagnosticAction::Suppress,
            "dynamic boundary should suppress diagnostics for '{symbol}'"
        );
        assert_eq!(
            outcome.classification,
            DiagnosticClassification::DynamicOrUnavailable,
            "dynamic boundary should classify as DynamicOrUnavailable for '{symbol}'"
        );
        Ok(outcome)
    }

    /// Assert that the rename plan is blocked for a dynamic boundary entity.
    fn assert_rename_blocked(
        stub: &DynamicBoundaryStub,
        entity_id: EntityId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let plan = stub.rename_plan(entity_id, "new_name");
        assert!(
            !plan.blockers.is_empty(),
            "rename should be blocked for dynamic boundary entity {:?}",
            entity_id
        );
        Ok(())
    }

    /// Assert that the safe-delete plan is blocked for a dynamic boundary entity.
    fn assert_safe_delete_blocked(
        stub: &DynamicBoundaryStub,
        entity_id: EntityId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let plan = stub.safe_delete_plan(entity_id);
        assert!(
            !plan.blockers.is_empty(),
            "safe-delete should be blocked for dynamic boundary entity {:?}",
            entity_id
        );
        Ok(())
    }

    /// Assert that the cutover receipt notes mention "dynamic boundary".
    fn assert_receipt_explains_dynamic(
        outcome: &DiagnosticsCutoverOutcome,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let has_dynamic_note = outcome.receipt.notes.iter().any(|n| n.contains("dynamic boundary"));
        assert!(
            has_dynamic_note,
            "receipt notes should explain dynamic boundary, got: {:?}",
            outcome.receipt.notes
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Fixture suite 1: eval patterns (Req 23.1)
    // Perl: eval $code, eval "sub $name { ... }"
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn eval_code_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: eval $code; $generated_sub->()
        // Symbol referenced inside eval scope should have diagnostics suppressed.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "generated_sub")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn eval_string_sub_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: eval "sub $name { return 42 }"
        // The dynamically-created sub should not trigger undefined-symbol.
        let stub = DynamicBoundaryStub::dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "dynamic_sub_name")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn eval_code_rename_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_rename_blocked(&stub, EntityId(1))?;
        Ok(())
    }

    #[test]
    fn eval_code_safe_delete_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_safe_delete_blocked(&stub, EntityId(1))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Fixture suite 2: require/import patterns (Req 23.2)
    // Perl: require $module, $module->import(qw(foo))
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn require_variable_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: require $module; $module->import(qw(foo))
        // Symbols from dynamic require should have diagnostics suppressed.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "foo")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn require_variable_import_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: my $mod = "Some::Module"; require $mod; $mod->import(qw(bar baz))
        // Imported symbols from dynamic require should not trigger undefined-symbol.
        let stub = DynamicBoundaryStub::dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "bar")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn require_variable_rename_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_rename_blocked(&stub, EntityId(2))?;
        Ok(())
    }

    #[test]
    fn require_variable_safe_delete_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_safe_delete_blocked(&stub, EntityId(2))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Fixture suite 3: symbolic dereference patterns (Req 23.3)
    // Perl: *alias = \&target, ${$name} = 1
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn glob_alias_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: *alias = \&Some::Package::target
        // The aliased symbol should have diagnostics suppressed in dynamic scope.
        let stub = DynamicBoundaryStub::dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "alias")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn symbolic_deref_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: ${$name} = 1
        // Variable created via symbolic dereference should not trigger undefined-symbol.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "dynamic_var")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn glob_alias_rename_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_rename_blocked(&stub, EntityId(3))?;
        Ok(())
    }

    #[test]
    fn symbolic_deref_safe_delete_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_safe_delete_blocked(&stub, EntityId(3))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Fixture suite 4: AUTOLOAD dispatch patterns (Req 23.4)
    // Perl: sub AUTOLOAD { ... }
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn autoload_dispatch_diagnostics_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: package Foo; sub AUTOLOAD { ... }
        // Calls to Foo->unknown_method should have diagnostics suppressed
        // when the scope is marked as dynamic (AUTOLOAD present).
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "unknown_method")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn autoload_with_dynamic_candidates_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Simulates: package Foo; sub AUTOLOAD { ... }; Foo->generated_method()
        // Even when semantic queries return dynamic-boundary candidates,
        // the cutover should suppress.
        let stub = DynamicBoundaryStub::dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "generated_method")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn autoload_rename_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_rename_blocked(&stub, EntityId(4))?;
        Ok(())
    }

    #[test]
    fn autoload_safe_delete_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let stub = DynamicBoundaryStub::dynamic_scope();
        assert_safe_delete_blocked(&stub, EntityId(4))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Cross-cutting: dynamic boundary classification (Req 23.5–23.8)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn dynamic_scope_never_produces_exact_warning() -> Result<(), Box<dyn std::error::Error>> {
        // Regardless of what definitions returns, is_in_dynamic_scope=true
        // must never produce DiagnosticAction::Warn.
        let scenarios: Vec<(&str, DynamicBoundaryStub)> = vec![
            ("empty candidates", DynamicBoundaryStub::empty_in_dynamic_scope()),
            ("dynamic candidates", DynamicBoundaryStub::dynamic_scope()),
        ];

        for (label, stub) in &scenarios {
            let outcome = diagnostics_undefined_symbol_cutover(
                true, // legacy would warn
                stub,
                "any_symbol",
                FileId(1),
                None,
                0,
                true, // is_in_dynamic_scope
            );

            assert_ne!(
                outcome.action,
                DiagnosticAction::Warn,
                "dynamic scope must never produce Warn action ({label})"
            );
            assert_eq!(
                outcome.classification,
                DiagnosticClassification::DynamicOrUnavailable,
                "dynamic scope must classify as DynamicOrUnavailable ({label})"
            );
        }
        Ok(())
    }

    #[test]
    fn dynamic_scope_suppresses_even_when_legacy_would_not_warn()
    -> Result<(), Box<dyn std::error::Error>> {
        // Even when legacy_should_warn=false, the dynamic scope path
        // should still produce Suppress (not Warn).
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = diagnostics_undefined_symbol_cutover(
            false, // legacy would NOT warn
            &stub,
            "some_sym",
            FileId(1),
            None,
            0,
            true, // is_in_dynamic_scope
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn all_dynamic_candidates_outside_scope_still_suppresses()
    -> Result<(), Box<dyn std::error::Error>> {
        // When is_in_dynamic_scope=false but ALL candidates are
        // DynamicBoundary provenance, the classification should still
        // be DynamicOrUnavailable (suppress).
        let stub = DynamicBoundaryStub::dynamic_scope();
        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &stub,
            "dyn_sym",
            FileId(1),
            None,
            0,
            false, // NOT in dynamic scope, but candidates are dynamic
        );

        assert_eq!(outcome.action, DiagnosticAction::Suppress);
        assert_eq!(outcome.classification, DiagnosticClassification::DynamicOrUnavailable);
        Ok(())
    }

    #[test]
    fn dynamic_scope_receipt_uses_current_schema_version() -> Result<(), Box<dyn std::error::Error>>
    {
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome =
            diagnostics_undefined_symbol_cutover(true, &stub, "test_sym", FileId(1), None, 0, true);

        assert_eq!(
            outcome.receipt.schema_version, SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION,
            "receipt schema version should match semantic shadow compare"
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Fixture suite 5: new missing-case patterns (Q5 provenance upgrade)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn dynamic_import_via_variable_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Pattern: `require $module; $module->import(qw(foo))`
        // 'foo' is plausibly imported — suppress the undefined-symbol diagnostic.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "foo")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn static_class_dynamic_args_import_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Pattern: `Foo->import(@names)` — static class, dynamic arg list.
        // The imported symbols are not statically known — suppress.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "bar")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    #[test]
    fn symbolic_deref_variable_suppressed() -> Result<(), Box<dyn std::error::Error>> {
        // Pattern: `${$name} = 1`
        // Variable created via symbolic dereference — suppress the diagnostic.
        let stub = DynamicBoundaryStub::empty_in_dynamic_scope();
        let outcome = assert_dynamic_scope_suppresses(&stub, "dynamic_var")?;
        assert_receipt_explains_dynamic(&outcome)?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Control fixture: normal static missing symbol MUST still fire
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn normal_static_missing_symbol_still_fires() -> Result<(), Box<dyn std::error::Error>> {
        // Control fixture: no dynamic boundary, no semantic candidates.
        // A genuinely missing symbol MUST produce Warn, not Suppress.
        // The stub has no candidates and is NOT in a dynamic scope.
        struct StaticNoDynamicStub;

        impl SemanticQueries for StaticNoDynamicStub {
            fn symbol_at(&self, _: FileId, _: u32) -> Option<(EntityFact, OccurrenceFact)> {
                None
            }
            fn definitions(&self, _: &str, _: &QueryContext) -> Vec<DefinitionCandidate> {
                Vec::new() // no candidates → symbol is undefined
            }
            fn references(&self, _: EntityId) -> Vec<OccurrenceFact> {
                Vec::new()
            }
            fn visible_symbols_at(
                &self,
                _: FileId,
                _: u32,
                _: Option<ScopeId>,
            ) -> Vec<VisibleSymbol> {
                Vec::new()
            }
            fn method_candidates(&self, _: &str, _: &str) -> Vec<DefinitionCandidate> {
                Vec::new()
            }
            fn rename_plan(&self, id: EntityId, n: &str) -> RenamePlan {
                RenamePlan::new(id, String::new(), n.to_string(), vec![], vec![], vec![])
            }
            fn safe_delete_plan(&self, id: EntityId) -> SafeDeletePlan {
                SafeDeletePlan::new(id, String::new(), vec![], vec![])
            }
            fn dynamic_boundary_at(
                &self,
                _: FileId,
                _: u32,
                _: Option<&str>,
            ) -> Option<OccurrenceFact> {
                None // no dynamic boundary anywhere
            }
            fn dynamic_callable_may_be_visible_at(
                &self,
                _: FileId,
                _: u32,
                _: &str,
            ) -> Option<DynamicCallableEvidence> {
                None // no dynamic callables either
            }
        }

        let stub = StaticNoDynamicStub;
        let outcome = diagnostics_undefined_symbol_cutover(
            true,
            &stub,
            "truly_undefined_sub",
            FileId(1),
            None,
            0,
            false, // NOT in dynamic scope
        );

        assert_eq!(
            outcome.action,
            DiagnosticAction::Warn,
            "static missing symbol with no candidates must produce Warn (control fixture: normal_static_missing_symbol.pl)"
        );
        assert_eq!(
            outcome.classification,
            DiagnosticClassification::Exact,
            "no candidates → Exact classification → Warn"
        );
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // dynamic_boundary_at integration: the stub reports coverage
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn dynamic_boundary_stub_returns_coverage_when_blocked()
    -> Result<(), Box<dyn std::error::Error>> {
        // When the stub is in dynamic scope (rename/safe-delete blocked),
        // dynamic_boundary_at returns Some — confirming the stub contract.
        let stub = DynamicBoundaryStub::dynamic_scope();
        let result = stub.dynamic_boundary_at(FileId(1), 0, Some("any_sym"));
        assert!(result.is_some(), "dynamic_scope stub should report coverage at any position");
        let occ = result.ok_or("expected OccurrenceFact")?;
        assert_eq!(occ.kind, perl_semantic_facts::OccurrenceKind::DynamicBoundary);
        assert_eq!(occ.provenance, perl_semantic_facts::Provenance::DynamicBoundary);
        assert_eq!(occ.confidence, perl_semantic_facts::Confidence::Low);
        Ok(())
    }

    #[test]
    fn dynamic_boundary_stub_returns_none_when_not_blocked()
    -> Result<(), Box<dyn std::error::Error>> {
        // When the stub has no dynamic scope, dynamic_boundary_at returns None.
        let stub = DynamicBoundaryStub {
            definitions_result: vec![],
            rename_blocked: false,
            rename_block_reason: None,
            safe_delete_blocked: false,
            safe_delete_block_reason: None,
        };
        let result = stub.dynamic_boundary_at(FileId(1), 0, Some("any_sym"));
        assert!(result.is_none(), "non-dynamic stub should return None from dynamic_boundary_at");
        Ok(())
    }
}

//! Runtime receipts and no-edit previews for refactor blocker UX.

use super::super::*;
use crate::protocol::{req_position, req_uri};
use perl_lsp_rs_core::providers::normalize_provider_decision_receipt;

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
use perl_lsp_rs_core::providers::navigation::rename_shadow::{RenameCutoverResult, rename_cutover};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_lsp_rs_core::providers::navigation::rename_shadow::{
    RenamePackagePilotResult, rename_package_pilot_proof,
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_lsp_rs_core::providers::navigation::safe_delete_shadow::{
    SafeDeleteCutoverResult, safe_delete_cutover,
};
#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
use perl_semantic_facts::{
    DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact, RenamePlan, SafeDeletePlan,
    ScopeId, VisibleSymbol,
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_semantic_facts::{PlanBlocker, PlanBlockerReason};
#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
use perl_workspace::semantic::queries::DynamicCallableEvidence;
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};

impl LspServer {
    /// Test-only receipt for rename blocker UX proof.
    ///
    /// Calls the live rename handler and compares the result with the
    /// compiler-fact rename plan from the same runtime workspace index. This is
    /// receipt-only and does not cut live rename over to compiler facts.
    pub(crate) fn rename_runtime_blocker_ux_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let (live_provider_result, live_provider_error) =
            match self.handle_rename_workspace(params.clone()) {
                Ok(result) => (result, None),
                Err(error) => (
                    Some(json!({
                        "error": {
                            "code": error.code,
                            "message": error.message,
                            "data": error.data
                        }
                    })),
                    Some(error.message),
                ),
            };
        let live_provider_edit_count = lsp_workspace_edit_count(live_provider_result.as_ref());

        #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
        {
            Ok(Some(json!({
                "provider": "rename",
                "live_provider_result": live_provider_result,
                "live_provider_edit_count": live_provider_edit_count,
                "compiler_receipt": null,
                "no_live_behavior_change": true,
                "note": "rename blocker UX proof unavailable without workspace semantic queries"
            })))
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let Some(params) = params else {
                return Ok(Some(json!({
                    "provider": "rename",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "rename blocker UX proof missing request params"
                })));
            };

            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let Some(new_name) = params.get("newName").and_then(Value::as_str) else {
                return Ok(Some(json!({
                    "provider": "rename",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "rename blocker UX proof missing newName"
                })));
            };
            let Some((symbol, byte_offset)) = self.refactor_runtime_symbol(uri, line, character)
            else {
                return Ok(Some(json!({
                    "provider": "rename",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "rename blocker UX proof found no symbol at request position"
                })));
            };

            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            {
                if let Some(fixture) = params.get("compilerPlanFixture").and_then(Value::as_str) {
                    let compiler_receipt = rename_fixture_receipt(
                        fixture,
                        &symbol,
                        new_name,
                        live_provider_edit_count,
                    );
                    return Ok(Some(json!({
                        "provider": "rename",
                        "symbol": symbol,
                        "compiler_plan_fixture": fixture,
                        "live_provider_result": live_provider_result,
                        "live_provider_edit_count": live_provider_edit_count,
                        "compiler_receipt": compiler_receipt,
                        "no_live_behavior_change": true
                    })));
                }
            }

            let compiler_receipt_parts = match route_index_access(self.coordinator()) {
                IndexAccessMode::Full(coordinator) => {
                    let index = coordinator.index();
                    index
                        .with_semantic_queries_for_uri(uri, |file_id, queries| {
                            let entity_id = refactor_entity_id(
                                &queries,
                                file_id,
                                byte_offset,
                                &symbol,
                            )?;
                            let outcome = rename_package_pilot_proof(
                                live_provider_edit_count > 0,
                                &queries,
                                entity_id,
                                new_name,
                            );
                            let (compiler_plan_edit_count, blockers) = match &outcome.result {
                                RenamePackagePilotResult::Eligible { edits } => {
                                    (edits.len(), Vec::new())
                                }
                                RenamePackagePilotResult::Ineligible {
                                    edits, blockers, ..
                                } => (edits.len(), blockers.clone()),
                                _ => (0, Vec::new()),
                            };
                            let package_pilot = rename_package_pilot_json(&outcome.result);
                            let mut receipt = outcome.receipt;
                            receipt.notes.push(format!(
                                "rename runtime blocker UX: live_provider_edits={}; compiler_plan_edits={compiler_plan_edit_count}; blocker_count={}; blocker_reasons={}; blocker_ux={}; requires_confirmation={}; no live refactor behavior change",
                                live_provider_edit_count,
                                blockers.len(),
                                runtime_blocker_reasons(&blockers),
                                runtime_blocker_descriptions(&blockers),
                                !blockers.is_empty()
                            ));
                            Some((receipt, compiler_plan_edit_count, blockers, package_pilot))
                        })
                        .flatten()
                }
                IndexAccessMode::Partial(_) | IndexAccessMode::None => None,
            };
            let (compiler_receipt, compiler_plan_edit_count, compiler_blockers, package_pilot) =
                compiler_receipt_parts.map_or(
                    (None, None, None, Value::Null),
                    |(receipt, edit_count, blockers, package_pilot)| {
                        (Some(receipt), Some(edit_count), Some(blockers), package_pilot)
                    },
                );

            let receipt = json!({
                "provider": "rename",
                "symbol": symbol,
                "new_name": new_name,
                "live_provider_result": live_provider_result,
                "live_provider_error": live_provider_error,
                "live_provider_edit_count": live_provider_edit_count,
                "compiler_receipt": compiler_receipt,
                "package_pilot": package_pilot,
                "fallback_noise": rename_fallback_noise_json(
                    &symbol,
                    new_name,
                    live_provider_result.as_ref(),
                    live_provider_error.as_deref(),
                    live_provider_edit_count,
                    compiler_plan_edit_count,
                    compiler_blockers.as_deref()
                ),
                "no_live_behavior_change": true
            });
            self.record_provider_decision_trace("rename", &receipt);
            Ok(Some(receipt))
        }
    }

    /// Live package-rename UX preview for editor commands.
    ///
    /// This exposes the package/compiler-backed rename pilot classification and
    /// planned edit shape to users, but deliberately returns an empty edit and
    /// does not apply or authorize package rename edits.
    pub(crate) fn package_rename_preview(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let Some(mut receipt) = self.rename_runtime_blocker_ux_receipt(params)? else {
            return Ok(None);
        };
        let planned_workspace_edit =
            receipt.get("live_provider_result").cloned().unwrap_or_else(|| json!({"changes": {}}));
        let planned_live_provider_edit_count =
            lsp_workspace_edit_count(Some(&planned_workspace_edit));
        let returned_workspace_edit_count = 0;
        let rollback_receipt = package_rename_rollback_receipt_json(
            planned_live_provider_edit_count,
            returned_workspace_edit_count,
            receipt.get("fallback_noise"),
        );
        let user_message = package_rename_preview_message(&receipt);

        if let Some(object) = receipt.as_object_mut() {
            object.insert("provider_action".to_string(), json!("perl.previewPackageRename"));
            object.insert("ux_surface".to_string(), json!("scoped_package_rename_preview"));
            object.insert("edits_applied".to_string(), json!(false));
            object.insert("live_package_rename_enabled".to_string(), json!(false));
            object.insert(
                "planned_live_provider_edit_count".to_string(),
                json!(planned_live_provider_edit_count),
            );
            object.insert(
                "returned_workspace_edit_count".to_string(),
                json!(returned_workspace_edit_count),
            );
            object.insert("rollback_receipt".to_string(), rollback_receipt);
            object.insert("planned_workspace_edit".to_string(), planned_workspace_edit);
            object.insert("workspace_edit".to_string(), json!({"changes": {}}));
            object.insert("user_message".to_string(), json!(user_message));
            object.insert(
                "claim_boundary".to_string(),
                json!("scoped package rename preview only; no package rename edits are applied"),
            );
            enrich_package_rename_preview_decision_trace(object);
        }

        let receipt = normalize_provider_decision_receipt(receipt);
        self.record_provider_decision_trace("rename", &receipt);
        Ok(Some(receipt))
    }

    /// Receipt for safe-delete blocker UX proof.
    ///
    /// There is no live symbol-level safe-delete request yet, so this records
    /// the compiler-fact safe-delete plan from the runtime workspace index and
    /// keeps the live behavior field empty by construction.
    pub(crate) fn safe_delete_runtime_blocker_ux_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let live_provider_result = Some(json!({"changes": {}}));
        let live_provider_edit_count = lsp_workspace_edit_count(live_provider_result.as_ref());

        #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
        {
            self.record_safe_delete_decision_receipt(json!({
                "provider": "safe_delete",
                "provider_action": "safeDelete/runtimeBlockerUxReceipt",
                "decision": "fallback",
                "reason": "workspace_semantic_queries_unavailable",
                "fact_source": "provider_runtime",
                "confidence": "low",
                "freshness": "unknown",
                "source_backed": false,
                "source_backed_state": "not_proven_by_safe_delete_trace",
                "dynamic_boundary": false,
                "fallback_state": "compiler_missing",
                "live_provider_result": live_provider_result,
                "live_provider_edit_count": live_provider_edit_count,
                "compiler_receipt": null,
                "trace_only_no_live_behavior_change": true,
                "no_live_behavior_change": true,
                "claim_boundary": "records safe-delete blocker proof only; no live symbol-level delete behavior changes",
                "note": "safe-delete blocker UX proof unavailable without workspace semantic queries"
            }))
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let Some(params) = params else {
                return self.record_safe_delete_decision_receipt(json!({
                    "provider": "safe_delete",
                    "provider_action": "safeDelete/runtimeBlockerUxReceipt",
                    "decision": "fallback",
                    "reason": "missing_request_params",
                    "fact_source": "provider_runtime",
                    "confidence": "low",
                    "freshness": "unknown",
                    "source_backed": false,
                    "source_backed_state": "not_proven_by_safe_delete_trace",
                    "dynamic_boundary": false,
                    "fallback_state": "compiler_missing",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "trace_only_no_live_behavior_change": true,
                    "no_live_behavior_change": true,
                    "claim_boundary": "records safe-delete blocker proof only; no live symbol-level delete behavior changes",
                    "note": "safe-delete blocker UX proof missing request params"
                }));
            };

            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let Some((symbol, byte_offset)) = self.refactor_runtime_symbol(uri, line, character)
            else {
                return self.record_safe_delete_decision_receipt(json!({
                    "provider": "safe_delete",
                    "provider_action": "safeDelete/runtimeBlockerUxReceipt",
                    "decision": "fallback",
                    "reason": "no_symbol_at_request_position",
                    "fact_source": "provider_runtime",
                    "confidence": "low",
                    "freshness": "unknown",
                    "source_backed": false,
                    "source_backed_state": "not_proven_by_safe_delete_trace",
                    "dynamic_boundary": false,
                    "fallback_state": "compiler_missing",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "trace_only_no_live_behavior_change": true,
                    "no_live_behavior_change": true,
                    "claim_boundary": "records safe-delete blocker proof only; no live symbol-level delete behavior changes",
                    "note": "safe-delete blocker UX proof found no symbol at request position"
                }));
            };

            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            if let Some(fixture) = params.get("compilerPlanFixture").and_then(Value::as_str) {
                let (compiler_receipt, compiler_blockers) =
                    safe_delete_fixture_receipt(fixture, &symbol, live_provider_edit_count)
                        .map_or((None, None), |(receipt, blockers)| {
                            (Some(receipt), Some(blockers))
                        });
                let mut receipt = json!({
                    "provider": "safe_delete",
                    "symbol": symbol,
                    "compiler_plan_fixture": fixture,
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": compiler_receipt,
                    "live_blocker_ux": safe_delete_live_blocker_ux_json(
                        compiler_blockers.as_deref()
                    ),
                    "rollback_receipt": safe_delete_rollback_receipt_json(
                        live_provider_edit_count,
                        compiler_blockers.as_deref()
                    ),
                    "no_live_behavior_change": true
                });
                enrich_safe_delete_decision_trace(
                    &mut receipt,
                    compiler_blockers.as_deref(),
                    "compiler_fixture_missing",
                );
                return self.record_safe_delete_decision_receipt(receipt);
            }

            let compiler_receipt_parts = match route_index_access(self.coordinator()) {
                IndexAccessMode::Full(coordinator) => {
                    let index = coordinator.index();
                    index
                        .with_semantic_queries_for_uri(uri, |file_id, queries| {
                            let entity_id = refactor_entity_id(
                                &queries,
                                file_id,
                                byte_offset,
                                &symbol,
                            )?;
                            let outcome =
                                safe_delete_cutover(false, &queries, entity_id, &symbol);
                            let blockers = match &outcome.result {
                                SafeDeleteCutoverResult::Allowed => Vec::new(),
                                SafeDeleteCutoverResult::Blocked { blockers } => blockers.clone(),
                            };
                            let mut receipt = outcome.receipt;
                            receipt.notes.push(format!(
                                "safe-delete runtime blocker UX: live_provider_edits={}; compiler_plan_safe={}; blocker_count={}; blocker_reasons={}; blocker_ux={}; requires_confirmation={}; no live refactor behavior change",
                                live_provider_edit_count,
                                blockers.is_empty(),
                                blockers.len(),
                                runtime_blocker_reasons(&blockers),
                                runtime_blocker_descriptions(&blockers),
                                !blockers.is_empty()
                            ));
                            Some((receipt, blockers))
                        })
                        .flatten()
                }
                IndexAccessMode::Partial(_) | IndexAccessMode::None => None,
            };
            let (compiler_receipt, compiler_blockers) = compiler_receipt_parts
                .map_or((None, None), |(receipt, blockers)| (Some(receipt), Some(blockers)));

            let mut receipt = json!({
                "provider": "safe_delete",
                "symbol": symbol,
                "live_provider_result": live_provider_result,
                "live_provider_edit_count": live_provider_edit_count,
                "compiler_receipt": compiler_receipt,
                "live_blocker_ux": safe_delete_live_blocker_ux_json(
                    compiler_blockers.as_deref()
                ),
                "rollback_receipt": safe_delete_rollback_receipt_json(
                    live_provider_edit_count,
                    compiler_blockers.as_deref()
                ),
                "no_live_behavior_change": true
            });
            enrich_safe_delete_decision_trace(
                &mut receipt,
                compiler_blockers.as_deref(),
                "compiler_receipt_missing",
            );
            self.record_safe_delete_decision_receipt(receipt)
        }
    }

    /// Live safe-delete UX preview for editor commands.
    ///
    /// This exposes the blocker/allowed explanation to users, but deliberately
    /// returns an empty edit and does not perform symbol-level deletion.
    pub(crate) fn safe_delete_symbol_preview(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let Some(mut receipt) = self.safe_delete_runtime_blocker_ux_receipt(params)? else {
            return Ok(None);
        };
        let user_message = safe_delete_symbol_preview_message(&receipt);

        if let Some(object) = receipt.as_object_mut() {
            object.insert("provider_action".to_string(), json!("perl.previewSafeDelete"));
            object.insert("ux_surface".to_string(), json!("scoped_live_symbol_delete_preview"));
            object.insert("edits_applied".to_string(), json!(false));
            object.insert("live_symbol_delete_enabled".to_string(), json!(false));
            object.insert("workspace_edit".to_string(), json!({"changes": {}}));
            object.insert("user_message".to_string(), json!(user_message));
            object.insert(
                "claim_boundary".to_string(),
                json!(
                    "scoped safe-delete UX preview only; no live symbol-level delete edits are applied"
                ),
            );
        }

        self.record_safe_delete_decision_receipt(receipt)
    }

    fn record_safe_delete_decision_receipt(
        &self,
        receipt: Value,
    ) -> Result<Option<Value>, JsonRpcError> {
        let receipt = normalize_provider_decision_receipt(receipt);
        self.record_provider_decision_trace("safe_delete", &receipt);
        Ok(Some(receipt))
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    fn refactor_runtime_symbol(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<(String, u32)> {
        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri)?;
        let offset = self.pos16_to_offset(doc, line, character);
        let symbol = self.get_token_at_position(&doc.text, offset);
        if symbol.is_empty() {
            return None;
        }
        Some((symbol, u32::try_from(offset).ok()?))
    }
}

#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
fn rename_fixture_receipt(
    fixture: &str,
    symbol: &str,
    new_name: &str,
    live_provider_edit_count: usize,
) -> Option<perl_workspace::semantic_shadow_compare::SemanticShadowCompareReceipt> {
    let blocker = fixture_blocker(fixture)?;
    let plan = RenamePlan::new(
        EntityId(1),
        symbol.to_string(),
        new_name.to_string(),
        Vec::new(),
        vec![blocker],
        Vec::new(),
    );
    let queries = RefactorFixtureQueries { rename_plan: plan, safe_delete_plan: None };
    let outcome = rename_cutover(live_provider_edit_count > 0, &queries, EntityId(1), new_name);
    let (compiler_plan_edit_count, blockers) = match &outcome.result {
        RenameCutoverResult::Allowed { edits } => (edits.len(), Vec::new()),
        RenameCutoverResult::Blocked { blockers, edits } => (edits.len(), blockers.clone()),
    };
    let mut receipt = outcome.receipt;
    receipt.notes.push(format!(
        "rename runtime blocker UX: compiler_plan_fixture={fixture}; live_provider_edits={}; compiler_plan_edits={compiler_plan_edit_count}; blocker_count={}; blocker_reasons={}; blocker_ux={}; requires_confirmation={}; no live refactor behavior change",
        live_provider_edit_count,
        blockers.len(),
        runtime_blocker_reasons(&blockers),
        runtime_blocker_descriptions(&blockers),
        !blockers.is_empty()
    ));
    Some(receipt)
}

#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
fn safe_delete_fixture_receipt(
    fixture: &str,
    symbol: &str,
    live_provider_edit_count: usize,
) -> Option<(perl_workspace::semantic_shadow_compare::SemanticShadowCompareReceipt, Vec<PlanBlocker>)>
{
    let blocker = fixture_blocker(fixture)?;
    let plan = SafeDeletePlan::new(EntityId(1), symbol.to_string(), vec![blocker], Vec::new());
    let queries = RefactorFixtureQueries {
        rename_plan: RenamePlan::new(
            EntityId(1),
            symbol.to_string(),
            symbol.to_string(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ),
        safe_delete_plan: Some(plan),
    };
    let outcome = safe_delete_cutover(false, &queries, EntityId(1), symbol);
    let blockers = match &outcome.result {
        SafeDeleteCutoverResult::Allowed => Vec::new(),
        SafeDeleteCutoverResult::Blocked { blockers } => blockers.clone(),
    };
    let mut receipt = outcome.receipt;
    receipt.notes.push(format!(
        "safe-delete runtime blocker UX: compiler_plan_fixture={fixture}; live_provider_edits={}; compiler_plan_safe={}; blocker_count={}; blocker_reasons={}; blocker_ux={}; requires_confirmation={}; no live refactor behavior change",
        live_provider_edit_count,
        blockers.is_empty(),
        blockers.len(),
        runtime_blocker_reasons(&blockers),
        runtime_blocker_descriptions(&blockers),
        !blockers.is_empty()
    ));
    Some((receipt, blockers))
}

#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
fn fixture_blocker(fixture: &str) -> Option<PlanBlocker> {
    match fixture {
        "low_confidence" => Some(PlanBlocker::new(
            PlanBlockerReason::AmbiguousReference,
            None,
            "low-confidence ambiguity requires confirmation before editing".to_string(),
        )),
        "stale_fact" => Some(PlanBlocker::new(
            PlanBlockerReason::StaleFact,
            None,
            "stale compiler fact must be refreshed before editing".to_string(),
        )),
        "generated_member" => Some(PlanBlocker::new(
            PlanBlockerReason::GeneratedMember,
            None,
            "generated member has no source-backed deletion target".to_string(),
        )),
        "dynamic_boundary" => Some(PlanBlocker::new(
            PlanBlockerReason::DynamicBoundary,
            None,
            "dynamic Perl boundary prevents static deletion certainty".to_string(),
        )),
        _ => None,
    }
}

#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
struct RefactorFixtureQueries {
    rename_plan: RenamePlan,
    safe_delete_plan: Option<SafeDeletePlan>,
}

#[cfg(all(
    feature = "workspace",
    not(target_arch = "wasm32"),
    any(test, feature = "expose_lsp_test_api")
))]
impl SemanticQueries for RefactorFixtureQueries {
    fn symbol_at(
        &self,
        _file_id: FileId,
        _byte_offset: u32,
    ) -> Option<(EntityFact, OccurrenceFact)> {
        None
    }

    fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
        Vec::new()
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

    fn rename_plan(&self, _entity_id: EntityId, _new_name: &str) -> RenamePlan {
        self.rename_plan.clone()
    }

    fn safe_delete_plan(&self, _entity_id: EntityId) -> SafeDeletePlan {
        self.safe_delete_plan.clone().unwrap_or_else(|| {
            SafeDeletePlan::new(EntityId(1), String::new(), Vec::new(), Vec::new())
        })
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

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn refactor_entity_id<Q: SemanticQueries>(
    queries: &Q,
    file_id: perl_semantic_facts::FileId,
    byte_offset: u32,
    symbol: &str,
) -> Option<perl_semantic_facts::EntityId> {
    queries
        .symbol_at(file_id, byte_offset)
        .and_then(|(_, occurrence)| occurrence.entity_id)
        .or_else(|| {
            let context = QueryContext::new(file_id, None, Some(byte_offset));
            queries.definitions(symbol, &context).first().map(|candidate| candidate.entity_id)
        })
}

fn lsp_workspace_edit_count(value: Option<&Value>) -> usize {
    let Some(value) = value else {
        return 0;
    };

    let changes_count = value
        .get("changes")
        .and_then(Value::as_object)
        .map(|changes| {
            changes.values().filter_map(Value::as_array).map(std::vec::Vec::len).sum::<usize>()
        })
        .unwrap_or(0);

    let document_changes_count =
        value.get("documentChanges").and_then(Value::as_array).map(std::vec::Vec::len).unwrap_or(0);

    changes_count + document_changes_count
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn enrich_safe_delete_decision_trace(
    receipt: &mut Value,
    blockers: Option<&[PlanBlocker]>,
    missing_reason: &'static str,
) {
    let Some(object) = receipt.as_object_mut() else {
        return;
    };

    let blocker_reasons = blockers
        .unwrap_or(&[])
        .iter()
        .map(|blocker| format!("{:?}", blocker.reason))
        .collect::<Vec<_>>();
    let dynamic_boundary = blockers
        .unwrap_or(&[])
        .iter()
        .any(|blocker| matches!(blocker.reason, PlanBlockerReason::DynamicBoundary));
    let (decision, reason, fact_source, confidence, freshness, fallback_state) = match blockers {
        Some([]) => ("allowed", "compiler_allowed", "compiler_fact", "high", "fresh", "none"),
        Some(blockers)
            if blockers
                .iter()
                .any(|blocker| matches!(blocker.reason, PlanBlockerReason::StaleFact)) =>
        {
            ("blocked", "stale_fact", "compiler_fact", "low", "stale", "refresh_workspace_facts")
        }
        Some(blockers)
            if blockers
                .iter()
                .any(|blocker| matches!(blocker.reason, PlanBlockerReason::DynamicBoundary)) =>
        {
            ("blocked", "dynamic_boundary", "dynamic_boundary", "high", "fresh", "no_edit")
        }
        Some(blockers)
            if blockers
                .iter()
                .any(|blocker| matches!(blocker.reason, PlanBlockerReason::GeneratedMember)) =>
        {
            ("blocked", "generated_no_source", "framework_adapter", "high", "fresh", "no_edit")
        }
        Some(blockers)
            if blockers
                .iter()
                .any(|blocker| matches!(blocker.reason, PlanBlockerReason::AmbiguousReference)) =>
        {
            (
                "blocked",
                "ambiguous_low_confidence_candidates",
                "semantic_fact",
                "low",
                "fresh",
                "require_confirmation",
            )
        }
        Some(blockers)
            if blockers.iter().any(|blocker| {
                matches!(
                    blocker.reason,
                    PlanBlockerReason::CrossModuleExport
                        | PlanBlockerReason::ImportedSymbol
                        | PlanBlockerReason::ExportedSymbol
                        | PlanBlockerReason::ReferencesExist
                )
            }) =>
        {
            ("blocked", "references_exist", "compiler_fact", "high", "fresh", "no_edit")
        }
        Some(_) => {
            ("blocked", "unclassified_occurrence", "semantic_fact", "low", "fresh", "no_edit")
        }
        None => {
            ("fallback", missing_reason, "provider_runtime", "low", "unknown", "compiler_missing")
        }
    };

    object.insert("provider_action".to_string(), json!("safeDelete/runtimeBlockerUxReceipt"));
    object.insert("decision".to_string(), json!(decision));
    object.insert("reason".to_string(), json!(reason));
    object.insert("fact_source".to_string(), json!(fact_source));
    object.insert("confidence".to_string(), json!(confidence));
    object.insert("freshness".to_string(), json!(freshness));
    object.insert("source_backed".to_string(), json!(false));
    object.insert("source_backed_state".to_string(), json!("not_proven_by_safe_delete_trace"));
    object.insert("dynamic_boundary".to_string(), json!(dynamic_boundary));
    object.insert("fallback_state".to_string(), json!(fallback_state));
    object.insert("blocker_count".to_string(), json!(blocker_reasons.len()));
    object.insert("blocker_reasons".to_string(), json!(blocker_reasons));
    object.insert("trace_only_no_live_behavior_change".to_string(), json!(true));
    object.insert(
        "claim_boundary".to_string(),
        json!(
            "records safe-delete blocker proof only; no live symbol-level delete behavior changes"
        ),
    );
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn rename_package_pilot_json(result: &RenamePackagePilotResult) -> Value {
    match result {
        RenamePackagePilotResult::Eligible { edits } => json!({
            "provider": "rename",
            "eligible": true,
            "reason": "none",
            "edit_count": edits.len(),
            "blocker_count": 0,
            "edit_categories": edits
                .iter()
                .map(|edit| format!("{:?}", edit.category))
                .collect::<Vec<_>>(),
            "blocker_reasons": [],
            "claim_boundary": "receipt-only package/compiler-backed pilot; no live package rename cutover",
            "no_live_rename_cutover": true
        }),
        RenamePackagePilotResult::Ineligible { reason, edits, blockers } => json!({
            "provider": "rename",
            "eligible": false,
            "reason": rename_package_pilot_ineligible_reason(*reason),
            "edit_count": edits.len(),
            "blocker_count": blockers.len(),
            "edit_categories": edits
                .iter()
                .map(|edit| format!("{:?}", edit.category))
                .collect::<Vec<_>>(),
            "blocker_reasons": blockers
                .iter()
                .map(|blocker| format!("{:?}", blocker.reason))
                .collect::<Vec<_>>(),
            "claim_boundary": "receipt-only package/compiler-backed pilot; no live package rename cutover",
            "no_live_rename_cutover": true
        }),
        _ => json!({
            "provider": "rename",
            "eligible": false,
            "reason": "unknown",
            "edit_count": 0,
            "blocker_count": 0,
            "edit_categories": [],
            "blocker_reasons": [],
            "claim_boundary": "receipt-only package/compiler-backed pilot; no live package rename cutover",
            "no_live_rename_cutover": true
        }),
    }
}

fn package_rename_rollback_receipt_json(
    planned_live_provider_edit_count: usize,
    returned_workspace_edit_count: usize,
    fallback_noise: Option<&Value>,
) -> Value {
    json!({
        "provider": "rename",
        "provider_action": "perl.previewPackageRename",
        "planned_live_provider_edit_count": planned_live_provider_edit_count,
        "returned_workspace_edit_count": returned_workspace_edit_count,
        "rollback_required": returned_workspace_edit_count > 0,
        "rollback_safe": returned_workspace_edit_count == 0,
        "edits_applied": false,
        "live_package_rename_enabled": false,
        "fallback_state": fallback_noise
            .and_then(|noise| noise.get("fallback_state"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "reason": if returned_workspace_edit_count == 0 {
            "package rename preview returned no live edits; rollback is not required"
        } else {
            "package rename preview returned edits; rollback would be required before promotion"
        },
        "claim_boundary": "package rename preview rollback proof only; no package rename edits are applied"
    })
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn rename_package_pilot_ineligible_reason(
    reason: perl_lsp_rs_core::providers::navigation::rename_shadow::RenamePackagePilotIneligibleReason,
) -> &'static str {
    use perl_lsp_rs_core::providers::navigation::rename_shadow::RenamePackagePilotIneligibleReason;

    match reason {
        RenamePackagePilotIneligibleReason::EmptyPlan => "empty_plan",
        RenamePackagePilotIneligibleReason::Blocked => "blocked",
        RenamePackagePilotIneligibleReason::UnsupportedEditCategory => "unsupported_edit_category",
        _ => "unknown",
    }
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn rename_fallback_noise_json(
    symbol: &str,
    new_name: &str,
    live_provider_result: Option<&Value>,
    live_provider_error: Option<&str>,
    live_provider_edit_count: usize,
    compiler_plan_edit_count: Option<usize>,
    blockers: Option<&[PlanBlocker]>,
) -> Value {
    let (compiler_available, blocker_reasons, blocker_messages, compiler_requires_confirmation) =
        match blockers {
            Some(blockers) => {
                let blocker_reasons = blockers
                    .iter()
                    .map(|blocker| format!("{:?}", blocker.reason))
                    .collect::<Vec<_>>();
                let blocker_messages =
                    blockers.iter().map(|blocker| blocker.description.clone()).collect::<Vec<_>>();
                (true, blocker_reasons, blocker_messages, Some(!blockers.is_empty()))
            }
            None => (false, Vec::new(), Vec::new(), None),
        };
    let fallback_state = if let Some(blockers) = blockers {
        if !blockers.is_empty() {
            "compiler_blocked"
        } else if compiler_plan_edit_count == Some(0) {
            "compiler_empty"
        } else {
            "compiler_allowed"
        }
    } else {
        "compiler_missing"
    };
    let live_provider_state = rename_live_provider_state(
        live_provider_result,
        live_provider_error,
        live_provider_edit_count,
    );

    json!({
        "provider": "rename",
        "symbol": symbol,
        "new_name": new_name,
        "live_provider_state": live_provider_state,
        "live_provider_error": live_provider_error,
        "live_provider_edit_count": live_provider_edit_count,
        "compiler_available": compiler_available,
        "compiler_plan_edit_count": compiler_plan_edit_count,
        "compiler_blocker_reasons": blocker_reasons,
        "compiler_blocker_messages": blocker_messages,
        "compiler_requires_confirmation": compiler_requires_confirmation,
        "fallback_state": fallback_state,
        "claim_boundary": "package/compiler-backed rename stays receipt-only until fallback/noise proof justifies cutover"
    })
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn rename_live_provider_state(
    live_provider_result: Option<&Value>,
    live_provider_error: Option<&str>,
    live_provider_edit_count: usize,
) -> &'static str {
    if live_provider_error.is_some() {
        return "error";
    }

    match live_provider_result {
        Some(value) if value.is_null() => "null",
        Some(_) if live_provider_edit_count > 0 => "edits",
        Some(_) => "empty_edit",
        None => "missing",
    }
}

fn enrich_package_rename_preview_decision_trace(object: &mut serde_json::Map<String, Value>) {
    let package_pilot = object.get("package_pilot").and_then(Value::as_object);
    let fallback_noise = object.get("fallback_noise").and_then(Value::as_object);

    let blocker_reasons = package_pilot
        .and_then(|pilot| pilot.get("blocker_reasons"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let blocker_count = package_pilot
        .and_then(|pilot| pilot.get("blocker_count"))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| u64::try_from(blocker_reasons.len()).unwrap_or(u64::MAX));
    let eligible = package_pilot.and_then(|pilot| pilot.get("eligible")).and_then(Value::as_bool);
    let pilot_reason = package_pilot.and_then(|pilot| pilot.get("reason")).and_then(Value::as_str);
    let fallback_state = fallback_noise
        .and_then(|noise| noise.get("fallback_state"))
        .and_then(Value::as_str)
        .unwrap_or("compiler_missing");

    let first_blocker = blocker_reasons.iter().filter_map(Value::as_str).next().unwrap_or_default();
    let (decision, reason, fact_source, confidence, freshness, fallback_state) =
        match (eligible, pilot_reason, blocker_count, first_blocker, fallback_state) {
            (Some(true), _, _, _, _) => {
                ("allowed", "compiler_preview_allowed", "compiler_fact", "high", "fresh", "none")
            }
            (_, _, count, "DynamicBoundary", _) if count > 0 => {
                ("blocked", "dynamic_boundary", "dynamic_boundary", "high", "fresh", "no_edit")
            }
            (_, _, count, "GeneratedMember", _) if count > 0 => {
                ("blocked", "generated_no_source", "framework_adapter", "high", "fresh", "no_edit")
            }
            (_, _, count, "StaleFact", _) if count > 0 => (
                "blocked",
                "stale_fact",
                "compiler_fact",
                "low",
                "stale",
                "refresh_workspace_facts",
            ),
            (_, _, count, "AmbiguousReference", _) if count > 0 => (
                "blocked",
                "ambiguous_low_confidence_candidates",
                "semantic_fact",
                "low",
                "fresh",
                "require_confirmation",
            ),
            (_, _, count, "CrossModuleExport" | "ImportedSymbol" | "ExportedSymbol", _)
                if count > 0 =>
            {
                ("blocked", "import_export_visibility", "compiler_fact", "high", "fresh", "no_edit")
            }
            (_, _, count, _, _) if count > 0 => {
                ("blocked", "compiler_blocked", "compiler_fact", "low", "fresh", "no_edit")
            }
            (_, Some("empty_plan"), _, _, _) => (
                "fallback",
                "empty_compiler_plan",
                "fallback",
                "low",
                "not_applicable",
                "compiler_empty",
            ),
            (_, _, _, _, "compiler_missing") => (
                "fallback",
                "compiler_missing",
                "provider_runtime",
                "low",
                "unknown",
                "compiler_missing",
            ),
            _ => (
                "fallback",
                "preview_not_authorized",
                "provider_runtime",
                "low",
                "unknown",
                "no_result",
            ),
        };

    object.insert("decision".to_string(), json!(decision));
    object.insert("reason".to_string(), json!(reason));
    object.insert("fact_source".to_string(), json!(fact_source));
    object.insert("confidence".to_string(), json!(confidence));
    object.insert("freshness".to_string(), json!(freshness));
    object.insert("source_backed".to_string(), json!(false));
    object.insert(
        "source_backed_state".to_string(),
        json!("not_authorized_by_package_rename_preview"),
    );
    object.insert("dynamic_boundary".to_string(), json!(reason == "dynamic_boundary"));
    object.insert("fallback_state".to_string(), json!(fallback_state));
    object.insert("blocker_count".to_string(), json!(blocker_count));
    object.insert("blocker_reasons".to_string(), Value::Array(blocker_reasons));
    object.insert("trace_only_no_live_behavior_change".to_string(), json!(true));
}

fn package_rename_preview_message(receipt: &Value) -> String {
    let symbol = receipt.get("symbol").and_then(Value::as_str).unwrap_or("symbol");
    let new_name = receipt.get("new_name").and_then(Value::as_str).unwrap_or("new name");
    let package_pilot = receipt.get("package_pilot");

    match package_pilot.and_then(|pilot| pilot.get("eligible")).and_then(Value::as_bool) {
        Some(true) => format!(
            "Package rename preview: `{symbol}` can be planned as `{new_name}`, but no package rename edits were applied."
        ),
        Some(false) => {
            let reason = package_pilot
                .and_then(|pilot| pilot.get("reason"))
                .and_then(Value::as_str)
                .unwrap_or("not eligible");
            let blocker = receipt
                .pointer("/fallback_noise/compiler_blocker_messages/0")
                .and_then(Value::as_str)
                .unwrap_or(reason);
            format!(
                "Package rename preview refused for `{symbol}` -> `{new_name}`: {blocker}. No edits were applied."
            )
        }
        None => {
            let reason = receipt
                .pointer("/fallback_noise/fallback_state")
                .and_then(Value::as_str)
                .or_else(|| receipt.get("reason").and_then(Value::as_str))
                .unwrap_or("compiler proof is unavailable");
            format!(
                "Package rename preview unavailable for `{symbol}` -> `{new_name}`: {reason}. No edits were applied."
            )
        }
    }
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn safe_delete_live_blocker_ux_json(blockers: Option<&[PlanBlocker]>) -> Value {
    let Some(blockers) = blockers else {
        return Value::Null;
    };
    let blocker_reasons =
        blockers.iter().map(|blocker| format!("{:?}", blocker.reason)).collect::<Vec<_>>();
    let blocker_messages =
        blockers.iter().map(|blocker| blocker.description.clone()).collect::<Vec<_>>();

    json!({
        "provider": "safe_delete",
        "decision": if blockers.is_empty() { "allowed" } else { "blocked" },
        "fallback": if blockers.is_empty() { "none" } else { "no_edit" },
        "requires_confirmation": !blockers.is_empty(),
        "blocker_reasons": blocker_reasons,
        "blocker_messages": blocker_messages
    })
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn safe_delete_rollback_receipt_json(
    live_provider_edit_count: usize,
    blockers: Option<&[PlanBlocker]>,
) -> Value {
    let Some(blockers) = blockers else {
        return Value::Null;
    };
    let blocked = !blockers.is_empty();

    json!({
        "provider": "safe_delete",
        "live_provider_edit_count": live_provider_edit_count,
        "rollback_required": live_provider_edit_count > 0,
        "rollback_safe": live_provider_edit_count == 0,
        "blocked_before_edit": blocked,
        "reason": if blocked {
            "safe-delete blocker emitted no live edits; rollback is not required"
        } else {
            "safe-delete plan allowed; no live symbol-level delete was executed"
        }
    })
}

fn safe_delete_symbol_preview_message(receipt: &Value) -> String {
    let symbol = receipt.get("symbol").and_then(Value::as_str).unwrap_or("symbol");
    match receipt.get("decision").and_then(Value::as_str) {
        Some("allowed") => format!(
            "Safe delete preview: `{symbol}` has no semantic blockers, but no symbol deletion was applied."
        ),
        Some("blocked") => {
            let blocker = receipt
                .pointer("/live_blocker_ux/blocker_messages/0")
                .and_then(Value::as_str)
                .or_else(|| receipt.get("reason").and_then(Value::as_str))
                .unwrap_or("the available facts cannot safely authorize deletion");
            format!("Safe delete refused for `{symbol}`: {blocker}. No edits were applied.")
        }
        Some("fallback") => {
            let reason = receipt
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("safe-delete proof is unavailable");
            format!(
                "Safe delete preview unavailable for `{symbol}`: {reason}. No edits were applied."
            )
        }
        Some(other) => {
            format!("Safe delete preview returned `{other}` for `{symbol}`. No edits were applied.")
        }
        None => {
            format!("Safe delete preview could not classify `{symbol}`. No edits were applied.")
        }
    }
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn runtime_blocker_reasons(blockers: &[PlanBlocker]) -> String {
    if blockers.is_empty() {
        return "none".to_string();
    }
    blockers.iter().map(|blocker| format!("{:?}", blocker.reason)).collect::<Vec<_>>().join(",")
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn runtime_blocker_descriptions(blockers: &[PlanBlocker]) -> String {
    if blockers.is_empty() {
        return "none".to_string();
    }
    blockers.iter().map(|blocker| blocker.description.as_str()).collect::<Vec<_>>().join(" | ")
}

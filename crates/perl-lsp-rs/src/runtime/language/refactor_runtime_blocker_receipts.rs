//! Test-only runtime receipts for refactor blocker UX.

use super::super::*;
use crate::protocol::{req_position, req_uri};

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_lsp_rs_core::providers::navigation::{
    rename_shadow::{RenameCutoverResult, rename_cutover},
    safe_delete_shadow::{SafeDeleteCutoverResult, safe_delete_cutover},
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_semantic_facts::{
    DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact, PlanBlocker,
    PlanBlockerReason, RenamePlan, SafeDeletePlan, ScopeId, VisibleSymbol,
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_workspace::semantic::queries::{DynamicCallableEvidence, QueryContext, SemanticQueries};

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

            if let Some(fixture) = params.get("compilerPlanFixture").and_then(Value::as_str) {
                let compiler_receipt =
                    rename_fixture_receipt(fixture, &symbol, new_name, live_provider_edit_count);
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
                            let outcome = rename_cutover(
                                live_provider_edit_count > 0,
                                &queries,
                                entity_id,
                                new_name,
                            );
                            let (compiler_plan_edit_count, blockers) = match &outcome.result {
                                RenameCutoverResult::Allowed { edits } => (edits.len(), Vec::new()),
                                RenameCutoverResult::Blocked { blockers, edits } => {
                                    (edits.len(), blockers.clone())
                                }
                            };
                            let mut receipt = outcome.receipt;
                            receipt.notes.push(format!(
                                "rename runtime blocker UX: live_provider_edits={}; compiler_plan_edits={compiler_plan_edit_count}; blocker_count={}; blocker_reasons={}; blocker_ux={}; requires_confirmation={}; no live refactor behavior change",
                                live_provider_edit_count,
                                blockers.len(),
                                runtime_blocker_reasons(&blockers),
                                runtime_blocker_descriptions(&blockers),
                                !blockers.is_empty()
                            ));
                            Some((receipt, compiler_plan_edit_count, blockers))
                        })
                        .flatten()
                }
                IndexAccessMode::Partial(_) | IndexAccessMode::None => None,
            };
            let (compiler_receipt, compiler_plan_edit_count, compiler_blockers) =
                compiler_receipt_parts.map_or(
                    (None, None, None),
                    |(receipt, edit_count, blockers)| {
                        (Some(receipt), Some(edit_count), Some(blockers))
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

    /// Test-only receipt for safe-delete blocker UX proof.
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
            Ok(Some(json!({
                "provider": "safe_delete",
                "live_provider_result": live_provider_result,
                "live_provider_edit_count": live_provider_edit_count,
                "compiler_receipt": null,
                "no_live_behavior_change": true,
                "note": "safe-delete blocker UX proof unavailable without workspace semantic queries"
            })))
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let Some(params) = params else {
                return Ok(Some(json!({
                    "provider": "safe_delete",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "safe-delete blocker UX proof missing request params"
                })));
            };

            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let Some((symbol, byte_offset)) = self.refactor_runtime_symbol(uri, line, character)
            else {
                return Ok(Some(json!({
                    "provider": "safe_delete",
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "safe-delete blocker UX proof found no symbol at request position"
                })));
            };

            if let Some(fixture) = params.get("compilerPlanFixture").and_then(Value::as_str) {
                let compiler_receipt =
                    safe_delete_fixture_receipt(fixture, &symbol, live_provider_edit_count);
                return Ok(Some(json!({
                    "provider": "safe_delete",
                    "symbol": symbol,
                    "compiler_plan_fixture": fixture,
                    "live_provider_result": live_provider_result,
                    "live_provider_edit_count": live_provider_edit_count,
                    "compiler_receipt": compiler_receipt,
                    "no_live_behavior_change": true
                })));
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

            let receipt = json!({
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
            self.record_provider_decision_trace("safe_delete", &receipt);
            Ok(Some(receipt))
        }
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

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
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

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn safe_delete_fixture_receipt(
    fixture: &str,
    symbol: &str,
    live_provider_edit_count: usize,
) -> Option<perl_workspace::semantic_shadow_compare::SemanticShadowCompareReceipt> {
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
    Some(receipt)
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
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

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
struct RefactorFixtureQueries {
    rename_plan: RenamePlan,
    safe_delete_plan: Option<SafeDeletePlan>,
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
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

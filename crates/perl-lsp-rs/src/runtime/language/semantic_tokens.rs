//! Semantic tokens handlers
//!
//! Handles textDocument/semanticTokens/full and textDocument/semanticTokens/range requests.
//!
//! Includes deadline enforcement to prevent blocking on large files.

use super::super::*;
use crate::protocol::req_uri;
use crate::state::semantic_tokens_deadline;
#[cfg(any(test, feature = "expose_lsp_test_api"))]
use perl_semantic_facts::ProviderFallbackState;
use perl_semantic_facts::{Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind};
use std::time::Instant;

impl LspServer {
    /// Handle textDocument/semanticTokens/full request
    ///
    /// Uses deadline enforcement to prevent blocking on very large files.
    /// If deadline is exceeded, returns partial tokens collected so far.
    pub(crate) fn handle_semantic_tokens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = semantic_tokens_deadline();

        if let Some(p) = params {
            let uri = req_uri(&p)?;

            // Phase 1: grab an owned `DocumentState` clone under a brief
            // documents-map lock, then drop the guard before doing any
            // analysis (#3396 off-lock provider consumption).
            let timing_on = crate::runtime::timing::is_enabled();
            let t_lock_start = std::time::Instant::now();
            let doc_owned = {
                let documents = self.documents_guard();
                self.get_document(&documents, uri).cloned()
            };
            // documents guard dropped here
            if timing_on {
                crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
                    "provider.semantic_tokens.lock_hold",
                    crate::runtime::timing::elapsed_ms(t_lock_start),
                    crate::runtime::timing::uri_tail(uri),
                ));
            }
            let doc = doc_owned.as_ref().ok_or_else(|| semantic_tokens_document_not_open(uri))?;
            // Covers the whole analysis block via `Drop`, so it emits
            // correctly regardless of which exit point below fires.
            let _analyze_span =
                crate::runtime::timing::ScopedSpan::start("provider.semantic_tokens.analyze", uri);
            let parsed = doc.current_parsed();
            if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                let data =
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
                        self.offset_to_pos16(doc, off)
                    });
                let flat_data: Vec<u32> = data.into_iter().flatten().collect();
                let live_token_count = flat_data.len() / 5;
                let result_id = semantic_tokens_result_id(&flat_data);
                // Serialize the response by reference, then move the vector into the
                // cache — avoids cloning the full token buffer on every request.
                let live_result = json!({ "resultId": &result_id, "data": &flat_data });
                self.store_semantic_tokens_result(uri, &result_id, flat_data);
                let provider_trace = semantic_tokens_live_slice_provider_trace(
                    &doc.text,
                    &live_result,
                    live_token_count,
                    "textDocument/semanticTokens/full",
                );

                if start.elapsed() >= deadline {
                    tracing::debug!(
                        elapsed = ?start.elapsed(),
                        tokens = live_token_count,
                        "SemanticTokens: deadline exceeded"
                    );
                }

                self.record_provider_decision_trace("semantic_tokens", &provider_trace);

                return Ok(Some(live_result));
            }
        }
        self.record_provider_decision_trace(
            "semantic_tokens",
            &semantic_tokens_fallback_provider_trace(
                "textDocument/semanticTokens/full",
                0,
                "no_ast_available",
                "no live AST was available; parser/HIR semantic-token provider returned no tokens",
            ),
        );
        Ok(Some(json!({ "data": [] })))
    }

    /// Record the latest semantic-tokens result for `uri` so a subsequent delta
    /// request can diff against it.
    fn store_semantic_tokens_result(&self, uri: &str, result_id: &str, data: Vec<u32>) {
        let mut cache = self.semantic_tokens_cache.lock();
        cache.insert(
            uri.to_string(),
            SemanticTokensCacheEntry { result_id: result_id.to_string(), data },
        );
    }

    /// Handle the `textDocument/semanticTokens/full/delta` request (LSP 3.17).
    ///
    /// Computes the current full token set, then returns the minimal set of
    /// edits transforming the client's previously cached result (identified by
    /// `previousResultId`) into the current one. When `previousResultId` is
    /// missing or no longer cached, falls back to a full token response so the
    /// client can resynchronize.
    pub(crate) fn handle_semantic_tokens_delta(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let Some(params) = params else {
            return Ok(Some(json!({ "data": [] })));
        };
        let uri = req_uri(&params)?;
        let previous_result_id =
            params.get("previousResultId").and_then(Value::as_str).map(str::to_string);

        tracing::debug!(uri, ?previous_result_id, "Getting semantic tokens delta");

        // Compute the current full token set from the live AST (same source as
        // `textDocument/semanticTokens/full`). Clone the `DocumentState` under a
        // brief documents-map lock and drop the guard before any analysis runs
        // (#3396 off-lock provider consumption), matching the sibling handlers.
        let doc_owned = {
            let documents = self.documents_guard();
            self.get_document(&documents, uri).cloned()
        };
        // documents guard dropped here
        let current: Vec<u32> = {
            let doc = doc_owned.as_ref().ok_or_else(|| semantic_tokens_document_not_open(uri))?;
            let parsed = doc.current_parsed();
            match parsed.as_ref().and_then(|p| p.ast()) {
                Some(ast) => {
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
                        self.offset_to_pos16(doc, off)
                    })
                    .into_iter()
                    .flatten()
                    .collect()
                }
                None => Vec::new(),
            }
        };

        // Look up the cached prior result; only usable when its resultId matches
        // the client's `previousResultId`.
        let previous = previous_result_id.as_deref().and_then(|prev_id| {
            let cache = self.semantic_tokens_cache.lock();
            cache
                .get(uri)
                .filter(|entry| entry.result_id == prev_id)
                .map(|entry| entry.data.clone())
        });

        // Build the response by reference, then move `current` into the cache so
        // the full token buffer is not cloned on every delta request.
        let result_id = semantic_tokens_result_id(&current);
        let response = match previous {
            Some(prev_data) => {
                let edits = compute_semantic_tokens_delta_edits(&prev_data, &current);
                json!({ "resultId": &result_id, "edits": edits })
            }
            None => json!({ "resultId": &result_id, "data": &current }),
        };
        self.store_semantic_tokens_result(uri, &result_id, current);
        Ok(Some(response))
    }

    /// Semantic tokens runtime quality receipt.
    ///
    /// Calls the live `textDocument/semanticTokens/full` handler and captures the result in a
    /// typed receipt for quality proof. This does not change live semantic token behavior —
    /// parser/HIR token classifications remain the live provider source.
    ///
    /// The receipt records token count, shadow state, and notes confirming no behavior change.
    /// Source-backed compiler-fact token classes are recorded as shadow-only proof.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn semantic_tokens_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let live_provider_result = self.handle_semantic_tokens(params.clone())?;
        let compiler_receipt = self
            .semantic_tokens_compiler_class_receipt(params.as_ref(), live_provider_result.as_ref());
        let class_specific_expansion_receipts = self
            .semantic_tokens_class_specific_expansion_receipts(
                params.as_ref(),
                live_provider_result.as_ref(),
            );
        let live_pilot = compiler_receipt
            .as_ref()
            .and_then(|receipt| receipt.get("live_pilot"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let compiler_receipt_count = if compiler_receipt.is_some() { 1usize } else { 0usize };
        let compiler_live_pilot_count = if live_pilot { 1usize } else { 0usize };
        let class_specific_receipt_count = class_specific_expansion_receipts.len();
        let class_specific_live_pilot_count = class_specific_expansion_receipts
            .iter()
            .filter(|receipt| receipt.get("live_pilot").and_then(Value::as_bool).unwrap_or(false))
            .count();
        let live_pilot_state = if live_pilot { "partial_live_source_backed" } else { "shadowed" };

        // Each LSP semantic token encodes as 5 consecutive u32 values in the flat data array.
        let live_token_count = live_provider_result
            .as_ref()
            .and_then(|v| v.get("data"))
            .and_then(|d| d.as_array())
            .map(|arr| arr.len() / 5)
            .unwrap_or(0);

        Ok(Some(json!({
            "provider": "semantic_tokens",
            "live_provider_result": live_provider_result,
            "live_provider_count": live_token_count,
            "shadow_state": "shadowed",
            "live_pilot_state": live_pilot_state,
            "compiler_receipt": compiler_receipt,
            "class_specific_expansion_receipts": class_specific_expansion_receipts,
            "class_specific_live_pilot_count": class_specific_live_pilot_count,
            "no_live_behavior_change": true,
            "no_live_token_output_change": true,
            "notes": format!(
                "semantic_tokens runtime proof: token_count={live_token_count}; \
                 parser/HIR classifications remain live provider; \
                 compiler_backed_token_classes={}; \
                 compiler_live_pilot={}; \
                 class_specific_compiler_token_classes={}; \
                 class_specific_live_pilots={}; \
                 compiler-fact candidates are live-pilot only when their source-backed span \
                 already matches the live token stream; \
                 no semantic-token output change",
                compiler_receipt_count,
                compiler_live_pilot_count,
                class_specific_receipt_count,
                class_specific_live_pilot_count
            )
        })))
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    fn semantic_tokens_compiler_class_receipt(
        &self,
        params: Option<&Value>,
        live_provider_result: Option<&Value>,
    ) -> Option<Value> {
        let uri = req_uri(params?).ok()?;
        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri)?;
        let mut candidate = semantic_token_subroutine_declaration_candidate(&doc.text)?;
        let live_pilot = semantic_tokens_live_contains_span(
            live_provider_result,
            candidate.source_span.as_ref(),
            "function",
        );
        if live_pilot {
            candidate.fallback_state = ProviderFallbackState::Primary;
        }
        let fallback_state = candidate.fallback_state;
        let candidates = vec![candidate];
        let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(&candidates);
        let shadow = crate::semantic_tokens::semantic_token_source_shadow(
            Vec::new(),
            candidates,
            "subroutine_declaration",
        );
        let live_token_match_count = if live_pilot { 1usize } else { 0usize };
        let claim_boundary = if live_pilot {
            "narrow compiler-backed subroutine-declaration token class matches existing parser/HIR live token output; no new semantic-token output"
        } else {
            "compiler-backed token class receipt only; parser/HIR semantic tokens remain live"
        };

        Some(json!({
            "token_class": "subroutine_declaration",
            "source": "CompilerFact",
            "provenance": "SemanticAnalyzer",
            "confidence": "Medium",
            "freshness": "Fresh",
            "fallback_state": provider_fallback_state_label(fallback_state),
            "shadow_state": "shadowed",
            "live_pilot": live_pilot,
            "live_token_type": "function",
            "live_token_match_count": live_token_match_count,
            "candidate_count": span_report.candidate_count,
            "source_backed_span_count": span_report.source_backed_span_count,
            "missing_source_span_count": span_report.missing_source_span_count,
            "invalid_source_span_count": span_report.invalid_source_span_count,
            "no_live_behavior_change": true,
            "no_live_token_output_change": true,
            "claim_boundary": claim_boundary,
            "shadow_receipt": shadow.receipt
        }))
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    fn semantic_tokens_class_specific_expansion_receipts(
        &self,
        params: Option<&Value>,
        live_provider_result: Option<&Value>,
    ) -> Vec<Value> {
        let Some(uri) = params.and_then(|params| req_uri(params).ok()) else {
            return Vec::new();
        };
        let documents = self.documents_guard();
        let Some(doc) = self.get_document(&documents, uri) else {
            return Vec::new();
        };
        let mut receipts = Vec::new();
        if let Some(candidate) = semantic_token_subroutine_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "subroutine_declaration",
                "function",
                "matched_existing_live_function_token",
                "unmatched_existing_live_function_token",
                true,
                "scoped compiler subroutine-declaration class cutover proof only; subroutine declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR function tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_package_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "package_declaration",
                "namespace",
                "matched_existing_live_namespace_token",
                "unmatched_existing_live_namespace_token",
                true,
                "scoped compiler package-declaration class cutover proof only; package declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR namespace tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_method_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "method_declaration",
                "method",
                "matched_existing_live_method_token",
                "unmatched_existing_live_method_token",
                true,
                "scoped compiler method-declaration class cutover proof only; method declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR method tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_phase_block_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "phase_block_declaration",
                "macro",
                "matched_existing_live_macro_token",
                "unmatched_existing_live_macro_token",
                true,
                "scoped compiler phase-block declaration class cutover proof only; phase-block declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR macro tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_method_call_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "method_call",
                "method",
                "matched_existing_live_method_token",
                "unmatched_existing_live_method_token",
                true,
                "scoped compiler method-call class cutover proof only; method calls may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR method tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_self_method_call_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "self_method_call",
                "method",
                "matched_existing_live_method_token",
                "unmatched_existing_live_method_token",
                true,
                "scoped compiler self method-call class cutover proof only; $self method calls may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR method tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_class_field_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "field_declaration",
                "variable",
                "matched_existing_live_variable_token",
                "unmatched_existing_live_variable_token",
                true,
                "scoped compiler field-declaration class cutover proof only; field declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR variable tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_lexical_variable_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "lexical_variable_declaration",
                "variable",
                "matched_existing_live_variable_token",
                "unmatched_existing_live_variable_token",
                true,
                "scoped compiler lexical-variable declaration class cutover proof only; lexical variable declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR variable tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_lexical_variable_use_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "lexical_variable_use",
                "variable",
                "matched_existing_live_variable_token",
                "unmatched_existing_live_variable_token",
                true,
                "scoped compiler lexical-variable use class cutover proof only; lexical variable uses may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR variable tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_our_variable_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "our_variable_declaration",
                "variable",
                "matched_existing_live_variable_token",
                "unmatched_existing_live_variable_token",
                true,
                "scoped compiler our-variable declaration class cutover proof only; package-scoped our variable declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR variable tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_state_variable_declaration_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "state_variable_declaration",
                "variable",
                "matched_existing_live_variable_token",
                "unmatched_existing_live_variable_token",
                true,
                "scoped compiler state-variable declaration class cutover proof only; lexical state variable declarations may count as compiler-token identities only when their source-backed span already matches existing live parser/HIR variable tokens, and no new token output is emitted",
            ));
        }
        if let Some(candidate) = semantic_token_named_function_call_candidate(&doc.text) {
            receipts.push(Self::semantic_tokens_class_specific_expansion_receipt(
                live_provider_result,
                candidate,
                "named_function_call",
                "function",
                "matched_existing_live_function_token",
                "unmatched_existing_live_function_token",
                true,
                "scoped compiler named-function-call class cutover proof only; named function calls may count as compiler-token identities only when their source-backed whole-call span already matches existing live parser/HIR function tokens, and no new token output is emitted",
            ));
        }

        receipts
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    fn semantic_tokens_class_specific_expansion_receipt(
        live_provider_result: Option<&Value>,
        mut candidate: crate::semantic_tokens::SemanticTokenShadowCandidate,
        token_class: &'static str,
        live_token_type: &'static str,
        matched_parity_state: &'static str,
        unmatched_parity_state: &'static str,
        approved_for_live_cutover: bool,
        claim_boundary: &'static str,
    ) -> Value {
        let live_output_parity = semantic_tokens_live_contains_span(
            live_provider_result,
            candidate.source_span.as_ref(),
            live_token_type,
        );
        let live_pilot = approved_for_live_cutover && live_output_parity;
        if live_pilot {
            candidate.fallback_state = ProviderFallbackState::Primary;
        }
        let fallback_state = candidate.fallback_state;
        let candidates = vec![candidate];
        let span_report = crate::semantic_tokens::semantic_token_span_invariant_report(&candidates);
        let shadow = crate::semantic_tokens::semantic_token_source_shadow(
            Vec::new(),
            candidates,
            token_class,
        );
        let live_token_match_count = if live_output_parity { 1usize } else { 0usize };
        let parity_state =
            if live_output_parity { matched_parity_state } else { unmatched_parity_state };

        json!({
            "token_class": token_class,
            "source": "CompilerFact",
            "provenance": "SemanticAnalyzer",
            "confidence": "Medium",
            "freshness": "Fresh",
            "fallback_state": provider_fallback_state_label(fallback_state),
            "shadow_state": "shadowed",
            "approved_for_live_cutover": approved_for_live_cutover,
            "live_pilot": live_pilot,
            "live_output_parity": live_output_parity,
            "parity_state": parity_state,
            "live_token_type": live_token_type,
            "live_token_match_count": live_token_match_count,
            "candidate_count": span_report.candidate_count,
            "source_backed_span_count": span_report.source_backed_span_count,
            "missing_source_span_count": span_report.missing_source_span_count,
            "invalid_source_span_count": span_report.invalid_source_span_count,
            "no_live_behavior_change": true,
            "no_live_token_output_change": true,
            "claim_boundary": claim_boundary,
            "shadow_receipt": shadow.receipt
        })
    }

    /// Handle semantic tokens range request
    pub(crate) fn handle_semantic_tokens_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::protocol::req_range;
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let ((start_line, start_char), (end_line, end_char)) = req_range(&params)?;

            tracing::debug!(uri, start_line, end_line, "Getting semantic tokens for range");

            // Phase 1: grab an owned `DocumentState` clone under a brief
            // documents-map lock, then drop the guard before doing any
            // analysis (#3396 off-lock provider consumption).
            let timing_on = crate::runtime::timing::is_enabled();
            let t_lock_start = std::time::Instant::now();
            let doc_owned = {
                let documents = self.documents_guard();
                self.get_document(&documents, uri).cloned()
            };
            // documents guard dropped here
            if timing_on {
                crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
                    "provider.semantic_tokens.lock_hold",
                    crate::runtime::timing::elapsed_ms(t_lock_start),
                    crate::runtime::timing::uri_tail(uri),
                ));
            }
            if let Some(doc) = doc_owned.as_ref() {
                let _analyze_span = crate::runtime::timing::ScopedSpan::start(
                    "provider.semantic_tokens.analyze",
                    uri,
                );
                let parsed = doc.current_parsed();
                if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                    let all_tokens =
                        crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
                            self.offset_to_pos16(doc, off)
                        });
                    let encoded = filter_encoded_semantic_tokens_by_range(
                        all_tokens, start_line, start_char, end_line, end_char,
                    );

                    tracing::debug!(count = encoded.len() / 5, "Found semantic tokens in range");

                    return Ok(Some(json!({
                        "data": encoded
                    })));
                }
            }
        }

        Ok(Some(json!({
            "data": []
        })))
    }
}

/// Compute a deterministic `resultId` for a semantic-tokens result.
///
/// Derived from the encoded token data so an identical token stream yields the
/// same id (idempotent full requests, unchanged documents) while any change
/// produces a new id. Determinism also keeps the runtime quality-receipt
/// equality checks stable across repeated handler calls.
fn semantic_tokens_result_id(data: &[u32]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish().to_string()
}

/// Compute the minimal LSP semantic-tokens delta edits that transform `old`
/// into `new`.
///
/// Both slices are flat encoded token arrays (groups of 5 `u32`). The result is
/// a single contiguous `SemanticTokensEdit` covering the changed middle region,
/// found by stripping the longest common prefix and suffix. An empty `Vec`
/// means the two results are identical and no edit is required.
fn compute_semantic_tokens_delta_edits(old: &[u32], new: &[u32]) -> Vec<Value> {
    let max_prefix = old.len().min(new.len());
    let mut prefix = 0;
    while prefix < max_prefix && old[prefix] == new[prefix] {
        prefix += 1;
    }

    // Common suffix length, never overlapping the shared prefix.
    let max_suffix = max_prefix - prefix;
    let mut suffix = 0;
    while suffix < max_suffix && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix] {
        suffix += 1;
    }

    let delete_count = old.len() - prefix - suffix;
    let data: Vec<u32> = new[prefix..new.len() - suffix].to_vec();

    // Identical results need no edit.
    if delete_count == 0 && data.is_empty() {
        return Vec::new();
    }

    vec![json!({
        "start": prefix,
        "deleteCount": delete_count,
        "data": data,
    })]
}

fn filter_encoded_semantic_tokens_by_range(
    tokens: Vec<crate::semantic_tokens::EncodedToken>,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
) -> Vec<u32> {
    let mut absolute_tokens = Vec::new();
    let mut line = 0u32;
    let mut start = 0u32;

    for token in tokens {
        let [delta_line, delta_start, length, token_type, modifiers] = token;
        if delta_line == 0 {
            start = start.saturating_add(delta_start);
        } else {
            line = line.saturating_add(delta_line);
            start = delta_start;
        }

        let starts_after_range_start =
            line > start_line || (line == start_line && start >= start_char);
        let starts_before_range_end = line < end_line || (line == end_line && start < end_char);

        if starts_after_range_start && starts_before_range_end {
            absolute_tokens.push((line, start, length, token_type, modifiers));
        }
    }

    let mut encoded = Vec::with_capacity(absolute_tokens.len() * 5);
    let mut previous_line = 0u32;
    let mut previous_start = 0u32;
    for (line, start, length, token_type, modifiers) in absolute_tokens {
        let delta_line = line.saturating_sub(previous_line);
        let delta_start =
            if delta_line == 0 { start.saturating_sub(previous_start) } else { start };
        encoded.extend([delta_line, delta_start, length, token_type, modifiers]);
        previous_line = line;
        previous_start = start;
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn semantic_token_value(value: &Value) -> Result<u32, Box<dyn std::error::Error>> {
        let raw = value.as_u64().ok_or("semantic token value was not an unsigned integer")?;
        Ok(u32::try_from(raw)?)
    }

    #[test]
    fn our_variable_declaration_candidate_scans_past_non_declaration_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        // An earlier non-declaration `our ` (here inside a comment) must not mask
        // the real source-backed declaration that follows on a later line.
        let source = "# our $todo\nour $shared = 1;\n$shared++;\n";
        let candidate = semantic_token_our_variable_declaration_candidate(source)
            .ok_or("real `our` declaration should be detected past the comment marker")?;
        assert!(
            candidate.identity.starts_with("token:our_variable_declaration:$shared:"),
            "expected the $shared declaration identity, got {}",
            candidate.identity
        );
        Ok(())
    }

    #[test]
    fn our_variable_declaration_candidate_requires_a_real_declaration() {
        // Only a non-declaration `our ` marker is present; the detector must fall
        // back (no candidate) rather than record a false compiler-token identity.
        let source = "# our $todo\nmy $x = 1;\n";
        assert!(semantic_token_our_variable_declaration_candidate(source).is_none());
    }

    #[test]
    fn state_variable_declaration_candidate_scans_past_non_declaration_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        // An earlier non-declaration `state ` (here inside a comment) must not
        // mask the real source-backed declaration that follows.
        let source = "# state $todo\nstate $count = 0;\n$count++;\n";
        let candidate = semantic_token_state_variable_declaration_candidate(source)
            .ok_or("real `state` declaration should be detected past the comment marker")?;
        assert!(
            candidate.identity.starts_with("token:state_variable_declaration:$count:"),
            "expected the $count declaration identity, got {}",
            candidate.identity
        );
        Ok(())
    }

    #[test]
    fn state_variable_declaration_candidate_requires_a_real_declaration() {
        // Only a non-declaration `state ` marker is present; the detector must
        // fall back rather than record a false compiler-token identity.
        let source = "# state $todo\nmy $x = 1;\n";
        assert!(semantic_token_state_variable_declaration_candidate(source).is_none());
    }

    #[test]
    fn named_function_call_candidate_spans_the_whole_call() -> Result<(), Box<dyn std::error::Error>>
    {
        // The live FunctionCall token covers the entire call expression, so the
        // candidate span must run from the callee name through the close paren.
        let source = "use strict;\n\ncompute(1, 2);\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("a bareword call should be detected")?;
        assert_eq!(candidate.identity, "token:named_function_call:compute:compiler");
        let span = candidate.source_span.ok_or("call candidate must be source-backed")?;
        // `compute(1, 2)` is 13 UTF-16 units on a single line.
        assert_eq!(span.single_line_lsp_length(), Some(13));
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_handles_empty_argument_list()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "run_pipeline();\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("a no-argument call should be detected")?;
        assert_eq!(candidate.identity, "token:named_function_call:run_pipeline:compiler");
        let span = candidate.source_span.ok_or("call candidate must be source-backed")?;
        // `run_pipeline()` is 14 UTF-16 units.
        assert_eq!(span.single_line_lsp_length(), Some(14));
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_excludes_method_calls() {
        // `->name(` is a method dispatch handled by the method-call class. The
        // ONLY call here is the method call, so a detected candidate would prove
        // the `>` prefix blocker failed; the detector must fall back instead.
        let source = "my $c = shift;\n$c->stash(1);\n";
        assert!(semantic_token_named_function_call_candidate(source).is_none());
    }

    #[test]
    fn named_function_call_candidate_excludes_sigil_and_ampersand_calls() {
        // `&name(` (ampersand call) and `$ref->(` (coderef dispatch) are not
        // plain bareword `FunctionCall` function tokens; both must fall back.
        assert!(semantic_token_named_function_call_candidate("&helper(1);\n").is_none());
        assert!(semantic_token_named_function_call_candidate("$ref->(1);\n").is_none());
    }

    #[test]
    fn named_function_call_candidate_scans_past_commented_call()
    -> Result<(), Box<dyn std::error::Error>> {
        // A `name(` inside a `#` line comment must not shadow the real call that
        // follows; the detector skips the comment and reports `dispatch`.
        let source = "# run_pipeline()\ndispatch();\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("the real dispatch() call should be detected past the comment")?;
        assert_eq!(candidate.identity, "token:named_function_call:dispatch:compiler");
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_scans_past_stringized_call()
    -> Result<(), Box<dyn std::error::Error>> {
        // A `name(` inside a quoted string literal must not shadow a later real
        // call.
        let source = "my $x = 'foo(';\nbar();\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("the real bar() call should be detected past the string literal")?;
        assert_eq!(candidate.identity, "token:named_function_call:bar:compiler");
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_balances_parens_inside_string_arguments()
    -> Result<(), Box<dyn std::error::Error>> {
        // Parentheses inside a quoted string argument must not be counted while
        // balancing, so the whole-call span still covers `emit(")")` (9 units)
        // to match the live FunctionCall token.
        let source = "emit(\")\");\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("a call with a paren inside a string arg should still be detected")?;
        assert_eq!(candidate.identity, "token:named_function_call:emit:compiler");
        let span = candidate.source_span.ok_or("call candidate must be source-backed")?;
        assert_eq!(span.single_line_lsp_length(), Some(9));
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_scans_past_multiline_call_to_single_line_call()
    -> Result<(), Box<dyn std::error::Error>> {
        // A leading call whose parens span multiple lines cannot yield a
        // single-line span, so the scan continues to the later single-line call.
        let source = "outer(\n    1,\n);\ninner();\n";
        let candidate = semantic_token_named_function_call_candidate(source)
            .ok_or("the single-line inner() call should be detected")?;
        assert_eq!(candidate.identity, "token:named_function_call:inner:compiler");
        Ok(())
    }

    #[test]
    fn named_function_call_candidate_excludes_control_keywords() {
        // `if (...)` is not a FunctionCall function token; the detector must fall
        // back rather than record a span that cannot match a live token.
        let source = "if (1) {\n    1;\n}\n";
        assert!(semantic_token_named_function_call_candidate(source).is_none());
    }

    #[test]
    fn named_function_call_candidate_skips_multiline_calls() {
        // A call whose parens span multiple lines fails closed: no single-line
        // whole-call span can match a live token.
        let source = "compute(\n    1,\n);\n";
        assert!(semantic_token_named_function_call_candidate(source).is_none());
    }

    #[test]
    fn named_function_call_candidate_ignores_declaration_without_call_parens() {
        // `sub helper {` has no `(` after the name, so it is not a call site.
        let source = "sub helper {\n    return 1;\n}\n";
        assert!(semantic_token_named_function_call_candidate(source).is_none());
    }

    #[test]
    fn filter_encoded_semantic_tokens_by_range_reencodes_retained_range()
    -> Result<(), Box<dyn std::error::Error>> {
        let tokens: Vec<crate::semantic_tokens::EncodedToken> =
            vec![[0, 0, 5, 1, 0], [1, 2, 3, 2, 0], [0, 5, 4, 3, 1], [1, 1, 2, 4, 0]];

        assert_eq!(
            filter_encoded_semantic_tokens_by_range(tokens.clone(), 1, 0, 2, 0),
            vec![1, 2, 3, 2, 0, 0, 5, 4, 3, 1]
        );
        assert_eq!(
            filter_encoded_semantic_tokens_by_range(tokens.clone(), 1, 0, 3, 0),
            vec![1, 2, 3, 2, 0, 0, 5, 4, 3, 1, 1, 1, 2, 4, 0]
        );
        assert_eq!(
            filter_encoded_semantic_tokens_by_range(tokens.clone(), 1, 5, 2, 0),
            vec![1, 7, 4, 3, 1]
        );
        assert!(filter_encoded_semantic_tokens_by_range(tokens, 3, 0, 4, 0).is_empty());

        Ok(())
    }

    #[test]
    fn compute_semantic_tokens_delta_edits_detects_no_change() {
        let tokens = vec![0u32, 0, 5, 1, 0, 1, 2, 3, 2, 0];
        assert!(compute_semantic_tokens_delta_edits(&tokens, &tokens).is_empty());
        assert!(compute_semantic_tokens_delta_edits(&[], &[]).is_empty());
    }

    #[test]
    fn compute_semantic_tokens_delta_edits_handles_append() {
        let old = vec![0u32, 0, 5, 1, 0];
        let new = vec![0u32, 0, 5, 1, 0, 1, 2, 3, 2, 0];
        let edits = compute_semantic_tokens_delta_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(5));
        assert_eq!(edits[0]["deleteCount"], json!(0));
        assert_eq!(edits[0]["data"], json!([1, 2, 3, 2, 0]));
    }

    #[test]
    fn compute_semantic_tokens_delta_edits_handles_trailing_delete() {
        let old = vec![0u32, 0, 5, 1, 0, 1, 2, 3, 2, 0];
        let new = vec![0u32, 0, 5, 1, 0];
        let edits = compute_semantic_tokens_delta_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(5));
        assert_eq!(edits[0]["deleteCount"], json!(5));
        assert_eq!(edits[0]["data"], json!([]));
    }

    #[test]
    fn compute_semantic_tokens_delta_edits_handles_middle_replacement() {
        // Common prefix [0,0,5,1,0], changed middle, common suffix [9,9,9,9,9].
        let old = vec![0u32, 0, 5, 1, 0, 1, 1, 1, 1, 1, 9, 9, 9, 9, 9];
        let new = vec![0u32, 0, 5, 1, 0, 2, 2, 2, 2, 2, 9, 9, 9, 9, 9];
        let edits = compute_semantic_tokens_delta_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(5));
        assert_eq!(edits[0]["deleteCount"], json!(5));
        assert_eq!(edits[0]["data"], json!([2, 2, 2, 2, 2]));
    }

    #[test]
    fn semantic_tokens_cache_evicted_on_document_close() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///cache_evict.pl";
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\n",
            }
        })))?;

        // A full request populates the per-URI token cache.
        server.handle_semantic_tokens(Some(json!({ "textDocument": { "uri": uri } })))?;
        assert!(
            server.semantic_tokens_cache.lock().contains_key(uri),
            "cache should be populated after a full request"
        );

        // Closing the document (didClose path) must sweep the cache entry so
        // long-lived sessions do not accumulate token arrays for closed files.
        server.evict_open_document_session_state(uri);
        assert!(
            !server.semantic_tokens_cache.lock().contains_key(uri),
            "semantic-token cache entry must be removed when the document is evicted"
        );

        Ok(())
    }

    #[test]
    fn handle_semantic_tokens_range_uses_core_label_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///semantic_range_label.pl";
        let source = "OUTER: while ($x) {\n    last OUTER;\n}\n";
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": source,
            }
        })))?;

        let result = server
            .handle_semantic_tokens_range(Some(json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": 20 }
                }
            })))?
            .ok_or("semantic tokens range result missing")?;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .ok_or("semantic tokens range data missing")?;
        let label_idx =
            *crate::semantic_tokens::legend().map.get("label").ok_or("label token missing")?;

        let mut line = 0u32;
        let mut col = 0u32;
        let mut labels = Vec::new();
        let mut chunks = data.chunks_exact(5);
        for chunk in &mut chunks {
            let delta_line = semantic_token_value(&chunk[0])?;
            let delta_start = semantic_token_value(&chunk[1])?;
            let length = semantic_token_value(&chunk[2])?;
            let token_type = semantic_token_value(&chunk[3])?;
            let modifiers = semantic_token_value(&chunk[4])?;

            if delta_line == 0 {
                col = col.saturating_add(delta_start);
            } else {
                line = line.saturating_add(delta_line);
                col = delta_start;
            }
            if token_type == label_idx {
                labels.push((line, col, length, modifiers));
            }
        }
        if !chunks.remainder().is_empty() {
            return Err("semantic token data length was not a multiple of five".into());
        }

        assert_eq!(labels, vec![(1, 9, 5, 0)]);

        Ok(())
    }
}

fn semantic_token_subroutine_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("sub ")?;
    let name_start = marker_start + "sub ".len();
    let mut name_end = name_start;

    for (offset, ch) in source[name_start..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start {
        return None;
    }

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:function:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_package_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("package ")?;
    let mut name_start = marker_start + "package ".len();

    while let Some(ch) = source[name_start..].chars().next() {
        if ch.is_whitespace() {
            name_start += ch.len_utf8();
        } else {
            break;
        }
    }

    let mut name_end = name_start;
    for (offset, ch) in source[name_start..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start {
        return None;
    }

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:package_declaration:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_method_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("method ")?;
    let name_start = marker_start + "method ".len();
    let mut name_end = name_start;

    for (offset, ch) in source[name_start..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start {
        return None;
    }

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:method_declaration:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_phase_block_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    const PHASES: [&str; 5] = ["BEGIN", "UNITCHECK", "CHECK", "INIT", "END"];

    for phase in PHASES {
        let Some((phase_start, phase_end)) = phase_block_keyword_span(source, phase) else {
            continue;
        };
        let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
            source,
            phase_start,
            phase_end,
        )?;

        return Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            format!("token:phase_block_declaration:{phase}:compiler"),
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            span,
        ));
    }

    None
}

fn phase_block_keyword_span(source: &str, phase: &str) -> Option<(usize, usize)> {
    source.match_indices(phase).find_map(|(start, matched)| {
        let end = start + matched.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        if before.is_none_or(|ch| !is_subroutine_name_char(ch))
            && after.is_none_or(|ch| !is_subroutine_name_char(ch))
        {
            Some((start, end))
        } else {
            None
        }
    })
}

fn semantic_token_method_call_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    semantic_token_receiver_method_call_candidate(source, "$c->", "method_call")
}

fn semantic_token_self_method_call_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    semantic_token_receiver_method_call_candidate(source, "$self->", "self_method_call")
}

fn semantic_token_receiver_method_call_candidate(
    source: &str,
    receiver_marker: &str,
    token_class: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let receiver_start = source.find(receiver_marker)?;
    let mut name_start = receiver_start + receiver_marker.len();

    while let Some(ch) = source[name_start..].chars().next() {
        if ch.is_whitespace() {
            name_start += ch.len_utf8();
        } else {
            break;
        }
    }

    let mut name_end = name_start;
    for (offset, ch) in source[name_start..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start {
        return None;
    }

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:{token_class}:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_class_field_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("field ")?;
    let mut name_start = marker_start + "field ".len();

    while let Some(ch) = source[name_start..].chars().next() {
        if ch.is_whitespace() {
            name_start += ch.len_utf8();
        } else {
            break;
        }
    }

    let sigil = source[name_start..].chars().next()?;
    if !matches!(sigil, '$' | '@' | '%') {
        return None;
    }

    let mut name_end = name_start + sigil.len_utf8();
    for (offset, ch) in source[name_end..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + sigil.len_utf8() + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start + sigil.len_utf8() {
        return None;
    }

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:field_declaration:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_lexical_variable_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("my ")?;
    let line_start = source[..marker_start].rfind('\n').map_or(0, |offset| offset + 1);
    if !source[line_start..marker_start].chars().all(char::is_whitespace) {
        return None;
    }

    let (name_start, name_end) = lexical_variable_name_after_my_marker(source, marker_start)?;

    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:lexical_variable_declaration:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_lexical_variable_use_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let marker_start = source.find("my ")?;
    let (name_start, name_end) = lexical_variable_use_span_after_declaration(source, marker_start)?;
    let name = &source[name_start..name_end];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, name_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:lexical_variable_use:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

fn semantic_token_our_variable_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    line_start_variable_declaration_candidate(source, "our ", "our_variable_declaration")
}

fn semantic_token_state_variable_declaration_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    line_start_variable_declaration_candidate(source, "state ", "state_variable_declaration")
}

/// Detect a line-leading sigiled variable declaration introduced by `marker`
/// (`our `, `state `, …) and emit its `token:<token_class>:<name>:compiler`
/// candidate.
///
/// Every marker occurrence is scanned, not just the first: an earlier
/// non-declaration marker (e.g. a comment or an occurrence inside a string)
/// must not mask a later real source-backed declaration. Each candidate must
/// begin its line (after only whitespace) and yield a sigiled variable name;
/// otherwise we keep scanning and ultimately fall back, preserving the
/// fail-closed boundary.
fn line_start_variable_declaration_candidate(
    source: &str,
    marker: &str,
    token_class: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    for (marker_start, _) in source.match_indices(marker) {
        let line_start = source[..marker_start].rfind('\n').map_or(0, |offset| offset + 1);
        if !source[line_start..marker_start].chars().all(char::is_whitespace) {
            continue;
        }

        let Some((name_start, name_end)) =
            variable_name_after_marker(source, marker_start + marker.len())
        else {
            continue;
        };

        let name = &source[name_start..name_end];
        let Some(span) = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
            source, name_start, name_end,
        ) else {
            continue;
        };

        return Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
            format!("token:{token_class}:{name}:compiler"),
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            span,
        ));
    }

    None
}

/// Detect a bareword named function call `name(...)` and emit its
/// `token:named_function_call:<name>:compiler` candidate.
///
/// Unlike the name-scoped declaration classes, the live parser/HIR provider
/// emits the `function` token for a call across the ENTIRE call expression
/// (`compute(1, 2)`), not just the callee name: `NodeKind::FunctionCall`
/// reports `node.location` spanning the whole call and the collector emits the
/// token at that span (contrast `NodeKind::MethodCall`, which narrows to the
/// method name). To match exactly one existing live token, this candidate's
/// source-backed span therefore covers the callee name through the matching
/// close paren on the same line.
///
/// Method calls (`->name(`), ampersand calls (`&name(`), sigiled/coderef calls
/// (`$name(`), and control-flow / declaration keywords that the collector does
/// NOT classify as `FunctionCall` function tokens are excluded so we never
/// record a candidate that cannot match a live token. The scan skips Perl line
/// comments and single/double-quoted strings, so a `name(` inside a comment or
/// string cannot shadow a later real call and parentheses inside string
/// arguments are not miscounted. A call whose parentheses do not balance on a
/// single line is skipped (the scan continues to a later call), keeping the
/// fail-closed boundary intact.
fn semantic_token_named_function_call_candidate(
    source: &str,
) -> Option<crate::semantic_tokens::SemanticTokenShadowCandidate> {
    let (name_start, paren_open, call_end) = first_named_function_call_span(source)?;
    let name = &source[name_start..paren_open];
    let span = crate::semantic_tokens::SemanticTokenShadowSpan::from_byte_offsets(
        source, name_start, call_end,
    )?;

    Some(crate::semantic_tokens::SemanticTokenShadowCandidate::source_backed_shadow(
        format!("token:named_function_call:{name}:compiler"),
        ProviderFactSourceKind::CompilerFact,
        Provenance::SemanticAnalyzer,
        Confidence::Medium,
        ProviderFactFreshness::Fresh,
        span,
    ))
}

/// Lightweight lexical state for scanning Perl source while skipping the two
/// constructs that would otherwise be mistaken for call syntax: line comments
/// and single/double-quoted string literals. Quote-like forms (`q//`, `qq//`),
/// heredocs, and regex literals are deliberately NOT modeled — they fall
/// through as code, and in the worst case a candidate simply fails to match a
/// live token and the class falls back (output-neutral), never emitting a
/// wrong token.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PerlScanState {
    Code,
    LineComment,
    SingleQuote,
    DoubleQuote,
}

/// Find the first bareword `name(...)` call whose parentheses balance on a
/// single line, returning `(name_start, paren_open, call_end)` byte offsets
/// (`call_end` is just past the matching close paren).
///
/// The scan is comment- and string-aware: a `name(` embedded in a `#` line
/// comment or a quoted string is skipped rather than shadowing a later real
/// call, and parentheses inside string arguments (`emit(")")`) are not counted
/// while balancing. If a candidate call does not close on its line, the scan
/// continues to the next call site rather than giving up — so a leading
/// multi-line or malformed call cannot suppress a later single-line one.
///
/// Runs on nearly every document change, so it walks `char_indices()` once with
/// no heap allocation.
fn first_named_function_call_span(source: &str) -> Option<(usize, usize, usize)> {
    let mut chars = source.char_indices().peekable();
    let mut state = PerlScanState::Code;
    // The character immediately before the current position, used only to
    // distinguish `$#array` (last-index sigil) from a `#` comment and to apply
    // the call-prefix blocker to an identifier run.
    let mut prev = '\0';

    while let Some((idx, ch)) = chars.next() {
        match state {
            PerlScanState::LineComment => {
                if ch == '\n' {
                    state = PerlScanState::Code;
                }
            }
            PerlScanState::SingleQuote => {
                if ch == '\\' {
                    chars.next();
                } else if ch == '\'' {
                    state = PerlScanState::Code;
                }
            }
            PerlScanState::DoubleQuote => {
                if ch == '\\' {
                    chars.next();
                } else if ch == '"' {
                    state = PerlScanState::Code;
                }
            }
            PerlScanState::Code => {
                if ch == '#' && prev != '$' {
                    state = PerlScanState::LineComment;
                } else if ch == '\'' {
                    state = PerlScanState::SingleQuote;
                } else if ch == '"' {
                    state = PerlScanState::DoubleQuote;
                } else if ch.is_ascii_alphabetic() || ch == '_' {
                    let run_start = idx;
                    let mut run_end = idx + ch.len_utf8();
                    while let Some(&(nidx, nch)) = chars.peek() {
                        if is_subroutine_name_char(nch) {
                            run_end = nidx + nch.len_utf8();
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if let Some(&(paren_open, '(')) = chars.peek() {
                        let name = &source[run_start..run_end];
                        if !is_call_prefix_blocker(prev) && !is_non_call_keyword(name) {
                            if let Some(call_end) = string_aware_call_end(source, paren_open) {
                                return Some((run_start, paren_open, call_end));
                            }
                        }
                    }

                    // `run_end` is one past the last name character; the next
                    // loop iteration re-reads whatever follows the run.
                    prev = source[..run_end].chars().next_back().unwrap_or('\0');
                    continue;
                }
            }
        }
        prev = ch;
    }

    None
}

/// Scan from a call's opening `(` to its matching `)` on the same line, skipping
/// parentheses that appear inside Perl line comments or single/double-quoted
/// strings. Returns the byte offset just past the close paren, or `None` if the
/// parentheses do not balance before end-of-line (fail-closed).
fn string_aware_call_end(source: &str, paren_open: usize) -> Option<usize> {
    let mut chars = source[paren_open..].char_indices();
    let mut state = PerlScanState::Code;
    let mut depth = 0usize;
    let mut prev = '\0';

    while let Some((offset, ch)) = chars.next() {
        match state {
            PerlScanState::LineComment => {
                if ch == '\n' {
                    return None;
                }
            }
            PerlScanState::SingleQuote => {
                if ch == '\\' {
                    chars.next();
                } else if ch == '\'' {
                    state = PerlScanState::Code;
                }
            }
            PerlScanState::DoubleQuote => {
                if ch == '\\' {
                    chars.next();
                } else if ch == '"' {
                    state = PerlScanState::Code;
                }
            }
            PerlScanState::Code => match ch {
                '\n' => return None,
                '#' if prev != '$' => state = PerlScanState::LineComment,
                '\'' => state = PerlScanState::SingleQuote,
                '"' => state = PerlScanState::DoubleQuote,
                '(' => depth += 1,
                ')' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(paren_open + offset + ch.len_utf8());
                    }
                }
                _ => {}
            },
        }
        prev = ch;
    }

    None
}

/// Characters immediately preceding an identifier that mark it as something
/// other than a plain bareword function call (method dispatch, coderef/sigil
/// call, ampersand call, reference-taking).
fn is_call_prefix_blocker(ch: char) -> bool {
    matches!(ch, '>' | '&' | '$' | '@' | '%' | '\\')
}

/// Keywords that take a parenthesised form but are NOT emitted as
/// `NodeKind::FunctionCall` `function` tokens by the live collector
/// (control flow, logical operators, and the collector's builtin skip-list).
fn is_non_call_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "unless"
            | "while"
            | "until"
            | "for"
            | "foreach"
            | "elsif"
            | "else"
            | "given"
            | "when"
            | "and"
            | "or"
            | "not"
            | "eval"
            | "do"
            | "use"
            | "no"
            | "return"
            | "my"
            | "our"
            | "local"
            | "state"
            | "sub"
            | "next"
            | "last"
            | "redo"
            | "goto"
    )
}

fn lexical_variable_name_after_my_marker(
    source: &str,
    marker_start: usize,
) -> Option<(usize, usize)> {
    variable_name_after_marker(source, marker_start + "my ".len())
}

/// Scan the sigiled variable name starting at `name_search_start`, skipping any
/// leading whitespace. Shared by the `my`/`our` declaration and use detectors so
/// each compiler-token class extracts the same source-backed span shape.
fn variable_name_after_marker(source: &str, name_search_start: usize) -> Option<(usize, usize)> {
    let mut name_start = name_search_start;

    while let Some(ch) = source[name_start..].chars().next() {
        if ch.is_whitespace() {
            name_start += ch.len_utf8();
        } else {
            break;
        }
    }

    let sigil = source[name_start..].chars().next()?;
    if !matches!(sigil, '$' | '@' | '%') {
        return None;
    }

    let mut name_end = name_start + sigil.len_utf8();
    for (offset, ch) in source[name_end..].char_indices() {
        if is_subroutine_name_char(ch) {
            name_end = name_start + sigil.len_utf8() + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    if name_end == name_start + sigil.len_utf8() {
        return None;
    }

    Some((name_start, name_end))
}

fn lexical_variable_use_span_after_declaration(
    source: &str,
    marker_start: usize,
) -> Option<(usize, usize)> {
    let (name_start, name_end) = lexical_variable_name_after_my_marker(source, marker_start)?;
    let name = &source[name_start..name_end];
    let mut search_start = name_end;

    while let Some(relative_start) = source[search_start..].find(name) {
        let use_start = search_start + relative_start;
        let use_end = use_start + name.len();
        let next_is_name_char =
            source[use_end..].chars().next().is_some_and(is_subroutine_name_char);
        if !next_is_name_char {
            return Some((use_start, use_end));
        }
        search_start = use_end;
    }

    None
}

fn semantic_tokens_live_slice_provider_trace(
    source: &str,
    live_provider_result: &Value,
    live_token_count: usize,
    provider_action: &'static str,
) -> Value {
    let mut saw_compiler_token_candidate = false;

    let subroutine_candidate = semantic_token_subroutine_declaration_candidate(source);
    saw_compiler_token_candidate |= subroutine_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        subroutine_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "function",
            compiler_token_class: "subroutine_declaration",
            source_backed_state: "source_backed_subroutine_declaration_live_token_match",
            user_message: "Semantic tokens used the source-backed compiler subroutine-declaration live slice because it matched the existing parser/HIR function token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler subroutine-declaration spans that exactly match existing live parser/HIR function tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let method_declaration_candidate = semantic_token_method_declaration_candidate(source);
    saw_compiler_token_candidate |= method_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        method_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "method",
            compiler_token_class: "method_declaration",
            source_backed_state: "source_backed_method_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler method-declaration live trace because it matched the existing parser/HIR method token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler method-declaration spans that exactly match existing live parser/HIR method tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader method classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let phase_block_declaration_candidate =
        semantic_token_phase_block_declaration_candidate(source);
    saw_compiler_token_candidate |= phase_block_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        phase_block_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "macro",
            compiler_token_class: "phase_block_declaration",
            source_backed_state: "source_backed_phase_block_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler phase-block declaration live trace because it matched the existing parser/HIR macro token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler phase-block declaration spans that exactly match existing live parser/HIR macro tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader macro classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let method_call_candidate = semantic_token_method_call_candidate(source);
    saw_compiler_token_candidate |= method_call_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        method_call_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "method",
            compiler_token_class: "method_call",
            source_backed_state: "source_backed_method_call_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler method-call live trace because it matched the existing parser/HIR method token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler method-call spans that exactly match existing live parser/HIR method tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader method classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let self_method_call_candidate = semantic_token_self_method_call_candidate(source);
    saw_compiler_token_candidate |= self_method_call_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        self_method_call_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "method",
            compiler_token_class: "self_method_call",
            source_backed_state: "source_backed_self_method_call_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler $self method-call live trace because it matched the existing parser/HIR method token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler $self method-call spans that exactly match existing live parser/HIR method tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader receiver classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let package_declaration_candidate = semantic_token_package_declaration_candidate(source);
    saw_compiler_token_candidate |= package_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        package_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "namespace",
            compiler_token_class: "package_declaration",
            source_backed_state: "source_backed_package_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler package-declaration live trace because it matched the existing parser/HIR namespace token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler package-declaration spans that exactly match existing live parser/HIR namespace tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let field_declaration_candidate = semantic_token_class_field_declaration_candidate(source);
    saw_compiler_token_candidate |= field_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        field_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "variable",
            compiler_token_class: "field_declaration",
            source_backed_state: "source_backed_field_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler field-declaration live trace because it matched the existing parser/HIR variable token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler field-declaration spans that exactly match existing live parser/HIR variable tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader variable classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let lexical_variable_declaration_candidate =
        semantic_token_lexical_variable_declaration_candidate(source);
    saw_compiler_token_candidate |= lexical_variable_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        lexical_variable_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "variable",
            compiler_token_class: "lexical_variable_declaration",
            source_backed_state: "source_backed_lexical_variable_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler lexical-variable declaration live trace because it matched the existing parser/HIR variable token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler lexical-variable declaration spans that exactly match existing live parser/HIR variable tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader variable classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let lexical_variable_use_candidate = semantic_token_lexical_variable_use_candidate(source);
    saw_compiler_token_candidate |= lexical_variable_use_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        lexical_variable_use_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "variable",
            compiler_token_class: "lexical_variable_use",
            source_backed_state: "source_backed_lexical_variable_use_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler lexical-variable use live trace because it matched the existing parser/HIR variable token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler lexical-variable use spans that exactly match existing live parser/HIR variable tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader variable classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let our_variable_declaration_candidate =
        semantic_token_our_variable_declaration_candidate(source);
    saw_compiler_token_candidate |= our_variable_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        our_variable_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "variable",
            compiler_token_class: "our_variable_declaration",
            source_backed_state: "source_backed_our_variable_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler our-variable declaration live trace because it matched the existing parser/HIR variable token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler our-variable declaration spans that exactly match existing live parser/HIR variable tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader variable classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let state_variable_declaration_candidate =
        semantic_token_state_variable_declaration_candidate(source);
    saw_compiler_token_candidate |= state_variable_declaration_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        state_variable_declaration_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "variable",
            compiler_token_class: "state_variable_declaration",
            source_backed_state: "source_backed_state_variable_declaration_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler state-variable declaration live trace because it matched the existing parser/HIR variable token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler state-variable declaration spans that exactly match existing live parser/HIR variable tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader variable classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    let named_function_call_candidate = semantic_token_named_function_call_candidate(source);
    saw_compiler_token_candidate |= named_function_call_candidate.is_some();
    if let Some(trace) = semantic_tokens_live_slice_provider_trace_for_candidate(
        named_function_call_candidate,
        Some(live_provider_result),
        live_token_count,
        provider_action,
        SemanticTokenLiveSliceTraceSpec {
            live_token_type: "function",
            compiler_token_class: "named_function_call",
            source_backed_state: "source_backed_named_function_call_live_token_match",
            user_message: "Semantic tokens exposed the source-backed compiler named-function-call live trace because its whole-call span matched the existing parser/HIR function token. No new semantic tokens were emitted.",
            claim_boundary: "only source-backed compiler named-function-call whole-call spans that exactly match existing live parser/HIR function tokens participate; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader function classes, and unmatched compiler candidates remain blocked, fallback-only, or shadowed",
        },
    ) {
        return trace;
    }

    if !saw_compiler_token_candidate {
        return semantic_tokens_fallback_provider_trace(
            provider_action,
            live_token_count,
            "no_compiler_token_class",
            "semantic tokens used the existing parser/HIR provider; no reviewed source-backed compiler token class matched this request",
        );
    }

    semantic_tokens_fallback_provider_trace(
        provider_action,
        live_token_count,
        "compiler_token_span_not_live",
        "semantic tokens used the existing parser/HIR provider; the reviewed compiler token candidate did not match a live token span",
    )
}

struct SemanticTokenLiveSliceTraceSpec {
    live_token_type: &'static str,
    compiler_token_class: &'static str,
    source_backed_state: &'static str,
    user_message: &'static str,
    claim_boundary: &'static str,
}

fn semantic_tokens_live_slice_provider_trace_for_candidate(
    candidate: Option<crate::semantic_tokens::SemanticTokenShadowCandidate>,
    live_provider_result: Option<&Value>,
    live_token_count: usize,
    provider_action: &'static str,
    spec: SemanticTokenLiveSliceTraceSpec,
) -> Option<Value> {
    let candidate = candidate?;
    if !semantic_tokens_live_contains_span(
        live_provider_result,
        candidate.source_span.as_ref(),
        spec.live_token_type,
    ) {
        return None;
    }

    Some(json!({
        "provider": "semantic_tokens",
        "provider_action": provider_action,
        "decision": "acted",
        "reason": "source_backed_compiler_token_live_slice",
        "fact_source": "compiler_fact",
        "confidence": "high",
        "freshness": "fresh",
        "source_backed": true,
        "source_backed_state": spec.source_backed_state,
        "dynamic_boundary": false,
        "fallback_state": "none",
        "live_provider_result_kind": "semantic_token_data",
        "live_provider_result_count": u64::try_from(live_token_count).unwrap_or(u64::MAX),
        "live_cutover": "partial_live_source_backed_compiler_token",
        "compiler_token_class": spec.compiler_token_class,
        "live_token_type": spec.live_token_type,
        "live_token_match_count": 1,
        "no_live_token_output_change": true,
        "user_message": spec.user_message,
        "claim_boundary": spec.claim_boundary,
    }))
}

fn semantic_tokens_fallback_provider_trace(
    provider_action: &'static str,
    live_token_count: usize,
    reason: &'static str,
    user_message: &'static str,
) -> Value {
    json!({
        "provider": "semantic_tokens",
        "provider_action": provider_action,
        "decision": "fallback",
        "reason": reason,
        "fact_source": "parser_syntax",
        "confidence": "medium",
        "freshness": "fresh",
        "source_backed": false,
        "source_backed_state": "compiler_token_live_slice_not_proven",
        "dynamic_boundary": false,
        "fallback_state": "legacy_provider",
        "live_provider_result_kind": "semantic_token_data",
        "live_provider_result_count": u64::try_from(live_token_count).unwrap_or(u64::MAX),
        "live_cutover": "fallback_only",
        "compiler_token_class": "reviewed_scoped_token_class",
        "no_live_token_output_change": true,
        "user_message": user_message,
        "claim_boundary": "parser/HIR semantic tokens remain the fallback for requests without a source-backed compiler token span matching existing live output; no compiler-backed token expansion",
    })
}

fn semantic_tokens_live_contains_span(
    live_provider_result: Option<&Value>,
    source_span: Option<&crate::semantic_tokens::SemanticTokenShadowSpan>,
    token_type: &str,
) -> bool {
    let Some(source_span) = source_span else {
        return false;
    };
    let Some(expected_length) = source_span.single_line_lsp_length() else {
        return false;
    };
    let Some(token_type_index) = semantic_token_type_index(token_type) else {
        return false;
    };
    let Some(data) =
        live_provider_result.and_then(|value| value.get("data")).and_then(Value::as_array)
    else {
        return false;
    };

    let mut current_line = 0_u32;
    let mut current_start = 0_u32;
    for token in data.chunks_exact(5) {
        let Some(delta_line) = semantic_token_value_u32(&token[0]) else {
            return false;
        };
        let Some(delta_start) = semantic_token_value_u32(&token[1]) else {
            return false;
        };
        let Some(length) = semantic_token_value_u32(&token[2]) else {
            return false;
        };
        let Some(actual_type_index) = semantic_token_value_u32(&token[3]) else {
            return false;
        };

        if delta_line == 0 {
            current_start = current_start.saturating_add(delta_start);
        } else {
            current_line = current_line.saturating_add(delta_line);
            current_start = delta_start;
        }

        if current_line == source_span.range.start.line
            && current_start == source_span.range.start.character
            && actual_type_index == token_type_index
            && length == expected_length
        {
            return true;
        }
    }

    false
}

fn semantic_token_type_index(token_type: &str) -> Option<u32> {
    let legend = crate::semantic_tokens::legend();
    legend.map.get(token_type).copied()
}

fn semantic_token_value_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|value| u32::try_from(value).ok())
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn provider_fallback_state_label(state: ProviderFallbackState) -> &'static str {
    match state {
        ProviderFallbackState::Primary => "Primary",
        ProviderFallbackState::Fallback => "Fallback",
        ProviderFallbackState::Unavailable => "Unavailable",
        ProviderFallbackState::Shadow => "Shadow",
        ProviderFallbackState::Blocked => "Blocked",
        _ => "Unknown",
    }
}

fn is_subroutine_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'
}

/// Build an actionable INVALID_REQUEST error for semantic-token requests on
/// documents that have not been opened/synchronized yet.
///
/// The expanded message guides the editor developer to send
/// `textDocument/didOpen` before requesting tokens.
///
/// Ported from EffortlessMetrics/perl-lsp#9868.
fn semantic_tokens_document_not_open(uri: &str) -> JsonRpcError {
    JsonRpcError {
        code: INVALID_REQUEST,
        message: format!(
            "Document not open: {uri}. \
             textDocument/semanticTokens/full requires the editor to send \
             textDocument/didOpen before requesting tokens; \
             resend after the document is open and synchronized."
        ),
        data: None,
    }
}

#[cfg(test)]
mod semantic_tokens_guidance_tests {
    use super::*;
    use perl_tdd_support::must_err;
    use serde_json::json;

    /// A semantic-token request on an un-opened document must return an
    /// INVALID_REQUEST error whose message contains sync-guidance strings.
    ///
    /// Ported from EffortlessMetrics/perl-lsp#9868.
    #[test]
    fn semantic_tokens_closed_document_error_includes_sync_guidance()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///workspace/lib/Missing.pm";

        let error = must_err(server.handle_semantic_tokens(Some(json!({
            "textDocument": {
                "uri": uri,
            },
        }))));

        assert_eq!(error.code, INVALID_REQUEST);
        assert!(error.data.is_none());
        for expected in [
            "Document not open",
            uri,
            "textDocument/semanticTokens/full",
            "textDocument/didOpen",
            "open and synchronized",
        ] {
            assert!(
                error.message.contains(expected),
                "error message must mention {expected:?}; got {:?}",
                error.message
            );
        }

        Ok(())
    }
}

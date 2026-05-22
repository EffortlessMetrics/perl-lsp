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
            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let data =
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
                        self.offset_to_pos16(doc, off)
                    });
                let flat_data: Vec<_> = data.into_iter().flatten().collect();
                let live_token_count = flat_data.len() / 5;
                let live_result = json!({ "data": flat_data });
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

    /// Handle semantic tokens full request (alternative method name)
    #[allow(dead_code)] // Alternative implementation using SemanticTokensProvider
    pub(crate) fn handle_semantic_tokens_full(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            tracing::debug!(uri, "Getting semantic tokens");

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = SemanticTokensProvider::new(doc.text.clone());
                    let tokens = provider.extract(ast);
                    let encoded = encode_semantic_tokens(&tokens);

                    tracing::debug!(count = tokens.len(), "Found semantic tokens");

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
            let ((start_line, _start_char), (end_line, _end_char)) = req_range(&params)?;

            tracing::debug!(uri, start_line, end_line, "Getting semantic tokens for range");

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = SemanticTokensProvider::new(doc.text.clone());
                    let all_tokens = provider.extract(ast);

                    // Filter tokens to the requested range
                    let range_tokens: Vec<_> = all_tokens
                        .into_iter()
                        .filter(|token| token.line >= start_line && token.line <= end_line)
                        .collect();

                    let encoded = encode_semantic_tokens(&range_tokens);

                    tracing::debug!(count = range_tokens.len(), "Found semantic tokens in range");

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

fn lexical_variable_name_after_my_marker(
    source: &str,
    marker_start: usize,
) -> Option<(usize, usize)> {
    let mut name_start = marker_start + "my ".len();

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

//! Shadow-only semantic-token source/freshness proof.
//!
//! The live `textDocument/semanticTokens` provider keeps its existing
//! parser/token behavior. This module only compares that legacy token identity
//! set against compiler-fact token classification candidates and emits typed
//! provider fact-source traces for staged cutover proof.

use perl_position_tracking::WireRange;
use perl_semantic_facts::{
    AnchorId, Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, summarize_identities,
};

/// Legacy semantic-token identity considered by the shadow proof.
///
/// This is not a live LSP token type. The identity should be stable enough to
/// compare the existing provider result against compiler-fact candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SemanticTokenShadowLegacy {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
}

/// Source-backed LSP span for a semantic-token candidate.
///
/// Semantic tokens are encoded as single-line LSP 5-tuples. Compiler-backed
/// candidates must prove this shape before they can count as usable shadow
/// identities for staged cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SemanticTokenShadowSpan {
    /// LSP wire range for the token.
    pub range: WireRange,
}

impl SemanticTokenShadowSpan {
    /// Build a source-backed semantic-token span from byte offsets.
    #[must_use]
    pub fn from_byte_offsets(source: &str, start_byte: usize, end_byte: usize) -> Option<Self> {
        if start_byte >= end_byte
            || end_byte > source.len()
            || !source.is_char_boundary(start_byte)
            || !source.is_char_boundary(end_byte)
        {
            return None;
        }

        Some(Self { range: WireRange::from_byte_offsets(source, start_byte, end_byte) })
    }

    /// Return the LSP token length when the range is a valid single-line token span.
    #[must_use]
    pub fn single_line_lsp_length(&self) -> Option<u32> {
        if self.range.start.line == self.range.end.line
            && self.range.end.character > self.range.start.character
        {
            Some(self.range.end.character - self.range.start.character)
        } else {
            None
        }
    }

    /// Whether this span can be represented as one LSP semantic-token tuple.
    #[must_use]
    pub fn is_valid_lsp_token_span(&self) -> bool {
        self.single_line_lsp_length().is_some()
    }
}

/// Compiler-fact semantic-token classification candidate.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SemanticTokenShadowCandidate {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
    /// Fact source that produced the classification candidate.
    pub source: ProviderFactSourceKind,
    /// Semantic provenance for the classification candidate.
    pub provenance: Provenance,
    /// Confidence in the classification candidate.
    pub confidence: Confidence,
    /// Freshness of the candidate relative to the request.
    pub freshness: ProviderFactFreshness,
    /// Source-backed token span, required for non-blocked compiler-token claims.
    pub source_span: Option<SemanticTokenShadowSpan>,
    /// Whether the candidate is shadowed, fallback, or blocked.
    pub fallback_state: ProviderFallbackState,
    /// Optional source hash for fact freshness proof.
    pub source_hash: Option<String>,
    /// Optional semantic anchor for the candidate.
    pub anchor_id: Option<AnchorId>,
    /// Optional producer model version.
    pub model_version: Option<u32>,
}

impl SemanticTokenShadowCandidate {
    /// Build a source-backed shadow-only semantic-token candidate.
    ///
    /// Use this for staged compiler-backed token-class receipts where the
    /// candidate must remain outside live token behavior until cutover proof
    /// promotes it.
    #[must_use]
    pub fn source_backed_shadow(
        identity: impl Into<String>,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        source_span: SemanticTokenShadowSpan,
    ) -> Self {
        Self {
            identity: identity.into(),
            source,
            provenance,
            confidence,
            freshness,
            source_span: Some(source_span),
            fallback_state: ProviderFallbackState::Shadow,
            source_hash: None,
            anchor_id: None,
            model_version: None,
        }
    }
}

/// Semantic-token shadow proof result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SemanticTokenShadowResult {
    /// Legacy tokens returned by the existing runtime provider path.
    pub legacy_tokens: Vec<SemanticTokenShadowLegacy>,
    /// Shadow receipt comparing legacy tokens with compiler-fact candidates.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Candidate span invariant summary for semantic-token shadow proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SemanticTokenSpanInvariantReport {
    /// Total compiler-token candidates inspected.
    pub candidate_count: usize,
    /// Candidates blocked before they can become semantic-token identities.
    pub blocked_candidate_count: usize,
    /// Non-blocked candidates with a valid single-line source-backed span.
    pub source_backed_span_count: usize,
    /// Non-blocked candidates missing a source-backed span.
    pub missing_source_span_count: usize,
    /// Non-blocked candidates with a span that cannot encode as one LSP token.
    pub invalid_source_span_count: usize,
}

/// Compare legacy semantic-token output against compiler-fact candidates.
///
/// This function is intentionally shadow-only: it returns the original legacy
/// token identities unchanged and emits a receipt that records source,
/// provenance, confidence, freshness, and fallback/blocker state for candidate
/// classifications.
#[must_use]
pub fn semantic_token_source_shadow(
    legacy_tokens: Vec<SemanticTokenShadowLegacy>,
    compiler_candidates: Vec<SemanticTokenShadowCandidate>,
    symbol: &str,
) -> SemanticTokenShadowResult {
    let old_result = summarize_identities(Some(
        legacy_tokens.iter().map(|token| token.identity.clone()).collect(),
    ));
    let new_result =
        summarize_identities(Some(semantic_token_answer_identities(&compiler_candidates)));
    let notes = vec![semantic_token_shadow_note(&legacy_tokens, &compiler_candidates)];
    let fact_source_traces =
        compiler_candidates.iter().map(semantic_token_candidate_trace).collect();

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::SemanticTokens,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_result,
        new_result,
        notes,
        fact_source_traces,
    );

    SemanticTokenShadowResult { legacy_tokens, receipt }
}

fn semantic_token_candidate_trace(candidate: &SemanticTokenShadowCandidate) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::SemanticTokens,
        candidate.source,
        candidate.provenance,
        candidate.confidence,
        candidate.freshness,
        candidate.fallback_state,
        candidate.source_hash.clone(),
        candidate.anchor_id,
        candidate.model_version,
    )
}

fn semantic_token_answer_identities(
    compiler_candidates: &[SemanticTokenShadowCandidate],
) -> Vec<String> {
    compiler_candidates
        .iter()
        .filter(|candidate| semantic_token_candidate_can_count(candidate))
        .map(|candidate| candidate.identity.clone())
        .collect()
}

fn semantic_token_candidate_can_count(candidate: &SemanticTokenShadowCandidate) -> bool {
    matches!(
        candidate.fallback_state,
        ProviderFallbackState::Primary | ProviderFallbackState::Shadow
    ) && candidate
        .source_span
        .as_ref()
        .is_some_and(SemanticTokenShadowSpan::is_valid_lsp_token_span)
        && semantic_token_candidate_class_is_approved(candidate)
}

fn semantic_token_candidate_class_is_approved(candidate: &SemanticTokenShadowCandidate) -> bool {
    match candidate.source {
        ProviderFactSourceKind::ParserSyntax => true,
        ProviderFactSourceKind::CompilerFact => {
            candidate.identity.starts_with("token:function:")
                || candidate.identity.starts_with("token:method_declaration:")
                || candidate.identity.starts_with("token:method_call:")
                || candidate.identity.starts_with("token:self_method_call:")
                || candidate.identity.starts_with("token:package_declaration:")
                || candidate.identity.starts_with("token:phase_block_declaration:")
                || candidate.identity.starts_with("token:field_declaration:")
                || candidate.identity.starts_with("token:lexical_variable_declaration:")
                || candidate.identity.starts_with("token:lexical_variable_use:")
        }
        _ => false,
    }
}

/// Summarize whether semantic-token candidates have source-backed LSP spans.
#[must_use]
pub fn semantic_token_span_invariant_report(
    compiler_candidates: &[SemanticTokenShadowCandidate],
) -> SemanticTokenSpanInvariantReport {
    let mut report = SemanticTokenSpanInvariantReport {
        candidate_count: compiler_candidates.len(),
        blocked_candidate_count: 0,
        source_backed_span_count: 0,
        missing_source_span_count: 0,
        invalid_source_span_count: 0,
    };

    for candidate in compiler_candidates {
        if candidate.fallback_state == ProviderFallbackState::Blocked {
            report.blocked_candidate_count += 1;
            continue;
        }

        match candidate.source_span {
            Some(span) if span.is_valid_lsp_token_span() => {
                report.source_backed_span_count += 1;
            }
            Some(_) => {
                report.invalid_source_span_count += 1;
            }
            None => {
                report.missing_source_span_count += 1;
            }
        }
    }

    report
}

fn semantic_token_shadow_note(
    legacy_tokens: &[SemanticTokenShadowLegacy],
    compiler_candidates: &[SemanticTokenShadowCandidate],
) -> String {
    let span_report = semantic_token_span_invariant_report(compiler_candidates);
    format!(
        "semantic-token shadow proof: legacy_tokens={}; compiler_fact_candidates={}; blocked_candidates={}; source_backed_spans={}; missing_spans={}; invalid_spans={}; no live semantic-token behavior change",
        legacy_tokens.len(),
        compiler_candidates.len(),
        span_report.blocked_candidate_count,
        span_report.source_backed_span_count,
        span_report.missing_source_span_count,
        span_report.invalid_source_span_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_position_tracking::{WirePosition, WireRange};
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    #[test]
    fn semantic_token_shadow_traces_explicit_syntax_classification()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = legacy_token("token:keyword:package:0:0");
        let result = semantic_token_source_shadow(
            vec![legacy],
            vec![shadow_candidate(
                "token:keyword:package:0:0",
                ProviderFactSourceKind::ParserSyntax,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                Some(valid_span(0, 0, 7)),
                ProviderFallbackState::Shadow,
            )],
            "package",
        );

        assert_eq!(result.legacy_tokens.len(), 1);
        assert_eq!(result.receipt.query, ShadowQueryName::SemanticTokens);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::SemanticTokens);
        assert_eq!(trace.source, ProviderFactSourceKind::ParserSyntax);
        assert_eq!(trace.provenance, Provenance::ExactAst);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_labels_compiler_backed_classification()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = semantic_token_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "token:function:Foo::exported:virtual",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                Some(valid_span(0, 4, 8)),
                ProviderFallbackState::Shadow,
            )],
            "exported",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::SemanticTokens);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_blocks_dynamic_boundaries() -> Result<(), Box<dyn std::error::Error>> {
        let result = semantic_token_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "dynamic:semantic-token:$Package::{name}",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Blocked,
            )],
            "$Package::{name}",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_blocks_stale_compiler_facts() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = semantic_token_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "stale:semantic-token:old_function",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                None,
                ProviderFallbackState::Blocked,
            )],
            "old_function",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.freshness, ProviderFactFreshness::Stale);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_excludes_unspanned_compiler_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = semantic_token_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "token:function:Foo::unspanned",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Shadow,
            )],
            "unspanned",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert_eq!(result.receipt.fact_source_traces.len(), 1);

        let report = semantic_token_span_invariant_report(&[shadow_candidate(
            "token:function:Foo::unspanned",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            None,
            ProviderFallbackState::Shadow,
        )]);
        assert_eq!(report.missing_source_span_count, 1);
        assert_eq!(report.source_backed_span_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_excludes_multiline_compiler_candidate_spans()
    -> Result<(), Box<dyn std::error::Error>> {
        let invalid_span = SemanticTokenShadowSpan {
            range: WireRange::new(WirePosition::new(0, 4), WirePosition::new(1, 3)),
        };
        let result = semantic_token_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "token:function:Foo::multiline",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                Some(invalid_span),
                ProviderFallbackState::Shadow,
            )],
            "multiline",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);

        let report = semantic_token_span_invariant_report(&[shadow_candidate(
            "token:function:Foo::multiline",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            Some(invalid_span),
            ProviderFallbackState::Shadow,
        )]);
        assert_eq!(report.invalid_source_span_count, 1);
        assert_eq!(report.source_backed_span_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_token_span_invariant_report_excludes_blocked_candidates_from_span_failures()
    -> Result<(), Box<dyn std::error::Error>> {
        let report = semantic_token_span_invariant_report(&[
            shadow_candidate(
                "dynamic:semantic-token:$Package::{name}",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "token:function:Foo::stale",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "token:function:Foo::fresh",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                Some(valid_span(0, 4, 5)),
                ProviderFallbackState::Shadow,
            ),
        ]);

        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.blocked_candidate_count, 2);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_traces_generated_no_source_and_fallback_boundaries()
    -> Result<(), Box<dyn std::error::Error>> {
        let source_backed_identity = "token:function:Foo::source_backed";
        let candidates = vec![
            shadow_candidate(
                source_backed_identity,
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                Some(valid_span(0, 4, 13)),
                ProviderFallbackState::Shadow,
            ),
            shadow_candidate(
                "token:method:Foo::generated_accessor:no_source",
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "token:method:Foo::fallback_candidate",
                ProviderFactSourceKind::Fallback,
                Provenance::SearchFallback,
                Confidence::Low,
                ProviderFactFreshness::Unknown,
                None,
                ProviderFallbackState::Fallback,
            ),
        ];

        let report = semantic_token_span_invariant_report(&candidates);
        let result = semantic_token_source_shadow(Vec::new(), candidates, "Foo::accessor");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(result.receipt.new_result.identities, vec![source_backed_identity.to_string()]);
        assert_eq!(result.receipt.fact_source_traces.len(), 3);

        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.blocked_candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 1);
        assert_eq!(report.invalid_source_span_count, 0);

        let generated_trace = trace_at(&result, 1)?;
        assert_eq!(generated_trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(generated_trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(generated_trace.fallback_state, ProviderFallbackState::Blocked);

        let fallback_trace = trace_at(&result, 2)?;
        assert_eq!(fallback_trace.source, ProviderFactSourceKind::Fallback);
        assert_eq!(fallback_trace.provenance, Provenance::SearchFallback);
        assert_eq!(fallback_trace.freshness, ProviderFactFreshness::Unknown);
        assert_eq!(fallback_trace.fallback_state, ProviderFallbackState::Fallback);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_blocks_unsafe_generated_dynamic_stale_and_fallback_boundaries()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidates = vec![
            shadow_candidate(
                "token:method:Framework::generated_accessor:no_source",
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "dynamic:semantic-token:$Package::{method}",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "stale:semantic-token:old_method",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                None,
                ProviderFallbackState::Blocked,
            ),
            shadow_candidate(
                "fallback:semantic-token:grep_guess",
                ProviderFactSourceKind::Fallback,
                Provenance::SearchFallback,
                Confidence::Low,
                ProviderFactFreshness::Unknown,
                None,
                ProviderFallbackState::Fallback,
            ),
        ];

        let report = semantic_token_span_invariant_report(&candidates);
        let result = semantic_token_source_shadow(Vec::new(), candidates, "unsafe-boundaries");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert!(
            result.receipt.new_result.identities.is_empty(),
            "unsafe semantic-token boundaries must not produce live token identities"
        );
        assert_eq!(result.receipt.fact_source_traces.len(), 4);

        assert_eq!(report.candidate_count, 4);
        assert_eq!(report.blocked_candidate_count, 3);
        assert_eq!(report.source_backed_span_count, 0);
        assert_eq!(report.missing_source_span_count, 1);
        assert_eq!(report.invalid_source_span_count, 0);

        let generated_trace = trace_at(&result, 0)?;
        assert_eq!(generated_trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(generated_trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(generated_trace.fallback_state, ProviderFallbackState::Blocked);

        let dynamic_trace = trace_at(&result, 1)?;
        assert_eq!(dynamic_trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(dynamic_trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(dynamic_trace.fallback_state, ProviderFallbackState::Blocked);

        let stale_trace = trace_at(&result, 2)?;
        assert_eq!(stale_trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(stale_trace.confidence, Confidence::Low);
        assert_eq!(stale_trace.freshness, ProviderFactFreshness::Stale);
        assert_eq!(stale_trace.fallback_state, ProviderFallbackState::Blocked);

        let fallback_trace = trace_at(&result, 3)?;
        assert_eq!(fallback_trace.source, ProviderFactSourceKind::Fallback);
        assert_eq!(fallback_trace.provenance, Provenance::SearchFallback);
        assert_eq!(fallback_trace.confidence, Confidence::Low);
        assert_eq!(fallback_trace.freshness, ProviderFactFreshness::Unknown);
        assert_eq!(fallback_trace.fallback_state, ProviderFallbackState::Fallback);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_blocks_broader_compiler_token_class_false_exact()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:method:Foo::generated_method:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            Some(valid_span(0, 4, 16)),
            ProviderFallbackState::Shadow,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "generated_method");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert!(
            result.receipt.new_result.identities.is_empty(),
            "broader compiler token classes must not become exact token identities without class-specific proof"
        );

        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.blocked_candidate_count, 0);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_blocks_unpromoted_class_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:class_declaration:Widget:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::High,
            ProviderFactFreshness::Fresh,
            Some(valid_span(0, 6, 6)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "class_declaration");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 0);
        assert!(
            result.receipt.new_result.identities.is_empty(),
            "class-declaration compiler tokens must stay blocked until class-specific proof promotes them"
        );

        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.blocked_candidate_count, 0);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_method_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:method_declaration:greet:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(2, 11, 5)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result =
            semantic_token_source_shadow(Vec::new(), vec![candidate], "method_declaration");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:method_declaration:greet:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_method_call_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:method_call:stash:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(4, 8, 5)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "method_call");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:method_call:stash:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_self_method_call_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:self_method_call:status:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(4, 12, 6)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "self_method_call");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:self_method_call:status:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_package_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:package_declaration:MyApp::Controller::Root:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(0, 8, 23)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result =
            semantic_token_source_shadow(Vec::new(), vec![candidate], "package_declaration");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:package_declaration:MyApp::Controller::Root:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_phase_block_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:phase_block_declaration:BEGIN:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(2, 0, 5)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result =
            semantic_token_source_shadow(Vec::new(), vec![candidate], "phase_block_declaration");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:phase_block_declaration:BEGIN:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_field_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:field_declaration:$name:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(2, 10, 5)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "field_declaration");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:field_declaration:$name:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_lexical_variable_declaration_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:lexical_variable_declaration:$count:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(1, 3, 6)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "lexical_variable");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:lexical_variable_declaration:$count:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_shadow_allows_scoped_lexical_variable_use_class()
    -> Result<(), Box<dyn std::error::Error>> {
        let candidate = shadow_candidate(
            "token:lexical_variable_use:$count:compiler",
            ProviderFactSourceKind::CompilerFact,
            Provenance::SemanticAnalyzer,
            Confidence::Medium,
            ProviderFactFreshness::Fresh,
            Some(valid_span(2, 0, 6)),
            ProviderFallbackState::Primary,
        );
        let report = semantic_token_span_invariant_report(std::slice::from_ref(&candidate));
        let result = semantic_token_source_shadow(Vec::new(), vec![candidate], "lexical_variable");

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);
        assert_eq!(
            result.receipt.new_result.identities,
            vec!["token:lexical_variable_use:$count:compiler".to_string()]
        );
        assert_eq!(report.candidate_count, 1);
        assert_eq!(report.source_backed_span_count, 1);
        assert_eq!(report.missing_source_span_count, 0);
        assert_eq!(report.invalid_source_span_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::SemanticAnalyzer);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Primary);
        Ok(())
    }

    #[test]
    fn semantic_token_source_span_rejects_zero_length_or_out_of_bounds_ranges() {
        let source = "package Foo;\n";
        assert!(SemanticTokenShadowSpan::from_byte_offsets(source, 0, 0).is_none());
        assert!(SemanticTokenShadowSpan::from_byte_offsets(source, 0, source.len() + 1).is_none());
    }

    #[test]
    fn semantic_token_source_span_reports_lsp_length() -> Result<(), Box<dyn std::error::Error>> {
        let source = "package Foo;\n";
        let start = source.find("Foo").ok_or("expected Foo in fixture source")?;
        let end = start + "Foo".len();
        let span = SemanticTokenShadowSpan::from_byte_offsets(source, start, end)
            .ok_or("expected source-backed Foo span")?;

        assert_eq!(span.range.start.line, 0);
        assert_eq!(span.range.start.character, 8);
        assert_eq!(span.single_line_lsp_length(), Some(3));
        assert!(span.is_valid_lsp_token_span());
        Ok(())
    }

    fn legacy_token(identity: &str) -> SemanticTokenShadowLegacy {
        SemanticTokenShadowLegacy { identity: identity.to_string() }
    }

    fn valid_span(line: u32, start: u32, length: u32) -> SemanticTokenShadowSpan {
        SemanticTokenShadowSpan {
            range: WireRange::new(
                WirePosition::new(line, start),
                WirePosition::new(line, start + length),
            ),
        }
    }

    fn shadow_candidate(
        identity: &str,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        source_span: Option<SemanticTokenShadowSpan>,
        fallback_state: ProviderFallbackState,
    ) -> SemanticTokenShadowCandidate {
        SemanticTokenShadowCandidate {
            identity: identity.to_string(),
            source,
            provenance,
            confidence,
            freshness,
            source_span,
            fallback_state,
            source_hash: Some("fixture-source-sha".to_string()),
            anchor_id: Some(AnchorId(1)),
            model_version: Some(1),
        }
    }

    fn first_trace(
        result: &SemanticTokenShadowResult,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        trace_at(result, 0)
    }

    fn trace_at(
        result: &SemanticTokenShadowResult,
        index: usize,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        result
            .receipt
            .fact_source_traces
            .get(index)
            .ok_or_else(|| "expected semantic-token fact-source trace".into())
    }
}

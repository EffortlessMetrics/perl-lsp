//! Shadow-only semantic-token source/freshness proof.
//!
//! The live `textDocument/semanticTokens` provider keeps its existing
//! parser/token behavior. This module only compares that legacy token identity
//! set against compiler-fact token classification candidates and emits typed
//! provider fact-source traces for staged cutover proof.

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
    /// Whether the candidate is shadowed, fallback, or blocked.
    pub fallback_state: ProviderFallbackState,
    /// Optional source hash for fact freshness proof.
    pub source_hash: Option<String>,
    /// Optional semantic anchor for the candidate.
    pub anchor_id: Option<AnchorId>,
    /// Optional producer model version.
    pub model_version: Option<u32>,
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
        .filter(|candidate| {
            matches!(
                candidate.fallback_state,
                ProviderFallbackState::Primary
                    | ProviderFallbackState::Shadow
                    | ProviderFallbackState::Fallback
            )
        })
        .map(|candidate| candidate.identity.clone())
        .collect()
}

fn semantic_token_shadow_note(
    legacy_tokens: &[SemanticTokenShadowLegacy],
    compiler_candidates: &[SemanticTokenShadowCandidate],
) -> String {
    let blocked_count = compiler_candidates
        .iter()
        .filter(|candidate| candidate.fallback_state == ProviderFallbackState::Blocked)
        .count();
    format!(
        "semantic-token shadow proof: legacy_tokens={}; compiler_fact_candidates={}; blocked_candidates={}; no live semantic-token behavior change",
        legacy_tokens.len(),
        compiler_candidates.len(),
        blocked_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn legacy_token(identity: &str) -> SemanticTokenShadowLegacy {
        SemanticTokenShadowLegacy { identity: identity.to_string() }
    }

    fn shadow_candidate(
        identity: &str,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
    ) -> SemanticTokenShadowCandidate {
        SemanticTokenShadowCandidate {
            identity: identity.to_string(),
            source,
            provenance,
            confidence,
            freshness,
            fallback_state,
            source_hash: Some("fixture-source-sha".to_string()),
            anchor_id: Some(AnchorId(1)),
            model_version: Some(1),
        }
    }

    fn first_trace(
        result: &SemanticTokenShadowResult,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        result
            .receipt
            .fact_source_traces
            .first()
            .ok_or_else(|| "expected semantic-token fact-source trace".into())
    }
}

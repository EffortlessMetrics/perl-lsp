//! Shadow-only document-symbol source/freshness proof.
//!
//! The live `textDocument/documentSymbol` handler still lives in the LSP
//! runtime and keeps its existing behavior. This module only compares that
//! legacy result shape against compiler-fact candidates and emits typed
//! provider fact-source traces for staged cutover proof.

use perl_semantic_facts::{
    AnchorId, Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, summarize_identities,
};

/// Legacy document-symbol identity considered by the shadow proof.
///
/// This is not a live LSP response type. The identity should be stable enough
/// to compare the existing provider result against compiler-fact candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DocumentSymbolShadowLegacy {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
}

/// Compiler-fact candidate considered by the document-symbol shadow proof.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DocumentSymbolShadowCandidate {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
    /// Fact source that produced the candidate.
    pub source: ProviderFactSourceKind,
    /// Semantic provenance for the candidate.
    pub provenance: Provenance,
    /// Confidence in the candidate.
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

/// Document-symbol shadow proof result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DocumentSymbolShadowResult {
    /// Legacy symbols returned by the existing runtime provider path.
    pub legacy_symbols: Vec<DocumentSymbolShadowLegacy>,
    /// Shadow receipt comparing legacy symbols with compiler-fact candidates.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Compare legacy document-symbol output against compiler-fact candidates.
///
/// This function is intentionally shadow-only: it returns the original legacy
/// symbols unchanged and emits a receipt that records source, provenance,
/// confidence, freshness, and fallback/blocker state for candidate facts.
#[must_use]
pub fn document_symbol_source_shadow(
    legacy_symbols: Vec<DocumentSymbolShadowLegacy>,
    compiler_candidates: Vec<DocumentSymbolShadowCandidate>,
    symbol: &str,
) -> DocumentSymbolShadowResult {
    let old_result = summarize_identities(Some(
        legacy_symbols.iter().map(|symbol| symbol.identity.clone()).collect(),
    ));
    let new_result =
        summarize_identities(Some(document_symbol_answer_identities(&compiler_candidates)));
    let notes = vec![document_symbol_shadow_note(&legacy_symbols, &compiler_candidates)];
    let fact_source_traces =
        compiler_candidates.iter().map(document_symbol_candidate_trace).collect();

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::DocumentSymbols,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_result,
        new_result,
        notes,
        fact_source_traces,
    );

    DocumentSymbolShadowResult { legacy_symbols, receipt }
}

fn document_symbol_candidate_trace(candidate: &DocumentSymbolShadowCandidate) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::DocumentSymbols,
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

fn document_symbol_answer_identities(
    compiler_candidates: &[DocumentSymbolShadowCandidate],
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

fn document_symbol_shadow_note(
    legacy_symbols: &[DocumentSymbolShadowLegacy],
    compiler_candidates: &[DocumentSymbolShadowCandidate],
) -> String {
    let blocked_count = compiler_candidates
        .iter()
        .filter(|candidate| candidate.fallback_state == ProviderFallbackState::Blocked)
        .count();
    format!(
        "document-symbol shadow proof: legacy_candidates={}; compiler_fact_candidates={}; blocked_candidates={}; no live document-symbol behavior change",
        legacy_symbols.len(),
        compiler_candidates.len(),
        blocked_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    #[test]
    fn document_symbol_shadow_traces_explicit_syntax_fact() -> Result<(), Box<dyn std::error::Error>>
    {
        let legacy = legacy_symbol("package:Foo:0:0");
        let result = document_symbol_source_shadow(
            vec![legacy],
            vec![shadow_candidate(
                "package:Foo:0:0",
                ProviderFactSourceKind::ParserSyntax,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "Foo",
        );

        assert_eq!(result.legacy_symbols.len(), 1);
        assert_eq!(result.receipt.query, ShadowQueryName::DocumentSymbols);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::DocumentSymbols);
        assert_eq!(trace.source, ProviderFactSourceKind::ParserSyntax);
        assert_eq!(trace.provenance, Provenance::ExactAst);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn document_symbol_shadow_labels_generated_candidates() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "generated:Foo::reader:virtual",
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "reader",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::DocumentSymbols);
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn document_symbol_shadow_blocks_dynamic_boundaries() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "dynamic:Foo::AUTOLOAD",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
            "AUTOLOAD",
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
    fn document_symbol_shadow_blocks_stale_compiler_facts() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "stale:Foo::old_sub",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
            "old_sub",
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

    fn legacy_symbol(identity: &str) -> DocumentSymbolShadowLegacy {
        DocumentSymbolShadowLegacy { identity: identity.to_string() }
    }

    fn shadow_candidate(
        identity: &str,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
    ) -> DocumentSymbolShadowCandidate {
        DocumentSymbolShadowCandidate {
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
        result: &DocumentSymbolShadowResult,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        result
            .receipt
            .fact_source_traces
            .first()
            .ok_or_else(|| "expected document-symbol fact-source trace".into())
    }
}

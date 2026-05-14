//! Provider decision explanation model.
//!
//! This module defines the internal, serializable shape that later LSP UX can
//! expose when a user asks why a provider acted, fell back, shadowed a result,
//! or blocked an unsafe edit. It does not change live provider behavior.

use serde::{Deserialize, Serialize};

use perl_semantic_facts::{
    Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind, ProviderFactTrace,
    ProviderSurface,
};

/// Editor surface that made or considered a provider decision.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionProvider {
    /// Completion provider.
    Completion,
    /// Goto-definition provider.
    GotoDefinition,
    /// References provider.
    References,
    /// Hover provider.
    Hover,
    /// Diagnostics provider.
    Diagnostics,
    /// Rename provider.
    Rename,
    /// Safe-delete provider.
    SafeDelete,
    /// Workspace-symbol provider.
    WorkspaceSymbols,
    /// Document-symbol provider.
    DocumentSymbols,
    /// Semantic-token provider.
    SemanticTokens,
    /// Module-resolution or `@INC` provider surface.
    ModuleResolution,
    /// DAP module-path surface.
    DapModulePaths,
    /// Perl or Perl-adjacent subprocess seam.
    PerlSubprocess,
    /// Surface is not known to this schema version.
    Unknown,
}

impl From<ProviderSurface> for ProviderDecisionProvider {
    fn from(surface: ProviderSurface) -> Self {
        match surface {
            ProviderSurface::Diagnostics => Self::Diagnostics,
            ProviderSurface::Completion => Self::Completion,
            ProviderSurface::Hover => Self::Hover,
            ProviderSurface::Definition => Self::GotoDefinition,
            ProviderSurface::References => Self::References,
            ProviderSurface::Rename => Self::Rename,
            ProviderSurface::SafeDelete => Self::SafeDelete,
            ProviderSurface::WorkspaceSymbols => Self::WorkspaceSymbols,
            ProviderSurface::DocumentSymbols => Self::DocumentSymbols,
            ProviderSurface::SemanticTokens => Self::SemanticTokens,
            _ => Self::Unknown,
        }
    }
}

/// High-level decision a provider made for a request.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionOutcome {
    /// Provider returned a live result.
    Acted,
    /// Provider used a conservative fallback.
    Fallback,
    /// Provider refused an unsafe or unsupported action.
    Blocked,
    /// Provider recorded proof without driving live behavior.
    Shadowed,
}

/// Machine-readable reason for a provider decision.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionReason {
    /// Fresh, high-confidence, source-backed fact was sufficient.
    SourceBackedHighConfidence,
    /// Multiple low-confidence candidates made the exact answer unsafe.
    AmbiguousLowConfidenceCandidates,
    /// Fact existed but was stale relative to the request.
    StaleFact,
    /// Fact existed but did not meet the confidence threshold.
    LowConfidenceFact,
    /// Candidate came from generated or no-source information.
    GeneratedNoSource,
    /// Dynamic Perl boundary prevented static certainty.
    DynamicBoundary,
    /// Provider surface is unsupported for this request.
    Unsupported,
    /// No usable fact existed.
    MissingFact,
    /// Provider only emitted a shadow receipt.
    ShadowOnly,
    /// Edit-producing provider blocked a potentially unsafe edit.
    UnsafeEditBlocked,
    /// Provider policy selected a fallback path.
    FallbackPolicy,
    /// Reason is unknown to this schema version.
    Unknown,
}

/// Coarse fact source serialized for explain-provider responses.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionFactSource {
    /// Parser syntax, token, or AST source.
    ParserSyntax,
    /// Legacy workspace index or provider-local data.
    LegacyWorkspace,
    /// Canonical semantic fact graph.
    SemanticFact,
    /// Compiler substrate fact.
    CompilerFact,
    /// Framework adapter projection.
    FrameworkAdapter,
    /// Dynamic-boundary fact.
    DynamicBoundary,
    /// Fallback path.
    Fallback,
    /// Unknown source.
    Unknown,
}

impl ProviderDecisionFactSource {
    /// Whether this source is allowed to drive a high-confidence live action.
    pub fn is_source_backed(self) -> bool {
        matches!(
            self,
            Self::ParserSyntax | Self::LegacyWorkspace | Self::SemanticFact | Self::CompilerFact
        )
    }
}

impl From<ProviderFactSourceKind> for ProviderDecisionFactSource {
    fn from(source: ProviderFactSourceKind) -> Self {
        match source {
            ProviderFactSourceKind::ParserSyntax => Self::ParserSyntax,
            ProviderFactSourceKind::LegacyWorkspace => Self::LegacyWorkspace,
            ProviderFactSourceKind::SemanticFact => Self::SemanticFact,
            ProviderFactSourceKind::CompilerFact => Self::CompilerFact,
            ProviderFactSourceKind::FrameworkAdapter => Self::FrameworkAdapter,
            ProviderFactSourceKind::DynamicBoundary => Self::DynamicBoundary,
            ProviderFactSourceKind::Fallback => Self::Fallback,
            ProviderFactSourceKind::Unknown => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

/// Confidence vocabulary for provider explanations.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionConfidence {
    /// High confidence.
    High,
    /// Medium confidence.
    Medium,
    /// Low confidence.
    Low,
}

impl From<Confidence> for ProviderDecisionConfidence {
    fn from(confidence: Confidence) -> Self {
        match confidence {
            Confidence::High => Self::High,
            Confidence::Medium => Self::Medium,
            Confidence::Low => Self::Low,
        }
    }
}

/// Freshness vocabulary for provider explanations.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionFreshness {
    /// Fact is fresh for this request.
    Fresh,
    /// Fact exists but is stale.
    Stale,
    /// Freshness is unknown.
    Unknown,
    /// Freshness does not apply to this source.
    NotApplicable,
}

impl From<ProviderFactFreshness> for ProviderDecisionFreshness {
    fn from(freshness: ProviderFactFreshness) -> Self {
        match freshness {
            ProviderFactFreshness::Fresh => Self::Fresh,
            ProviderFactFreshness::Stale => Self::Stale,
            ProviderFactFreshness::Unknown => Self::Unknown,
            ProviderFactFreshness::NotApplicable => Self::NotApplicable,
            _ => Self::Unknown,
        }
    }
}

/// Fallback or refusal path selected by a provider.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionFallback {
    /// No fallback was needed.
    None,
    /// Existing provider implementation handled the request.
    LegacyProvider,
    /// Provider returned no result.
    NoResult,
    /// Provider refused to edit.
    NoEdit,
    /// User confirmation is required before acting.
    RequireConfirmation,
    /// Workspace facts must be refreshed before acting.
    RefreshWorkspaceFacts,
    /// Provider emitted only a shadow receipt.
    ShadowReceiptOnly,
}

/// Serializable explanation for one provider decision.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDecisionExplanation {
    /// Provider surface that made the decision.
    pub provider: ProviderDecisionProvider,
    /// Live, fallback, blocked, or shadowed outcome.
    pub decision: ProviderDecisionOutcome,
    /// Machine-readable reason for the decision.
    pub reason: ProviderDecisionReason,
    /// Coarse fact source used for the decision.
    pub fact_source: ProviderDecisionFactSource,
    /// Confidence in the fact or fallback.
    pub confidence: ProviderDecisionConfidence,
    /// Freshness of the fact relative to the request.
    pub freshness: ProviderDecisionFreshness,
    /// Whether the decision crossed a dynamic Perl boundary.
    pub dynamic_boundary: bool,
    /// Fallback or refusal path.
    pub fallback: ProviderDecisionFallback,
    /// Optional receipt identifier for bug reports and support triage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    /// Optional real-workspace or UX scenario identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
}

impl ProviderDecisionExplanation {
    /// Construct a provider decision explanation from explicit fields.
    // Justification: the model mirrors the stable explain-provider payload fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: ProviderDecisionProvider,
        decision: ProviderDecisionOutcome,
        reason: ProviderDecisionReason,
        fact_source: ProviderDecisionFactSource,
        confidence: ProviderDecisionConfidence,
        freshness: ProviderDecisionFreshness,
        dynamic_boundary: bool,
        fallback: ProviderDecisionFallback,
    ) -> Self {
        Self {
            provider,
            decision,
            reason,
            fact_source,
            confidence,
            freshness,
            dynamic_boundary,
            fallback,
            receipt_id: None,
            scenario: None,
        }
    }

    /// Construct a provider decision explanation from a provider fact trace.
    pub fn from_trace(
        decision: ProviderDecisionOutcome,
        reason: ProviderDecisionReason,
        fallback: ProviderDecisionFallback,
        trace: &ProviderFactTrace,
    ) -> Self {
        Self::new(
            trace.surface.into(),
            decision,
            reason,
            trace.source.into(),
            trace.confidence.into(),
            trace.freshness.into(),
            trace_is_dynamic_boundary(trace),
            fallback,
        )
    }

    /// Attach a stable receipt identifier.
    pub fn with_receipt_id(mut self, receipt_id: impl Into<String>) -> Self {
        self.receipt_id = Some(receipt_id.into());
        self
    }

    /// Attach a real-workspace or UX scenario identifier.
    pub fn with_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.scenario = Some(scenario.into());
        self
    }

    /// Whether this decision may safely drive a live provider action.
    ///
    /// Edit-producing providers still need their own narrower safety checks. This
    /// method only captures the shared trust baseline: acted, fresh, high
    /// confidence, source-backed, no dynamic boundary, and no fallback.
    pub fn is_safe_to_act(&self) -> bool {
        self.decision == ProviderDecisionOutcome::Acted
            && self.fact_source.is_source_backed()
            && self.confidence == ProviderDecisionConfidence::High
            && self.freshness == ProviderDecisionFreshness::Fresh
            && !self.dynamic_boundary
            && self.fallback == ProviderDecisionFallback::None
    }

    /// Whether this explanation records a provider refusal that protects edits.
    pub fn blocks_user_edit(&self) -> bool {
        self.decision == ProviderDecisionOutcome::Blocked
            && matches!(
                self.fallback,
                ProviderDecisionFallback::NoEdit | ProviderDecisionFallback::RequireConfirmation
            )
    }
}

fn trace_is_dynamic_boundary(trace: &ProviderFactTrace) -> bool {
    trace.source == ProviderFactSourceKind::DynamicBoundary
        || trace.provenance == Provenance::DynamicBoundary
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{AnchorId, ProviderFallbackState};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn provider_trace(
        surface: ProviderSurface,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
    ) -> ProviderFactTrace {
        ProviderFactTrace::new(
            surface,
            source,
            provenance,
            confidence,
            freshness,
            ProviderFallbackState::Primary,
            Some("source-hash".to_string()),
            Some(AnchorId(7)),
            Some(1),
        )
    }

    #[test]
    fn provider_decision_serializes_snake_case_payload() -> TestResult {
        let decision = ProviderDecisionExplanation::new(
            ProviderDecisionProvider::GotoDefinition,
            ProviderDecisionOutcome::Fallback,
            ProviderDecisionReason::AmbiguousLowConfidenceCandidates,
            ProviderDecisionFactSource::CompilerFact,
            ProviderDecisionConfidence::Low,
            ProviderDecisionFreshness::Fresh,
            false,
            ProviderDecisionFallback::LegacyProvider,
        )
        .with_receipt_id("semantic-shadow-compare")
        .with_scenario("mojolicious-navigation");

        let value = serde_json::to_value(&decision)?;

        assert_eq!(
            value.get("provider").and_then(serde_json::Value::as_str),
            Some("goto_definition")
        );
        assert_eq!(value.get("decision").and_then(serde_json::Value::as_str), Some("fallback"));
        assert_eq!(
            value.get("reason").and_then(serde_json::Value::as_str),
            Some("ambiguous_low_confidence_candidates")
        );
        assert_eq!(
            value.get("fact_source").and_then(serde_json::Value::as_str),
            Some("compiler_fact")
        );
        assert_eq!(value.get("confidence").and_then(serde_json::Value::as_str), Some("low"));
        assert_eq!(value.get("freshness").and_then(serde_json::Value::as_str), Some("fresh"));
        assert_eq!(
            value.get("fallback").and_then(serde_json::Value::as_str),
            Some("legacy_provider")
        );
        assert_eq!(
            value.get("receipt_id").and_then(serde_json::Value::as_str),
            Some("semantic-shadow-compare")
        );
        assert_eq!(
            value.get("scenario").and_then(serde_json::Value::as_str),
            Some("mojolicious-navigation")
        );
        assert_eq!(value.get("dynamic_boundary").and_then(serde_json::Value::as_bool), Some(false));
        Ok(())
    }

    #[test]
    fn provider_decision_from_dynamic_trace_blocks_user_edit() {
        let trace = provider_trace(
            ProviderSurface::SafeDelete,
            ProviderFactSourceKind::DynamicBoundary,
            Provenance::DynamicBoundary,
            Confidence::Low,
            ProviderFactFreshness::Fresh,
        );

        let decision = ProviderDecisionExplanation::from_trace(
            ProviderDecisionOutcome::Blocked,
            ProviderDecisionReason::DynamicBoundary,
            ProviderDecisionFallback::NoEdit,
            &trace,
        );

        assert_eq!(decision.provider, ProviderDecisionProvider::SafeDelete);
        assert!(decision.dynamic_boundary);
        assert!(decision.blocks_user_edit());
        assert!(!decision.is_safe_to_act());
    }

    #[test]
    fn provider_decision_safe_to_act_requires_fresh_high_source_backed_fact() {
        let trace = provider_trace(
            ProviderSurface::Definition,
            ProviderFactSourceKind::CompilerFact,
            Provenance::ExactAst,
            Confidence::High,
            ProviderFactFreshness::Fresh,
        );

        let safe = ProviderDecisionExplanation::from_trace(
            ProviderDecisionOutcome::Acted,
            ProviderDecisionReason::SourceBackedHighConfidence,
            ProviderDecisionFallback::None,
            &trace,
        );
        assert!(safe.is_safe_to_act());

        let stale = ProviderDecisionExplanation {
            freshness: ProviderDecisionFreshness::Stale,
            ..safe.clone()
        };
        assert!(!stale.is_safe_to_act());

        let low_confidence = ProviderDecisionExplanation {
            confidence: ProviderDecisionConfidence::Low,
            ..safe.clone()
        };
        assert!(!low_confidence.is_safe_to_act());

        let generated = ProviderDecisionExplanation {
            fact_source: ProviderDecisionFactSource::FrameworkAdapter,
            reason: ProviderDecisionReason::GeneratedNoSource,
            ..safe
        };
        assert!(!generated.is_safe_to_act());
    }
}

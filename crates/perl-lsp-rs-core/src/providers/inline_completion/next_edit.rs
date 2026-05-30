//! Feature-gated next-edit scaffold for semantic inline completion.
//!
//! This module intentionally does not produce runtime suggestions yet. It
//! defines the request, gate, candidate-family, and safety-policy boundary that
//! later next-edit providers must use before any editor-visible behavior lands.

use super::PreparedInlineCompletionContext;
use serde::{Deserialize, Serialize};

/// Source that controls the next-edit feature gate.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextEditGateSource {
    /// Default state: no next-edit suggestions are emitted.
    DefaultOff,
    /// Explicit user or workspace configuration.
    ExplicitConfig,
    /// Receipt-only proof mode for local validation.
    ReceiptOnly,
}

/// Runtime gate for next-edit suggestions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditFeatureGate {
    /// Whether a future runtime provider may emit next-edit suggestions.
    pub enabled: bool,
    /// The source that enabled, disabled, or receipt-gated the feature.
    pub source: NextEditGateSource,
}

impl NextEditFeatureGate {
    /// Explicit receipt-only mode: validates the boundary without emitting text.
    #[must_use]
    pub const fn receipt_only() -> Self {
        Self { enabled: false, source: NextEditGateSource::ReceiptOnly }
    }

    /// Explicit opt-in mode for future providers.
    #[must_use]
    pub const fn explicit_enabled() -> Self {
        Self { enabled: true, source: NextEditGateSource::ExplicitConfig }
    }
}

impl Default for NextEditFeatureGate {
    fn default() -> Self {
        Self { enabled: false, source: NextEditGateSource::DefaultOff }
    }
}

/// Planned deterministic next-edit families.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextEditCandidateFamily {
    /// Missing-module diagnostic suggests adding a reachable `use Module;`.
    MissingImport,
    /// A new test name or setup suggests the next assertion body.
    TestAssertionBody,
    /// A signature or parameter edit suggests call-site updates.
    CallSiteUpdate,
    /// A local rename suggests the next matching occurrence.
    RenameOccurrence,
}

impl NextEditCandidateFamily {
    /// Stable list of deterministic families planned for next-edit work.
    #[must_use]
    pub const fn planned() -> &'static [Self] {
        &[
            Self::MissingImport,
            Self::TestAssertionBody,
            Self::CallSiteUpdate,
            Self::RenameOccurrence,
        ]
    }
}

/// Safety policy every future next-edit candidate must satisfy.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditSafetyPolicy {
    /// Candidate must use editor-safe LSP ranges.
    pub requires_editor_safe_range: bool,
    /// Candidate must pass the same parse-safety filter as inline completion.
    pub requires_parse_safety: bool,
    /// Candidate must not conflict with selected completion popup state.
    pub requires_selected_completion_compatibility: bool,
    /// Candidate families must be deterministic before optional AI is considered.
    pub deterministic_sources_only: bool,
    /// Optional AI must remain disabled for the next-edit scaffold.
    pub ai_source_enabled: bool,
}

impl Default for NextEditSafetyPolicy {
    fn default() -> Self {
        Self {
            requires_editor_safe_range: true,
            requires_parse_safety: true,
            requires_selected_completion_compatibility: true,
            deterministic_sources_only: true,
            ai_source_enabled: false,
        }
    }
}

/// Request boundary for future next-edit providers.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditRequest {
    /// Prepared inline-completion context at the current cursor.
    pub context: PreparedInlineCompletionContext,
    /// Candidate families that may be considered by a future provider.
    pub candidate_families: Vec<NextEditCandidateFamily>,
    /// Runtime gate for the request.
    pub gate: NextEditFeatureGate,
    /// Safety policy for future providers.
    pub safety_policy: NextEditSafetyPolicy,
}

impl NextEditRequest {
    /// Construct the current receipt-only request shape.
    #[must_use]
    pub fn receipt_only(context: PreparedInlineCompletionContext) -> Self {
        Self {
            context,
            candidate_families: NextEditCandidateFamily::planned().to_vec(),
            gate: NextEditFeatureGate::receipt_only(),
            safety_policy: NextEditSafetyPolicy::default(),
        }
    }
}

/// A future next-edit suggestion. The current scaffold never emits this.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditSuggestion {
    /// Deterministic source family that produced this suggestion.
    pub family: NextEditCandidateFamily,
    /// Text that would be inserted or replaced by a future edit operation.
    pub new_text: String,
}

impl NextEditSuggestion {
    /// Construct a suggestion value for tests and future provider implementations.
    #[must_use]
    pub fn new(family: NextEditCandidateFamily, new_text: impl Into<String>) -> Self {
        Self { family, new_text: new_text.into() }
    }
}

/// Response state for the next-edit scaffold.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextEditStatus {
    /// The default gate is off.
    Disabled,
    /// Receipt-only validation ran and intentionally emitted no suggestions.
    ReceiptOnly,
    /// A future gate is enabled, but no runtime provider is wired yet.
    RuntimeProviderNotRegistered,
}

/// Response from the next-edit scaffold.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditResponse {
    /// Scaffold status explaining why suggestions are empty.
    pub status: NextEditStatus,
    /// Future next-edit suggestions. Empty in the current scaffold.
    pub suggestions: Vec<NextEditSuggestion>,
}

impl NextEditResponse {
    /// Construct a response value for tests and future provider implementations.
    #[must_use]
    pub fn new(status: NextEditStatus, suggestions: Vec<NextEditSuggestion>) -> Self {
        Self { status, suggestions }
    }
}

/// Disabled-by-default next-edit provider boundary.
#[derive(Debug, Default, Clone, Copy)]
pub struct NextEditProvider;

impl NextEditProvider {
    /// Evaluate the scaffold request. The current implementation never emits
    /// suggestions because no runtime next-edit provider is registered.
    #[must_use]
    pub fn suggest(&self, request: &NextEditRequest) -> NextEditResponse {
        let status = match (request.gate.enabled, request.gate.source) {
            (_, NextEditGateSource::ReceiptOnly) => NextEditStatus::ReceiptOnly,
            (false, _) => NextEditStatus::Disabled,
            (true, _) => NextEditStatus::RuntimeProviderNotRegistered,
        };

        NextEditResponse::new(status, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared_context() -> PreparedInlineCompletionContext {
        PreparedInlineCompletionContext {
            prefix: "use My::".to_string(),
            current_line: "use My::".to_string(),
            previous_non_empty_line: Some("use strict;".to_string()),
            current_function: None,
            current_package: Some("Demo".to_string()),
            variables: vec!["$got".to_string()],
            imports: vec!["strict".to_string(), "warnings".to_string()],
        }
    }

    #[test]
    fn next_edit_gate_defaults_off() -> Result<(), Box<dyn std::error::Error>> {
        let gate = NextEditFeatureGate::default();

        assert!(!gate.enabled);
        assert_eq!(gate.source, NextEditGateSource::DefaultOff);
        Ok(())
    }

    #[test]
    fn receipt_only_request_keeps_all_planned_families_and_safety_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let request = NextEditRequest::receipt_only(prepared_context());

        assert_eq!(request.candidate_families, NextEditCandidateFamily::planned());
        assert_eq!(request.gate, NextEditFeatureGate::receipt_only());
        assert!(request.safety_policy.requires_editor_safe_range);
        assert!(request.safety_policy.requires_parse_safety);
        assert!(request.safety_policy.requires_selected_completion_compatibility);
        assert!(request.safety_policy.deterministic_sources_only);
        assert!(!request.safety_policy.ai_source_enabled);
        Ok(())
    }

    #[test]
    fn scaffold_never_emits_runtime_suggestions() -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let mut request = NextEditRequest::receipt_only(prepared_context());

        let receipt_only = provider.suggest(&request);
        assert_eq!(receipt_only.status, NextEditStatus::ReceiptOnly);
        assert!(receipt_only.suggestions.is_empty());

        request.gate = NextEditFeatureGate::default();
        let disabled = provider.suggest(&request);
        assert_eq!(disabled.status, NextEditStatus::Disabled);
        assert!(disabled.suggestions.is_empty());

        request.gate = NextEditFeatureGate::explicit_enabled();
        let enabled = provider.suggest(&request);
        assert_eq!(enabled.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(enabled.suggestions.is_empty());
        Ok(())
    }
}

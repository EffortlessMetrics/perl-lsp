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

/// Byte-range text edit prepared for a future next-edit workspace edit.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextEditTextEdit {
    /// Start byte offset in the current document.
    pub start_byte: usize,
    /// End byte offset in the current document.
    pub end_byte: usize,
    /// Replacement text for the edit range.
    pub new_text: String,
}

impl NextEditTextEdit {
    /// Construct a byte-range text edit.
    #[must_use]
    pub fn new(start_byte: usize, end_byte: usize, new_text: impl Into<String>) -> Self {
        Self { start_byte, end_byte, new_text: new_text.into() }
    }

    /// Apply the edit to a source document.
    #[must_use]
    pub fn apply_to(&self, source: &str) -> Option<String> {
        if self.start_byte > self.end_byte || self.end_byte > source.len() {
            return None;
        }
        source.get(..self.start_byte)?;
        source.get(self.end_byte..)?;

        let mut edited = source.to_string();
        edited.replace_range(self.start_byte..self.end_byte, &self.new_text);
        Some(edited)
    }
}

/// Rejection reason for receipt-only next-edit candidate proofs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextEditRejectionReason {
    /// The feature gate remains disabled.
    GateDisabled,
    /// The runtime provider is not registered for editor-visible next edits.
    RuntimeProviderNotRegistered,
    /// The requested module name is not a safe Perl package name.
    InvalidModuleName,
    /// The requested module is not reachable from the current include context.
    UnreachableModule,
    /// The requested module is already imported.
    DuplicateImport,
    /// The insertion point could not be prepared safely.
    UnsafeInsertionPoint,
}

/// Receipt-only request for the first deterministic next-edit family:
/// adding a missing `use Module;` line for a reachable module.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingImportNextEditRequest {
    /// Current document text.
    pub document_text: String,
    /// Module that a diagnostic or future intent engine wants to import.
    pub module: String,
    /// Modules reachable from this file's effective include context.
    pub reachable_modules: Vec<String>,
    /// Modules already imported in the current document.
    pub existing_imports: Vec<String>,
    /// Gate used for this receipt-only proof.
    pub gate: NextEditFeatureGate,
}

impl MissingImportNextEditRequest {
    /// Construct a receipt-only missing-import request.
    #[must_use]
    pub fn receipt_only(
        document_text: impl Into<String>,
        module: impl Into<String>,
        reachable_modules: Vec<String>,
        existing_imports: Vec<String>,
    ) -> Self {
        Self {
            document_text: document_text.into(),
            module: module.into(),
            reachable_modules,
            existing_imports,
            gate: NextEditFeatureGate::receipt_only(),
        }
    }
}

/// Receipt-only missing-import candidate prepared by the deterministic
/// next-edit scaffold. It is not editor-visible runtime output.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingImportNextEditCandidate {
    /// Candidate family.
    pub family: NextEditCandidateFamily,
    /// Module inserted by the candidate.
    pub module: String,
    /// Reason recorded for receipts and debugging.
    pub reason: String,
    /// Edit that would insert the import.
    pub edit: NextEditTextEdit,
    /// Receipt-only candidates must not be editor-visible runtime suggestions.
    pub editor_visible: bool,
}

/// Receipt-only missing-import proof result.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingImportNextEditProof {
    /// Scaffold status for this proof.
    pub status: NextEditStatus,
    /// Candidate when all deterministic gates pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<MissingImportNextEditCandidate>,
    /// Reasons the candidate was not prepared.
    pub rejection_reasons: Vec<NextEditRejectionReason>,
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

    /// Prepare a receipt-only missing-import next-edit candidate.
    ///
    /// This does not register a runtime provider and does not emit editor-visible
    /// suggestions. It proves that the first next-edit family can be prepared
    /// only when the module is reachable, not duplicated, and safe to insert.
    #[must_use]
    pub fn prove_missing_import(
        &self,
        request: &MissingImportNextEditRequest,
    ) -> MissingImportNextEditProof {
        match (request.gate.enabled, request.gate.source) {
            (_, NextEditGateSource::ReceiptOnly) => {}
            (false, _) => {
                return MissingImportNextEditProof {
                    status: NextEditStatus::Disabled,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::GateDisabled],
                };
            }
            (true, _) => {
                return MissingImportNextEditProof {
                    status: NextEditStatus::RuntimeProviderNotRegistered,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::RuntimeProviderNotRegistered],
                };
            }
        }

        let mut rejection_reasons = Vec::new();
        if !is_safe_module_name(&request.module) {
            rejection_reasons.push(NextEditRejectionReason::InvalidModuleName);
        }
        if !request.reachable_modules.iter().any(|module| module == &request.module) {
            rejection_reasons.push(NextEditRejectionReason::UnreachableModule);
        }
        if request.existing_imports.iter().any(|module| module == &request.module)
            || document_imports_module(&request.document_text, &request.module)
        {
            rejection_reasons.push(NextEditRejectionReason::DuplicateImport);
        }
        let insertion_offset = import_insertion_offset(&request.document_text);
        if insertion_offset.is_none() {
            rejection_reasons.push(NextEditRejectionReason::UnsafeInsertionPoint);
        }

        if !rejection_reasons.is_empty() {
            return MissingImportNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: None,
                rejection_reasons,
            };
        }

        let offset = insertion_offset.unwrap_or(0);
        let edit = NextEditTextEdit::new(offset, offset, format!("use {};\n", request.module));
        MissingImportNextEditProof {
            status: NextEditStatus::ReceiptOnly,
            candidate: Some(MissingImportNextEditCandidate {
                family: NextEditCandidateFamily::MissingImport,
                module: request.module.clone(),
                reason: "reachable_module_from_effective_inc".to_string(),
                edit,
                editor_visible: false,
            }),
            rejection_reasons,
        }
    }
}

fn is_safe_module_name(module: &str) -> bool {
    !module.is_empty() && module.split("::").all(is_safe_module_segment)
}

fn is_safe_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn document_imports_module(document_text: &str, module: &str) -> bool {
    document_text.lines().any(|line| {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("use ") else {
            return false;
        };
        let rest = rest.trim_start();
        rest == format!("{module};")
            || rest.starts_with(&format!("{module} "))
            || rest.starts_with(&format!("{module};"))
    })
}

fn import_insertion_offset(document_text: &str) -> Option<usize> {
    if document_text.trim_start().starts_with("__DATA__")
        || document_text.trim_start().starts_with("=pod")
    {
        return None;
    }

    let mut offset = 0;
    let mut insertion_offset = 0;
    for line in document_text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#!") || trimmed.starts_with("package ") {
            offset += line.len();
            insertion_offset = offset;
            continue;
        }
        if trimmed.starts_with("use ") {
            offset += line.len();
            insertion_offset = offset;
            continue;
        }
        break;
    }
    Some(insertion_offset)
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

    #[test]
    fn missing_import_receipt_prepares_reachable_non_duplicate_module()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "use strict;\nuse warnings;\nmy $value = My::App->new;\n";
        let request = MissingImportNextEditRequest::receipt_only(
            source,
            "My::App",
            vec!["My::App".to_string()],
            vec!["strict".to_string(), "warnings".to_string()],
        );

        let proof = provider.prove_missing_import(&request);

        assert_eq!(proof.status, NextEditStatus::ReceiptOnly);
        assert!(proof.rejection_reasons.is_empty());
        let candidate = proof.candidate.ok_or("missing import candidate not prepared")?;
        assert_eq!(candidate.family, NextEditCandidateFamily::MissingImport);
        assert_eq!(candidate.module, "My::App");
        assert!(!candidate.editor_visible);
        assert_eq!(candidate.edit.new_text, "use My::App;\n");
        let edited = candidate.edit.apply_to(source).ok_or("edit did not apply")?;
        assert_eq!(edited, "use strict;\nuse warnings;\nuse My::App;\nmy $value = My::App->new;\n");
        Ok(())
    }

    #[test]
    fn missing_import_receipt_rejects_unreachable_and_duplicate_modules()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source_with_import =
            "use strict;\nuse warnings;\nuse My::App;\nmy $value = My::App->new;\n";
        let source_without_import = "use strict;\nuse warnings;\nmy $value = My::Missing->new;\n";
        let duplicate = MissingImportNextEditRequest::receipt_only(
            source_with_import,
            "My::App",
            vec!["My::App".to_string()],
            vec!["My::App".to_string()],
        );
        let unreachable = MissingImportNextEditRequest::receipt_only(
            source_without_import,
            "My::Missing",
            vec!["My::App".to_string()],
            vec![],
        );

        let duplicate_proof = provider.prove_missing_import(&duplicate);
        assert!(duplicate_proof.candidate.is_none());
        assert_eq!(
            duplicate_proof.rejection_reasons,
            vec![NextEditRejectionReason::DuplicateImport]
        );

        let unreachable_proof = provider.prove_missing_import(&unreachable);
        assert!(unreachable_proof.candidate.is_none());
        assert_eq!(
            unreachable_proof.rejection_reasons,
            vec![NextEditRejectionReason::UnreachableModule]
        );
        Ok(())
    }

    #[test]
    fn missing_import_receipt_rejects_invalid_module_names()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let request = MissingImportNextEditRequest::receipt_only(
            "use strict;\nmy $value = My::App->new;\n",
            "My::App; system",
            vec!["My::App; system".to_string()],
            vec![],
        );

        let proof = provider.prove_missing_import(&request);

        assert!(proof.candidate.is_none());
        assert_eq!(proof.rejection_reasons, vec![NextEditRejectionReason::InvalidModuleName]);
        Ok(())
    }

    #[test]
    fn missing_import_receipt_rejects_unsafe_insertion_points()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let request = MissingImportNextEditRequest::receipt_only(
            "__DATA__\nMy::App\n",
            "My::App",
            vec!["My::App".to_string()],
            vec![],
        );

        let proof = provider.prove_missing_import(&request);

        assert!(proof.candidate.is_none());
        assert_eq!(proof.rejection_reasons, vec![NextEditRejectionReason::UnsafeInsertionPoint]);
        Ok(())
    }

    #[test]
    fn next_edit_text_edit_rejects_invalid_ranges() -> Result<(), Box<dyn std::error::Error>> {
        let source = "use utf8;\nmy $sigil = 'lambda: λ';\n";
        let reversed = NextEditTextEdit::new(8, 4, "use strict;\n");
        let too_long = NextEditTextEdit::new(0, source.len() + 1, "use strict;\n");
        let lambda_mid_byte = source.find('λ').ok_or("lambda fixture missing")? + 1;
        let non_boundary = NextEditTextEdit::new(lambda_mid_byte, lambda_mid_byte, "x");

        assert_eq!(reversed.apply_to(source), None);
        assert_eq!(too_long.apply_to(source), None);
        assert_eq!(non_boundary.apply_to(source), None);
        assert_eq!(
            NextEditTextEdit::new(0, 0, "use strict;\n").apply_to(source),
            Some("use strict;\nuse utf8;\nmy $sigil = 'lambda: λ';\n".to_string())
        );
        Ok(())
    }

    #[test]
    fn missing_import_receipt_does_not_bypass_runtime_gate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let mut request = MissingImportNextEditRequest::receipt_only(
            "use strict;\nmy $value = My::App->new;\n",
            "My::App",
            vec!["My::App".to_string()],
            vec![],
        );

        request.gate = NextEditFeatureGate::default();
        let disabled = provider.prove_missing_import(&request);
        assert_eq!(disabled.status, NextEditStatus::Disabled);
        assert!(disabled.candidate.is_none());
        assert_eq!(disabled.rejection_reasons, vec![NextEditRejectionReason::GateDisabled]);

        request.gate = NextEditFeatureGate::explicit_enabled();
        let runtime = provider.prove_missing_import(&request);
        assert_eq!(runtime.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(runtime.candidate.is_none());
        assert_eq!(
            runtime.rejection_reasons,
            vec![NextEditRejectionReason::RuntimeProviderNotRegistered]
        );
        Ok(())
    }
}

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
    /// Test-body candidates only apply in test files.
    TestFileRequired,
    /// The current test framework is unknown or unsupported.
    UnsupportedTestFramework,
    /// No suitable visible actual/expected variables were available.
    MissingAssertionVariables,
    /// The target call name is not a safe Perl subroutine name.
    InvalidCallTarget,
    /// No safe visible argument was available for the call-site update.
    MissingCallArgument,
    /// The target call already contains the requested argument.
    DuplicateCallArgument,
    /// No safe call site was available after the current edit point.
    MissingCallSite,
    /// The rename source or replacement symbol is not a safe Perl variable.
    InvalidRenameSymbol,
    /// No safe next occurrence was available after the current edit point.
    MissingRenameOccurrence,
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

/// Test framework used by a receipt-only test-body next-edit candidate.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestAssertionNextEditFramework {
    /// `Test::More` assertion style.
    TestMore,
    /// `Test2::V0` assertion style.
    Test2V0,
}

/// Receipt-only request for preparing a test assertion body next edit.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestAssertionNextEditRequest {
    /// Current document text.
    pub document_text: String,
    /// Byte offset where the assertion body would be inserted.
    pub insertion_byte: usize,
    /// Whether the current file role is known to be a Perl test file.
    pub file_role_is_test: bool,
    /// Imports visible in the current document.
    pub imports: Vec<String>,
    /// Visible lexicals near the intended insertion point.
    pub visible_variables: Vec<String>,
    /// Gate used for this receipt-only proof.
    pub gate: NextEditFeatureGate,
}

impl TestAssertionNextEditRequest {
    /// Construct a receipt-only test assertion request.
    #[must_use]
    pub fn receipt_only(
        document_text: impl Into<String>,
        insertion_byte: usize,
        imports: Vec<String>,
        visible_variables: Vec<String>,
    ) -> Self {
        Self {
            document_text: document_text.into(),
            insertion_byte,
            file_role_is_test: true,
            imports,
            visible_variables,
            gate: NextEditFeatureGate::receipt_only(),
        }
    }
}

/// Receipt-only test assertion candidate prepared by the deterministic
/// next-edit scaffold. It is not editor-visible runtime output.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestAssertionNextEditCandidate {
    /// Candidate family.
    pub family: NextEditCandidateFamily,
    /// Framework that shaped the assertion.
    pub framework: TestAssertionNextEditFramework,
    /// Reason recorded for receipts and debugging.
    pub reason: String,
    /// Edit that would insert the assertion.
    pub edit: NextEditTextEdit,
    /// Receipt-only candidates must not be editor-visible runtime suggestions.
    pub editor_visible: bool,
}

/// Receipt-only test assertion proof result.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestAssertionNextEditProof {
    /// Scaffold status for this proof.
    pub status: NextEditStatus,
    /// Candidate when all deterministic gates pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<TestAssertionNextEditCandidate>,
    /// Reasons the candidate was not prepared.
    pub rejection_reasons: Vec<NextEditRejectionReason>,
}

/// Receipt-only request for preparing a call-site update after a signature or
/// parameter edit.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSiteUpdateNextEditRequest {
    /// Current document text.
    pub document_text: String,
    /// Bare subroutine or package-qualified function name to update.
    pub callee_name: String,
    /// Visible argument that should be added to the next matching call.
    pub argument: String,
    /// Byte offset after the user's current edit.
    pub cursor_byte: usize,
    /// Gate used for this receipt-only proof.
    pub gate: NextEditFeatureGate,
}

impl CallSiteUpdateNextEditRequest {
    /// Construct a receipt-only call-site update request.
    #[must_use]
    pub fn receipt_only(
        document_text: impl Into<String>,
        callee_name: impl Into<String>,
        argument: impl Into<String>,
        cursor_byte: usize,
    ) -> Self {
        Self {
            document_text: document_text.into(),
            callee_name: callee_name.into(),
            argument: argument.into(),
            cursor_byte,
            gate: NextEditFeatureGate::receipt_only(),
        }
    }
}

/// Receipt-only call-site update candidate prepared by the deterministic
/// next-edit scaffold. It is not editor-visible runtime output.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSiteUpdateNextEditCandidate {
    /// Candidate family.
    pub family: NextEditCandidateFamily,
    /// Callee whose call site would be updated.
    pub callee_name: String,
    /// Argument the candidate would add.
    pub argument: String,
    /// Reason recorded for receipts and debugging.
    pub reason: String,
    /// Edit that would update the call site.
    pub edit: NextEditTextEdit,
    /// Receipt-only candidates must not be editor-visible runtime suggestions.
    pub editor_visible: bool,
}

/// Receipt-only call-site update proof result.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSiteUpdateNextEditProof {
    /// Scaffold status for this proof.
    pub status: NextEditStatus,
    /// Candidate when all deterministic gates pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<CallSiteUpdateNextEditCandidate>,
    /// Reasons the candidate was not prepared.
    pub rejection_reasons: Vec<NextEditRejectionReason>,
}

/// Receipt-only request for preparing the next occurrence in a rename flow.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOccurrenceNextEditRequest {
    /// Current document text.
    pub document_text: String,
    /// Symbol still needing replacement.
    pub original_symbol: String,
    /// Replacement symbol the user already chose.
    pub replacement_symbol: String,
    /// Byte offset after the user's current edit.
    pub cursor_byte: usize,
    /// Gate used for this receipt-only proof.
    pub gate: NextEditFeatureGate,
}

impl RenameOccurrenceNextEditRequest {
    /// Construct a receipt-only rename occurrence request.
    #[must_use]
    pub fn receipt_only(
        document_text: impl Into<String>,
        original_symbol: impl Into<String>,
        replacement_symbol: impl Into<String>,
        cursor_byte: usize,
    ) -> Self {
        Self {
            document_text: document_text.into(),
            original_symbol: original_symbol.into(),
            replacement_symbol: replacement_symbol.into(),
            cursor_byte,
            gate: NextEditFeatureGate::receipt_only(),
        }
    }
}

/// Receipt-only rename occurrence candidate prepared by the deterministic
/// next-edit scaffold. It is not editor-visible runtime output.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOccurrenceNextEditCandidate {
    /// Candidate family.
    pub family: NextEditCandidateFamily,
    /// Symbol still needing replacement.
    pub original_symbol: String,
    /// Replacement symbol the candidate would apply.
    pub replacement_symbol: String,
    /// Reason recorded for receipts and debugging.
    pub reason: String,
    /// Edit that would replace the next occurrence.
    pub edit: NextEditTextEdit,
    /// Receipt-only candidates must not be editor-visible runtime suggestions.
    pub editor_visible: bool,
}

/// Receipt-only rename occurrence proof result.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOccurrenceNextEditProof {
    /// Scaffold status for this proof.
    pub status: NextEditStatus,
    /// Candidate when all deterministic gates pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<RenameOccurrenceNextEditCandidate>,
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
        let line_ending = insertion_line_ending(&request.document_text);
        let edit =
            NextEditTextEdit::new(offset, offset, format!("use {};{line_ending}", request.module));
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

    /// Prepare a receipt-only test assertion next-edit candidate.
    ///
    /// This does not register a runtime provider and does not emit editor-visible
    /// suggestions. It proves that a second deterministic next-edit family can
    /// be prepared only for known test files with supported frameworks and
    /// visible actual/expected variables.
    #[must_use]
    pub fn prove_test_assertion(
        &self,
        request: &TestAssertionNextEditRequest,
    ) -> TestAssertionNextEditProof {
        match (request.gate.enabled, request.gate.source) {
            (_, NextEditGateSource::ReceiptOnly) => {}
            (false, _) => {
                return TestAssertionNextEditProof {
                    status: NextEditStatus::Disabled,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::GateDisabled],
                };
            }
            (true, _) => {
                return TestAssertionNextEditProof {
                    status: NextEditStatus::RuntimeProviderNotRegistered,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::RuntimeProviderNotRegistered],
                };
            }
        }

        let mut rejection_reasons = Vec::new();
        if !request.file_role_is_test {
            rejection_reasons.push(NextEditRejectionReason::TestFileRequired);
        }
        let framework = test_assertion_framework(&request.imports);
        if framework.is_none() {
            rejection_reasons.push(NextEditRejectionReason::UnsupportedTestFramework);
        }
        let variables = test_assertion_variables(&request.visible_variables);
        if variables.is_none() {
            rejection_reasons.push(NextEditRejectionReason::MissingAssertionVariables);
        }
        if !is_safe_next_edit_insertion(&request.document_text, request.insertion_byte) {
            rejection_reasons.push(NextEditRejectionReason::UnsafeInsertionPoint);
        }

        if !rejection_reasons.is_empty() {
            return TestAssertionNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: None,
                rejection_reasons,
            };
        }

        let Some(framework) = framework else {
            return TestAssertionNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: None,
                rejection_reasons: vec![NextEditRejectionReason::UnsupportedTestFramework],
            };
        };
        let Some((actual, expected)) = variables else {
            return TestAssertionNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: None,
                rejection_reasons: vec![NextEditRejectionReason::MissingAssertionVariables],
            };
        };
        let line_ending = insertion_line_ending(&request.document_text);
        let assertion = format!("is({actual}, {expected}, 'test description');{line_ending}");
        TestAssertionNextEditProof {
            status: NextEditStatus::ReceiptOnly,
            candidate: Some(TestAssertionNextEditCandidate {
                family: NextEditCandidateFamily::TestAssertionBody,
                framework,
                reason: "visible_lexical_assertion".to_string(),
                edit: NextEditTextEdit::new(
                    request.insertion_byte,
                    request.insertion_byte,
                    assertion,
                ),
                editor_visible: false,
            }),
            rejection_reasons,
        }
    }

    /// Prepare a receipt-only call-site update next-edit candidate.
    ///
    /// This does not register a runtime provider and does not emit
    /// editor-visible suggestions. It proves that a deterministic call-site
    /// follow-up can be prepared only for a safe visible argument on a safe
    /// same-line Perl call.
    #[must_use]
    pub fn prove_call_site_update(
        &self,
        request: &CallSiteUpdateNextEditRequest,
    ) -> CallSiteUpdateNextEditProof {
        match (request.gate.enabled, request.gate.source) {
            (_, NextEditGateSource::ReceiptOnly) => {}
            (false, _) => {
                return CallSiteUpdateNextEditProof {
                    status: NextEditStatus::Disabled,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::GateDisabled],
                };
            }
            (true, _) => {
                return CallSiteUpdateNextEditProof {
                    status: NextEditStatus::RuntimeProviderNotRegistered,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::RuntimeProviderNotRegistered],
                };
            }
        }

        let mut rejection_reasons = Vec::new();
        if !is_safe_call_target(&request.callee_name) {
            rejection_reasons.push(NextEditRejectionReason::InvalidCallTarget);
        }
        if !is_safe_rename_symbol(&request.argument) {
            rejection_reasons.push(NextEditRejectionReason::MissingCallArgument);
        }
        if !rejection_reasons.is_empty() {
            return CallSiteUpdateNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: None,
                rejection_reasons,
            };
        }

        let update = next_call_site_update(
            &request.document_text,
            &request.callee_name,
            &request.argument,
            request.cursor_byte,
        );
        match update {
            CallSiteUpdateSearch::Found { insert_byte, new_text } => CallSiteUpdateNextEditProof {
                status: NextEditStatus::ReceiptOnly,
                candidate: Some(CallSiteUpdateNextEditCandidate {
                    family: NextEditCandidateFamily::CallSiteUpdate,
                    callee_name: request.callee_name.clone(),
                    argument: request.argument.clone(),
                    reason: "visible_argument_call_site_update".to_string(),
                    edit: NextEditTextEdit::new(insert_byte, insert_byte, new_text),
                    editor_visible: false,
                }),
                rejection_reasons,
            },
            CallSiteUpdateSearch::DuplicateArgument => {
                rejection_reasons.push(NextEditRejectionReason::DuplicateCallArgument);
                CallSiteUpdateNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: None,
                    rejection_reasons,
                }
            }
            CallSiteUpdateSearch::Missing => {
                rejection_reasons.push(NextEditRejectionReason::MissingCallSite);
                CallSiteUpdateNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: None,
                    rejection_reasons,
                }
            }
            CallSiteUpdateSearch::Unsafe => {
                rejection_reasons.push(NextEditRejectionReason::UnsafeInsertionPoint);
                CallSiteUpdateNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: None,
                    rejection_reasons,
                }
            }
        }
    }

    /// Prepare a receipt-only rename occurrence next-edit candidate.
    ///
    /// This does not register a runtime provider and does not emit
    /// editor-visible suggestions. It proves that a deterministic rename
    /// follow-up can be prepared only for safe Perl variable occurrences.
    #[must_use]
    pub fn prove_rename_occurrence(
        &self,
        request: &RenameOccurrenceNextEditRequest,
    ) -> RenameOccurrenceNextEditProof {
        match (request.gate.enabled, request.gate.source) {
            (_, NextEditGateSource::ReceiptOnly) => {}
            (false, _) => {
                return RenameOccurrenceNextEditProof {
                    status: NextEditStatus::Disabled,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::GateDisabled],
                };
            }
            (true, _) => {
                return RenameOccurrenceNextEditProof {
                    status: NextEditStatus::RuntimeProviderNotRegistered,
                    candidate: None,
                    rejection_reasons: vec![NextEditRejectionReason::RuntimeProviderNotRegistered],
                };
            }
        }

        let mut rejection_reasons = Vec::new();
        if !is_safe_rename_symbol(&request.original_symbol)
            || !is_safe_rename_symbol(&request.replacement_symbol)
        {
            rejection_reasons.push(NextEditRejectionReason::InvalidRenameSymbol);
        }

        let occurrence = next_rename_occurrence(
            &request.document_text,
            &request.original_symbol,
            request.cursor_byte,
        );
        match occurrence {
            RenameOccurrenceSearch::Found { start, end } => {
                if !rejection_reasons.is_empty() {
                    return RenameOccurrenceNextEditProof {
                        status: NextEditStatus::ReceiptOnly,
                        candidate: None,
                        rejection_reasons,
                    };
                }
                RenameOccurrenceNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: Some(RenameOccurrenceNextEditCandidate {
                        family: NextEditCandidateFamily::RenameOccurrence,
                        original_symbol: request.original_symbol.clone(),
                        replacement_symbol: request.replacement_symbol.clone(),
                        reason: "next_safe_rename_occurrence".to_string(),
                        edit: NextEditTextEdit::new(start, end, request.replacement_symbol.clone()),
                        editor_visible: false,
                    }),
                    rejection_reasons,
                }
            }
            RenameOccurrenceSearch::Missing => {
                rejection_reasons.push(NextEditRejectionReason::MissingRenameOccurrence);
                RenameOccurrenceNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: None,
                    rejection_reasons,
                }
            }
            RenameOccurrenceSearch::Unsafe => {
                rejection_reasons.push(NextEditRejectionReason::UnsafeInsertionPoint);
                RenameOccurrenceNextEditProof {
                    status: NextEditStatus::ReceiptOnly,
                    candidate: None,
                    rejection_reasons,
                }
            }
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

fn insertion_line_ending(document_text: &str) -> &'static str {
    if document_text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn test_assertion_framework(imports: &[String]) -> Option<TestAssertionNextEditFramework> {
    if imports.iter().any(|import| import == "Test2::V0") {
        return Some(TestAssertionNextEditFramework::Test2V0);
    }
    if imports.iter().any(|import| import == "Test::More") {
        return Some(TestAssertionNextEditFramework::TestMore);
    }
    None
}

fn test_assertion_variables(variables: &[String]) -> Option<(&str, &str)> {
    let actual = variables.iter().find(|variable| is_actual_assertion_variable(variable))?;
    let expected = variables.iter().find(|variable| is_expected_assertion_variable(variable))?;
    (actual != expected).then_some((actual.as_str(), expected.as_str()))
}

fn is_actual_assertion_variable(variable: &str) -> bool {
    matches!(variable, "$got" | "$result" | "$actual")
}

fn is_expected_assertion_variable(variable: &str) -> bool {
    matches!(variable, "$expected" | "$want")
}

fn is_safe_next_edit_insertion(document_text: &str, insertion_byte: usize) -> bool {
    if insertion_byte > document_text.len()
        || document_text.get(..insertion_byte).is_none()
        || document_text.get(insertion_byte..).is_none()
    {
        return false;
    }
    let prefix = &document_text[..insertion_byte];
    if prefix.lines().any(|line| line.trim_start().starts_with("__DATA__")) {
        return false;
    }
    !is_inside_pod(prefix)
}

enum RenameOccurrenceSearch {
    Found { start: usize, end: usize },
    Missing,
    Unsafe,
}

enum CallSiteUpdateSearch {
    Found { insert_byte: usize, new_text: String },
    DuplicateArgument,
    Missing,
    Unsafe,
}

fn next_call_site_update(
    document_text: &str,
    callee_name: &str,
    argument: &str,
    cursor_byte: usize,
) -> CallSiteUpdateSearch {
    if cursor_byte > document_text.len()
        || document_text.get(..cursor_byte).is_none()
        || document_text.get(cursor_byte..).is_none()
    {
        return CallSiteUpdateSearch::Unsafe;
    }

    let needle = format!("{callee_name}(");
    if let Some(relative) = document_text[cursor_byte..].find(&needle) {
        let start = cursor_byte + relative;
        let open_paren = start + callee_name.len();
        let line_end = document_text[open_paren..]
            .find('\n')
            .map_or(document_text.len(), |index| open_paren + index);
        let Some(close_relative) = document_text[open_paren + 1..line_end].find(')') else {
            return CallSiteUpdateSearch::Unsafe;
        };
        let close_paren = open_paren + 1 + close_relative;

        if !has_call_target_boundaries(document_text, start, open_paren)
            || !is_safe_rename_occurrence(document_text, start)
        {
            return CallSiteUpdateSearch::Unsafe;
        }

        let arguments = &document_text[open_paren + 1..close_paren];
        if call_arguments_contain(arguments, argument) {
            return CallSiteUpdateSearch::DuplicateArgument;
        }
        let new_text = if arguments.trim().is_empty() {
            argument.to_string()
        } else {
            format!(", {argument}")
        };
        return CallSiteUpdateSearch::Found { insert_byte: close_paren, new_text };
    }
    CallSiteUpdateSearch::Missing
}

fn call_arguments_contain(arguments: &str, argument: &str) -> bool {
    arguments.split(',').any(|item| item.trim() == argument)
}

fn has_call_target_boundaries(document_text: &str, start: usize, open_paren: usize) -> bool {
    let previous = document_text[..start].chars().next_back();
    let next = document_text[open_paren..].chars().next();
    previous.is_none_or(|ch| !is_perl_identifier_char(ch)) && matches!(next, Some('('))
}

fn is_safe_call_target(callee_name: &str) -> bool {
    !callee_name.is_empty() && callee_name.split("::").all(is_safe_module_segment)
}

fn next_rename_occurrence(
    document_text: &str,
    original_symbol: &str,
    cursor_byte: usize,
) -> RenameOccurrenceSearch {
    if cursor_byte > document_text.len()
        || document_text.get(..cursor_byte).is_none()
        || document_text.get(cursor_byte..).is_none()
    {
        return RenameOccurrenceSearch::Unsafe;
    }

    let mut search_start = cursor_byte;
    while let Some(relative) = document_text[search_start..].find(original_symbol) {
        let start = search_start + relative;
        let end = start + original_symbol.len();
        if !has_rename_boundaries(document_text, start, end) {
            search_start = end;
            continue;
        }
        if !is_safe_rename_occurrence(document_text, start) {
            return RenameOccurrenceSearch::Unsafe;
        }
        return RenameOccurrenceSearch::Found { start, end };
    }
    RenameOccurrenceSearch::Missing
}

fn has_rename_boundaries(document_text: &str, start: usize, end: usize) -> bool {
    let previous = document_text[..start].chars().next_back();
    let next = document_text[end..].chars().next();
    previous.is_none_or(|ch| !is_perl_identifier_char(ch))
        && next.is_none_or(|ch| !is_perl_identifier_char(ch))
}

fn is_safe_rename_occurrence(document_text: &str, start: usize) -> bool {
    if !is_safe_next_edit_insertion(document_text, start) {
        return false;
    }
    let line_start = document_text[..start].rfind('\n').map_or(0, |index| index + 1);
    let prefix = &document_text[line_start..start];
    let trimmed = prefix.trim_start();
    if trimmed.starts_with('#') || prefix.contains('#') {
        return false;
    }
    let single_quotes = prefix.chars().filter(|ch| *ch == '\'').count();
    let double_quotes = prefix.chars().filter(|ch| *ch == '"').count();
    single_quotes % 2 == 0 && double_quotes % 2 == 0
}

fn is_safe_rename_symbol(symbol: &str) -> bool {
    let mut chars = symbol.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '$' | '@' | '%') {
        return false;
    }
    let Some(second) = chars.next() else {
        return false;
    };
    (second == '_' || second.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_perl_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_inside_pod(prefix: &str) -> bool {
    let mut in_pod = false;
    for line in prefix.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("=cut") {
            in_pod = false;
        } else if trimmed.starts_with('=') {
            in_pod = true;
        }
    }
    in_pod
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
    fn missing_import_receipt_preserves_document_line_endings()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "package Demo;\r\nuse strict;\r\nmy $value = My::App->new;\r\n";
        let request = MissingImportNextEditRequest::receipt_only(
            source,
            "My::App",
            vec!["My::App".to_string()],
            vec!["strict".to_string()],
        );

        let proof = provider.prove_missing_import(&request);

        let candidate = proof.candidate.ok_or("missing import candidate not prepared")?;
        assert_eq!(candidate.edit.new_text, "use My::App;\r\n");
        let edited = candidate.edit.apply_to(source).ok_or("edit did not apply")?;
        assert_eq!(
            edited,
            "package Demo;\r\nuse strict;\r\nuse My::App;\r\nmy $value = My::App->new;\r\n"
        );
        assert!(!edited.contains(";\nmy $value"));
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

    #[test]
    fn test_assertion_receipt_prepares_test_more_visible_lexical_assertion()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n";
        let request = TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec!["Test::More".to_string()],
            vec!["$got".to_string(), "$expected".to_string()],
        );

        let proof = provider.prove_test_assertion(&request);

        assert_eq!(proof.status, NextEditStatus::ReceiptOnly);
        assert!(proof.rejection_reasons.is_empty());
        let candidate = proof.candidate.ok_or("test assertion candidate not prepared")?;
        assert_eq!(candidate.family, NextEditCandidateFamily::TestAssertionBody);
        assert_eq!(candidate.framework, TestAssertionNextEditFramework::TestMore);
        assert!(!candidate.editor_visible);
        assert_eq!(candidate.edit.new_text, "is($got, $expected, 'test description');\n");
        let edited = candidate.edit.apply_to(source).ok_or("edit did not apply")?;
        assert_eq!(
            edited,
            "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nis($got, $expected, 'test description');\n"
        );
        Ok(())
    }

    #[test]
    fn test_assertion_receipt_supports_test2_visible_result_assertion()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "use Test2::V0;\nmy $result = compute();\nmy $want = 42;\n";
        let request = TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec!["Test2::V0".to_string()],
            vec!["$result".to_string(), "$want".to_string()],
        );

        let proof = provider.prove_test_assertion(&request);

        let candidate = proof.candidate.ok_or("test assertion candidate not prepared")?;
        assert_eq!(candidate.framework, TestAssertionNextEditFramework::Test2V0);
        assert_eq!(candidate.edit.new_text, "is($result, $want, 'test description');\n");
        assert!(!candidate.editor_visible);
        Ok(())
    }

    #[test]
    fn test_assertion_receipt_rejects_non_test_and_unknown_framework()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "my $got = compute();\nmy $expected = 42;\n";
        let mut request = TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec![],
            vec!["$got".to_string(), "$expected".to_string()],
        );
        request.file_role_is_test = false;

        let proof = provider.prove_test_assertion(&request);

        assert!(proof.candidate.is_none());
        assert_eq!(
            proof.rejection_reasons,
            vec![
                NextEditRejectionReason::TestFileRequired,
                NextEditRejectionReason::UnsupportedTestFramework,
            ]
        );
        Ok(())
    }

    #[test]
    fn test_assertion_receipt_rejects_missing_variables_and_unsafe_insertion()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "=pod\nmy $got = compute();\n";
        let request = TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec!["Test::More".to_string()],
            vec!["$got".to_string()],
        );

        let proof = provider.prove_test_assertion(&request);

        assert!(proof.candidate.is_none());
        assert_eq!(
            proof.rejection_reasons,
            vec![
                NextEditRejectionReason::MissingAssertionVariables,
                NextEditRejectionReason::UnsafeInsertionPoint,
            ]
        );
        Ok(())
    }

    #[test]
    fn test_assertion_receipt_rejects_data_section_and_invalid_offsets()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let data_source = "use Test::More;\n__DATA__\n";
        let data_request = TestAssertionNextEditRequest::receipt_only(
            data_source,
            data_source.len(),
            vec!["Test::More".to_string()],
            vec!["$got".to_string(), "$expected".to_string()],
        );
        let invalid_offset_request = TestAssertionNextEditRequest::receipt_only(
            "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n",
            usize::MAX,
            vec!["Test::More".to_string()],
            vec!["$got".to_string(), "$expected".to_string()],
        );

        let data_proof = provider.prove_test_assertion(&data_request);
        assert!(data_proof.candidate.is_none());
        assert_eq!(
            data_proof.rejection_reasons,
            vec![NextEditRejectionReason::UnsafeInsertionPoint]
        );

        let invalid_offset_proof = provider.prove_test_assertion(&invalid_offset_request);
        assert!(invalid_offset_proof.candidate.is_none());
        assert_eq!(
            invalid_offset_proof.rejection_reasons,
            vec![NextEditRejectionReason::UnsafeInsertionPoint]
        );
        Ok(())
    }

    #[test]
    fn test_assertion_receipt_does_not_bypass_runtime_gate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n";
        let mut request = TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec!["Test::More".to_string()],
            vec!["$got".to_string(), "$expected".to_string()],
        );

        request.gate = NextEditFeatureGate::default();
        let disabled = provider.prove_test_assertion(&request);
        assert_eq!(disabled.status, NextEditStatus::Disabled);
        assert!(disabled.candidate.is_none());
        assert_eq!(disabled.rejection_reasons, vec![NextEditRejectionReason::GateDisabled]);

        request.gate = NextEditFeatureGate::explicit_enabled();
        let runtime = provider.prove_test_assertion(&request);
        assert_eq!(runtime.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(runtime.candidate.is_none());
        assert_eq!(
            runtime.rejection_reasons,
            vec![NextEditRejectionReason::RuntimeProviderNotRegistered]
        );
        Ok(())
    }

    #[test]
    fn call_site_update_receipt_prepares_visible_argument_edit()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "sub build_user ($name, $age) { }\nmy $user = build_user($name);\n";
        let cursor = source.find("my $user").ok_or("call-site fixture missing call")?;
        let request =
            CallSiteUpdateNextEditRequest::receipt_only(source, "build_user", "$age", cursor);

        let proof = provider.prove_call_site_update(&request);

        assert_eq!(proof.status, NextEditStatus::ReceiptOnly);
        assert!(proof.rejection_reasons.is_empty());
        let candidate = proof.candidate.ok_or("call-site update candidate not prepared")?;
        assert_eq!(candidate.family, NextEditCandidateFamily::CallSiteUpdate);
        assert_eq!(candidate.callee_name, "build_user");
        assert_eq!(candidate.argument, "$age");
        assert_eq!(candidate.reason, "visible_argument_call_site_update");
        assert!(!candidate.editor_visible);
        assert_eq!(candidate.edit.new_text, ", $age");
        let edited = candidate.edit.apply_to(source).ok_or("edit did not apply")?;
        assert_eq!(
            edited,
            "sub build_user ($name, $age) { }\nmy $user = build_user($name, $age);\n"
        );
        Ok(())
    }

    #[test]
    fn call_site_update_receipt_rejects_duplicate_and_missing_call_sites()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let duplicate = CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name, $age);\n",
            "build_user",
            "$age",
            0,
        );
        let missing = CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = other_builder($name);\n",
            "build_user",
            "$age",
            0,
        );

        let duplicate_proof = provider.prove_call_site_update(&duplicate);
        assert!(duplicate_proof.candidate.is_none());
        assert_eq!(
            duplicate_proof.rejection_reasons,
            vec![NextEditRejectionReason::DuplicateCallArgument]
        );

        let missing_proof = provider.prove_call_site_update(&missing);
        assert!(missing_proof.candidate.is_none());
        assert_eq!(missing_proof.rejection_reasons, vec![NextEditRejectionReason::MissingCallSite]);
        Ok(())
    }

    #[test]
    fn call_site_update_receipt_rejects_unsafe_and_invalid_inputs()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let unsafe_request = CallSiteUpdateNextEditRequest::receipt_only(
            "# build_user($name)\n",
            "build_user",
            "$age",
            0,
        );
        let invalid_target = CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name);\n",
            "build-user",
            "$age",
            0,
        );
        let missing_argument = CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name);\n",
            "build_user",
            "system($age)",
            0,
        );

        let unsafe_proof = provider.prove_call_site_update(&unsafe_request);
        assert!(unsafe_proof.candidate.is_none());
        assert_eq!(
            unsafe_proof.rejection_reasons,
            vec![NextEditRejectionReason::UnsafeInsertionPoint]
        );

        let invalid_target_proof = provider.prove_call_site_update(&invalid_target);
        assert!(invalid_target_proof.candidate.is_none());
        assert_eq!(
            invalid_target_proof.rejection_reasons,
            vec![NextEditRejectionReason::InvalidCallTarget]
        );

        let missing_argument_proof = provider.prove_call_site_update(&missing_argument);
        assert!(missing_argument_proof.candidate.is_none());
        assert_eq!(
            missing_argument_proof.rejection_reasons,
            vec![NextEditRejectionReason::MissingCallArgument]
        );
        Ok(())
    }

    #[test]
    fn call_site_update_receipt_does_not_bypass_runtime_gate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "my $user = build_user($name);\n";
        let mut request =
            CallSiteUpdateNextEditRequest::receipt_only(source, "build_user", "$age", 0);

        request.gate = NextEditFeatureGate::default();
        let disabled = provider.prove_call_site_update(&request);
        assert_eq!(disabled.status, NextEditStatus::Disabled);
        assert!(disabled.candidate.is_none());
        assert_eq!(disabled.rejection_reasons, vec![NextEditRejectionReason::GateDisabled]);

        request.gate = NextEditFeatureGate::explicit_enabled();
        let runtime = provider.prove_call_site_update(&request);
        assert_eq!(runtime.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(runtime.candidate.is_none());
        assert_eq!(
            runtime.rejection_reasons,
            vec![NextEditRejectionReason::RuntimeProviderNotRegistered]
        );
        Ok(())
    }

    #[test]
    fn rename_occurrence_receipt_prepares_next_safe_variable_edit()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "use strict;\nmy $new = compute();\nreturn $old + $old;\n";
        let cursor = source.find("return").ok_or("rename fixture missing return")?;
        let request = RenameOccurrenceNextEditRequest::receipt_only(source, "$old", "$new", cursor);

        let proof = provider.prove_rename_occurrence(&request);

        assert_eq!(proof.status, NextEditStatus::ReceiptOnly);
        assert!(proof.rejection_reasons.is_empty());
        let candidate = proof.candidate.ok_or("rename occurrence candidate not prepared")?;
        assert_eq!(candidate.family, NextEditCandidateFamily::RenameOccurrence);
        assert_eq!(candidate.original_symbol, "$old");
        assert_eq!(candidate.replacement_symbol, "$new");
        assert!(!candidate.editor_visible);
        assert_eq!(candidate.edit.new_text, "$new");
        let edited = candidate.edit.apply_to(source).ok_or("edit did not apply")?;
        assert_eq!(edited, "use strict;\nmy $new = compute();\nreturn $new + $old;\n");
        Ok(())
    }

    #[test]
    fn rename_occurrence_receipt_rejects_unsafe_and_missing_occurrences()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let unsafe_request = RenameOccurrenceNextEditRequest::receipt_only(
            "# rename $old here\n",
            "$old",
            "$new",
            0,
        );
        let missing_request = RenameOccurrenceNextEditRequest::receipt_only(
            "my $new = compute();\n",
            "$old",
            "$new",
            0,
        );

        let unsafe_proof = provider.prove_rename_occurrence(&unsafe_request);
        assert!(unsafe_proof.candidate.is_none());
        assert_eq!(
            unsafe_proof.rejection_reasons,
            vec![NextEditRejectionReason::UnsafeInsertionPoint]
        );

        let missing_proof = provider.prove_rename_occurrence(&missing_request);
        assert!(missing_proof.candidate.is_none());
        assert_eq!(
            missing_proof.rejection_reasons,
            vec![NextEditRejectionReason::MissingRenameOccurrence]
        );
        Ok(())
    }

    #[test]
    fn rename_occurrence_receipt_rejects_invalid_symbols_and_partial_matches()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let invalid_request = RenameOccurrenceNextEditRequest::receipt_only(
            "return $old;\n",
            "$old; system",
            "$new",
            0,
        );
        let partial_match_request =
            RenameOccurrenceNextEditRequest::receipt_only("return $older;\n", "$old", "$new", 0);

        let invalid_proof = provider.prove_rename_occurrence(&invalid_request);
        assert!(invalid_proof.candidate.is_none());
        assert_eq!(
            invalid_proof.rejection_reasons,
            vec![
                NextEditRejectionReason::InvalidRenameSymbol,
                NextEditRejectionReason::MissingRenameOccurrence,
            ]
        );

        let partial_match_proof = provider.prove_rename_occurrence(&partial_match_request);
        assert!(partial_match_proof.candidate.is_none());
        assert_eq!(
            partial_match_proof.rejection_reasons,
            vec![NextEditRejectionReason::MissingRenameOccurrence]
        );
        Ok(())
    }

    #[test]
    fn rename_occurrence_receipt_does_not_bypass_runtime_gate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = NextEditProvider;
        let source = "my $new = compute();\nreturn $old;\n";
        let mut request = RenameOccurrenceNextEditRequest::receipt_only(source, "$old", "$new", 0);

        request.gate = NextEditFeatureGate::default();
        let disabled = provider.prove_rename_occurrence(&request);
        assert_eq!(disabled.status, NextEditStatus::Disabled);
        assert!(disabled.candidate.is_none());
        assert_eq!(disabled.rejection_reasons, vec![NextEditRejectionReason::GateDisabled]);

        request.gate = NextEditFeatureGate::explicit_enabled();
        let runtime = provider.prove_rename_occurrence(&request);
        assert_eq!(runtime.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(runtime.candidate.is_none());
        assert_eq!(
            runtime.rejection_reasons,
            vec![NextEditRejectionReason::RuntimeProviderNotRegistered]
        );
        Ok(())
    }
}

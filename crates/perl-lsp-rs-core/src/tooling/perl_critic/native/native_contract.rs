//! Core native critic rule and diagnostic contracts.

use super::super::{CriticConfig, Severity, Violation};
use perl_parser_core::Node;
use perl_parser_core::position::Range;
use serde::{Deserialize, Serialize};

/// Native critic rule category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticCategory {
    /// Syntax-level policy that can be checked from tokens or AST shape.
    Syntax,
    /// Semantic policy that needs bindings, scopes, or inferred facts.
    Semantic,
    /// Workspace-aware policy that needs cross-file/module facts.
    Workspace,
    /// Style convention policy.
    Style,
    /// Maintainability or complexity policy.
    Maintainability,
    /// Security or unsafe-code policy.
    Security,
    /// Documentation or POD policy.
    Documentation,
}

/// Safety level for an optional native critic fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixSafety {
    /// Safe to apply automatically.
    Safe,
    /// Useful suggestion, but should require user confirmation.
    Suggested,
    /// Diagnostic-only guidance with no automatic edit.
    ManualOnly,
}

/// Text edit attached to a native critic fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticTextEdit {
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// Optional fix attached to a native critic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFix {
    /// Human-readable action title.
    pub title: String,
    /// How safe the edit is.
    pub safety: FixSafety,
    /// Edits needed to apply the fix.
    pub edits: Vec<CriticTextEdit>,
}

/// Related span for richer native critic diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticRelatedInformation {
    /// Related source range.
    pub range: Range,
    /// Explanation for the related span.
    pub message: String,
}

/// Native critic finding emitted by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFinding {
    /// Stable native rule ID, such as `native.variables.unused_lexical`.
    pub rule_id: String,
    /// Rule category.
    pub category: CriticCategory,
    /// Finding severity.
    pub severity: Severity,
    /// Precise source span for the finding.
    pub range: Range,
    /// Short diagnostic message.
    pub message: String,
    /// Longer explanation shown in editor/CI details.
    pub explanation: String,
    /// Suppression key accepted by inline or file-level suppression handling.
    pub suppression_key: String,
    /// Related spans, when useful.
    pub related: Vec<CriticRelatedInformation>,
    /// Optional fix.
    pub fix: Option<CriticFix>,
}

impl CriticFinding {
    /// Convert this native finding into the existing violation shape.
    ///
    /// This is the bridge that lets native rules flow through current LSP,
    /// execute-command, and diagnostic consumers while those consumers still
    /// expect `Violation` values.
    #[must_use]
    pub fn to_violation(&self, file: impl Into<String>) -> Violation {
        Violation {
            policy: self.rule_id.clone(),
            description: self.message.clone(),
            explanation: self.explanation.clone(),
            severity: self.severity,
            range: self.range,
            file: file.into(),
        }
    }
}

/// Native critic rule context.
pub struct CriticContext<'a> {
    /// Source text being analyzed.
    pub source: &'a str,
    /// Parsed AST for the source.
    pub ast: &'a Node,
    /// Critic configuration.
    pub config: &'a CriticConfig,
}

impl<'a> CriticContext<'a> {
    /// Build a native critic context.
    pub fn new(source: &'a str, ast: &'a Node, config: &'a CriticConfig) -> Self {
        Self { source, ast, config }
    }
}

/// Native critic rule interface.
pub trait CriticRule: Send + Sync {
    /// Stable native rule ID.
    fn id(&self) -> &'static str;

    /// Rule category.
    fn category(&self) -> CriticCategory;

    /// Default severity before profile/config remapping.
    fn default_severity(&self) -> Severity;

    /// Check the source and append findings.
    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>);
}

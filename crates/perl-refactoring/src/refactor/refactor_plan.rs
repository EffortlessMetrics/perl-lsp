//! Shared planning contract for refactoring operations.

use crate::workspace_refactor::FileEdit;
use serde::{Deserialize, Serialize};

/// Shared, operation-level refactoring plan emitted by analyzers before apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefactorPlan {
    /// Refactor operation kind.
    pub operation: RefactorOperationKind,
    /// Planned file edits.
    pub edits: Vec<FileEdit>,
    /// Non-fatal diagnostics discovered while planning.
    pub diagnostics: Vec<RefactorDiagnostic>,
    /// Confidence level for this plan.
    pub confidence: RefactorConfidence,
    /// Safety posture for apply-time behavior.
    pub safety: RefactorSafety,
    /// Operation statistics.
    pub stats: RefactorStats,
}

/// Refactor operation families supported by the planner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefactorOperationKind {
    /// Import optimization operation.
    OptimizeImports,
}

/// Safety level for a refactor plan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefactorSafety {
    /// Plan is safe for direct apply.
    Safe,
    /// Plan should be previewed before apply.
    NeedsPreview,
    /// Plan is blocked from apply.
    UnsafeBlocked,
}

/// Confidence level attached to a plan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefactorConfidence {
    /// Exact symbol/range matching.
    Exact,
    /// Scope-aware but not exact.
    ScopeAware,
    /// Heuristic-only inference.
    Heuristic,
}

/// Human-readable diagnostic for refactoring plans.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefactorDiagnostic {
    /// Machine-readable code.
    pub code: String,
    /// Diagnostic message.
    pub message: String,
}

/// Basic operation stats to keep planning measurable.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefactorStats {
    /// Number of changed files.
    pub files_changed: usize,
    /// Number of edits across all files.
    pub edits_count: usize,
}

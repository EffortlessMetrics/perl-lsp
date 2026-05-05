//! Shared refactor planning contract.

use crate::refactor::workspace_refactor::FileEdit;
use serde::{Deserialize, Serialize};

/// Internal operation kind used by the refactoring plan contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefactorOperationKind {
    /// Symbol rename operation.
    Rename,
}

/// Safety classification for a planned refactor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefactorSafety {
    /// Safe to apply directly.
    Safe,
    /// Must be reviewed in a preview flow before applying.
    NeedsPreview,
    /// Blocked from application.
    UnsafeBlocked,
}

/// Confidence classification for a planned refactor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefactorConfidence {
    /// Exact, index-backed transformation.
    Exact,
    /// Scope-aware but not fully exact.
    ScopeAware,
    /// Heuristic result.
    Heuristic,
}

/// Non-fatal planning/validation note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefactorDiagnostic {
    /// Free-form diagnostic message.
    pub message: String,
}

/// Shared statistics for plan-stage observability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RefactorStats {
    /// Number of files touched by the plan.
    pub files_changed: usize,
    /// Number of text edits in the plan.
    pub edit_count: usize,
}

/// A normalized internal plan for refactor operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefactorPlan {
    /// Operation being planned.
    pub operation: RefactorOperationKind,
    /// File-level edits for the operation.
    pub edits: Vec<FileEdit>,
    /// Diagnostics produced while analyzing or validating the plan.
    pub diagnostics: Vec<RefactorDiagnostic>,
    /// Confidence level for the plan.
    pub confidence: RefactorConfidence,
    /// Safety level for the plan.
    pub safety: RefactorSafety,
    /// Aggregate stats for metrics and scorecards.
    pub stats: RefactorStats,
}

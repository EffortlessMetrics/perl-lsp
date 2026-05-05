use crate::refactor::workspace_refactor::FileEdit;
use serde::{Deserialize, Serialize};

/// Operation kind for refactoring plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefactorOperationKind {
    /// Symbol rename operations.
    Rename,
    /// Import optimization operations.
    OptimizeImports,
    /// Module extraction operations.
    ExtractModule,
    /// Subroutine move operations.
    MoveSubroutine,
}

/// Safety level for a generated refactoring plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefactorSafety {
    /// Operation is safe to apply directly.
    Safe,
    /// Operation should be previewed before apply.
    NeedsPreview,
    /// Operation was blocked as unsafe.
    UnsafeBlocked,
}

/// Confidence level for the analysis producing a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefactorConfidence {
    /// Exact symbol/range evidence.
    Exact,
    /// Scope-aware but not exact identity.
    ScopeAware,
    /// Heuristic/textual matching.
    Heuristic,
}

/// Machine-readable diagnostic attached to a refactoring plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefactorDiagnostic {
    /// Diagnostic code for stable assertions in tests.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Lightweight statistics for a generated plan.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefactorStats {
    /// Count of files touched by the plan.
    pub files_touched: usize,
    /// Count of edits emitted by the plan.
    pub edits_emitted: usize,
}

/// Normalized internal refactoring contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefactorPlan {
    /// Operation that produced this plan.
    pub operation: RefactorOperationKind,
    /// File edits to apply for the operation.
    pub edits: Vec<FileEdit>,
    /// Non-fatal diagnostics generated during planning.
    pub diagnostics: Vec<RefactorDiagnostic>,
    /// Confidence of the generated edits.
    pub confidence: RefactorConfidence,
    /// Safety classification for the plan.
    pub safety: RefactorSafety,
    /// Emitted plan statistics.
    pub stats: RefactorStats,
}

impl RefactorPlan {
    /// Build a plan and derive simple stats from edits.
    pub fn with_edits(
        operation: RefactorOperationKind,
        edits: Vec<FileEdit>,
        diagnostics: Vec<RefactorDiagnostic>,
        confidence: RefactorConfidence,
        safety: RefactorSafety,
    ) -> Self {
        let edits_emitted = edits.iter().map(|f| f.edits.len()).sum();
        let stats = RefactorStats { files_touched: edits.len(), edits_emitted };
        Self { operation, edits, diagnostics, confidence, safety, stats }
    }
}

use crate::refactor::workspace_refactor::FileEdit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorOperationKind {
    Rename,
    ExtractModule,
    MoveSubroutine,
    InlineVariable,
    OptimizeImports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorSafety {
    Safe,
    NeedsPreview,
    UnsafeBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorConfidence {
    Exact,
    ScopeAware,
    Heuristic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorDiagnostic {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefactorStats {
    pub files_changed: usize,
    pub edits: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorPlan {
    pub operation: RefactorOperationKind,
    pub edits: Vec<FileEdit>,
    pub diagnostics: Vec<RefactorDiagnostic>,
    pub confidence: RefactorConfidence,
    pub safety: RefactorSafety,
    pub stats: RefactorStats,
}

impl RefactorPlan {
    pub fn new(operation: RefactorOperationKind, edits: Vec<FileEdit>) -> Self {
        let edits_count = edits.iter().map(|file_edit| file_edit.edits.len()).sum();
        Self {
            operation,
            stats: RefactorStats { files_changed: edits.len(), edits: edits_count },
            edits,
            diagnostics: vec![],
            confidence: RefactorConfidence::Exact,
            safety: RefactorSafety::Safe,
        }
    }
}

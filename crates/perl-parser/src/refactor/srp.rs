//! SRP-oriented facades over refactoring capabilities.

/// Import analysis and organization APIs.
pub mod imports {
    pub use crate::refactor::import_optimizer::{
        DuplicateImport, ImportAnalysis, ImportEntry, ImportOptimizer, MissingImport,
        OrganizationSuggestion, SuggestionPriority, UnusedImport,
    };
}

/// Core refactoring engine APIs.
pub mod engine {
    pub use crate::refactor::refactoring::{
        ModernizationPattern, RefactoringConfig, RefactoringEngine, RefactoringOperation,
        RefactoringResult, RefactoringScope, RefactoringType,
    };
}

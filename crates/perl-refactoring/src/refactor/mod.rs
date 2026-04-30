//! Refactoring and modernization helpers.

pub mod import_optimizer;
pub mod inline;
pub mod modernize;
pub mod modernize_refactored;
pub mod module_move_imports;
pub mod refactoring;
pub mod workspace_refactor;
pub mod workspace_rename;

#[cfg(test)]
mod scoped_rename_tests;

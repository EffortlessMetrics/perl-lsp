//! Refactoring and modernization helpers.

pub mod import_optimizer;
pub mod modernize;
pub mod modernize_refactored;
pub mod refactoring;
pub mod workspace_refactor;
pub mod workspace_rename;

#[cfg(test)]
mod scoped_rename_tests;

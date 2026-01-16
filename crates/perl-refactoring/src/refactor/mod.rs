//! Refactoring and modernization helpers.

pub mod import_optimizer;
/// Code modernization utilities for Perl best practices.
pub mod modernize;
/// Enhanced code modernization with refactoring capabilities.
pub mod modernize_refactored;
pub mod refactoring;
#[cfg(not(target_arch = "wasm32"))]
pub mod workspace_refactor;

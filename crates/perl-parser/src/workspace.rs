//! Compatibility re-export of workspace indexing modules.

#[cfg(not(target_arch = "wasm32"))]
pub use crate::refactor::workspace_refactor;
/// Workspace indexing and refactoring orchestration.
pub use perl_workspace::workspace::*;

//! Deprecated rename compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::rename`.
//! Prefer `perl_lsp_providers::rename` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: Source code parsed into AST
//! 2. **Index**: Symbol tables built for cross-file references
//! 3. **Navigate**: Find all references for rename targets
//! 4. **Complete**: Context-aware rename suggestions
//! 5. **Analyze**: Safe rename with workspace-wide updates (this module)
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.rename` client capabilities
//!   including prepare support, default behavior, and position encoding.
//! - **Protocol compliance**: Implements `textDocument/rename` request from
//!   LSP 3.17 specification with support for prepareRename.
//!
//! # Usage Examples
//!
//! ```rust
//! // Use the new module path for new code:
//! use perl_lsp_providers::rename::RenameProvider;
//!
//! // Legacy path still works but is deprecated:
//! use perl_lsp_providers::ide::lsp_compat::rename::RenameProvider;
//! ```

pub use perl_lsp_rename::*;

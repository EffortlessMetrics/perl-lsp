//! Deprecated code actions compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::code_actions`.
//! Prefer `perl_lsp_providers::code_actions` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: AST generation with pattern detection for actionable constructs
//! 2. **Index**: Workspace symbol table for cross-file code actions
//! 3. **Navigate**: Reference finding for refactoring actions
//! 4. **Complete**: Context-aware action suggestions
//! 5. **Analyze**: Code action generation and execution (this module)
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.codeAction` client capabilities
//!   including code action literals, disabled support, and resolve support.
//! - **Protocol compliance**: Implements `textDocument/codeAction` request from
//!   LSP 3.17 specification with support for code action resolve.
//!
//! # Usage Examples
//!
//! ```rust
//! // Use the new module path for new code:
//! use perl_lsp_providers::code_actions::CodeActionProvider;
//!
//! // Legacy path still works but is deprecated:
//! use perl_lsp_providers::ide::lsp_compat::code_actions::CodeActionProvider;
//! ```
//!
//! # See Also
//!
//! - [`crate::ide::lsp_compat::completion`] for completion support
//! - [`crate::ide::lsp_compat::diagnostics`] for diagnostic publishing
//! - [`perl_lsp_code_actions::CodeActionProvider`] for the main implementation

pub use perl_lsp_code_actions::*;

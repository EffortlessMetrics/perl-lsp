//! Deprecated completion compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::completion`.
//! Prefer `perl_lsp_providers::completion` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: Source code parsed into AST
//! 2. **Index**: Symbols extracted and indexed
//! 3. **Navigate**: Cross-file symbol resolution
//! 4. **Complete**: Context-aware completion suggestions (this module)
//! 5. **Analyze**: Semantic analysis and refactoring
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.completion` client capabilities
//!   including snippet support, commit characters, and completion item kinds.
//! - **Protocol compliance**: Implements `textDocument/completion` request from
//!   LSP 3.17 specification with support for completion item resolve.
//!
//! # Usage Examples
//!
//! ```rust
//! // Use the new module path for new code:
//! use perl_lsp_providers::completion::CompletionProvider;
//!
//! // Legacy path still works but is deprecated:
//! use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
//! ```

pub use perl_lsp_completion::*;

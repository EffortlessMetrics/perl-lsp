//! Deprecated inlay hints compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::inlay_hints`.
//! Prefer `perl_lsp_providers::inlay_hints` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: AST generation with type inference points
//! 2. **Index**: Symbol table for type information lookup
//! 3. **Navigate**: Cross-file type resolution
//! 4. **Complete**: Type-aware completion suggestions
//! 5. **Analyze**: Inlay hint generation for types and parameters (this module)
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.inlayHint` client capabilities
//!   including resolve support, tooltip rendering, and text document manipulation.
//! - **Protocol compliance**: Implements `textDocument/inlayHint` request from
//!   LSP 3.17 specification with support for inlay hint resolve.
//!
//! # Usage Examples
//!
//! ```rust
//! // Use the new module path for new code:
//! use perl_lsp_providers::inlay_hints::InlayHintsProvider;
//!
//! // Legacy path still works but is deprecated:
//! use perl_lsp_providers::ide::lsp_compat::inlay_hints::InlayHintsProvider;
//! ```

pub use perl_lsp_inlay_hints::*;

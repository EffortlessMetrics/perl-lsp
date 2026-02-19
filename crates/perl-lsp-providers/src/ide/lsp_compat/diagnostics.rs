//! Deprecated diagnostics compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::diagnostics`.
//! Prefer `perl_lsp_providers::diagnostics` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: Source code parsed into AST with error recovery
//! 2. **Index**: Symbols and issues extracted for workspace analysis
//! 3. **Navigate**: Cross-file navigation for related diagnostics
//! 4. **Complete**: Completion with diagnostic context awareness
//! 5. **Analyze**: Diagnostic generation and reporting (this module)
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.publishDiagnostics` client
//!   capabilities including related information, tag support, and code actions.
//! - **Protocol compliance**: Implements diagnostic notification flow from
//!   LSP 3.17 specification with support for diagnostic codes and severity.
//!
//! # Usage Examples
//!
//! ```rust
//! // Use the new module path for new code:
//! use perl_lsp_providers::diagnostics::DiagnosticProvider;
//!
//! // Legacy path still works but is deprecated:
//! use perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticProvider;
//! ```

pub use perl_lsp_diagnostics::*;

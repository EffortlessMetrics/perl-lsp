//! Server and document state management
//!
//! This module manages the stateful aspects of the LSP server:
//! - Document content and AST caching
//! - Server configuration
//! - Cancellation tracking
//! - Resource limits and bounded behavior

mod config;
mod document;

pub use config::*;
pub use document::*;
pub use perl_lsp_rs_core::runtime::limits::*;

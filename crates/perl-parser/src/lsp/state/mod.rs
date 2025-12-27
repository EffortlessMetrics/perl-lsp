//! Server and document state management
//!
//! This module manages the stateful aspects of the LSP server:
//! - Document content and AST caching
//! - Server configuration
//! - Cancellation tracking

mod config;
mod document;

pub use config::*;
pub use document::*;

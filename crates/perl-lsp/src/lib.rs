//! Perl LSP Runtime Crate
//!
//! This crate provides the runtime implementation for the Perl Language Server Protocol (LSP).
//! It handles protocol communication, message framing, server state management, and LSP feature
//! dispatching.
//!
//! # Architecture
//!
//! The runtime is organized into the following modules:
//!
//! - [`protocol`] - JSON-RPC message types and protocol definitions
//! - [`transport`] - Message framing and transport layer (stdio, TCP, etc.)
//! - [`state`] - Document and server state management
//! - [`runtime`] - Core server implementation and lifecycle management
//! - [`features`] - LSP feature providers (completion, hover, definitions, etc.)
//! - [`convert`] - Conversions between engine types and lsp_types
//! - [`util`] - URI handling, UTF-16 conversion, and other utilities
//! - [`fallback`] - Text-based fallback implementations when parsing fails
//! - [`handlers`] - LSP request/notification handlers
//! - [`dispatch`] - Request routing and dispatch logic
//! - [`server`] - Public server interface
//!
//! # Usage
//!
//! The primary entry point is [`run_stdio()`], which starts the LSP server in stdio mode:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! perl_lsp::run_stdio()?;
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow until migration is complete

// Module declarations - migrated from perl-parser
pub mod protocol;
pub mod transport;
pub mod state;
pub mod runtime;
pub mod features;
pub mod convert;
pub mod util;
pub mod fallback;
pub mod handlers;
pub mod dispatch;
pub mod server;

// Re-exports for key types
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::LspServer;

// =============================================================================
// Internal compatibility re-exports (crate-internal, not API surface)
// =============================================================================
// These re-exports allow migrated code to use `crate::...` paths for engine
// pieces while we incrementally update paths to `perl_parser::...`

/// Parser re-export for migrated code
pub(crate) use perl_parser::Parser;

/// Position utilities re-export
pub(crate) mod position {
    pub use perl_parser::position::*;
}

/// Positions module re-export (LSP-style line/character based)
pub(crate) mod positions {
    pub use perl_parser::positions::*;
}

/// Declaration types re-export
pub(crate) mod declaration {
    pub use perl_parser::declaration::*;
}

/// Workspace index re-export
pub(crate) mod workspace_index {
    pub use perl_parser::workspace_index::*;
}

/// Symbol types re-export
pub(crate) mod symbol {
    pub use perl_parser::symbol::*;
}

/// AST types re-export
pub(crate) mod ast {
    pub use perl_parser::ast::*;
}

/// Feature re-exports for old intra-crate paths
pub(crate) mod code_actions_enhanced {
    pub use crate::features::code_actions_enhanced::*;
}

pub(crate) mod code_lens_provider {
    pub use crate::features::code_lens_provider::*;
}

pub(crate) mod diagnostics {
    #[allow(unused_imports)]
    pub use crate::features::diagnostics::*;
}

// More feature re-exports for runtime imports
pub(crate) mod inlay_hints {
    pub use crate::features::inlay_hints::*;
}

pub(crate) mod document_links {
    pub use crate::features::document_links::*;
}

pub(crate) mod lsp_document_link {
    pub use crate::features::lsp_document_link::*;
}

pub(crate) mod selection_range {
    pub use crate::features::selection_range::*;
}

pub(crate) mod linked_editing {
    pub use crate::features::linked_editing::*;
}

pub(crate) mod code_actions_pragmas {
    pub use crate::features::code_actions_pragmas::*;
}

// Engine re-exports for runtime
pub(crate) mod perl_critic {
    pub use perl_parser::perl_critic::*;
}

pub(crate) mod semantic {
    pub use perl_parser::semantic::*;
}

pub(crate) mod error {
    pub use perl_parser::error::*;
}

pub(crate) mod completion {
    pub use crate::features::completion::*;
}

pub(crate) mod on_type_formatting {
    pub use crate::features::on_type_formatting::*;
}

pub(crate) mod inline_completions {
    pub use crate::features::inline_completions::*;
}

pub(crate) mod formatting {
    pub use crate::features::formatting::*;
}

pub(crate) mod type_hierarchy {
    pub use crate::features::type_hierarchy::*;
}

// Re-export SourceLocation at crate root for convenience
pub(crate) use perl_parser::ast::SourceLocation;

// Engine modules needed by runtime
pub(crate) mod type_inference {
    pub use perl_parser::type_inference::*;
}

pub(crate) mod builtin_signatures {
    pub use perl_parser::builtin_signatures::*;
}

pub(crate) mod workspace_rename {
    pub use crate::features::workspace_rename::*;
}

pub(crate) mod semantic_tokens {
    pub use crate::features::semantic_tokens::*;
}

pub(crate) mod call_hierarchy_provider {
    pub use perl_parser::call_hierarchy_provider::*;
}

// Parser module re-export for tests using crate::parser::Parser
pub(crate) mod parser {
    #[allow(unused_imports)]
    pub use perl_parser::parser::*;
}

// Folding re-export
pub(crate) mod folding {
    pub use crate::features::folding::*;
}

// References re-export
pub(crate) mod references {
    #[allow(unused_imports)]
    pub use crate::features::references::*;
}

// Rename re-export
pub(crate) mod rename {
    #[allow(unused_imports)]
    pub use crate::features::rename::*;
}

// Signature help re-export
pub(crate) mod signature_help {
    #[allow(unused_imports)]
    pub use crate::features::signature_help::*;
}

/// Run the LSP server in stdio mode.
///
/// This is the main entry point for the LSP server. It reads JSON-RPC messages from stdin
/// and writes responses to stdout, following the Language Server Protocol specification.
///
/// # Errors
///
/// Returns an error if:
/// - The transport layer fails to initialize
/// - Message framing or parsing fails
/// - The server encounters an unrecoverable error
///
/// # Example
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// perl_lsp::run_stdio()?;
/// # Ok(())
/// # }
/// ```
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();
    server.run().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

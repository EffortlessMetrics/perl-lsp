//! LSP semantic tokens provider for Perl
//!
//! This crate provides semantic token generation for syntax highlighting.
//!
//! ## Features
//!
//! - Token generation from AST
//! - LSP protocol compatibility
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_semantic_tokens::collect_semantic_tokens;
//!
//! let tokens = collect_semantic_tokens(&ast, source, &to_pos16);
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod semantic_tokens;

pub use semantic_tokens::{EncodedToken, TokensLegend, collect_semantic_tokens, legend};

/// Semantic tokens provider for LSP
///
/// This is a placeholder for future enhancement. Currently, semantic tokens
/// are generated using the functional `collect_semantic_tokens` API.
pub struct SemanticTokensProvider;

impl SemanticTokensProvider {
    /// Create a new semantic tokens provider
    pub fn new() -> Self {
        Self
    }
}

impl Default for SemanticTokensProvider {
    fn default() -> Self {
        Self::new()
    }
}

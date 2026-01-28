//! LSP inlay hints provider for Perl
//!
//! This crate provides inlay hint generation for type information.
//!
//! ## Features
//!
//! - Type inference
//! - Parameter hints
//! - LSP protocol compatibility
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_inlay_hints::InlayHintsProvider;
//!
//! let provider = InlayHintsProvider::new();
//! let hints = provider.generate_hints(&ast, source, &symbol_table)?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod inlay_hints;

pub use inlay_hints::{InlayHintsProvider, InlayHint, InlayHintKind};

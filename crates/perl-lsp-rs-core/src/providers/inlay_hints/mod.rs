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
//! ```rust,ignore
//! use perl_lsp_inlay_hints::InlayHintsProvider;
//!
//! let provider = InlayHintsProvider::new();
//! let hints = provider.generate_hints(&ast, source, &symbol_table)?;
//! ```

mod impl_inlay_hints;

pub use impl_inlay_hints::{
    InlayHint, InlayHintKind, InlayHintsProvider, extract_param_names, parameter_hints,
    trivial_type_hints,
};

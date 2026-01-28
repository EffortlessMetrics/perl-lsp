//! LSP rename provider for Perl
//!
//! This crate provides symbol renaming functionality.
//!
//! ## Features
//!
//! - Symbol rename
//! - Cross-file references
//! - Workspace integration
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_rename::RenameProvider;
//!
//! let provider = RenameProvider::new(&ast, source.to_string());
//! let edit = provider.prepare_rename(position)?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod rename;

pub use rename::{RenameOptions, RenameProvider, RenameResult, TextEdit};

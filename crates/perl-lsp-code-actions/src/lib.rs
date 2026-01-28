//! LSP code actions provider for Perl
//!
//! This crate provides code action functionality for Perl.
//!
//! ## Features
//!
//! - Quick fixes for common mistakes
//! - Refactoring operations
//! - Enhanced actions (extract variable/subroutine, import management)
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_code_actions::CodeActionsProvider;
//!
//! let provider = CodeActionsProvider::new();
//! let actions = provider.get_code_actions(&ast, source, range, Some(&workspace_index))?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod code_actions;

pub use code_actions::{CodeActionsProvider, CodeAction, CodeActionKind};

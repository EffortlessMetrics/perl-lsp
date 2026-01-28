//! LSP completion provider for Perl
//!
//! This crate provides code completion functionality for Perl.
//!
//! ## Features
//!
//! - Context-aware completion
//! - Multiple completion sources (builtins, functions, variables, etc.)
//! - Workspace integration
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_completion::CompletionProvider;
//!
//! let provider = CompletionProvider::new(&ast, Some(&workspace_index))?;
//! let completions = provider.get_completions(source, position)?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod completion;

pub use completion::{CompletionContext, CompletionItem, CompletionItemKind, CompletionProvider};

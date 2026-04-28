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
//! ```rust,ignore
//! use perl_lsp_completion::CompletionProvider;
//!
//! let provider = CompletionProvider::new(&ast, Some(&workspace_index))?;
//! let completions = provider.get_completions(source, position)?;
//! ```

#[allow(clippy::module_inception)]
mod completion;

pub use completion::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionProvider,
    add_xs_api_completions_for_prefix, get_dbi_method_documentation, get_test_more_documentation,
    get_xs_api_documentation, is_xs_source,
};

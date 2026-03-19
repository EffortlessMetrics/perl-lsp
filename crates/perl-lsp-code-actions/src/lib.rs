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
//! ```rust,ignore
//! use perl_lsp_code_actions::CodeActionsProvider;
//!
//! let source = String::from("my $x = 1;");
//! let provider = CodeActionsProvider::new(source);
//! let actions = provider.get_code_actions(&ast, (0, 10), &diagnostics);
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod code_actions;
mod enhanced;
/// Error-to-action mapping for common Perl mistakes
pub mod error_to_action;
mod quick_fixes;
mod refactors;
mod types;

pub use code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
pub use enhanced::EnhancedCodeActionsProvider;
pub use error_to_action::{actions_for_builtin_typo, actions_for_error_diagnostic};
pub use types::CodeActionEdit;

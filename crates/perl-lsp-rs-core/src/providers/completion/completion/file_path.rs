//! File path completion compatibility facade.
//!
//! The implementation now lives in the `perl-lsp-file-completion` microcrate.

pub use crate::providers::file_completion::{FileCompletionContext, complete_file_paths};

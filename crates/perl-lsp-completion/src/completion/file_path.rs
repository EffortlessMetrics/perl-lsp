//! File path completion compatibility facade.
//!
//! The implementation now lives in the `perl-lsp-file-completion` microcrate.

pub use perl_lsp_rs_core::providers::file_completion::{
    FileCompletionContext, complete_file_paths,
};

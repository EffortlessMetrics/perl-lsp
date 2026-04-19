//! Folding range extraction compatibility shim.
//!
//! The implementation now lives in the `perl-lsp-folding` microcrate.

pub use perl_lsp_rs_core::providers::folding::{
    FoldingRange, FoldingRangeExtractor, FoldingRangeKind,
};

//! Document color support for Perl LSP
//!
//! Re-exports from the `perl-lsp-color-provider` microcrate.

pub(crate) use perl_lsp_rs_core::providers::color::{Color, color_to_presentations, detect_colors};

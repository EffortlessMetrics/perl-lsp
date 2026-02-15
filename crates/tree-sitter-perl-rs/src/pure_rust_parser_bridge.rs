//! Bridge module that exposes the v2 Pest parser from `perl-parser-pest`.
//!
//! This keeps the existing `tree_sitter_perl::pure_rust_parser` module path
//! stable while moving implementation ownership to a dedicated microcrate.

pub use perl_parser_pest::pure_rust_parser::*;

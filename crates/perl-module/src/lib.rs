//! Unified Perl module facade.
//!
//! This crate absorbs 13 `perl-module-*` microcrates into a single published
//! facade with internal module folders.

// Internal modules (pub so sub-modules can be accessed from outside;
// items within each mod default to pub for cross-module reuse,
// but the public contract is only what api.rs re-exports).
pub mod boundary;
pub mod import;
pub mod import_match;
pub mod name;
pub mod path;
pub mod reference;
pub mod rename;
pub mod resolution;
pub mod token;
pub mod token_core;
pub mod token_parser;

pub mod api;
pub use api::*;

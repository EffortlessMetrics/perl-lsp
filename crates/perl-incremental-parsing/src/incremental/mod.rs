//! Compatibility shim for incremental parsing APIs.
//!
//! `perl-parser` is the single source of truth for incremental parsing logic.
//! This module re-exports that implementation to preserve legacy import paths.

#![allow(missing_docs)]

#[allow(deprecated)]
pub use perl_parser::incremental::*;

//! Deprecated compatibility module forwarding to `perl_parser::incremental`.
//!
//! `perl-parser` owns incremental parsing implementation. This module intentionally
//! contains no independent logic to prevent drift.

#![allow(missing_docs)]

pub use perl_parser::incremental::*;

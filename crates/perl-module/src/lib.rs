//! Unified Perl module facade.
//!
//! This crate absorbs 13 `perl-module-*` microcrates into a single published
//! facade with internal module folders.

/// Boundary-facing helpers shared across module resolution flows.
pub mod boundary;
/// `use`/`require` import modeling and utilities.
pub mod import;
/// Import matching and candidate filtering.
pub mod import_match;
/// Canonical Perl module name representation and parsing.
pub mod name;
/// Filesystem path derivation and normalization for modules.
pub mod path;
/// Symbol reference types used by resolver and index layers.
pub mod reference;
/// Safe module/file rename primitives.
pub mod rename;
/// Core module resolution pipeline and result types.
pub mod resolution;
/// Token-facing adapters used by module parsing.
pub mod token;
/// Shared token model primitives and low-level helpers.
pub mod token_core;
/// Lightweight token parser for module-related syntax.
pub mod token_parser;

/// Stable facade exports for external consumers.
pub mod api;
pub use api::*;

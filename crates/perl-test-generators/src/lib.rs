//! Property-based test generators for Perl domain objects.
//!
//! This crate provides reusable [`proptest::Strategy`] implementations for
//! generating Perl variables, module paths, code snippets, and Unicode
//! strings suitable for fuzzing parsers and LSP components.
//!
//! # Example
//!
//! ```rust
//! use proptest::prelude::*;
//! use perl_test_generators::{variable, module_path, unicode_string};
//!
//! proptest! {
//!     #[test]
//!     fn vars_parse(sym in variable()) {
//!         assert!(sym.starts_with('$') || sym.starts_with('@') || sym.starts_with('%'));
//!     }
//! }
//! ```

mod module;
mod unicode;
mod variable;

pub use module::{module_path, module_path_segments};
pub use unicode::unicode_string;
pub use variable::variable;

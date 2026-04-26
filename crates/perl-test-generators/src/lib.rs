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
//! use perl_test_generators::{variable, module_path, non_empty_unicode_string};
//!
//! proptest! {
//!     #[test]
//!     fn vars_parse(sym in variable()) {
//!         assert!(sym.starts_with('$') || sym.starts_with('@') || sym.starts_with('%'));
//!     }
//!
//!     #[test]
//!     fn non_empty_text(text in non_empty_unicode_string()) {
//!         assert!(!text.is_empty());
//!     }
//! }
//! ```

mod module;
mod unicode;
mod variable;

pub use module::{module_path, module_path_segments};
pub use unicode::{non_empty_unicode_string, unicode_string};
pub use variable::variable;

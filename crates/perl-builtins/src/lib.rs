//! Builtin function signatures and metadata for Perl.
//!
//! Provides [`BuiltinSignature`](builtin_signatures::BuiltinSignature) entries
//! covering Perl's built-in functions, including signature variants and
//! documentation strings. Used by the LSP completion, hover, and signature-help
//! providers to surface accurate information without an external Perl runtime.

pub mod builtin_signatures;
pub mod builtin_signatures_phf;

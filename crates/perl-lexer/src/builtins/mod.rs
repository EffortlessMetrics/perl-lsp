//! Builtin function signatures and metadata for Perl.
//!
//! Provides [`BuiltinSignature`](builtin_signatures::BuiltinSignature) entries
//! covering Perl's built-in functions, including signature variants and
//! documentation strings. Used by the LSP completion, hover, and signature-help
//! providers to surface accurate information without an external Perl runtime.

pub mod builtin_signatures;
pub mod phf_lookup;

// Preserve legacy public path `perl_builtins::builtin_signatures_phf::*`
// (which is now accessible via `perl_lexer::builtins::builtin_signatures_phf::*`)
pub use phf_lookup as builtin_signatures_phf;

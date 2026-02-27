//! Deterministic and secure Perl module resolution helpers.
//!
//! This crate combines URI and filesystem module resolution strategies. Filesystem
//! lookup is delegated to [`perl_module_resolution_path`](crate::resolve_module_path)
//! to preserve a strict single-responsibility boundary for path handling.

pub use perl_module_resolution_path::resolve_module_path;
pub use perl_module_resolution_uri::{ModuleUriResolution, resolve_module_uri};

//! Deterministic and secure Perl module resolution helpers.
//!
//! This crate combines URI and filesystem module resolution strategies. Filesystem
//! lookup is delegated to [`perl_module_resolution_path`](crate::resolve_module_path)
//! to preserve a strict single-responsibility boundary for path handling.
//!
//! The [`use_lib`] module extracts additional include paths from `use lib` pragmas
//! and `FindBin` patterns in Perl source text.

pub mod use_lib;

pub use perl_module_resolution_path::resolve_module_path;
pub use perl_module_resolution_uri::{
    IncRoot, IncRootKind, ModuleUriResolution, resolve_module_uri,
    resolve_module_uri_with_effective_inc,
};

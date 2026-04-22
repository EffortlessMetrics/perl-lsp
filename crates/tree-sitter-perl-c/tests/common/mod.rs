//! Shared test utilities for tree-sitter-perl-c integration tests.
//!
//! This module provides common helpers used across multiple test files,
//! eliminating duplication and ensuring consistent test infrastructure.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tree_sitter_perl_c::parse_perl_code;

/// Parses Perl code and returns the S-expression representation of the root node.
///
/// # Errors
///
/// Returns an error if parsing fails or the tree cannot be converted to sexp.
pub fn sexp(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(parse_perl_code(code)?.root_node().to_sexp())
}

/// Creates a unique temporary file containing Perl source code.
///
/// The file is placed in the system temp directory with a unique name
/// based on the current timestamp in nanoseconds.
pub fn temp_perl_file(name: &str, content: &str) -> PathBuf {
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0_u128, |duration| duration.as_nanos());

    let path = std::env::temp_dir().join(format!("tree_sitter_perl_c_{name}_{nanos}.pl"));
    let _ = std::fs::write(&path, content);
    path
}

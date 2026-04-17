//! Public tree-sitter query helpers for the Perl C grammar.
//!
//! This module exposes the upstream tree-sitter-perl query files as compile-time
//! strings and provides typed loaders that validate them against the grammar.
//!
//! # Example
//!
//! ```rust
//! use tree_sitter_perl_c::queries;
//!
//! // Get the raw query string
//! let injections = queries::injections_query_str();
//! assert!(injections.contains("injection"));
//!
//! // Load and use a validated Query object
//! let query = queries::load_injections_query().unwrap();
//! assert!(!query.capture_names().is_empty());
//! ```

use tree_sitter::Query;

// Re-export QueryError so callers can handle load failures
pub use tree_sitter::QueryError;

/// Returns the `injections.scm` query string from the upstream tree-sitter-perl grammar.
pub fn injections_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/injections.scm")
}

/// Returns the `highlights.scm` query string from the upstream tree-sitter-perl grammar.
pub fn highlights_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/highlights.scm")
}

/// Returns the `folds.scm` query string from the upstream tree-sitter-perl grammar.
pub fn folds_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/folds.scm")
}

/// Returns the `matchup.scm` query string from the upstream tree-sitter-perl grammar.
pub fn matchup_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/matchup.scm")
}

/// Loads and validates the `injections.scm` query against the Perl grammar.
pub fn load_injections_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), injections_query_str())
}

/// Loads and validates the `highlights.scm` query against the Perl grammar.
pub fn load_highlights_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), highlights_query_str())
}

/// Loads and validates the `folds.scm` query against the Perl grammar.
pub fn load_folds_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), folds_query_str())
}

/// Loads and validates the `matchup.scm` query against the Perl grammar.
pub fn load_matchup_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), matchup_query_str())
}

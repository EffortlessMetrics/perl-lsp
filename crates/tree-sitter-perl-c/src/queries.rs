//! Public query helpers for tree-sitter Perl queries.
//!
//! This module exposes the four upstream tree-sitter query files from
//! `tree-sitter-perl/queries/` via two function types:
//!
//! - **Raw string accessors** (`*_query_str()`) — return `&'static str` for callers
//!   who want to construct [`tree_sitter::Query`] themselves
//! - **Convenience constructors** (`load_*_query()`) — return `Result<Query, QueryError>`
//!   and wrap the `Query::new(&language(), QUERY_STR)?` pattern
//!
//! # Example
//!
//! ```rust
//! use tree_sitter_perl_c::queries;
//!
//! // Option 1: Get the raw string
//! let injections_str = queries::injections_query_str();
//!
//! // Option 2: Get a pre-constructed Query
//! let injections_query = queries::load_injections_query().unwrap();
//! ```
//!
//! # Query Files
//!
//! | File | Purpose |
//! |------|---------|
//! | `injections.scm` | Language injection rules (e.g., embedded C in Inline::C heredocs) |
//! | `highlights.scm` | Syntax highlighting captures |
//! | `folds.scm` | Code folding markers |
//! | `matchup.scm` | Tag matching for navigation |

use tree_sitter::Query;

pub use tree_sitter::QueryError;

/// Returns the raw content of `injections.scm`.
///
/// Use this when you need to construct a [`Query`] yourself or pass the query
/// string to external tooling.
///
/// # See Also
///
/// - [`load_injections_query()`] for a pre-constructed [`Query`]
/// - [`crate::language()`] for the Perl language needed to create a [`Query`]
pub fn injections_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/injections.scm")
}

/// Returns the raw content of `highlights.scm`.
///
/// Use this when you need to construct a [`Query`] yourself or pass the query
/// string to external tooling.
///
/// # See Also
///
/// - [`load_highlights_query()`] for a pre-constructed [`Query`]
/// - [`crate::language()`] for the Perl language needed to create a [`Query`]
pub fn highlights_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/highlights.scm")
}

/// Returns the raw content of `folds.scm`.
///
/// Use this when you need to construct a [`Query`] yourself or pass the query
/// string to external tooling.
///
/// # See Also
///
/// - [`load_folds_query()`] for a pre-constructed [`Query`]
/// - [`crate::language()`] for the Perl language needed to create a [`Query`]
pub fn folds_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/folds.scm")
}

/// Returns the raw content of `matchup.scm`.
///
/// Use this when you need to construct a [`Query`] yourself or pass the query
/// string to external tooling.
///
/// # See Also
///
/// - [`load_matchup_query()`] for a pre-constructed [`Query`]
/// - [`crate::language()`] for the Perl language needed to create a [`Query`]
pub fn matchup_query_str() -> &'static str {
    include_str!("../../../tree-sitter-perl/queries/matchup.scm")
}

/// Constructs an injection [`Query`] from `injections.scm`.
///
/// This is a convenience function that combines [`crate::language()`] and
/// [`Query::new()`] to return a ready-to-use query object.
///
/// # Errors
///
/// Returns [`QueryError`] if the query string contains invalid syntax for the
/// current language version.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_c::queries;
///
/// let query = queries::load_injections_query().unwrap();
/// let capture_names: Vec<&str> = query.capture_names().iter().copied().collect();
/// ```
pub fn load_injections_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), injections_query_str())
}

/// Constructs a highlights [`Query`] from `highlights.scm`.
///
/// This is a convenience function that combines [`crate::language()`] and
/// [`Query::new()`] to return a ready-to-use query object.
///
/// # Errors
///
/// Returns [`QueryError`] if the query string contains invalid syntax for the
/// current language version.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_c::queries;
///
/// let query = queries::load_highlights_query().unwrap();
/// let capture_names: Vec<&str> = query.capture_names().iter().copied().collect();
/// ```
pub fn load_highlights_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), highlights_query_str())
}

/// Constructs a folds [`Query`] from `folds.scm`.
///
/// This is a convenience function that combines [`crate::language()`] and
/// [`Query::new()`] to return a ready-to-use query object.
///
/// # Errors
///
/// Returns [`QueryError`] if the query string contains invalid syntax for the
/// current language version.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_c::queries;
///
/// let query = queries::load_folds_query().unwrap();
/// ```
pub fn load_folds_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), folds_query_str())
}

/// Constructs a matchup [`Query`] from `matchup.scm`.
///
/// This is a convenience function that combines [`crate::language()`] and
/// [`Query::new()`] to return a ready-to-use query object.
///
/// # Errors
///
/// Returns [`QueryError`] if the query string contains invalid syntax for the
/// current language version.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_c::queries;
///
/// let query = queries::load_matchup_query().unwrap();
/// ```
pub fn load_matchup_query() -> Result<Query, QueryError> {
    Query::new(&super::language(), matchup_query_str())
}

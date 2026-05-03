//! Semantic substrate module tree for workspace-level semantic analysis.
//!
//! This module groups the canonical semantic population, indexing, and query
//! infrastructure that replaces per-provider semantic approximations with a
//! single layered substrate.
//!
//! # Submodules
//!
//! - [`facts`] — canonical `FileFactShard` population from semantic fact producers.

/// Canonical `FileFactShard` population from semantic fact producers.
pub mod facts;

/// Typed reference index for cross-file reference lookups.
pub mod references;

/// Import/export index for cross-file import and export lookups.
pub mod imports;

/// Scorecard aggregation for semantic shadow-compare receipts.
pub mod scorecard;

/// Visibility resolution for symbols at a given query point.
pub mod visibility;

/// Package graph index for cross-file inheritance and role-composition lookups.
pub mod package_graph;

/// Value-shape index mapping entity IDs to lightweight type approximations.
pub mod value_shape;

/// Per-category semantic fact invalidation planning.
pub mod invalidation;

/// Semantic query facade: `SemanticQueries` trait and `WorkspaceSemanticQueries` impl.
pub mod queries;

/// Literal-eval sub extractor for dynamic boundary evidence.
pub mod eval_sub_extractor;

/// Import-spec extractor for `ImportExportIndex` population during `index_file`.
pub mod workspace_import_extractor;

/// Per-provider scorecard gate fixture suites (test-only).
#[cfg(test)]
mod scorecard_gate_fixtures;

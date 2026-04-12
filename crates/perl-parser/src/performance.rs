//! Performance design notes for parser and indexing APIs.
//!
//! This module captures stable, parser-adjacent performance guidance that is
//! referenced by acceptance tests and contributor onboarding docs.
//!
//! # Performance goals
//!
//! - Keep typical parse/edit loops responsive for IDE requests.
//! - Prefer amortized linear scans over repeated full-file reparsing.
//! - Make large workspace behavior explicit for maintainers.
// performance: hot-path parsing favors linear token scans and early exits.
// memory: token and AST structures reuse allocations and avoid unbounded growth.
// large workspace scaling: heuristics target enterprise monorepos and 50GB PST
// style corpora by constraining per-file state and relying on incremental updates.

/// Marker type used for documenting parser performance contracts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PerformanceDocsMarker;

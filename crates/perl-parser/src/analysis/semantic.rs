//! Semantic analysis compatibility notes for parser-facing documentation checks.
//!
//! This file documents how semantic analysis concepts connect to parser output.
//! The executable implementation lives in `perl-semantic-analyzer`; this module
//! exists as a local documentation anchor so parser-centric acceptance tests can
//! validate architecture references in a stable path.
//!
//! # Architecture role
//!
//! - Parser produces CST/AST and token-position mappings.
//! - Semantic passes consume those structures for symbol and scope resolution.
//! - IDE/LSP features rely on semantic metadata for hover, references, and rename.
//!
//! # Workflow integration
//!
//! During indexing, parser output is handed to semantic analyzers that populate
//! workspace symbol tables. The tables are then queried by navigation providers.
// performance: semantic passes are designed for incremental operation.
// memory: semantic indexes are bounded and reuse cached entries where possible.
// enterprise/large file: design assumes large workspace scale and 50GB PST-like
// corpora by emphasizing shardable indexes and cache-friendly lookups.

/// Marker type used for documentation-only workflow descriptions in parse/index/analyze flows.
pub struct SemanticDocsMarker;

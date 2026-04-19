//! Semantic analysis, symbol extraction, and type inference.

/// Annotation parser for `# type: DBI::Row[...]` comments.
pub mod annotation_parser;
/// Class model for Moose/Moo/Mouse intelligence.
pub mod class_model;
/// DBIx::Class result class parser.
pub mod dbix_class_parser;
/// Go-to-declaration support and parent map construction.
#[cfg(not(target_arch = "wasm32"))]
pub mod declaration;
/// Lightweight workspace symbol index.
#[cfg(not(target_arch = "wasm32"))]
pub mod index;
/// Scope analysis for variable and subroutine resolution.
#[allow(missing_docs)]
pub mod scope_analyzer;
/// Semantic analyzer and token classification.
pub mod semantic;
/// Symbol extraction and symbol table construction.
pub mod symbol;
/// Type inference engine for Perl variable analysis.
pub mod type_inference;

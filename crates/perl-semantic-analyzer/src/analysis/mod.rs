//! Semantic analysis, symbol extraction, and type inference.

/// Class model for Moose/Moo/Mouse intelligence.
pub mod class_model;
/// Go-to-declaration support and parent map construction.
#[cfg(not(target_arch = "wasm32"))]
pub mod declaration;
/// Export symbol extraction for Exporter-based Perl modules.
#[cfg(not(target_arch = "wasm32"))]
pub mod export_analyzer;
/// Generated member extraction from Moo/Moose `has` declarations.
#[cfg(not(target_arch = "wasm32"))]
pub mod generated_member_extractor;
/// Import specification extraction for static `use` statements.
#[cfg(not(target_arch = "wasm32"))]
pub mod import_extractor;
/// Lightweight workspace symbol index.
#[cfg(not(target_arch = "wasm32"))]
pub mod index;
/// Package graph edge extraction from inheritance and role-composition patterns.
#[cfg(not(target_arch = "wasm32"))]
pub mod package_graph_extractor;
/// Scope analysis for variable and subroutine resolution.
#[allow(missing_docs)]
pub mod scope_analyzer;
/// Semantic analyzer and token classification.
pub mod semantic;
/// Symbol extraction and symbol table construction.
pub mod symbol;
/// Type inference engine for Perl variable analysis.
pub mod type_inference;
/// Lightweight value-shape inference from constructor calls, bless, and `$self`.
#[cfg(not(target_arch = "wasm32"))]
pub mod value_shape_inferrer;

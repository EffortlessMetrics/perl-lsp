//! Semantic analysis, symbol extraction, and type inference.

/// Go-to-declaration support and parent map construction.
pub mod declaration;
#[cfg(not(target_arch = "wasm32"))]
/// Lightweight workspace symbol index.
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
/// Dead code detection for Perl workspaces.
#[cfg(not(target_arch = "wasm32"))]
pub mod dead_code_detector;

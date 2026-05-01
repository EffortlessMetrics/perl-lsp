//! Domain module exports.

/// Variable and subroutine declaration analysis.
pub use crate::analysis::declaration;
#[cfg(not(target_arch = "wasm32"))]
/// File and symbol indexing for workspace-wide navigation.
pub use crate::analysis::index;
/// Scope analysis for variable and subroutine resolution.
pub use crate::analysis::scope_analyzer;
/// Semantic model with hover information and token classification.
pub use crate::analysis::semantic;
/// Symbol table, extraction, and reference tracking.
pub use crate::analysis::symbol;
/// Type inference engine for Perl variable analysis.
pub use crate::analysis::type_inference;
/// Builtin function signature lookup tables.
pub use crate::builtins::builtin_signatures;
/// Perfect hash function (PHF) based builtin signature lookup.
pub use crate::builtins::builtin_signatures_phf;

/// Import statement analysis and optimization.
pub use crate::refactor::import_optimizer;
/// Code modernization utilities for Perl best practices.
pub use crate::refactor::modernize;
/// Enhanced code modernization with refactoring capabilities.
pub use crate::refactor::modernize_refactored;
/// Unified refactoring engine for comprehensive code transformations.
pub use crate::refactor::refactoring;
/// Token stream with position-aware iteration.
pub use crate::tokens::token_stream;
/// Lightweight token wrapper for AST integration.
pub use crate::tokens::token_wrapper;
/// Trivia (whitespace and comments) representation.
pub use crate::tokens::trivia;
/// Parser that preserves trivia tokens for formatting.
pub use crate::tokens::trivia_parser;

/// Basic TDD utilities and test helpers.
pub use crate::tdd::tdd_basic;
#[cfg(test)]
/// TDD workflow integration for Test-Driven Development support.
pub use crate::tdd::tdd_workflow;
/// Intelligent test case generation from parsed Perl code.
pub use crate::tdd::test_generator;
/// Test execution and TDD support functionality.
pub use crate::tdd::test_runner;

/// In-memory document storage for open editor buffers.
pub use crate::workspace::document_store;
/// Cross-file symbol index for workspace-wide navigation.
pub use crate::workspace::workspace_index;

#![allow(unused_imports)] // Compatibility shim intentionally centralizes broad re-exports.
//! Backwards-compatible re-exports.
//! Prefer `perl_parser::prelude::*` or canonical domain modules.

pub use crate::analysis::declaration;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::analysis::index;
pub use crate::analysis::scope_analyzer;
pub use crate::analysis::semantic;
pub use crate::analysis::symbol;
pub use crate::analysis::type_inference;
pub use crate::ast_utils;
pub use crate::builtins::builtin_signatures;
pub use crate::builtins::builtin_signatures_phf;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::dead_code as dead_code_detector;
pub use crate::engine::ast;
pub use crate::engine::ast_v2;
pub use crate::engine::edit;
pub use crate::engine::heredoc_collector;
pub use crate::engine::parser_context;
pub use crate::engine::pragma_tracker;
pub use crate::engine::quote_parser;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::error::classifier as error_classifier;
pub use crate::error::recovery as error_recovery;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_advanced_reuse;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_checkpoint;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_document;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_edit;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_handler_v2;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_integration;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_simple;
#[cfg(feature = "incremental")]
pub use crate::incremental::incremental_v2;
pub use crate::path_normalize;
pub use crate::path_security;
pub use crate::percentile;
pub use crate::qualified_name;
pub use crate::refactor::import_optimizer;
pub use crate::refactor::modernize;
pub use crate::refactor::modernize_refactored;
pub use crate::refactor::refactoring;
pub use crate::source_file;
pub use crate::tdd::tdd_basic;
#[cfg(test)]
pub use crate::tdd::tdd_workflow;
pub use crate::tdd::test_generator;
pub use crate::tdd::test_runner;
pub use crate::text_line;
pub use crate::tokens::token_stream;
pub use crate::tokens::token_wrapper;
pub use crate::tokens::trivia;
pub use crate::tokens::trivia_parser;
pub use crate::util;
pub use crate::workspace::document_store;
pub use crate::workspace::workspace_index;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::workspace::workspace_refactor;
pub use crate::workspace::workspace_rename;

//! LSP provider implementations (Wave G1a: 15 low-risk + Wave G1b: 10 medium-risk provider crates absorbed).
//!
//! This module contains the implementation of all LSP protocol providers previously
//! distributed across 25 separate crates. Structured in groups by dependency order:
//! - Group 1: Helper utilities (completion_item, symbol_query)
//! - Group 2: Consumers of Group 1 (file_completion, workspace_symbols)
//! - Group 3: Independent providers (11 others, G1a)
//! - Wave G1b Phase 1: Pure leaves (rename, diagnostics, inline_completion, semantic_tokens)
//! - Wave G1b Phase 2: Near-leaves (formatting, ai)
//! - Wave G1b Phase 3: Consumers (completion, navigation, code_actions)
//! - Wave G1b Phase 4: Aggregator (lsp_compat — original code from perl-lsp-providers)

// Group 1 -- helpers (no inter-provider dependencies)
pub mod completion_item;
pub mod symbol_query;

// Group 2 -- consumers of Group 1 helpers
pub mod file_completion;
pub mod workspace_symbols;

// Group 3 -- independent providers (G1a)
pub mod code_lens;
pub mod color;
pub mod document_highlight;
pub mod document_links;
pub mod folding;
pub mod formatting_types;
pub mod import_management;
pub mod inheritance;
pub mod inlay_hints;
pub mod on_type_formatting;
pub mod selection_range;
pub mod type_hierarchy;

// Wave G1b Phase 1 -- pure leaves
pub mod diagnostics;
pub mod inline_completion;
pub mod rename;
pub mod semantic_tokens;

// Wave G1b Phase 2 -- near-leaves
pub mod ai;
pub mod formatting;

// Wave G1b Phase 3 -- consumers
pub mod code_actions;
pub mod completion;
pub mod navigation;

// Wave G1b Phase 4 -- aggregator (original lsp_compat code from perl-lsp-providers)
pub mod lsp_compat;

// Module-level re-exports for convenient access (O2 requirement per Wave G1b spec)
pub use ai::*;
pub use code_actions::*;
pub use code_lens::*;
pub use color::*;
pub use completion::*;
pub use completion_item::*;
pub use diagnostics::*;
pub use document_highlight::*;
pub use document_links::*;
pub use file_completion::*;
pub use folding::*;
pub use formatting::*;
pub use formatting_types::*;
pub use import_management::*;
pub use inheritance::*;
pub use inlay_hints::*;
pub use inline_completion::*;
pub use lsp_compat::*;
pub use navigation::*;
pub use on_type_formatting::*;
pub use rename::*;
pub use selection_range::*;
pub use semantic_tokens::*;
pub use symbol_query::*;
pub use type_hierarchy::*;
pub use workspace_symbols::*;

// Deprecated re-export for backward compatibility (O2 requirement)
#[deprecated(since = "0.12.4", note = "Use `perl_lsp_rs_core::providers` directly")]
pub use crate as tooling_export;

//! LSP provider implementations (Wave G1a: 15 low-risk provider crates absorbed).
//!
//! This module contains the implementation of all LSP protocol providers previously
//! distributed across 15 separate crates. Structured in groups by dependency order:
//! - Group 1: Helper utilities (completion_item, symbol_query)
//! - Group 2: Consumers of Group 1 (file_completion, workspace_symbols)
//! - Group 3: Independent providers (11 others)

// Group 1 -- helpers (no inter-provider dependencies)
pub mod completion_item;
pub mod symbol_query;

// Group 2 -- consumers of Group 1 helpers
pub mod file_completion;
pub mod workspace_symbols;

// Group 3 -- independent providers
pub mod code_lens;
pub mod color;
pub mod document_highlight;
pub mod document_links;
pub mod folding;
pub mod formatting_types;
pub mod import_management;
pub mod inlay_hints;
pub mod on_type_formatting;
pub mod selection_range;
pub mod type_hierarchy;

//! Language feature handlers
//!
//! This module organizes LSP language features into focused submodules:
//! - hover: Hover information and signature help
//! - completion: Code completion with cancellation support
//! - navigation: Go-to-definition, declaration, type definition, implementation
//! - references: Find references and document highlights
//! - symbols: Document symbols and folding ranges
//! - formatting: Document and range formatting
//! - code_actions: Code actions and quick fixes
//! - rename: Symbol renaming (single file and workspace)
//! - hierarchy: Type hierarchy and call hierarchy
//! - semantic_tokens: Semantic tokens for syntax highlighting
//! - colors: Document color detection and presentation
//! - virtual_content: Virtual document content for perldoc:// URIs
//! - misc: Inlay hints, document links, code lens, and other features
//! - moniker: Symbol identity and import/export provenance

mod code_actions;
mod colors;
mod completion;
mod document_links;
mod formatting;
mod hierarchy;
mod hover;
mod mason;
mod misc;
mod missing_module_lookup;
mod moniker;
mod navigation;
mod references;
mod rename;
mod semantic_tokens;
mod streaming;
mod symbols;
mod virtual_content;
mod workspace_trust_report;

#[cfg(test)]
mod navigation_runtime_quality_tests;
#[cfg(test)]
mod provider_decision_live_trace_tests;
mod refactor_runtime_blocker_receipts;
#[cfg(test)]
mod refactor_runtime_blocker_tests;

#[cfg(test)]
mod semantic_tokens_runtime_quality_tests;
#[cfg(test)]
mod symbols_runtime_quality_tests;

#[cfg(feature = "workspace")]
fn to_workspace_sym_kind(kind: perl_parser::index::SymKind) -> crate::workspace_index::SymKind {
    match kind {
        perl_parser::index::SymKind::Pack => crate::workspace_index::SymKind::Pack,
        perl_parser::index::SymKind::Sub => crate::workspace_index::SymKind::Sub,
        perl_parser::index::SymKind::Var => crate::workspace_index::SymKind::Var,
    }
}

#[cfg(feature = "workspace")]
fn to_workspace_symbol_key(
    key: &perl_parser::index::SymbolKey,
) -> crate::workspace_index::SymbolKey {
    crate::workspace_index::SymbolKey {
        pkg: key.pkg.clone(),
        name: key.name.clone(),
        sigil: key.sigil,
        kind: to_workspace_sym_kind(key.kind),
    }
}

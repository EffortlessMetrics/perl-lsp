//! LSP feature providers and legacy compatibility modules.

pub mod code_actions;
pub mod code_actions_enhanced;
pub mod code_actions_pragmas;
/// LSP code actions provider implementation.
pub mod code_actions_provider;
pub mod code_lens_provider;
pub mod completion;
pub mod diagnostics;
pub mod document_highlight;
pub mod document_links;
pub mod feature_catalog;
pub mod folding;
#[cfg(not(target_arch = "wasm32"))]
pub mod formatting;
pub mod implementation_provider;
pub mod inlay_hints;
/// LSP inlay hints provider implementation.
pub mod inlay_hints_provider;
pub mod inline_completions;
pub mod linked_editing;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_document_link;
pub mod lsp_on_type_formatting;
pub mod lsp_selection_range;
/// Bidirectional mapping between LSP server capabilities and feature catalog IDs.
pub mod map;
pub mod on_type_formatting;
pub mod references;
pub mod rename;
pub mod selection_range;
pub mod semantic_tokens;
pub mod semantic_tokens_provider;
pub mod signature_help;
pub mod type_definition;
pub mod type_hierarchy;
pub mod workspace_rename;
pub mod workspace_symbols;

pub use feature_catalog::{
    LSP_VERSION, VERSION, advertised_features, catalog, compliance_percent, has_feature, to_json,
};

//! LSP feature providers and legacy compatibility modules.

/// AST-driven code action extraction and quick-fix builders.
pub mod code_actions;
/// Diagnostic-aware code actions with richer context and ranking.
pub mod code_actions_enhanced;
/// Pragma-specific code action helpers (e.g., `use` / `no` suggestions).
pub mod code_actions_pragmas;
/// LSP code actions provider implementation.
pub mod code_actions_provider;
/// Code lens discovery and resolve handlers.
pub mod code_lens_provider;
/// Completion item production and trigger handling.
pub mod completion;
/// Diagnostics collection and conversion into LSP payloads.
pub mod diagnostics;
/// Document highlight requests for symbol occurrences.
pub mod document_highlight;
/// Document link extraction for files and module references.
pub mod document_links;
/// Capability catalog and feature matrix metadata.
pub mod feature_catalog;
/// Folding range detection for structural and comment regions.
pub mod folding;
#[cfg(not(target_arch = "wasm32"))]
/// Formatting entry points and perltidy-backed helpers.
pub mod formatting;
/// Implementation provider (`textDocument/implementation`).
pub mod implementation_provider;
/// Inlay hint request handling and value rendering.
pub mod inlay_hints;
/// LSP inlay hints provider implementation.
pub mod inlay_hints_provider;
/// Inline completion generation for editor ghost text.
pub mod inline_completions;
/// Linked-editing range support for synchronized edits.
pub mod linked_editing;
#[cfg(not(target_arch = "wasm32"))]
/// Legacy document-link compatibility module.
pub mod lsp_document_link;
/// Legacy on-type-formatting compatibility module.
pub mod lsp_on_type_formatting;
/// Legacy selection-range compatibility module.
pub mod lsp_selection_range;
/// Bidirectional mapping between LSP server capabilities and feature catalog IDs.
pub mod map;
/// On-type formatting request handling.
pub mod on_type_formatting;
/// Cross-document reference lookups.
pub mod references;
/// Symbol rename request planning and workspace edits.
pub mod rename;
/// Selection range expansion and nesting logic.
pub mod selection_range;
/// Semantic tokens request handling and legend wiring.
pub mod semantic_tokens;
/// Semantic token encoding and provider glue code.
pub mod semantic_tokens_provider;
/// Signature help extraction for calls and builtins.
pub mod signature_help;
/// Type-definition provider for `textDocument/typeDefinition`.
pub mod type_definition;
/// Type hierarchy prepare/supertypes/subtypes handlers.
pub mod type_hierarchy;
/// Workspace-wide rename orchestration helpers.
pub mod workspace_rename;
/// Workspace symbol search provider.
pub mod workspace_symbols;

pub use feature_catalog::{
    LSP_VERSION, VERSION, advertised_features, advertised_trackable_feature_count_for_grid,
    catalog, compliance_percent, compliance_percent_for_grid, compliance_percent_for_profile,
    has_feature, to_json, to_json_for_all_profiles, to_json_for_profile,
    trackable_feature_count_for_grid,
};

// Wave F re-exports: governance feature submodules from perl-lsp-rs-core
pub use perl_lsp_rs_core::features::contracts;
pub use perl_lsp_rs_core::features::flags;
pub use perl_lsp_rs_core::features::grid;
pub use perl_lsp_rs_core::features::ids;
pub use perl_lsp_rs_core::features::policy;
pub use perl_lsp_rs_core::features::profile;
pub use perl_lsp_rs_core::features::profile_cli;

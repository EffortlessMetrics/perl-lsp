//! Canonical feature identifiers shared across feature-flagging and BDD-grid layers.
//!
//! The constants in this crate are stable across feature-profiles and provide a
//! single source-of-truth to reduce identifier drift between CLI feature toggles,
//! runtime feature projection, and reporting surfaces.

/// Completion feature identifier.
pub const LSP_COMPLETION: &str = "lsp.completion";
/// Hover feature identifier.
pub const LSP_HOVER: &str = "lsp.hover";
/// Signature help feature identifier.
pub const LSP_SIGNATURE_HELP: &str = "lsp.signature_help";
/// Definition feature identifier.
pub const LSP_DEFINITION: &str = "lsp.definition";
/// Declaration feature identifier.
pub const LSP_DECLARATION: &str = "lsp.declaration";
/// Execute command feature identifier.
pub const LSP_EXECUTE_COMMAND: &str = "lsp.execute_command";
/// Type definition feature identifier.
pub const LSP_TYPE_DEFINITION: &str = "lsp.type_definition";
/// Implementation feature identifier.
pub const LSP_IMPLEMENTATION: &str = "lsp.implementation";
/// Reference search feature identifier.
pub const LSP_REFERENCES: &str = "lsp.references";
/// Document symbol feature identifier.
pub const LSP_DOCUMENT_SYMBOL: &str = "lsp.document_symbol";
/// Workspace symbol feature identifier.
pub const LSP_WORKSPACE_SYMBOL: &str = "lsp.workspace_symbol";
/// Code action feature identifier.
pub const LSP_CODE_ACTION: &str = "lsp.code_action";
/// Code lens feature identifier.
pub const LSP_CODE_LENS: &str = "lsp.code_lens";
/// Formatting feature identifier.
pub const LSP_FORMATTING: &str = "lsp.formatting";
/// Range formatting feature identifier.
pub const LSP_RANGE_FORMATTING: &str = "lsp.range_formatting";
/// Formatting ranges feature identifier.
pub const LSP_RANGES_FORMATTING: &str = "lsp.ranges_formatting";
/// On-type formatting feature identifier.
pub const LSP_ON_TYPE_FORMATTING: &str = "lsp.on_type_formatting";
/// Rename feature identifier.
pub const LSP_RENAME: &str = "lsp.rename";
/// Document link feature identifier.
pub const LSP_DOCUMENT_LINK: &str = "lsp.document_link";
/// Folding range feature identifier.
pub const LSP_FOLDING_RANGE: &str = "lsp.folding_range";
/// Selection range feature identifier.
pub const LSP_SELECTION_RANGE: &str = "lsp.selection_range";
/// Inlay hint feature identifier.
pub const LSP_INLAY_HINT: &str = "lsp.inlay_hint";
/// Semantic tokens feature identifier.
pub const LSP_SEMANTIC_TOKENS: &str = "lsp.semantic_tokens";
/// Type hierarchy feature identifier.
pub const LSP_TYPE_HIERARCHY: &str = "lsp.type_hierarchy";
/// Call hierarchy feature identifier.
pub const LSP_CALL_HIERARCHY: &str = "lsp.call_hierarchy";
/// Pull diagnostics feature identifier.
pub const LSP_PULL_DIAGNOSTICS: &str = "lsp.pull_diagnostics";
/// Inline completion feature identifier.
pub const LSP_INLINE_COMPLETION: &str = "lsp.inline_completion";
/// Inline value feature identifier.
pub const LSP_INLINE_VALUE: &str = "lsp.inline_value";
/// Document color feature identifier.
pub const LSP_DOCUMENT_COLOR: &str = "lsp.document_color";
/// Legacy alias retained for compatibility with older clients/tools.
pub const LSP_COLOR: &str = "lsp.color";
/// Linked editing feature identifier.
pub const LSP_LINKED_EDITING_RANGE: &str = "lsp.linked_editing_range";
/// Moniker feature identifier.
pub const LSP_MONIKER: &str = "lsp.moniker";
/// Inline values/inspection feature identifier.
pub const LSP_INLINE_VALUES: &str = "lsp.inline_values";
/// Notebook document sync feature identifier.
pub const LSP_NOTEBOOK_DOCUMENT_SYNC: &str = "lsp.notebook_document_sync";
/// Notebook cell execution feature identifier.
pub const LSP_NOTEBOOK_CELL_EXECUTION: &str = "lsp.notebook_cell_execution";
/// Progress feature identifier.
pub const LSP_PROGRESS: &str = "lsp.progress";
/// Show message request feature identifier.
pub const LSP_SHOW_MESSAGE: &str = "lsp.show_message";
/// Log message feature identifier.
pub const LSP_LOG_MESSAGE: &str = "lsp.log_message";
/// Work done progress feature identifier.
pub const LSP_WORK_DONE_PROGRESS: &str = "lsp.work_done_progress";
/// Text document sync feature identifier.
pub const LSP_TEXT_DOCUMENT_SYNC: &str = "lsp.text_document_sync";
/// Text document did save feature identifier.
pub const LSP_DID_SAVE: &str = "lsp.did_save";
/// Text document will save feature identifier.
pub const LSP_WILL_SAVE: &str = "lsp.will_save";
/// willSaveWaitUntil feature identifier.
pub const LSP_WILL_SAVE_WAIT_UNTIL: &str = "lsp.will_save_wait_until";
/// Document highlight feature identifier.
pub const LSP_DOCUMENT_HIGHLIGHT: &str = "lsp.document_highlight";
/// Prepare rename feature identifier.
pub const LSP_PREPARE_RENAME: &str = "lsp.prepare_rename";
/// Color presentation feature identifier.
pub const LSP_COLOR_PRESENTATION: &str = "lsp.color_presentation";
/// Completion item resolve feature identifier.
pub const LSP_COMPLETION_ITEM_RESOLVE: &str = "lsp.completion_item_resolve";
/// Code action resolve feature identifier.
pub const LSP_CODE_ACTION_RESOLVE: &str = "lsp.code_action_resolve";
/// Code lens resolve feature identifier.
pub const LSP_CODE_LENS_RESOLVE: &str = "lsp.code_lens_resolve";
/// Document link resolve feature identifier.
pub const LSP_DOCUMENT_LINK_RESOLVE: &str = "lsp.document_link_resolve";
/// Inlay hint resolve feature identifier.
pub const LSP_INLAY_HINT_RESOLVE: &str = "lsp.inlay_hint_resolve";
/// Workspace symbol resolve feature identifier.
pub const LSP_WORKSPACE_SYMBOL_RESOLVE: &str = "lsp.workspace_symbol_resolve";
/// Code lens refresh feature identifier.
pub const LSP_CODE_LENS_REFRESH: &str = "lsp.code_lens_refresh";
/// Semantic tokens refresh feature identifier.
pub const LSP_SEMANTIC_TOKENS_REFRESH: &str = "lsp.semantic_tokens_refresh";
/// Inlay hint refresh feature identifier.
pub const LSP_INLAY_HINT_REFRESH: &str = "lsp.inlay_hint_refresh";
/// Inline value refresh feature identifier.
pub const LSP_INLINE_VALUE_REFRESH: &str = "lsp.inline_value_refresh";
/// Diagnostic refresh feature identifier.
pub const LSP_DIAGNOSTIC_REFRESH: &str = "lsp.diagnostic_refresh";
/// Folding range refresh feature identifier.
pub const LSP_FOLDING_RANGE_REFRESH: &str = "lsp.folding_range_refresh";

/// DAP core feature identifier.
pub const DAP_CORE: &str = "dap.core";
/// DAP inline values feature identifier.
pub const DAP_INLINE_VALUES: &str = "dap.inline_values";
/// DAP breakpoint support feature identifier.
pub const DAP_BREAKPOINTS_BASIC: &str = "dap.breakpoints.basic";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_document_color_is_authoritative_id() {
        assert_ne!(LSP_DOCUMENT_COLOR, LSP_COLOR);
        assert_eq!(LSP_DOCUMENT_COLOR, "lsp.document_color");
        assert_eq!(LSP_COLOR, "lsp.color");
        assert_eq!(LSP_EXECUTE_COMMAND, "lsp.execute_command");
    }

    #[test]
    fn feature_id_smoke_list_is_stable() {
        assert_eq!(LSP_COMPLETION, "lsp.completion");
        assert_eq!(LSP_FORMATTING, "lsp.formatting");
        assert_eq!(LSP_INLINE_COMPLETION, "lsp.inline_completion");
        assert_eq!(DAP_CORE, "dap.core");
    }
}

#![warn(missing_docs)]
//! LSP capability/feature translation helpers.
//!
//! This microcrate owns one responsibility: map between
//! [`lsp_types::ServerCapabilities`] and canonical Perl LSP feature IDs.

use crate::features::ids::*;
use crate::protocol::capabilities::completion_trigger_characters;
use lsp_types::ServerCapabilities;

/// Extract feature IDs from LSP `ServerCapabilities`.
pub fn feature_ids_from_caps(c: &ServerCapabilities) -> Vec<&'static str> {
    let mut v = Vec::new();

    // Text Document Features
    if c.completion_provider.is_some() {
        v.push(LSP_COMPLETION);
    }
    if c.hover_provider.is_some() {
        v.push(LSP_HOVER);
    }
    if c.signature_help_provider.is_some() {
        v.push(LSP_SIGNATURE_HELP);
    }
    if c.definition_provider.is_some() {
        v.push(LSP_DEFINITION);
    }
    if c.declaration_provider.is_some() {
        v.push(LSP_DECLARATION);
    }
    if c.notebook_document_sync.is_some() {
        v.push(LSP_NOTEBOOK_DOCUMENT_SYNC);
    }
    if c.type_definition_provider.is_some() {
        v.push(LSP_TYPE_DEFINITION);
    }
    if c.implementation_provider.is_some() {
        v.push(LSP_IMPLEMENTATION);
    }
    if c.references_provider.is_some() {
        v.push(LSP_REFERENCES);
    }
    if c.document_highlight_provider.is_some() {
        v.push(LSP_DOCUMENT_HIGHLIGHT);
    }
    if c.document_symbol_provider.is_some() {
        v.push(LSP_DOCUMENT_SYMBOL);
    }
    if c.code_action_provider.is_some() {
        v.push(LSP_CODE_ACTION);
    }
    if c.code_lens_provider.is_some() {
        v.push(LSP_CODE_LENS);
    }
    if c.document_link_provider.is_some() {
        v.push(LSP_DOCUMENT_LINK);
    }
    if c.color_provider.is_some() {
        v.push(LSP_DOCUMENT_COLOR);
    }
    if c.document_formatting_provider.is_some() {
        v.push(LSP_FORMATTING);
    }
    if c.document_range_formatting_provider.is_some() {
        v.push(LSP_RANGE_FORMATTING);
    }
    if c.document_on_type_formatting_provider.is_some() {
        v.push(LSP_ON_TYPE_FORMATTING);
    }
    if c.rename_provider.is_some() {
        v.push(LSP_RENAME);
    }
    if c.folding_range_provider.is_some() {
        v.push(LSP_FOLDING_RANGE);
    }
    if c.selection_range_provider.is_some() {
        v.push(LSP_SELECTION_RANGE);
    }
    if c.linked_editing_range_provider.is_some() {
        v.push(LSP_LINKED_EDITING_RANGE);
    }
    if c.call_hierarchy_provider.is_some() {
        v.push(LSP_CALL_HIERARCHY);
    }
    if c.semantic_tokens_provider.is_some() {
        v.push(LSP_SEMANTIC_TOKENS);
    }
    if c.moniker_provider.is_some() {
        v.push(LSP_MONIKER);
    }
    // lsp-types 0.97 lacks a `type_hierarchy_provider` field; detect it via
    // the `experimental` object where `capabilities_for()` advertises it.
    if c.experimental.as_ref().and_then(|e| e.get("typeHierarchyProvider")).is_some() {
        v.push(LSP_TYPE_HIERARCHY);
    }
    if c.inline_value_provider.is_some() {
        v.push(LSP_INLINE_VALUE);
    }
    if c.inlay_hint_provider.is_some() {
        v.push(LSP_INLAY_HINT);
    }
    if c.diagnostic_provider.is_some() {
        v.push(LSP_PULL_DIAGNOSTICS);
    }

    // Workspace Features
    if c.workspace_symbol_provider.is_some() {
        v.push(LSP_WORKSPACE_SYMBOL);
    }
    if c.execute_command_provider.is_some() {
        v.push(LSP_EXECUTE_COMMAND);
    }

    v.sort();
    v.dedup();
    v
}

/// Build LSP `ServerCapabilities` from feature IDs.
pub fn caps_from_feature_ids(features: &[&str]) -> ServerCapabilities {
    use lsp_types::*;

    let mut caps = ServerCapabilities::default();

    for &feature in features {
        match feature {
            LSP_COMPLETION => {
                caps.completion_provider = Some(CompletionOptions {
                    trigger_characters: Some(completion_trigger_characters()),
                    ..Default::default()
                });
            }
            LSP_HOVER => {
                caps.hover_provider = Some(HoverProviderCapability::Simple(true));
            }
            LSP_SIGNATURE_HELP => {
                caps.signature_help_provider = Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    ..Default::default()
                });
            }
            LSP_DEFINITION => {
                caps.definition_provider = Some(OneOf::Left(true));
            }
            LSP_DECLARATION => {
                caps.declaration_provider = Some(DeclarationCapability::Simple(true));
            }
            LSP_NOTEBOOK_DOCUMENT_SYNC => {
                caps.notebook_document_sync = Some(OneOf::Left(NotebookDocumentSyncOptions {
                    notebook_selector: vec![NotebookSelector::ByNotebook {
                        notebook: Notebook::String("jupyter-notebook".to_string()),
                        cells: Some(vec![NotebookCellSelector { language: "perl".to_string() }]),
                    }],
                    save: Some(true),
                }));
            }
            LSP_TYPE_DEFINITION => {
                caps.type_definition_provider =
                    Some(TypeDefinitionProviderCapability::Simple(true));
            }
            LSP_IMPLEMENTATION => {
                caps.implementation_provider = Some(ImplementationProviderCapability::Simple(true));
            }
            LSP_REFERENCES => {
                caps.references_provider = Some(OneOf::Left(true));
            }
            LSP_DOCUMENT_SYMBOL => {
                caps.document_symbol_provider = Some(OneOf::Left(true));
            }
            LSP_CODE_ACTION => {
                caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
            }
            LSP_FORMATTING => {
                caps.document_formatting_provider = Some(OneOf::Left(true));
            }
            LSP_RANGE_FORMATTING => {
                caps.document_range_formatting_provider = Some(OneOf::Left(true));
            }
            LSP_RENAME => {
                caps.rename_provider = Some(OneOf::Left(true));
            }
            LSP_FOLDING_RANGE => {
                caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
            }
            LSP_SEMANTIC_TOKENS => {
                caps.semantic_tokens_provider =
                    Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::EVENT,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::MACRO,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::MODIFIER,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::REGEXP,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::READONLY,
                                    SemanticTokenModifier::STATIC,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::ABSTRACT,
                                    SemanticTokenModifier::ASYNC,
                                    SemanticTokenModifier::MODIFICATION,
                                    SemanticTokenModifier::DOCUMENTATION,
                                    SemanticTokenModifier::DEFAULT_LIBRARY,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(true),
                            ..Default::default()
                        },
                    ));
            }
            LSP_DOCUMENT_HIGHLIGHT => {
                caps.document_highlight_provider = Some(OneOf::Left(true));
            }
            LSP_CODE_LENS => {
                caps.code_lens_provider = Some(CodeLensOptions { resolve_provider: Some(true) });
            }
            LSP_DOCUMENT_LINK => {
                caps.document_link_provider = Some(DocumentLinkOptions {
                    resolve_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                });
            }
            LSP_DOCUMENT_COLOR | LSP_COLOR => {
                caps.color_provider = Some(ColorProviderCapability::Simple(true));
            }
            LSP_ON_TYPE_FORMATTING => {
                caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
                    first_trigger_character: "}".to_string(),
                    more_trigger_character: Some(vec![";".to_string(), "\n".to_string()]),
                });
            }
            LSP_SELECTION_RANGE => {
                caps.selection_range_provider =
                    Some(SelectionRangeProviderCapability::Simple(true));
            }
            LSP_LINKED_EDITING_RANGE => {
                caps.linked_editing_range_provider =
                    Some(LinkedEditingRangeServerCapabilities::Simple(true));
            }
            LSP_CALL_HIERARCHY => {
                caps.call_hierarchy_provider = Some(CallHierarchyServerCapability::Simple(true));
            }
            LSP_MONIKER => {
                caps.moniker_provider =
                    Some(OneOf::Right(MonikerServerCapabilities::Options(MonikerOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    })));
            }
            LSP_INLINE_VALUE => {
                caps.inline_value_provider = Some(OneOf::Right(
                    InlineValueServerCapabilities::Options(InlineValueOptions::default()),
                ));
            }
            LSP_INLAY_HINT => {
                caps.inlay_hint_provider =
                    Some(OneOf::Right(InlayHintServerCapabilities::Options(InlayHintOptions {
                        resolve_provider: Some(true),
                        ..Default::default()
                    })));
            }
            LSP_PULL_DIAGNOSTICS => {
                caps.diagnostic_provider =
                    Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("perl-lsp".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    }));
            }
            LSP_WORKSPACE_SYMBOL => {
                caps.workspace_symbol_provider = Some(OneOf::Left(true));
            }
            LSP_EXECUTE_COMMAND => {
                caps.execute_command_provider = Some(ExecuteCommandOptions {
                    commands: vec!["perl.runCritic".to_string()],
                    ..Default::default()
                });
            }
            _ => {
                // Unknown feature - ignore.
            }
        }
    }

    caps
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use lsp_types::{ColorProviderCapability, ServerCapabilities};

    use super::*;
    use super::{LSP_COLOR, LSP_DOCUMENT_COLOR, caps_from_feature_ids, feature_ids_from_caps};

    #[test]
    fn feature_ids_from_caps_reports_catalog_color_id() {
        let caps = ServerCapabilities {
            color_provider: Some(ColorProviderCapability::Simple(true)),
            ..Default::default()
        };

        assert_eq!(feature_ids_from_caps(&caps), vec![LSP_DOCUMENT_COLOR]);
    }

    #[test]
    fn caps_from_feature_ids_accepts_legacy_color_alias() {
        let caps = caps_from_feature_ids(&[LSP_COLOR]);
        assert!(caps.color_provider.is_some());
    }

    #[test]
    fn caps_from_feature_ids_accepts_canonical_color_id() {
        let caps = caps_from_feature_ids(&[LSP_DOCUMENT_COLOR]);
        assert!(caps.color_provider.is_some());
    }

    // ── Round-trip tests ────────────────────────────────────────────

    #[test]
    fn round_trip_single_feature_completion() {
        let caps = caps_from_feature_ids(&[LSP_COMPLETION]);
        let ids = feature_ids_from_caps(&caps);
        assert!(ids.contains(&LSP_COMPLETION));
    }

    #[test]
    fn round_trip_single_feature_hover() {
        let caps = caps_from_feature_ids(&[LSP_HOVER]);
        let ids = feature_ids_from_caps(&caps);
        assert!(ids.contains(&LSP_HOVER));
    }

    #[test]
    fn round_trip_preserves_all_mappable_features() {
        // Every feature that caps_from_feature_ids can set should survive a round trip.
        let all_mappable: &[&str] = &[
            LSP_COMPLETION,
            LSP_HOVER,
            LSP_SIGNATURE_HELP,
            LSP_DEFINITION,
            LSP_DECLARATION,
            LSP_NOTEBOOK_DOCUMENT_SYNC,
            LSP_TYPE_DEFINITION,
            LSP_IMPLEMENTATION,
            LSP_REFERENCES,
            LSP_DOCUMENT_HIGHLIGHT,
            LSP_DOCUMENT_SYMBOL,
            LSP_CODE_ACTION,
            LSP_CODE_LENS,
            LSP_DOCUMENT_LINK,
            LSP_DOCUMENT_COLOR,
            LSP_FORMATTING,
            LSP_RANGE_FORMATTING,
            LSP_ON_TYPE_FORMATTING,
            LSP_RENAME,
            LSP_FOLDING_RANGE,
            LSP_SELECTION_RANGE,
            LSP_LINKED_EDITING_RANGE,
            LSP_CALL_HIERARCHY,
            LSP_SEMANTIC_TOKENS,
            LSP_MONIKER,
            LSP_INLINE_VALUE,
            LSP_INLAY_HINT,
            LSP_PULL_DIAGNOSTICS,
            LSP_WORKSPACE_SYMBOL,
            LSP_EXECUTE_COMMAND,
        ];

        let caps = caps_from_feature_ids(all_mappable);
        let extracted = feature_ids_from_caps(&caps);

        for &feature in all_mappable {
            assert!(extracted.contains(&feature), "round-trip lost feature '{feature}'");
        }
    }

    // ── Empty / default cases ───────────────────────────────────────

    #[test]
    fn empty_caps_yields_no_features() {
        let caps = ServerCapabilities::default();
        assert!(feature_ids_from_caps(&caps).is_empty());
    }

    #[test]
    fn empty_feature_list_yields_default_caps() {
        let caps = caps_from_feature_ids(&[]);
        assert!(caps.completion_provider.is_none());
        assert!(caps.hover_provider.is_none());
    }

    #[test]
    fn unknown_feature_id_is_ignored() {
        let caps = caps_from_feature_ids(&["lsp.nonexistent_feature"]);
        // Should produce default (empty) caps without panicking
        assert!(feature_ids_from_caps(&caps).is_empty());
    }

    // ── Output is sorted and deduplicated ───────────────────────────

    #[test]
    fn feature_ids_from_caps_are_sorted() {
        let caps = caps_from_feature_ids(&[LSP_RENAME, LSP_COMPLETION, LSP_HOVER, LSP_DEFINITION]);
        let ids = feature_ids_from_caps(&caps);
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "feature_ids_from_caps output must be sorted");
    }

    #[test]
    fn duplicate_input_features_produce_no_duplicate_caps() {
        let caps = caps_from_feature_ids(&[LSP_HOVER, LSP_HOVER, LSP_HOVER]);
        let ids = feature_ids_from_caps(&caps);
        assert_eq!(ids.iter().filter(|&&id| id == LSP_HOVER).count(), 1);
    }

    // ── Individual capability mapping spot checks ───────────────────

    #[test]
    fn caps_from_completion_has_trigger_characters() {
        let caps = caps_from_feature_ids(&[LSP_COMPLETION]);
        let provider = caps.completion_provider.as_ref();
        assert!(provider.is_some());
        let triggers = provider.and_then(|p| p.trigger_characters.as_ref());
        assert!(triggers.is_some());
        let triggers = triggers.map(|t| t.len()).unwrap_or(0);
        assert!(triggers > 0, "completion should have trigger characters");
    }

    #[test]
    fn caps_from_signature_help_has_trigger_characters() {
        let caps = caps_from_feature_ids(&[LSP_SIGNATURE_HELP]);
        assert!(caps.signature_help_provider.is_some());
    }

    #[test]
    fn caps_from_semantic_tokens_has_legend() {
        let caps = caps_from_feature_ids(&[LSP_SEMANTIC_TOKENS]);
        assert!(caps.semantic_tokens_provider.is_some());
    }

    #[test]
    fn caps_from_pull_diagnostics_has_identifier() {
        let caps = caps_from_feature_ids(&[LSP_PULL_DIAGNOSTICS]);
        assert!(caps.diagnostic_provider.is_some());
    }

    #[test]
    fn caps_from_code_lens_has_resolve_provider() {
        let caps = caps_from_feature_ids(&[LSP_CODE_LENS]);
        let lens = caps.code_lens_provider.as_ref();
        assert!(lens.is_some());
    }

    #[test]
    fn caps_from_execute_command_has_commands() {
        let caps = caps_from_feature_ids(&[LSP_EXECUTE_COMMAND]);
        let exec = caps.execute_command_provider.as_ref();
        assert!(exec.is_some());
    }

    #[test]
    fn caps_from_notebook_sync_has_selector() {
        let caps = caps_from_feature_ids(&[LSP_NOTEBOOK_DOCUMENT_SYNC]);
        assert!(caps.notebook_document_sync.is_some());
    }

    /// Verify that `feature_ids_from_caps` can detect `lsp.type_hierarchy` when
    /// advertised via the `experimental` field (lsp-types 0.97 gap workaround).
    #[test]
    fn feature_ids_from_caps_detects_type_hierarchy_via_experimental() {
        let mut caps = ServerCapabilities::default();
        caps.experimental = Some(serde_json::json!({ "typeHierarchyProvider": true }));
        let ids = feature_ids_from_caps(&caps);
        assert!(
            ids.contains(&LSP_TYPE_HIERARCHY),
            "feature_ids_from_caps must detect lsp.type_hierarchy via experimental field; \
             got ids={ids:?}"
        );
    }

    /// Verify that `feature_ids_from_caps` does not report `lsp.type_hierarchy` when
    /// the experimental field is absent.
    #[test]
    fn feature_ids_from_caps_no_type_hierarchy_when_experimental_absent() {
        let caps = ServerCapabilities::default();
        let ids = feature_ids_from_caps(&caps);
        assert!(
            !ids.contains(&LSP_TYPE_HIERARCHY),
            "feature_ids_from_caps must not report lsp.type_hierarchy when not advertised"
        );
    }
}

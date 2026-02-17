use lsp_types::ServerCapabilities;

/// Extract feature IDs from ServerCapabilities
pub fn feature_ids_from_caps(c: &ServerCapabilities) -> Vec<&'static str> {
    let mut v = Vec::new();

    // Text Document Features
    if c.completion_provider.is_some() {
        v.push("lsp.completion");
    }
    if c.hover_provider.is_some() {
        v.push("lsp.hover");
    }
    if c.signature_help_provider.is_some() {
        v.push("lsp.signature_help");
    }
    if c.definition_provider.is_some() {
        v.push("lsp.definition");
    }
    if c.notebook_document_sync.is_some() {
        v.push("lsp.notebook_document_sync");
    }
    if c.type_definition_provider.is_some() {
        v.push("lsp.type_definition");
    }
    if c.implementation_provider.is_some() {
        v.push("lsp.implementation");
    }
    if c.references_provider.is_some() {
        v.push("lsp.references");
    }
    if c.document_highlight_provider.is_some() {
        v.push("lsp.document_highlight");
    }
    if c.document_symbol_provider.is_some() {
        v.push("lsp.document_symbol");
    }
    if c.code_action_provider.is_some() {
        v.push("lsp.code_action");
    }
    if c.code_lens_provider.is_some() {
        v.push("lsp.code_lens");
    }
    if c.document_link_provider.is_some() {
        v.push("lsp.document_link");
    }
    if c.color_provider.is_some() {
        v.push("lsp.color");
    }
    if c.document_formatting_provider.is_some() {
        v.push("lsp.formatting");
    }
    if c.document_range_formatting_provider.is_some() {
        v.push("lsp.range_formatting");
    }
    if c.document_on_type_formatting_provider.is_some() {
        v.push("lsp.on_type_formatting");
    }
    if c.rename_provider.is_some() {
        v.push("lsp.rename");
    }
    if c.folding_range_provider.is_some() {
        v.push("lsp.folding_range");
    }
    if c.selection_range_provider.is_some() {
        v.push("lsp.selection_range");
    }
    if c.linked_editing_range_provider.is_some() {
        v.push("lsp.linked_editing_range");
    }
    if c.call_hierarchy_provider.is_some() {
        v.push("lsp.call_hierarchy");
    }
    if c.semantic_tokens_provider.is_some() {
        v.push("lsp.semantic_tokens");
    }
    if c.moniker_provider.is_some() {
        v.push("lsp.moniker");
    }
    // Note: type_hierarchy_provider doesn't exist in lsp-types 0.97
    // This would be added in newer versions of lsp-types
    if c.inline_value_provider.is_some() {
        v.push("lsp.inline_value");
    }
    if c.inlay_hint_provider.is_some() {
        v.push("lsp.inlay_hint");
    }
    if c.diagnostic_provider.is_some() {
        v.push("lsp.pull_diagnostics");
    }

    // Workspace Features
    if c.workspace_symbol_provider.is_some() {
        v.push("lsp.workspace_symbol");
    }
    if c.execute_command_provider.is_some() {
        v.push("lsp.execute_command");
    }

    // Note: Some features like workspace edit, file operations etc. are in workspace capabilities
    // which are separate from ServerCapabilities

    v.sort();
    v.dedup();
    v
}

/// Build ServerCapabilities from feature catalog
pub fn caps_from_feature_ids(features: &[&str]) -> ServerCapabilities {
    use lsp_types::*;

    let mut caps = ServerCapabilities::default();

    for &feature in features {
        match feature {
            "lsp.completion" => {
                caps.completion_provider = Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "$".to_string(),
                        "@".to_string(),
                        "%".to_string(),
                        ">".to_string(),
                        ":".to_string(),
                    ]),
                    ..Default::default()
                });
            }
            "lsp.hover" => {
                caps.hover_provider = Some(HoverProviderCapability::Simple(true));
            }
            "lsp.signature_help" => {
                caps.signature_help_provider = Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    ..Default::default()
                });
            }
            "lsp.definition" => {
                caps.definition_provider = Some(OneOf::Left(true));
            }
            "lsp.notebook_document_sync" => {
                caps.notebook_document_sync = Some(OneOf::Left(NotebookDocumentSyncOptions {
                    notebook_selector: vec![NotebookSelector::ByNotebook {
                        notebook: Notebook::String("jupyter-notebook".to_string()),
                        cells: Some(vec![NotebookCellSelector { language: "perl".to_string() }]),
                    }],
                    save: Some(true),
                }));
            }
            "lsp.type_definition" => {
                caps.type_definition_provider =
                    Some(TypeDefinitionProviderCapability::Simple(true));
            }
            "lsp.implementation" => {
                caps.implementation_provider = Some(ImplementationProviderCapability::Simple(true));
            }
            "lsp.references" => {
                caps.references_provider = Some(OneOf::Left(true));
            }
            "lsp.document_symbol" => {
                caps.document_symbol_provider = Some(OneOf::Left(true));
            }
            "lsp.code_action" => {
                caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
            }
            "lsp.formatting" => {
                caps.document_formatting_provider = Some(OneOf::Left(true));
            }
            "lsp.range_formatting" => {
                caps.document_range_formatting_provider = Some(OneOf::Left(true));
            }
            "lsp.rename" => {
                caps.rename_provider = Some(OneOf::Left(true));
            }
            "lsp.folding_range" => {
                caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
            }
            "lsp.semantic_tokens" => {
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
            "lsp.document_highlight" => {
                caps.document_highlight_provider = Some(OneOf::Left(true));
            }
            "lsp.code_lens" => {
                caps.code_lens_provider = Some(CodeLensOptions { resolve_provider: Some(true) });
            }
            "lsp.document_link" => {
                caps.document_link_provider = Some(DocumentLinkOptions {
                    resolve_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                });
            }
            "lsp.color" => {
                caps.color_provider = Some(ColorProviderCapability::Simple(true));
            }
            "lsp.on_type_formatting" => {
                caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
                    first_trigger_character: ";".to_string(),
                    more_trigger_character: Some(vec!["}".to_string()]),
                });
            }
            "lsp.selection_range" => {
                caps.selection_range_provider =
                    Some(SelectionRangeProviderCapability::Simple(true));
            }
            "lsp.linked_editing_range" => {
                caps.linked_editing_range_provider =
                    Some(LinkedEditingRangeServerCapabilities::Simple(true));
            }
            "lsp.call_hierarchy" => {
                caps.call_hierarchy_provider = Some(CallHierarchyServerCapability::Simple(true));
            }
            "lsp.moniker" => {
                caps.moniker_provider =
                    Some(OneOf::Right(MonikerServerCapabilities::Options(MonikerOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    })));
            }
            "lsp.inline_value" => {
                caps.inline_value_provider = Some(OneOf::Right(
                    InlineValueServerCapabilities::Options(InlineValueOptions::default()),
                ));
            }
            "lsp.inlay_hint" => {
                caps.inlay_hint_provider =
                    Some(OneOf::Right(InlayHintServerCapabilities::Options(InlayHintOptions {
                        resolve_provider: Some(true),
                        ..Default::default()
                    })));
            }
            "lsp.pull_diagnostics" => {
                caps.diagnostic_provider =
                    Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("perl-lsp".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    }));
            }
            "lsp.workspace_symbol" => {
                caps.workspace_symbol_provider = Some(OneOf::Left(true));
            }
            "lsp.execute_command" => {
                caps.execute_command_provider = Some(ExecuteCommandOptions {
                    commands: vec!["perl.runCritic".to_string()],
                    ..Default::default()
                });
            }
            _ => {
                // Unknown feature - ignore
            }
        }
    }

    caps
}

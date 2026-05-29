use super::{BuildFlags, completion_trigger_characters, get_supported_commands};
use lsp_types::*;

pub(super) fn apply_document_sync(caps: &mut ServerCapabilities) {
    // Use Options instead of Kind to comply with LSP 3.18 shape requirements.
    // TextDocumentSyncKind::FULL (1): the server always reparses the full document
    // on every didChange notification. INCREMENTAL (2) would be inaccurate — no
    // incremental AST state is maintained between edits.
    caps.text_document_sync = Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::FULL),
        will_save: None,
        will_save_wait_until: None,
        // The server handles didSave for diagnostics refresh and post-save hooks.
        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
    }));
}

pub(super) fn apply_navigation_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    if build.hover {
        caps.hover_provider = Some(HoverProviderCapability::Simple(true));
    }

    if build.document_highlight {
        caps.document_highlight_provider = Some(OneOf::Left(true));
    }

    if build.declaration {
        caps.declaration_provider = Some(DeclarationCapability::Simple(true));
    }

    if build.definition {
        caps.definition_provider = Some(OneOf::Left(true));
    }

    if build.type_definition {
        caps.type_definition_provider = Some(TypeDefinitionProviderCapability::Simple(true));
    }

    if build.implementation {
        caps.implementation_provider = Some(ImplementationProviderCapability::Simple(true));
    }

    if build.references {
        caps.references_provider = Some(OneOf::Left(true));
    }
}

pub(super) fn apply_editing_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    if build.signature_help {
        caps.signature_help_provider = Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: Some(vec![
                ",".to_string(),
                "@".to_string(),
                "%".to_string(),
                "{".to_string(),
                "[".to_string(),
            ]),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
    }

    if build.completion {
        caps.completion_provider = Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(completion_trigger_characters()),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: Some(CompletionOptionsCompletionItem {
                label_details_support: Some(true),
            }),
        });
    }

    if build.formatting {
        caps.document_formatting_provider = Some(OneOf::Left(true));
    }

    if build.range_formatting {
        caps.document_range_formatting_provider = Some(OneOf::Left(true));
    }

    if build.rename {
        caps.rename_provider = Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }));
    }

    if build.on_type_formatting {
        caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: "}".to_string(),
            more_trigger_character: Some(vec![";".to_string(), "\n".to_string()]),
        });
    }

    if build.linked_editing {
        caps.linked_editing_range_provider =
            Some(LinkedEditingRangeServerCapabilities::Simple(true));
    }
}

pub(super) fn apply_symbol_and_workspace_features(
    caps: &mut ServerCapabilities,
    build: &BuildFlags,
) {
    if build.document_symbol {
        caps.document_symbol_provider = Some(OneOf::Left(true));
    }

    if build.workspace_symbol {
        caps.workspace_symbol_provider = Some(OneOf::Left(true));
    }

    if build.workspace_symbol_resolve {
        caps.workspace_symbol_provider = Some(OneOf::Right(WorkspaceSymbolOptions {
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }));
    }

    if build.notebook_document_sync {
        caps.notebook_document_sync = Some(OneOf::Left(NotebookDocumentSyncOptions {
            notebook_selector: vec![NotebookSelector::ByNotebook {
                notebook: Notebook::String("jupyter-notebook".to_string()),
                cells: Some(vec![NotebookCellSelector { language: "perl".to_string() }]),
            }],
            save: Some(true),
        }));
    }
}

pub(super) fn apply_analysis_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    if build.folding_range {
        caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
    }

    if build.inlay_hints {
        caps.inlay_hint_provider =
            Some(OneOf::Right(InlayHintServerCapabilities::Options(InlayHintOptions {
                resolve_provider: Some(true), // Resolver implemented in misc.rs:handle_inlay_hint_resolve
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })));
    }

    if build.pull_diagnostics {
        caps.diagnostic_provider = Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            inter_file_dependencies: false,
            workspace_diagnostics: true,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            identifier: Some("perl-lsp".to_string()),
        }));
    }

    if build.semantic_tokens {
        caps.semantic_tokens_provider =
            Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: semantic_token_types(),
                    token_modifiers: semantic_token_modifiers(),
                },
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            }));
    }
}

pub(super) fn apply_code_action_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    if build.code_actions {
        caps.code_action_provider =
            Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(code_action_kinds(build)),
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if build.execute_command {
        // Only advertise commands that are actually implemented and tested.
        caps.execute_command_provider = Some(ExecuteCommandOptions {
            commands: get_supported_commands(),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
    }
}

pub(super) fn apply_misc_features(caps: &mut ServerCapabilities, build: &BuildFlags) {
    if build.document_links {
        caps.document_link_provider = Some(DocumentLinkOptions {
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
    }

    if build.selection_ranges {
        caps.selection_range_provider = Some(SelectionRangeProviderCapability::Simple(true));
    }

    if build.code_lens {
        caps.code_lens_provider = Some(CodeLensOptions { resolve_provider: Some(true) });
    }

    if build.inline_values {
        caps.inline_value_provider = Some(OneOf::Left(true));
    }

    if build.moniker {
        caps.moniker_provider = Some(OneOf::Left(true));
    }

    if build.document_color {
        caps.color_provider = Some(ColorProviderCapability::Simple(true));
    }

    if build.call_hierarchy {
        caps.call_hierarchy_provider = Some(CallHierarchyServerCapability::Simple(true));
    }
}

fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::TYPE,
        SemanticTokenType::CLASS,
        SemanticTokenType::INTERFACE,
        SemanticTokenType::ENUM,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::TYPE_PARAMETER,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::METHOD,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::MACRO,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::MODIFIER,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::REGEXP,
        SemanticTokenType::OPERATOR,
        SemanticTokenType::new("sql_string"), // DBI/SQL string context (Issue #2337)
        SemanticTokenType::new("sql_heredoc_keyword"), // SQL keyword in <<SQL heredoc (Issue #2059)
        SemanticTokenType::new("json_heredoc_key"), // JSON key in <<JSON heredoc (Issue #2059)
        // SemanticTokenType::LABEL is not available in lsp-types 0.97.
        SemanticTokenType::new("label"),
    ]
}

fn semantic_token_modifiers() -> Vec<SemanticTokenModifier> {
    vec![
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
        SemanticTokenModifier::new("scalarVariable"),
        SemanticTokenModifier::new("arrayVariable"),
        SemanticTokenModifier::new("hashVariable"),
    ]
}

fn code_action_kinds(build: &BuildFlags) -> Vec<CodeActionKind> {
    // Build code action kinds based on flags.
    let mut kinds = vec![CodeActionKind::QUICKFIX];

    if build.source_organize_imports {
        kinds.push(CodeActionKind::SOURCE_ORGANIZE_IMPORTS);
    }

    // Advertise generic `refactor` plus concrete sub-kinds so clients can
    // surface the full refactoring menu and send precise `context.only`
    // filters (for example `refactor.inline` and `refactor.rewrite`).
    kinds.push(CodeActionKind::REFACTOR);

    // REFACTOR_EXTRACT is implemented in code_actions_enhanced.rs.
    // Tests verified in lsp_code_actions_tests.rs (Issue #181).
    kinds.push(CodeActionKind::REFACTOR_EXTRACT);
    kinds.push(CodeActionKind::REFACTOR_INLINE);
    kinds.push(CodeActionKind::REFACTOR_REWRITE);

    // SOURCE_FIX_ALL aggregates every safe `quickfix` action into a single invocation.
    kinds.push(CodeActionKind::SOURCE_FIX_ALL);

    kinds
}

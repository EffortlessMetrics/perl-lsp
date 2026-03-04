//! Comprehensive unit tests for `perl-lsp-capability-map`.
//!
//! Covers both public functions:
//!   - `feature_ids_from_caps`: ServerCapabilities → feature ID list
//!   - `caps_from_feature_ids`: feature ID list → ServerCapabilities

use lsp_types::*;
use perl_lsp_capability_map::{caps_from_feature_ids, feature_ids_from_caps};
use perl_lsp_feature_ids::*;

// ---------------------------------------------------------------------------
// feature_ids_from_caps — empty / default
// ---------------------------------------------------------------------------

#[test]
fn empty_caps_yields_no_features() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities::default();
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.is_empty(), "default ServerCapabilities should map to zero features");
    Ok(())
}

// ---------------------------------------------------------------------------
// feature_ids_from_caps — individual capability detection
// ---------------------------------------------------------------------------

#[test]
fn detects_completion() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_COMPLETION));
    Ok(())
}

#[test]
fn detects_hover() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_HOVER));
    Ok(())
}

#[test]
fn detects_signature_help() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        signature_help_provider: Some(SignatureHelpOptions::default()),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_SIGNATURE_HELP));
    Ok(())
}

#[test]
fn detects_definition() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { definition_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DEFINITION));
    Ok(())
}

#[test]
fn detects_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DECLARATION));
    Ok(())
}

#[test]
fn detects_notebook_document_sync() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        notebook_document_sync: Some(OneOf::Left(NotebookDocumentSyncOptions {
            notebook_selector: vec![],
            save: None,
        })),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_NOTEBOOK_DOCUMENT_SYNC));
    Ok(())
}

#[test]
fn detects_type_definition() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_TYPE_DEFINITION));
    Ok(())
}

#[test]
fn detects_implementation() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_IMPLEMENTATION));
    Ok(())
}

#[test]
fn detects_references() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { references_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_REFERENCES));
    Ok(())
}

#[test]
fn detects_document_highlight() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_highlight_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DOCUMENT_HIGHLIGHT));
    Ok(())
}

#[test]
fn detects_document_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DOCUMENT_SYMBOL));
    Ok(())
}

#[test]
fn detects_code_action() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_CODE_ACTION));
    Ok(())
}

#[test]
fn detects_code_lens() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        code_lens_provider: Some(CodeLensOptions { resolve_provider: None }),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_CODE_LENS));
    Ok(())
}

#[test]
fn detects_document_link() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DOCUMENT_LINK));
    Ok(())
}

#[test]
fn detects_document_color() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        color_provider: Some(ColorProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_DOCUMENT_COLOR));
    Ok(())
}

#[test]
fn detects_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_FORMATTING));
    Ok(())
}

#[test]
fn detects_range_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_range_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_RANGE_FORMATTING));
    Ok(())
}

#[test]
fn detects_on_type_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: ";".to_string(),
            more_trigger_character: None,
        }),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_ON_TYPE_FORMATTING));
    Ok(())
}

#[test]
fn detects_rename() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { rename_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_RENAME));
    Ok(())
}

#[test]
fn detects_folding_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_FOLDING_RANGE));
    Ok(())
}

#[test]
fn detects_selection_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_SELECTION_RANGE));
    Ok(())
}

#[test]
fn detects_linked_editing_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        linked_editing_range_provider: Some(LinkedEditingRangeServerCapabilities::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_LINKED_EDITING_RANGE));
    Ok(())
}

#[test]
fn detects_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_CALL_HIERARCHY));
    Ok(())
}

#[test]
fn detects_semantic_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend { token_types: vec![], token_modifiers: vec![] },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                ..Default::default()
            },
        )),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_SEMANTIC_TOKENS));
    Ok(())
}

#[test]
fn detects_moniker() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { moniker_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_MONIKER));
    Ok(())
}

#[test]
fn detects_inline_value() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { inline_value_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_INLINE_VALUE));
    Ok(())
}

#[test]
fn detects_inlay_hint() -> Result<(), Box<dyn std::error::Error>> {
    let caps =
        ServerCapabilities { inlay_hint_provider: Some(OneOf::Left(true)), ..Default::default() };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_INLAY_HINT));
    Ok(())
}

#[test]
fn detects_pull_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: None,
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            ..Default::default()
        })),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_PULL_DIAGNOSTICS));
    Ok(())
}

#[test]
fn detects_workspace_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        workspace_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_WORKSPACE_SYMBOL));
    Ok(())
}

#[test]
fn detects_execute_command() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![],
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(feature_ids_from_caps(&caps).contains(&LSP_EXECUTE_COMMAND));
    Ok(())
}

// ---------------------------------------------------------------------------
// feature_ids_from_caps — multiple capabilities
// ---------------------------------------------------------------------------

#[test]
fn detects_multiple_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&LSP_HOVER));
    assert!(ids.contains(&LSP_DEFINITION));
    assert!(ids.contains(&LSP_REFERENCES));
    assert_eq!(ids.len(), 3, "exactly three features expected");
    Ok(())
}

#[test]
fn result_is_sorted() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        rename_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions::default()),
        ..Default::default()
    };
    let ids = feature_ids_from_caps(&caps);
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "feature_ids_from_caps output must be sorted");
    Ok(())
}

#[test]
fn result_has_no_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        color_provider: Some(ColorProviderCapability::Simple(true)),
        ..Default::default()
    };
    let ids = feature_ids_from_caps(&caps);
    let mut deduped = ids.clone();
    deduped.dedup();
    assert_eq!(ids, deduped, "feature_ids_from_caps output must have no duplicates");
    Ok(())
}

// ---------------------------------------------------------------------------
// caps_from_feature_ids — empty input
// ---------------------------------------------------------------------------

#[test]
fn empty_features_yields_default_caps() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.is_empty(), "no features should produce default caps");
    Ok(())
}

// ---------------------------------------------------------------------------
// caps_from_feature_ids — individual features
// ---------------------------------------------------------------------------

#[test]
fn builds_completion_with_trigger_characters() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_COMPLETION]);
    let provider = caps.completion_provider.as_ref().ok_or("missing completion_provider")?;
    let triggers = provider.trigger_characters.as_ref().ok_or("missing trigger_characters")?;
    assert!(triggers.contains(&"$".to_string()));
    assert!(triggers.contains(&"@".to_string()));
    assert!(triggers.contains(&"%".to_string()));
    assert!(triggers.contains(&">".to_string()));
    assert!(triggers.contains(&":".to_string()));
    Ok(())
}

#[test]
fn builds_hover() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_HOVER]);
    assert!(caps.hover_provider.is_some());
    Ok(())
}

#[test]
fn builds_signature_help_with_triggers() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_SIGNATURE_HELP]);
    let provider =
        caps.signature_help_provider.as_ref().ok_or("missing signature_help_provider")?;
    let triggers = provider.trigger_characters.as_ref().ok_or("missing trigger_characters")?;
    assert!(triggers.contains(&"(".to_string()));
    assert!(triggers.contains(&",".to_string()));
    Ok(())
}

#[test]
fn builds_definition() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DEFINITION]);
    assert!(caps.definition_provider.is_some());
    Ok(())
}

#[test]
fn builds_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DECLARATION]);
    assert!(caps.declaration_provider.is_some());
    Ok(())
}

#[test]
fn builds_notebook_document_sync() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_NOTEBOOK_DOCUMENT_SYNC]);
    assert!(caps.notebook_document_sync.is_some());
    Ok(())
}

#[test]
fn builds_type_definition() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_TYPE_DEFINITION]);
    assert!(caps.type_definition_provider.is_some());
    Ok(())
}

#[test]
fn builds_implementation() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_IMPLEMENTATION]);
    assert!(caps.implementation_provider.is_some());
    Ok(())
}

#[test]
fn builds_references() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_REFERENCES]);
    assert!(caps.references_provider.is_some());
    Ok(())
}

#[test]
fn builds_document_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DOCUMENT_SYMBOL]);
    assert!(caps.document_symbol_provider.is_some());
    Ok(())
}

#[test]
fn builds_code_action() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_CODE_ACTION]);
    assert!(caps.code_action_provider.is_some());
    Ok(())
}

#[test]
fn builds_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_FORMATTING]);
    assert!(caps.document_formatting_provider.is_some());
    Ok(())
}

#[test]
fn builds_range_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_RANGE_FORMATTING]);
    assert!(caps.document_range_formatting_provider.is_some());
    Ok(())
}

#[test]
fn builds_rename() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_RENAME]);
    assert!(caps.rename_provider.is_some());
    Ok(())
}

#[test]
fn builds_folding_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_FOLDING_RANGE]);
    assert!(caps.folding_range_provider.is_some());
    Ok(())
}

#[test]
fn builds_semantic_tokens_with_legend() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_SEMANTIC_TOKENS]);
    let provider = caps.semantic_tokens_provider.as_ref().ok_or("missing semantic_tokens")?;
    match provider {
        SemanticTokensServerCapabilities::SemanticTokensOptions(opts) => {
            assert!(!opts.legend.token_types.is_empty(), "token_types should be populated");
            assert!(!opts.legend.token_modifiers.is_empty(), "token_modifiers should be populated");
            assert_eq!(opts.full, Some(SemanticTokensFullOptions::Bool(true)));
            assert_eq!(opts.range, Some(true));
        }
        _ => return Err("expected SemanticTokensOptions variant".into()),
    }
    Ok(())
}

#[test]
fn builds_document_highlight() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DOCUMENT_HIGHLIGHT]);
    assert!(caps.document_highlight_provider.is_some());
    Ok(())
}

#[test]
fn builds_code_lens_with_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_CODE_LENS]);
    let provider = caps.code_lens_provider.as_ref().ok_or("missing code_lens_provider")?;
    assert_eq!(provider.resolve_provider, Some(true));
    Ok(())
}

#[test]
fn builds_document_link_with_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DOCUMENT_LINK]);
    let provider = caps.document_link_provider.as_ref().ok_or("missing document_link_provider")?;
    assert_eq!(provider.resolve_provider, Some(true));
    Ok(())
}

#[test]
fn builds_document_color_from_canonical_id() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_DOCUMENT_COLOR]);
    assert!(caps.color_provider.is_some());
    Ok(())
}

#[test]
fn builds_document_color_from_legacy_alias() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_COLOR]);
    assert!(caps.color_provider.is_some(), "legacy LSP_COLOR alias should set color_provider");
    Ok(())
}

#[test]
fn builds_on_type_formatting_with_triggers() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_ON_TYPE_FORMATTING]);
    let provider = caps
        .document_on_type_formatting_provider
        .as_ref()
        .ok_or("missing on_type_formatting_provider")?;
    assert_eq!(provider.first_trigger_character, ";");
    let extra = provider.more_trigger_character.as_ref().ok_or("missing more_trigger_character")?;
    assert!(extra.contains(&"}".to_string()));
    Ok(())
}

#[test]
fn builds_selection_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_SELECTION_RANGE]);
    assert!(caps.selection_range_provider.is_some());
    Ok(())
}

#[test]
fn builds_linked_editing_range() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_LINKED_EDITING_RANGE]);
    assert!(caps.linked_editing_range_provider.is_some());
    Ok(())
}

#[test]
fn builds_call_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_CALL_HIERARCHY]);
    assert!(caps.call_hierarchy_provider.is_some());
    Ok(())
}

#[test]
fn builds_moniker() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_MONIKER]);
    assert!(caps.moniker_provider.is_some());
    Ok(())
}

#[test]
fn builds_inline_value() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_INLINE_VALUE]);
    assert!(caps.inline_value_provider.is_some());
    Ok(())
}

#[test]
fn builds_inlay_hint_with_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_INLAY_HINT]);
    let provider = caps.inlay_hint_provider.as_ref().ok_or("missing inlay_hint_provider")?;
    match provider {
        OneOf::Right(InlayHintServerCapabilities::Options(opts)) => {
            assert_eq!(opts.resolve_provider, Some(true));
        }
        _ => return Err("expected InlayHintOptions variant".into()),
    }
    Ok(())
}

#[test]
fn builds_pull_diagnostics_with_options() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_PULL_DIAGNOSTICS]);
    let provider = caps.diagnostic_provider.as_ref().ok_or("missing diagnostic_provider")?;
    match provider {
        DiagnosticServerCapabilities::Options(opts) => {
            assert_eq!(opts.identifier.as_deref(), Some("perl-lsp"));
            assert!(opts.inter_file_dependencies);
            assert!(opts.workspace_diagnostics);
        }
        _ => return Err("expected DiagnosticOptions variant".into()),
    }
    Ok(())
}

#[test]
fn builds_workspace_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_WORKSPACE_SYMBOL]);
    assert!(caps.workspace_symbol_provider.is_some());
    Ok(())
}

#[test]
fn builds_execute_command_with_commands() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_EXECUTE_COMMAND]);
    let provider =
        caps.execute_command_provider.as_ref().ok_or("missing execute_command_provider")?;
    assert!(provider.commands.contains(&"perl.runCritic".to_string()));
    Ok(())
}

// ---------------------------------------------------------------------------
// caps_from_feature_ids — unknown / unsupported features
// ---------------------------------------------------------------------------

#[test]
fn unknown_feature_id_is_silently_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&["totally.unknown.feature"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.is_empty(), "unknown feature should not set any capability");
    Ok(())
}

#[test]
fn mixed_known_and_unknown_features() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_HOVER, "unknown.feature", LSP_DEFINITION]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&LSP_HOVER));
    assert!(ids.contains(&LSP_DEFINITION));
    assert_eq!(ids.len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------------
// Round-trip: feature_ids → caps → feature_ids
// ---------------------------------------------------------------------------

#[test]
fn round_trip_single_feature() -> Result<(), Box<dyn std::error::Error>> {
    let original = [LSP_COMPLETION];
    let caps = caps_from_feature_ids(&original);
    let recovered = feature_ids_from_caps(&caps);
    assert_eq!(recovered, vec![LSP_COMPLETION]);
    Ok(())
}

#[test]
fn round_trip_all_mapped_features() -> Result<(), Box<dyn std::error::Error>> {
    let all_features = [
        LSP_CALL_HIERARCHY,
        LSP_CODE_ACTION,
        LSP_CODE_LENS,
        LSP_COMPLETION,
        LSP_DECLARATION,
        LSP_DEFINITION,
        LSP_DOCUMENT_COLOR,
        LSP_DOCUMENT_HIGHLIGHT,
        LSP_DOCUMENT_LINK,
        LSP_DOCUMENT_SYMBOL,
        LSP_EXECUTE_COMMAND,
        LSP_FOLDING_RANGE,
        LSP_FORMATTING,
        LSP_HOVER,
        LSP_IMPLEMENTATION,
        LSP_INLAY_HINT,
        LSP_INLINE_VALUE,
        LSP_LINKED_EDITING_RANGE,
        LSP_MONIKER,
        LSP_NOTEBOOK_DOCUMENT_SYNC,
        LSP_ON_TYPE_FORMATTING,
        LSP_PULL_DIAGNOSTICS,
        LSP_RANGE_FORMATTING,
        LSP_REFERENCES,
        LSP_RENAME,
        LSP_SELECTION_RANGE,
        LSP_SEMANTIC_TOKENS,
        LSP_SIGNATURE_HELP,
        LSP_TYPE_DEFINITION,
        LSP_WORKSPACE_SYMBOL,
    ];

    let caps = caps_from_feature_ids(&all_features);
    let mut recovered = feature_ids_from_caps(&caps);
    recovered.sort();

    let mut expected: Vec<&str> = all_features.to_vec();
    expected.sort();

    assert_eq!(recovered, expected, "round-trip should recover all mapped features");
    Ok(())
}

#[test]
fn round_trip_preserves_sort_order() -> Result<(), Box<dyn std::error::Error>> {
    let features = [LSP_RENAME, LSP_HOVER, LSP_COMPLETION];
    let caps = caps_from_feature_ids(&features);
    let ids = feature_ids_from_caps(&caps);
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "round-trip result should always be sorted");
    Ok(())
}

// ---------------------------------------------------------------------------
// caps_from_feature_ids — duplicate inputs
// ---------------------------------------------------------------------------

#[test]
fn duplicate_feature_ids_produce_same_result() -> Result<(), Box<dyn std::error::Error>> {
    let single = caps_from_feature_ids(&[LSP_HOVER]);
    let duplicated = caps_from_feature_ids(&[LSP_HOVER, LSP_HOVER, LSP_HOVER]);
    let ids_single = feature_ids_from_caps(&single);
    let ids_dup = feature_ids_from_caps(&duplicated);
    assert_eq!(ids_single, ids_dup, "duplicate inputs should produce identical caps");
    Ok(())
}

// ---------------------------------------------------------------------------
// Semantic tokens legend completeness
// ---------------------------------------------------------------------------

#[test]
fn semantic_tokens_legend_has_expected_token_types() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_SEMANTIC_TOKENS]);
    let provider = caps.semantic_tokens_provider.as_ref().ok_or("missing semantic_tokens")?;
    match provider {
        SemanticTokensServerCapabilities::SemanticTokensOptions(opts) => {
            let types = &opts.legend.token_types;
            assert!(types.contains(&SemanticTokenType::FUNCTION));
            assert!(types.contains(&SemanticTokenType::VARIABLE));
            assert!(types.contains(&SemanticTokenType::KEYWORD));
            assert!(types.contains(&SemanticTokenType::STRING));
            assert!(types.contains(&SemanticTokenType::NUMBER));
            assert!(types.contains(&SemanticTokenType::COMMENT));
            assert!(types.contains(&SemanticTokenType::REGEXP));
            assert!(types.contains(&SemanticTokenType::OPERATOR));
            assert!(types.contains(&SemanticTokenType::NAMESPACE));
            assert!(types.contains(&SemanticTokenType::METHOD));
            assert!(types.contains(&SemanticTokenType::MACRO));
            assert!(types.contains(&SemanticTokenType::PARAMETER));
            assert_eq!(types.len(), 22, "expected 22 token types in legend");
        }
        _ => return Err("expected SemanticTokensOptions variant".into()),
    }
    Ok(())
}

#[test]
fn semantic_tokens_legend_has_expected_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let caps = caps_from_feature_ids(&[LSP_SEMANTIC_TOKENS]);
    let provider = caps.semantic_tokens_provider.as_ref().ok_or("missing semantic_tokens")?;
    match provider {
        SemanticTokensServerCapabilities::SemanticTokensOptions(opts) => {
            let mods = &opts.legend.token_modifiers;
            assert!(mods.contains(&SemanticTokenModifier::DECLARATION));
            assert!(mods.contains(&SemanticTokenModifier::DEFINITION));
            assert!(mods.contains(&SemanticTokenModifier::READONLY));
            assert!(mods.contains(&SemanticTokenModifier::DEPRECATED));
            assert!(mods.contains(&SemanticTokenModifier::DOCUMENTATION));
            assert_eq!(mods.len(), 10, "expected 10 token modifiers in legend");
        }
        _ => return Err("expected SemanticTokensOptions variant".into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge cases: color alias produces same caps as canonical
// ---------------------------------------------------------------------------

#[test]
fn color_alias_and_canonical_produce_equivalent_caps() -> Result<(), Box<dyn std::error::Error>> {
    let from_alias = caps_from_feature_ids(&[LSP_COLOR]);
    let from_canonical = caps_from_feature_ids(&[LSP_DOCUMENT_COLOR]);
    assert!(from_alias.color_provider.is_some());
    assert!(from_canonical.color_provider.is_some());
    // Both route through the same match arm, so both should be Simple(true)
    let ids_alias = feature_ids_from_caps(&from_alias);
    let ids_canonical = feature_ids_from_caps(&from_canonical);
    assert_eq!(ids_alias, ids_canonical);
    Ok(())
}

// ---------------------------------------------------------------------------
// feature_ids_from_caps — all capabilities at once
// ---------------------------------------------------------------------------

#[test]
fn all_capabilities_set_yields_all_features() -> Result<(), Box<dyn std::error::Error>> {
    let caps = ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: Some(SignatureHelpOptions::default()),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        notebook_document_sync: Some(OneOf::Left(NotebookDocumentSyncOptions {
            notebook_selector: vec![],
            save: None,
        })),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions { resolve_provider: None }),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        color_provider: Some(ColorProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: ";".to_string(),
            more_trigger_character: None,
        }),
        rename_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        linked_editing_range_provider: Some(LinkedEditingRangeServerCapabilities::Simple(true)),
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend { token_types: vec![], token_modifiers: vec![] },
                full: None,
                range: None,
                ..Default::default()
            },
        )),
        moniker_provider: Some(OneOf::Left(true)),
        inline_value_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
            DiagnosticOptions::default(),
        )),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![],
            ..Default::default()
        }),
        ..Default::default()
    };

    let ids = feature_ids_from_caps(&caps);
    assert_eq!(ids.len(), 30, "expected 30 mapped capabilities");
    Ok(())
}

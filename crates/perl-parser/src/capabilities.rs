//! LSP Server Capabilities Configuration
//!
//! Single source of truth for advertised LSP capabilities

use lsp_types::*;
use serde_json::Value;

/// Build flags for conditional capabilities
#[derive(Debug, Clone, Default)]
pub struct BuildFlags {
    pub inlay_hints: bool,
    pub pull_diagnostics: bool,
    pub workspace_symbol_resolve: bool,
    pub semantic_tokens: bool,
    pub code_actions: bool,
    pub rename: bool,
    pub document_links: bool,
    pub selection_ranges: bool,
    pub on_type_formatting: bool,
    pub code_lens: bool,  // Not advertised by default
    pub call_hierarchy: bool,  // Not advertised by default
    pub type_hierarchy: bool,  // Not implemented
}

impl BuildFlags {
    /// Default production-ready capabilities
    pub fn production() -> Self {
        Self {
            inlay_hints: true,
            pull_diagnostics: true,
            workspace_symbol_resolve: true,
            semantic_tokens: true,
            code_actions: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            code_lens: false,  // Partial implementation
            call_hierarchy: false,  // Partial implementation
            type_hierarchy: false,  // Not implemented
        }
    }
    
    /// All capabilities for testing
    pub fn all() -> Self {
        Self {
            inlay_hints: true,
            pull_diagnostics: true,
            workspace_symbol_resolve: true,
            semantic_tokens: true,
            code_actions: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            code_lens: true,
            call_hierarchy: true,
            type_hierarchy: true,
        }
    }
    
    /// Conservative GA-lock capabilities
    pub fn ga_lock() -> Self {
        Self {
            inlay_hints: false,
            pull_diagnostics: false,
            workspace_symbol_resolve: false,
            semantic_tokens: false,
            code_actions: false,
            rename: false,
            document_links: false,
            selection_ranges: false,
            on_type_formatting: false,
            code_lens: false,
            call_hierarchy: false,
            type_hierarchy: false,
        }
    }
}

/// Generate server capabilities from build flags
pub fn capabilities_for(build: BuildFlags) -> ServerCapabilities {
    let mut caps = ServerCapabilities::default();
    
    // Always-on capabilities
    caps.text_document_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
    
    caps.hover_provider = Some(HoverProviderCapability::Simple(true));
    
    caps.completion_provider = Some(CompletionOptions {
        resolve_provider: Some(false),
        trigger_characters: Some(vec![
            "$".to_string(), "@".to_string(), "%".to_string(),
            ":".to_string(), ">".to_string(),
        ]),
        all_commit_characters: None,
        work_done_progress_options: WorkDoneProgressOptions::default(),
        completion_item: None,
    });
    
    caps.definition_provider = Some(OneOf::Left(true));
    caps.references_provider = Some(OneOf::Left(true));
    caps.document_symbol_provider = Some(OneOf::Left(true));
    caps.workspace_symbol_provider = Some(OneOf::Left(true));
    
    caps.document_formatting_provider = Some(OneOf::Left(true));
    caps.document_range_formatting_provider = Some(OneOf::Left(true));
    
    caps.signature_help_provider = Some(SignatureHelpOptions {
        trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
        retrigger_characters: Some(vec![",".to_string()]),
        work_done_progress_options: WorkDoneProgressOptions::default(),
    });
    
    caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
    
    // Conditional capabilities
    if build.inlay_hints {
        caps.inlay_hint_provider = Some(OneOf::Right(InlayHintServerCapabilities::Options(
            InlayHintOptions {
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }
        )));
    }
    
    if build.pull_diagnostics {
        caps.diagnostic_provider = Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            inter_file_dependencies: false,
            workspace_diagnostics: true,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            identifier: None,
        }));
    }
    
    if build.workspace_symbol_resolve {
        caps.workspace_symbol_provider = Some(OneOf::Right(WorkspaceSymbolOptions {
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }));
    }
    
    if build.semantic_tokens {
        caps.semantic_tokens_provider = Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: vec![
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
                        // SemanticTokenType::LABEL, // Not available in lsp-types 0.97
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
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            }
        ));
    }
    
    if build.code_actions {
        caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
    }
    
    if build.rename {
        caps.rename_provider = Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }));
    }
    
    if build.document_links {
        caps.document_link_provider = Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
    }
    
    if build.selection_ranges {
        caps.selection_range_provider = Some(SelectionRangeProviderCapability::Simple(true));
    }
    
    if build.on_type_formatting {
        caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: "}".to_string(),
            more_trigger_character: Some(vec![";".to_string()]),
        });
    }
    
    if build.code_lens {
        caps.code_lens_provider = Some(CodeLensOptions {
            resolve_provider: Some(false),
        });
    }
    
    if build.call_hierarchy {
        caps.call_hierarchy_provider = Some(CallHierarchyServerCapability::Simple(true));
    }
    
    // Type hierarchy not available in lsp-types 0.97
    // if build.type_hierarchy {
    //     caps.type_hierarchy_provider = Some(TypeHierarchyServerCapability::Options(
    //         TypeHierarchyOptions {
    //             work_done_progress_options: WorkDoneProgressOptions::default(),
    //         }
    //     ));
    // }
    
    caps
}

/// Generate capabilities as JSON Value for testing
pub fn capabilities_json(build: BuildFlags) -> Value {
    let caps = capabilities_for(build);
    serde_json::to_value(caps).unwrap()
}

/// Check if a capability is a boolean or object (for flexible assertions)
pub fn cap_bool_or_object(caps: &Value, key: &str) -> bool {
    caps.get(key).map_or(false, |v| v.is_boolean() || v.is_object())
}

/// Default capabilities for the current build
pub fn default_capabilities() -> ServerCapabilities {
    #[cfg(feature = "lsp-ga-lock")]
    let flags = BuildFlags::ga_lock();
    
    #[cfg(not(feature = "lsp-ga-lock"))]
    let flags = BuildFlags::production();
    
    capabilities_for(flags)
}
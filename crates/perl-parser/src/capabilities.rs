//! LSP Server Capabilities Configuration for Email Script Processing
//!
//! This module provides centralized configuration for LSP server capabilities
//! advertised to clients during Perl script development within the LSP workflow.
//! Serves as the single source of truth for feature availability and build-time
//! capability gating for optimal Perl parsing workflows.
//!
//! # LSP Workflow Integration
//!
//! - **Extract**: Provides capabilities for parsing and syntax analysis
//! - **Normalize**: Enables code formatting and standardization features
//! - **Thread**: Supports workspace navigation and symbol resolution
//! - **Render**: Facilitates syntax highlighting and code presentation
//! - **Index**: Powers workspace symbols and cross-file navigation

use lsp_types::*;
use serde_json::Value;

/// LSP features advertised to clients for Perl script development
///
/// Controls which LSP capabilities are announced during server initialization,
/// enabling clients to provide appropriate UI elements and functionality
/// for Perl script editing within LSP workflows.
#[derive(Debug, Clone, Default)]
pub struct AdvertisedFeatures {
    /// Code completion for variables, functions, and keywords
    pub completion: bool,
    /// Hover information for symbols and documentation
    pub hover: bool,
    /// Go-to-definition navigation for symbols
    pub definition: bool,
    /// Find-all-references for symbol usage analysis
    pub references: bool,
    /// Document symbol outline for Perl script structure
    pub document_symbol: bool,
    /// Workspace-wide symbol search across Perl parsing files
    pub workspace_symbol: bool,
    /// Automated code actions and refactoring suggestions
    pub code_action: bool,
    /// Code lens with reference counts and actionable information
    pub code_lens: bool,
    /// Full document formatting with perltidy integration
    pub formatting: bool,
    /// Range-specific formatting for selected code sections
    pub range_formatting: bool,
    /// Symbol renaming with workspace-wide updates
    pub rename: bool,
    /// Code folding for improved Perl script navigation
    pub folding_range: bool,
    /// Smart text selection expansion for efficient editing
    pub selection_range: bool,
    /// Linked editing for synchronized symbol updates
    pub linked_editing: bool,
    /// Inline type and parameter hints for clarity
    pub inlay_hints: bool,
    /// Semantic syntax highlighting for Perl scripts
    pub semantic_tokens: bool,
    /// Call hierarchy navigation for function relationships
    pub call_hierarchy: bool,
    /// Type hierarchy for object-oriented Perl parsing
    pub type_hierarchy: bool,
    /// Pull-based diagnostic reporting for error detection
    pub diagnostic_provider: bool,
}

/// Build-time feature flags for conditional LSP capability compilation
///
/// Controls which capabilities are compiled into the LSP server binary,
/// allowing for optimized builds targeted at specific Perl parsing
/// deployment scenarios within enterprise LSP environments.
#[derive(Debug, Clone, Default)]
pub struct BuildFlags {
    /// Code completion provider compilation flag
    pub completion: bool,
    /// Hover information provider compilation flag
    pub hover: bool,
    /// Go-to-definition provider compilation flag
    pub definition: bool,
    /// Type definition navigation compilation flag
    pub type_definition: bool,
    /// Implementation finding compilation flag
    pub implementation: bool,
    /// Find-all-references provider compilation flag
    pub references: bool,
    /// Document symbol outline provider compilation flag
    pub document_symbol: bool,
    /// Workspace symbol search provider compilation flag
    pub workspace_symbol: bool,
    /// Inlay hints provider compilation flag
    pub inlay_hints: bool,
    /// Pull-based diagnostics provider compilation flag
    pub pull_diagnostics: bool,
    /// Workspace symbol resolution provider compilation flag
    pub workspace_symbol_resolve: bool,
    /// Semantic token highlighting provider compilation flag
    pub semantic_tokens: bool,
    /// Code actions provider compilation flag
    pub code_actions: bool,
    /// Command execution provider compilation flag
    pub execute_command: bool,
    /// Symbol renaming provider compilation flag
    pub rename: bool,
    /// Document links provider compilation flag
    pub document_links: bool,
    /// Smart text selection ranges provider compilation flag
    pub selection_ranges: bool,
    /// On-type formatting provider compilation flag
    pub on_type_formatting: bool,
    /// Code lens provider compilation flag
    pub code_lens: bool,
    /// Call hierarchy navigation provider compilation flag
    pub call_hierarchy: bool,
    /// Type hierarchy navigation provider compilation flag
    pub type_hierarchy: bool,
    /// Linked editing ranges provider compilation flag
    pub linked_editing: bool,
    /// Inline completion suggestions provider compilation flag
    pub inline_completion: bool,
    /// Inline values for debugging provider compilation flag
    pub inline_values: bool,
    /// Stable symbol identifiers provider compilation flag
    pub moniker: bool,
    /// Document color provider compilation flag for color swatches in strings and comments
    pub document_color: bool,
    /// Source organize imports capability (GA-lock excludes this)
    pub source_organize_imports: bool,
    /// Document formatting provider compilation flag
    pub formatting: bool,
    /// Range formatting provider compilation flag
    pub range_formatting: bool,
    /// Folding range provider compilation flag
    pub folding_range: bool,
}

impl BuildFlags {
    /// Convert build flags to advertised features
    pub fn to_advertised_features(&self) -> AdvertisedFeatures {
        AdvertisedFeatures {
            completion: self.completion,
            hover: self.hover,
            definition: self.definition,
            references: self.references,
            document_symbol: self.document_symbol,
            workspace_symbol: self.workspace_symbol,
            code_action: self.code_actions,
            code_lens: self.code_lens,
            formatting: self.formatting,
            range_formatting: self.range_formatting,
            rename: self.rename,
            folding_range: self.folding_range,
            selection_range: self.selection_ranges,
            linked_editing: self.linked_editing,
            inlay_hints: self.inlay_hints,
            semantic_tokens: self.semantic_tokens,
            call_hierarchy: self.call_hierarchy,
            type_hierarchy: self.type_hierarchy,
            diagnostic_provider: self.pull_diagnostics,
        }
    }

    /// Default production-ready capabilities
    pub fn production() -> Self {
        Self {
            completion: true,
            hover: true,
            definition: true,
            type_definition: true,
            implementation: true,
            references: true,
            document_symbol: true,
            workspace_symbol: true,
            inlay_hints: true,
            pull_diagnostics: true,
            workspace_symbol_resolve: true,
            semantic_tokens: true,
            code_actions: true,
            execute_command: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            code_lens: true,         // Reference counts & run/test lenses (v0.8.9)
            call_hierarchy: true,    // Call hierarchy support (v0.8.9)
            type_hierarchy: true,    // Type hierarchy support (v0.8.9)
            linked_editing: true,    // Implemented for paired delimiters
            inline_completion: true, // Deterministic inline completions
            inline_values: true,     // Debug inline values
            moniker: true,           // Stable symbol identifiers
            document_color: false,   // Handler not implemented (returns -32601)
            source_organize_imports: true, // Fully implemented and tested
            formatting: false,       // Set based on perltidy availability
            range_formatting: false, // Set based on perltidy availability
            folding_range: true,
        }
    }

    /// All capabilities for testing
    pub fn all() -> Self {
        Self {
            completion: true,
            hover: true,
            definition: true,
            type_definition: true,
            implementation: true,
            references: true,
            document_symbol: true,
            workspace_symbol: true,
            inlay_hints: true,
            pull_diagnostics: true,
            workspace_symbol_resolve: true,
            semantic_tokens: true,
            code_actions: true,
            execute_command: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            code_lens: true,
            call_hierarchy: true,
            type_hierarchy: true,
            linked_editing: true,
            inline_completion: true,
            inline_values: true,
            moniker: true,
            document_color: true,
            source_organize_imports: true,
            formatting: true,
            range_formatting: true,
            folding_range: true,
        }
    }

    /// Conservative GA-lock capabilities
    pub fn ga_lock() -> Self {
        Self {
            completion: true,
            hover: true,
            definition: true,
            type_definition: false, // New feature, not GA yet
            implementation: false,  // New feature, not GA yet
            references: true,
            document_symbol: true,
            workspace_symbol: true, // Working via index
            inlay_hints: true,      // v0.8.4 feature - working
            pull_diagnostics: true, // v0.8.5 feature - working
            workspace_symbol_resolve: true,
            semantic_tokens: true,          // v0.8.4 feature - working
            code_actions: true, // v0.8.4 feature - working (enhanced v0.8.9 with refactoring)
            execute_command: true, // v0.8.5 feature - working
            rename: true, // v0.8.4 feature - working (enhanced v0.8.9 with workspace refactoring)
            document_links: true, // v0.8.4 feature - working
            selection_ranges: true, // v0.8.4 feature - working
            on_type_formatting: true, // v0.8.4 feature - working
            code_lens: false, // Only ~20% functional → don't advertise in GA-lock
            call_hierarchy: true, // Call hierarchy support (v0.8.9)
            type_hierarchy: true, // Type hierarchy support (v0.8.9)
            linked_editing: false, // Not GA yet
            inline_completion: false, // New feature, not GA yet
            inline_values: false, // New feature, not GA yet
            moniker: false, // New feature, not GA yet
            document_color: false, // New feature, not GA yet
            source_organize_imports: false, // Excluded from GA-lock contract
            formatting: false,
            range_formatting: false,
            folding_range: true,
        }
    }
}

/// Generate server capabilities from build flags
#[allow(clippy::field_reassign_with_default)]
pub fn capabilities_for(build: BuildFlags) -> ServerCapabilities {
    let mut caps = ServerCapabilities::default();

    // Always-on capabilities
    // Use Options instead of Kind to comply with LSP 3.18 shape requirements
    caps.text_document_sync = Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::FULL),
        will_save: None,
        will_save_wait_until: None,
        save: None,
    }));

    caps.hover_provider = Some(HoverProviderCapability::Simple(true));
    caps.document_highlight_provider = Some(OneOf::Left(true));

    caps.signature_help_provider = Some(SignatureHelpOptions {
        trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
        retrigger_characters: Some(vec![",".to_string()]),
        work_done_progress_options: WorkDoneProgressOptions::default(),
    });

    caps.completion_provider = Some(CompletionOptions {
        resolve_provider: Some(false),
        trigger_characters: Some(vec![
            "$".to_string(),
            "@".to_string(),
            "%".to_string(),
            "->".to_string(),
        ]),
        all_commit_characters: None,
        work_done_progress_options: WorkDoneProgressOptions::default(),
        completion_item: None,
    });

    caps.definition_provider = Some(OneOf::Left(true));

    if build.type_definition {
        caps.type_definition_provider =
            Some(lsp_types::TypeDefinitionProviderCapability::Simple(true));
    }

    if build.implementation {
        caps.implementation_provider =
            Some(lsp_types::ImplementationProviderCapability::Simple(true));
    }

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
        caps.inlay_hint_provider =
            Some(OneOf::Right(InlayHintServerCapabilities::Options(InlayHintOptions {
                resolve_provider: Some(true),
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

    if build.workspace_symbol_resolve {
        caps.workspace_symbol_provider = Some(OneOf::Right(WorkspaceSymbolOptions {
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }));
    }

    if build.semantic_tokens {
        caps.semantic_tokens_provider =
            Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
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
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            }));
    }

    if build.code_actions {
        // Build code action kinds based on flags
        let mut kinds = vec![CodeActionKind::QUICKFIX];

        // Only advertise refactoring capabilities that are fully implemented and tested
        // NOTE: REFACTOR_EXTRACT is implemented in code_actions_enhanced.rs but not advertised.
        // To enable: verify lsp_code_actions_tests.rs tests pass, then add CodeActionKind::REFACTOR_EXTRACT
        // See Issue #181 for tracking workspace feature completeness
        if build.source_organize_imports {
            kinds.push(CodeActionKind::SOURCE_ORGANIZE_IMPORTS);
        }

        caps.code_action_provider =
            Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(kinds),
                resolve_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }));
    }

    #[cfg(not(target_arch = "wasm32"))]
    if build.execute_command {
        // Only advertise commands that are actually implemented and tested
        let commands = crate::execute_command::get_supported_commands();
        caps.execute_command_provider = Some(ExecuteCommandOptions {
            commands,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
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
        caps.code_lens_provider = Some(CodeLensOptions { resolve_provider: Some(true) });
    }

    if build.linked_editing {
        caps.linked_editing_range_provider =
            Some(lsp_types::LinkedEditingRangeServerCapabilities::Simple(true));
    }

    // Inline completion via experimental until lsp-types has the field
    if build.inline_completion {
        let mut experimental = caps.experimental.take().unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = experimental.as_object_mut() {
            obj.insert("inlineCompletionProvider".to_string(), serde_json::json!({}));
        }
        caps.experimental = Some(experimental);
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

    // Placeholder for type hierarchy: always add to JSON in capabilities_json
    // Type hierarchy not directly supported in current lsp-types

    caps
}

/// Generate capabilities as JSON Value for testing
pub fn capabilities_json(build: BuildFlags) -> Value {
    let caps = capabilities_for(build.clone());
    let mut json = serde_json::to_value(caps).unwrap_or_else(|e| {
        eprintln!("Failed to serialize capabilities to JSON: {}", e);
        serde_json::json!({})
    });

    // Manually add typeHierarchyProvider for LSP compatibility
    if build.type_hierarchy {
        json["typeHierarchyProvider"] = serde_json::json!({
            "workDoneProgressOptions": {}
        });
    }

    json
}

/// Check if a capability is a boolean or object (for flexible assertions)
pub fn cap_bool_or_object(caps: &Value, key: &str) -> bool {
    caps.get(key).is_some_and(|v| v.is_boolean() || v.is_object())
}

/// Default capabilities for the current build
pub fn default_capabilities() -> ServerCapabilities {
    #[cfg(feature = "lsp-ga-lock")]
    let flags = BuildFlags::ga_lock();

    #[cfg(not(feature = "lsp-ga-lock"))]
    let flags = BuildFlags::production();

    capabilities_for(flags)
}

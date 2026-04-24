//! LSP Server Capabilities Configuration for Perl Tooling
//!
//! This module provides centralized configuration for LSP server capabilities
//! advertised to clients during Perl script development within the LSP workflow.
//! Serves as the single source of truth for feature availability and build-time
//! capability gating for optimal Perl parsing workflows.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Provides capabilities for parsing and syntax analysis
//! - **Index**: Powers workspace symbols and cross-file navigation
//! - **Navigate**: Supports definition, reference, and hierarchy lookups
//! - **Complete**: Enables completion, signature help, and inline hints
//! - **Analyze**: Drives diagnostics, code actions, and refactoring support

use lsp_types::*;
use serde_json::Value;

pub use crate::features::flags::{AdvertisedFeatures, BuildFlags};
/// Re-export `ServerCapabilities` from `lsp_types` for public access.
pub use lsp_types::ServerCapabilities;

/// Canonical completion trigger characters advertised to LSP clients.
///
/// LSP requires each trigger to be a single character. Multi-character Perl
/// operators (`->`, `::`) are supported by advertising their component chars.
#[must_use]
pub fn completion_trigger_characters() -> Vec<String> {
    vec![
        "$".to_string(),
        "@".to_string(),
        "%".to_string(),
        // Method and package separators.
        "-".to_string(),
        ">".to_string(),
        ":".to_string(),
        // File path completion inside string literals.
        "/".to_string(),
        "\\".to_string(),
        "\"".to_string(),
        "'".to_string(),
    ]
}
/// Generate server capabilities from build flags
#[allow(clippy::field_reassign_with_default)]
pub fn capabilities_for(build: BuildFlags) -> ServerCapabilities {
    let mut caps = ServerCapabilities::default();

    // Always-on capabilities
    // Use Options instead of Kind to comply with LSP 3.18 shape requirements.
    // TextDocumentSyncKind::FULL (1): the server always reparses the full document
    // on every didChange notification.  INCREMENTAL (2) would be inaccurate — no
    // incremental AST state is maintained between edits.
    caps.text_document_sync = Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::FULL),
        will_save: None,
        will_save_wait_until: None,
        save: None,
    }));

    if build.hover {
        caps.hover_provider = Some(HoverProviderCapability::Simple(true));
    }

    if build.document_highlight {
        caps.document_highlight_provider = Some(OneOf::Left(true));
    }

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

    if build.declaration {
        caps.declaration_provider = Some(DeclarationCapability::Simple(true));
    }

    if build.completion {
        caps.completion_provider = Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(completion_trigger_characters()),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: None,
        });
    }

    if build.definition {
        caps.definition_provider = Some(OneOf::Left(true));
    }

    if build.type_definition {
        caps.type_definition_provider =
            Some(lsp_types::TypeDefinitionProviderCapability::Simple(true));
    }

    if build.implementation {
        caps.implementation_provider =
            Some(lsp_types::ImplementationProviderCapability::Simple(true));
    }

    if build.references {
        caps.references_provider = Some(OneOf::Left(true));
    }
    if build.document_symbol {
        caps.document_symbol_provider = Some(OneOf::Left(true));
    }
    if build.workspace_symbol {
        caps.workspace_symbol_provider = Some(OneOf::Left(true));
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

    if build.formatting {
        caps.document_formatting_provider = Some(OneOf::Left(true));
    }
    if build.range_formatting {
        caps.document_range_formatting_provider = Some(OneOf::Left(true));
    }

    if build.folding_range {
        caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
    }

    // Conditional capabilities
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
                        SemanticTokenType::new("sql_string"), // DBI/SQL string context (Issue #2337)
                        SemanticTokenType::new("sql_heredoc_keyword"), // SQL keyword in <<SQL heredoc (Issue #2059)
                        SemanticTokenType::new("json_heredoc_key"), // JSON key in <<JSON heredoc (Issue #2059)
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
                        SemanticTokenModifier::new("scalarVariable"),
                        SemanticTokenModifier::new("arrayVariable"),
                        SemanticTokenModifier::new("hashVariable"),
                    ],
                },
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
            }));
    }

    if build.code_actions {
        // Build code action kinds based on flags
        let mut kinds = vec![CodeActionKind::QUICKFIX];

        if build.source_organize_imports {
            kinds.push(CodeActionKind::SOURCE_ORGANIZE_IMPORTS);
        }

        // Advertise generic `refactor` plus concrete sub-kinds so clients can
        // surface the full refactoring menu and send precise `context.only`
        // filters (for example `refactor.inline` and `refactor.rewrite`).
        kinds.push(CodeActionKind::REFACTOR);

        // REFACTOR_EXTRACT is implemented in code_actions_enhanced.rs
        // Tests verified in lsp_code_actions_tests.rs (Issue #181)
        kinds.push(CodeActionKind::REFACTOR_EXTRACT);
        kinds.push(CodeActionKind::REFACTOR_INLINE);
        kinds.push(CodeActionKind::REFACTOR_REWRITE);

        // SOURCE_FIX_ALL aggregates every safe `quickfix` action into a
        // single invocation. Implemented in
        // `crates/perl-lsp-rs/src/runtime/language/code_actions.rs`
        // (`build_source_fix_all`) so client "fix all" keybindings work
        // without an extra round-trip.
        kinds.push(CodeActionKind::SOURCE_FIX_ALL);

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
        let commands = get_supported_commands();
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
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        });
    }

    if build.selection_ranges {
        caps.selection_range_provider = Some(SelectionRangeProviderCapability::Simple(true));
    }

    if build.on_type_formatting {
        caps.document_on_type_formatting_provider = Some(DocumentOnTypeFormattingOptions {
            first_trigger_character: "}".to_string(),
            more_trigger_character: Some(vec![";".to_string(), "\n".to_string()]),
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

    // Type hierarchy via experimental: lsp-types 0.97 lacks a `type_hierarchy_provider`
    // field on `ServerCapabilities`. We advertise it via `experimental` so that
    // `capabilities_for()` users and `feature_ids_from_caps` can detect the capability.
    // The `handle_initialize` response also injects it at the top-level for clients.
    if build.type_hierarchy {
        let mut experimental = caps.experimental.take().unwrap_or_else(|| serde_json::json!({}));
        if let Some(obj) = experimental.as_object_mut() {
            obj.insert("typeHierarchyProvider".to_string(), serde_json::json!(true));
        }
        caps.experimental = Some(experimental);
    }

    caps
}

/// Generate capabilities as JSON Value for testing
pub fn capabilities_json(build: BuildFlags) -> Value {
    let caps = capabilities_for(build.clone());
    let mut json = serde_json::to_value(caps).unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to serialize capabilities to JSON");
        serde_json::json!({})
    });

    // Manually add typeHierarchyProvider for LSP compatibility
    if build.type_hierarchy {
        json["typeHierarchyProvider"] = serde_json::json!({
            "workDoneProgressOptions": {}
        });
    }

    // Manually add documentRangesFormattingProvider (LSP 3.18) because lsp-types 0.97
    // predates this field.  The handler already exists in formatting.rs.
    if build.range_formatting {
        json["documentRangesFormattingProvider"] = serde_json::json!(true);
    }

    json
}

/// Get the list of supported commands for the LSP executeCommand capability.
///
/// Returns all command identifiers that can be executed via the LSP executeCommand
/// method. This list is used for capability registration and command validation.
pub fn get_supported_commands() -> Vec<String> {
    vec![
        "perl.runTests".to_string(),
        "perl.runFile".to_string(),
        "perl.runTestSub".to_string(),
        "perl.runCritic".to_string(),
        "perl.runTest".to_string(),
        "perl.runTestFile".to_string(),
        "perl.runSubtest".to_string(),
        "perl.debugFile".to_string(),
        "perl.debugTest".to_string(),
        "perl.goToTest".to_string(),
        "perl.goToImplementation".to_string(),
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::contracts::feature_ids_from_caps;
    use std::collections::BTreeSet;

    /// Feature IDs that `to_feature_ids()` correctly emits but
    /// `feature_ids_from_caps()` cannot detect because lsp-types 0.97
    /// lacks the corresponding `ServerCapabilities` field.
    ///
    /// - `inline_completion`: advertised via `experimental` JSON, no typed field
    /// - `notebook_cell_execution`: sub-feature of notebook sync, no own field
    /// - `ranges_formatting`: injected in `capabilities_json()` (LSP 3.18, not in lsp-types 0.97)
    ///
    /// Note: `type_hierarchy` was previously a gap but is now advertised via
    /// `experimental` in `capabilities_for()` and detected by `feature_ids_from_caps`.
    const KNOWN_STRUCTURAL_GAPS: &[&str] =
        &["lsp.inline_completion", "lsp.notebook_cell_execution", "lsp.ranges_formatting"];

    /// Guard: feature IDs from BuildFlags must match feature IDs extracted
    /// from the ServerCapabilities that `capabilities_for()` actually builds.
    ///
    /// Any mismatch means `--features-json` under-reports or over-reports
    /// vs the actual initialize response.
    fn assert_feature_id_alignment(profile: &str, flags: BuildFlags) {
        let flag_ids: BTreeSet<&str> = flags.to_feature_ids().into_iter().collect();
        let caps = capabilities_for(flags);
        let cap_ids: BTreeSet<&str> = feature_ids_from_caps(&caps).into_iter().collect();

        let gaps: BTreeSet<&str> = KNOWN_STRUCTURAL_GAPS.iter().copied().collect();

        let in_flags_not_caps: BTreeSet<_> =
            flag_ids.difference(&cap_ids).copied().filter(|id| !gaps.contains(id)).collect();
        let in_caps_not_flags: BTreeSet<_> = cap_ids.difference(&flag_ids).collect();

        assert!(
            in_flags_not_caps.is_empty() && in_caps_not_flags.is_empty(),
            "feature ID mismatch for {profile} profile:\n  \
             in to_feature_ids() but not in capabilities: {in_flags_not_caps:?}\n  \
             in capabilities but not in to_feature_ids(): {in_caps_not_flags:?}",
        );
    }

    #[test]
    fn feature_id_alignment_ga_lock() {
        assert_feature_id_alignment("ga-lock", BuildFlags::ga_lock());
    }

    #[test]
    fn feature_id_alignment_production() {
        assert_feature_id_alignment("production", BuildFlags::production());
    }

    #[test]
    fn feature_id_alignment_all() {
        assert_feature_id_alignment("all", BuildFlags::all());
    }

    /// Verify that `documentRangesFormattingProvider` is present in the JSON
    /// capabilities when `range_formatting` is enabled (LSP 3.18 gap fix).
    #[test]
    fn ranges_formatting_advertised_in_json_when_enabled() {
        let flags = BuildFlags { range_formatting: true, ..BuildFlags::default() };
        let json = capabilities_json(flags);
        assert!(
            json.get("documentRangesFormattingProvider").is_some(),
            "documentRangesFormattingProvider must be present in capabilities JSON when \
             range_formatting is enabled"
        );
    }

    /// Verify that `documentRangesFormattingProvider` is absent when disabled.
    #[test]
    fn ranges_formatting_absent_in_json_when_disabled() {
        let flags = BuildFlags { range_formatting: false, ..BuildFlags::default() };
        let json = capabilities_json(flags);
        assert!(
            json.get("documentRangesFormattingProvider").is_none(),
            "documentRangesFormattingProvider must not be present when range_formatting is disabled"
        );
    }

    /// Verify that `perl.runSubtest` is included in the supported commands list.
    #[test]
    fn test_subtest_lens_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.runSubtest"),
            "perl.runSubtest must be in get_supported_commands"
        );
    }

    /// Verify resolve providers are advertised in the full capabilities JSON.
    #[test]
    fn resolve_providers_advertised_in_full_profile() {
        let json = capabilities_json(BuildFlags::all());
        assert!(
            json["completionProvider"]["resolveProvider"].as_bool().unwrap_or(false),
            "completionProvider.resolveProvider must be true"
        );
        assert!(
            json["codeActionProvider"]["resolveProvider"].as_bool().unwrap_or(false),
            "codeActionProvider.resolveProvider must be true"
        );
        assert!(
            json["codeLensProvider"]["resolveProvider"].as_bool().unwrap_or(false),
            "codeLensProvider.resolveProvider must be true"
        );
    }

    #[test]
    fn completion_trigger_characters_include_file_path_and_perl_tokens() {
        let triggers = completion_trigger_characters();
        for expected in ["$", "@", "%", "-", ">", ":", "/", "\\", "\"", "'"] {
            assert!(
                triggers.iter().any(|trigger| trigger == expected),
                "missing completion trigger character: {expected}"
            );
        }
    }
}

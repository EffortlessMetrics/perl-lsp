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

use serde_json::Value;

mod experimental;
mod sections;

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
pub fn capabilities_for(build: BuildFlags) -> ServerCapabilities {
    let mut caps = ServerCapabilities::default();

    sections::apply_document_sync(&mut caps);
    sections::apply_navigation_features(&mut caps, &build);
    sections::apply_editing_features(&mut caps, &build);
    sections::apply_symbol_and_workspace_features(&mut caps, &build);
    sections::apply_analysis_features(&mut caps, &build);
    sections::apply_code_action_features(&mut caps, &build);
    sections::apply_misc_features(&mut caps, &build);
    experimental::apply_experimental_features(&mut caps, &build);

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
    // Manually add inlineCompletionProvider (LSP 3.18) because lsp-types 0.97
    // predates this field. This JSON surface has no client context and
    // represents the static/default advertisement; runtime initialize removes
    // it when a client opts into dynamic inline-completion registration.
    if build.inline_completion {
        json["inlineCompletionProvider"] = serde_json::json!({});
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
        "perl.explainProviderDecision".to_string(),
        "perl.workspaceTrustReport".to_string(),
        "perl.previewSafeDelete".to_string(),
        "perl.safeDeleteSymbol".to_string(),
        "perl.previewPackageRename".to_string(),
        "perl.explainMissingModuleLookup".to_string(),
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
    use lsp_types::{TextDocumentSyncCapability, TextDocumentSyncSaveOptions};
    use std::collections::BTreeSet;

    /// Feature IDs that `to_feature_ids()` correctly emits but
    /// `feature_ids_from_caps()` cannot detect because lsp-types 0.97
    /// lacks the corresponding `ServerCapabilities` field.
    ///
    /// - `inline_completion`: injected in `capabilities_json()` (LSP 3.18, not in lsp-types 0.97)
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

    #[test]
    fn inline_completion_advertised_as_top_level_json_when_enabled() {
        let flags = BuildFlags { inline_completion: true, ..BuildFlags::default() };
        let json = capabilities_json(flags);
        assert_eq!(
            json.get("inlineCompletionProvider"),
            Some(&serde_json::json!({})),
            "inlineCompletionProvider must be advertised as an empty top-level object when enabled"
        );
    }

    #[test]
    fn inline_completion_absent_when_disabled() {
        let flags = BuildFlags { inline_completion: false, ..BuildFlags::default() };
        let json = capabilities_json(flags);
        assert!(
            json.get("inlineCompletionProvider").is_none(),
            "inlineCompletionProvider must be absent when inline completion is disabled"
        );
    }

    #[test]
    fn inline_completion_not_advertised_under_experimental() {
        let flags = BuildFlags { inline_completion: true, ..BuildFlags::default() };
        let json = capabilities_json(flags);
        assert!(
            json.pointer("/experimental/inlineCompletionProvider").is_none(),
            "inlineCompletionProvider must not be advertised under capabilities.experimental"
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

    /// Verify that `perl.explainProviderDecision` is included in the supported commands list.
    #[test]
    fn explain_provider_decision_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.explainProviderDecision"),
            "perl.explainProviderDecision must be in get_supported_commands"
        );
    }

    /// Verify that `perl.workspaceTrustReport` is included in the supported commands list.
    #[test]
    fn workspace_trust_report_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.workspaceTrustReport"),
            "perl.workspaceTrustReport must be in get_supported_commands"
        );
    }

    /// Verify that `perl.previewSafeDelete` is included in the supported commands list.
    #[test]
    fn preview_safe_delete_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.previewSafeDelete"),
            "perl.previewSafeDelete must be in get_supported_commands"
        );
    }

    /// Verify that `perl.safeDeleteSymbol` is included in the supported commands list.
    #[test]
    fn safe_delete_symbol_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.safeDeleteSymbol"),
            "perl.safeDeleteSymbol must be in get_supported_commands"
        );
    }

    /// Verify that `perl.previewPackageRename` is included in the supported commands list.
    #[test]
    fn preview_package_rename_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.previewPackageRename"),
            "perl.previewPackageRename must be in get_supported_commands"
        );
    }

    /// Verify that `perl.explainMissingModuleLookup` is included in the supported commands list.
    #[test]
    fn explain_missing_module_lookup_command_id_is_registered() {
        let cmds = get_supported_commands();
        assert!(
            cmds.iter().any(|c| c == "perl.explainMissingModuleLookup"),
            "perl.explainMissingModuleLookup must be in get_supported_commands"
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

    #[test]
    fn text_document_sync_advertises_did_save_support() {
        let caps = capabilities_for(BuildFlags::default());
        let save = caps.text_document_sync.as_ref().and_then(|sync| match sync {
            TextDocumentSyncCapability::Options(opts) => opts.save.as_ref(),
            TextDocumentSyncCapability::Kind(_) => None,
        });

        assert_eq!(
            save,
            Some(&TextDocumentSyncSaveOptions::Supported(true)),
            "textDocumentSync.save must advertise didSave support"
        );
    }
}

//! Snapshot tests for LSP server capabilities.
//!
//! These tests capture the full set of capabilities advertised by the LSP
//! server so that changes to advertised capabilities are visible as intentional
//! diff in code review rather than silent regressions.
//!
//! Run with `cargo test -p perl-lsp-rs --test lsp_cap_snap` to execute.
//! Update snapshots with `cargo insta review` after intentional changes.

use insta::assert_yaml_snapshot;
use serde_json::json;

mod support;
use support::lsp_harness::LspHarness;

// ---------------------------------------------------------------------------
// Capability profile: minimal client (no optional features declared)
//
// Most returned capabilities are driven by build flags and the active feature
// profile. Inline completion is static for this minimal client and dynamically
// registered for the full client fixture below.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_server_capabilities_minimal_client() -> Result<(), Box<dyn std::error::Error>> {
    let minimal_caps = json!({});
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(minimal_caps))?;

    let caps = &init_result["capabilities"];
    assert_yaml_snapshot!("server_capabilities_minimal_client", caps);
    Ok(())
}

// ---------------------------------------------------------------------------
// Capability profile: full client (all optional features declared)
//
// The full fixture includes textDocument.inlineCompletion with
// dynamicRegistration=true, so initialize omits static inlineCompletionProvider.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_server_capabilities_full_client() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    assert_yaml_snapshot!("server_capabilities_full_client", caps);
    Ok(())
}

// ---------------------------------------------------------------------------
// Code action kinds: the set of code action kinds must remain stable
// ---------------------------------------------------------------------------

#[test]
fn snapshot_code_action_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let kinds = caps.get("codeActionProvider").and_then(|p| p.get("codeActionKinds"));
    assert!(
        kinds.is_some(),
        "codeActionProvider.codeActionKinds must be present in server capabilities"
    );
    assert_yaml_snapshot!("code_action_kinds", &kinds);
    Ok(())
}

// ---------------------------------------------------------------------------
// Completion trigger characters: changes affect editor UX immediately
// ---------------------------------------------------------------------------

#[test]
fn snapshot_completion_trigger_characters() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let triggers = caps.get("completionProvider").and_then(|p| p.get("triggerCharacters"));
    assert!(
        triggers.is_some(),
        "completionProvider.triggerCharacters must be present in server capabilities"
    );
    assert_yaml_snapshot!("completion_trigger_characters", &triggers);
    Ok(())
}

// ---------------------------------------------------------------------------
// Signature help triggers: changes alter call-hint behavior across editors
// ---------------------------------------------------------------------------

#[test]
fn snapshot_signature_help_triggers() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let signature_help = caps.get("signatureHelpProvider");
    assert!(
        signature_help.is_some(),
        "signatureHelpProvider must be present in server capabilities"
    );
    assert_yaml_snapshot!("signature_help_provider", &signature_help);
    Ok(())
}

// ---------------------------------------------------------------------------
// Semantic tokens legend as advertised in the initialize response.
// Any reordering of token types or modifiers is a breaking change for clients.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_semantic_tokens_legend() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let legend = caps.get("semanticTokensProvider").and_then(|p| p.get("legend"));
    assert!(
        legend.is_some(),
        "semanticTokensProvider.legend must be present in server capabilities; \
         removing it is a breaking change for all connected clients"
    );
    assert_yaml_snapshot!("semantic_tokens_legend_from_capabilities", &legend);
    Ok(())
}

// ---------------------------------------------------------------------------
// Server info: name and version must be present
// ---------------------------------------------------------------------------

#[test]
fn snapshot_server_info() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let server_info = &init_result["serverInfo"];
    assert!(
        server_info.get("name").and_then(|n| n.as_str()).is_some(),
        "serverInfo.name must be present"
    );
    let name = server_info.get("name");
    assert_yaml_snapshot!("server_info_name", &name);
    Ok(())
}

// ---------------------------------------------------------------------------
// Execute command provider command list: command IDs are an API surface.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_execute_command_ids() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let commands = caps.get("executeCommandProvider").and_then(|p| p.get("commands"));
    assert!(
        commands.is_some(),
        "executeCommandProvider.commands must be present in server capabilities"
    );
    assert_yaml_snapshot!("execute_command_ids", &commands);
    Ok(())
}

// ---------------------------------------------------------------------------
// File operation filters: globs define workspace event subscription scope.
// ---------------------------------------------------------------------------

#[test]
fn snapshot_workspace_file_operation_filters() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = support::client_caps::full();
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    let caps = &init_result["capabilities"];
    let file_operations = caps.get("workspace").and_then(|w| w.get("fileOperations"));
    assert!(
        file_operations.is_some(),
        "workspace.fileOperations must be present in server capabilities"
    );
    assert_yaml_snapshot!("workspace_file_operation_filters", &file_operations);
    Ok(())
}

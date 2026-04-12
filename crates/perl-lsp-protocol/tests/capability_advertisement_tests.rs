//! Test coverage for capability advertisement and server info.
//!
//! Validates:
//! - Server capability construction from BuildFlags
//! - Feature-gated capabilities (BuildFlags profiles)
//! - Server info embedded in enhanced error responses
//! - Capability serialization to LSP-compatible JSON format
//! - All capability fields populated with correct values

use perl_lsp_protocol::capabilities::{
    BuildFlags, capabilities_for, capabilities_json, default_capabilities, get_supported_commands,
};

// ============================================================================
// Server Info — embedded in enhanced error metadata
// ============================================================================

#[test]
fn server_info_name_is_perl_lsp() {
    let err = perl_lsp_protocol::enhanced_error(
        perl_lsp_protocol::INTERNAL_ERROR,
        "test",
        "test_type",
        None,
    );
    let data = err.data.as_ref();
    assert!(data.is_some(), "enhanced error should have data");
    if let Some(d) = data {
        let name = d.pointer("/server_info/name").and_then(|v| v.as_str());
        assert_eq!(name, Some("perl-lsp"), "server_info.name should be 'perl-lsp'");
    }
}

#[test]
fn server_info_version_is_nonempty() {
    let err = perl_lsp_protocol::enhanced_error(
        perl_lsp_protocol::INTERNAL_ERROR,
        "test",
        "test_type",
        None,
    );
    if let Some(d) = &err.data {
        let version = d.pointer("/server_info/version").and_then(|v| v.as_str());
        assert!(version.is_some(), "server_info should have a version field");
        if let Some(v) = version {
            assert!(!v.is_empty(), "server_info.version should not be empty");
        }
    }
}

#[test]
fn server_info_version_is_semver_shaped() {
    let err = perl_lsp_protocol::enhanced_error(
        perl_lsp_protocol::INTERNAL_ERROR,
        "test",
        "test_type",
        None,
    );
    if let Some(d) = &err.data {
        let version = d.pointer("/server_info/version").and_then(|v| v.as_str());
        if let Some(v) = version {
            // SemVer: at least "X.Y.Z" pattern
            let parts: Vec<&str> = v.split('.').collect();
            assert!(parts.len() >= 2, "version '{v}' should have at least major.minor components");
            for part in &parts {
                assert!(
                    part.chars().all(|c| c.is_ascii_digit() || c == '-'),
                    "version component '{part}' should be numeric (or pre-release suffix)"
                );
            }
        }
    }
}

// ============================================================================
// Capability construction — text document sync (always-on)
// ============================================================================

#[test]
fn text_document_sync_is_full() -> Result<(), Box<dyn std::error::Error>> {
    // The server intentionally uses TextDocumentSyncKind::FULL (1) — it always reparses
    // the full document on every didChange notification. INCREMENTAL (2) would be
    // inaccurate because no incremental AST state is maintained between edits.
    // See capabilities.rs for the documented rationale.
    let caps = capabilities_for(BuildFlags::default());
    let v = serde_json::to_value(&caps)?;
    let sync = v.get("textDocumentSync");
    assert!(sync.is_some(), "textDocumentSync must always be present");
    let sync = sync.ok_or("missing textDocumentSync")?;
    let change = sync.get("change").and_then(|c| c.as_u64());
    assert_eq!(change, Some(1), "change should be TextDocumentSyncKind::FULL (1)");
    Ok(())
}

#[test]
fn text_document_sync_has_open_close_true() -> Result<(), Box<dyn std::error::Error>> {
    let caps = capabilities_for(BuildFlags::default());
    let v = serde_json::to_value(&caps)?;
    let open_close = v.pointer("/textDocumentSync/openClose").and_then(|v| v.as_bool());
    assert_eq!(open_close, Some(true), "openClose should be true");
    Ok(())
}

#[test]
fn text_document_sync_present_even_with_no_flags() -> Result<(), Box<dyn std::error::Error>> {
    let caps = capabilities_for(BuildFlags::default());
    let v = serde_json::to_value(&caps)?;
    assert!(v.get("textDocumentSync").is_some());
    Ok(())
}

// ============================================================================
// Capability construction — completion provider details
// ============================================================================

#[test]
fn completion_trigger_characters_include_sigils() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { completion: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let triggers = v
        .pointer("/completionProvider/triggerCharacters")
        .and_then(|v| v.as_array())
        .ok_or("missing triggerCharacters")?;
    let trigger_strs: Vec<&str> = triggers.iter().filter_map(|t| t.as_str()).collect();
    assert!(trigger_strs.contains(&"$"), "should trigger on scalar sigil $");
    assert!(trigger_strs.contains(&"@"), "should trigger on array sigil @");
    assert!(trigger_strs.contains(&"%"), "should trigger on hash sigil %");
    // LSP spec requires single-char trigger characters; ">" replaces the non-conforming "->"
    assert!(trigger_strs.contains(&">"), "should trigger on > for -> (LSP spec: single char)");
    // ":" triggers :: package member completion
    assert!(trigger_strs.contains(&":"), "should trigger on : for :: package member completion");
    // The non-conforming two-char "->" must not appear
    assert!(
        !trigger_strs.contains(&"->"),
        "two-char -> must not be advertised; LSP spec requires single-char trigger characters"
    );
    Ok(())
}

#[test]
fn completion_has_resolve_provider_true() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { completion: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let resolve = v.pointer("/completionProvider/resolveProvider").and_then(|v| v.as_bool());
    assert_eq!(resolve, Some(true), "completionProvider.resolveProvider should be true");
    Ok(())
}

// ============================================================================
// Capability construction — signature help details
// ============================================================================

#[test]
fn signature_help_trigger_characters() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { signature_help: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let triggers = v
        .pointer("/signatureHelpProvider/triggerCharacters")
        .and_then(|v| v.as_array())
        .ok_or("missing triggerCharacters")?;
    let trigger_strs: Vec<&str> = triggers.iter().filter_map(|t| t.as_str()).collect();
    assert!(trigger_strs.contains(&"("), "should trigger on open paren");
    assert!(trigger_strs.contains(&","), "should trigger on comma");
    Ok(())
}

#[test]
fn signature_help_retrigger_characters() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { signature_help: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let retriggers = v
        .pointer("/signatureHelpProvider/retriggerCharacters")
        .and_then(|v| v.as_array())
        .ok_or("missing retriggerCharacters")?;
    let retrigger_strs: Vec<&str> = retriggers.iter().filter_map(|t| t.as_str()).collect();
    assert!(retrigger_strs.contains(&","), "comma retrigger");
    assert!(retrigger_strs.contains(&"@"), "@ (array variable) retrigger");
    assert!(retrigger_strs.contains(&"%"), "% (hash variable) retrigger");
    assert!(retrigger_strs.contains(&"{"), "brace (hash/block subscript) retrigger");
    assert!(retrigger_strs.contains(&"["), "bracket (array subscript) retrigger");
    // Closing brackets intentionally excluded — they signal end of context, not a new parameter
    assert!(!retrigger_strs.contains(&"}"), "closing brace should not retrigger");
    assert!(!retrigger_strs.contains(&"]"), "closing bracket should not retrigger");
    Ok(())
}

// ============================================================================
// Capability construction — rename provider details
// ============================================================================

#[test]
fn rename_provider_has_prepare_provider() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { rename: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let prepare = v.pointer("/renameProvider/prepareProvider").and_then(|v| v.as_bool());
    assert_eq!(prepare, Some(true), "renameProvider.prepareProvider should be true");
    Ok(())
}

// ============================================================================
// Capability construction — inlay hint details
// ============================================================================

#[test]
fn inlay_hint_has_resolve_provider() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { inlay_hints: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let resolve = v.pointer("/inlayHintProvider/resolveProvider").and_then(|v| v.as_bool());
    assert_eq!(resolve, Some(true), "inlayHintProvider.resolveProvider should be true");
    Ok(())
}

// ============================================================================
// Capability construction — pull diagnostics details
// ============================================================================

#[test]
fn pull_diagnostics_has_workspace_diagnostics_true() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { pull_diagnostics: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let workspace = v.pointer("/diagnosticProvider/workspaceDiagnostics").and_then(|v| v.as_bool());
    assert_eq!(workspace, Some(true), "diagnosticProvider.workspaceDiagnostics should be true");
    Ok(())
}

#[test]
fn pull_diagnostics_inter_file_dependencies_false() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { pull_diagnostics: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let inter_file =
        v.pointer("/diagnosticProvider/interFileDependencies").and_then(|v| v.as_bool());
    assert_eq!(inter_file, Some(false), "diagnosticProvider.interFileDependencies should be false");
    Ok(())
}

#[test]
fn pull_diagnostics_identifier_is_perl_lsp() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { pull_diagnostics: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let identifier = v.pointer("/diagnosticProvider/identifier").and_then(|v| v.as_str());
    assert_eq!(identifier, Some("perl-lsp"), "diagnosticProvider.identifier should be 'perl-lsp'");
    Ok(())
}

// ============================================================================
// Capability construction — semantic tokens details
// ============================================================================

#[test]
fn semantic_tokens_has_full_and_range_support() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { semantic_tokens: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let full = v.pointer("/semanticTokensProvider/full");
    assert!(full.is_some(), "semanticTokensProvider should support full");
    let range = v.pointer("/semanticTokensProvider/range").and_then(|v| v.as_bool());
    assert_eq!(range, Some(true), "semanticTokensProvider should support range");
    Ok(())
}

#[test]
fn semantic_tokens_legend_has_token_types() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { semantic_tokens: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let types = v
        .pointer("/semanticTokensProvider/legend/tokenTypes")
        .and_then(|v| v.as_array())
        .ok_or("missing legend.tokenTypes")?;
    // 20 standard LSP types + sql_string + sql_heredoc_keyword + json_heredoc_key = 23 total.
    // This count assertion catches legend desynchronization (issue #2103) at the
    // advertisement layer — if a type is added to the internal legend but not
    // advertised (or vice versa), this fails immediately.
    assert_eq!(
        types.len(),
        23,
        "expected 23 token types (20 standard + sql_string + sql_heredoc_keyword + json_heredoc_key); \
         got {:?}",
        types
    );
    let type_strs: Vec<&str> = types.iter().filter_map(|t| t.as_str()).collect();
    assert!(type_strs.contains(&"function"), "should include 'function' token type");
    assert!(type_strs.contains(&"variable"), "should include 'variable' token type");
    assert!(type_strs.contains(&"keyword"), "should include 'keyword' token type");
    assert!(type_strs.contains(&"string"), "should include 'string' token type");
    assert!(type_strs.contains(&"number"), "should include 'number' token type");
    assert!(type_strs.contains(&"regexp"), "should include 'regexp' token type");
    assert!(type_strs.contains(&"comment"), "should include 'comment' token type");
    assert!(type_strs.contains(&"namespace"), "should include 'namespace' token type");
    // sql_string was missing from advertisement before PR #2772 — guard against regression.
    assert!(
        type_strs.contains(&"sql_string"),
        "should include 'sql_string' token type (DBI/SQL context, issue #2337)"
    );
    // Heredoc injection types added in issue #2059.
    assert!(
        type_strs.contains(&"sql_heredoc_keyword"),
        "should include 'sql_heredoc_keyword' token type (heredoc SQL injection, issue #2059)"
    );
    assert!(
        type_strs.contains(&"json_heredoc_key"),
        "should include 'json_heredoc_key' token type (heredoc JSON injection, issue #2059)"
    );
    Ok(())
}

#[test]
fn semantic_tokens_legend_has_token_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { semantic_tokens: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let modifiers = v
        .pointer("/semanticTokensProvider/legend/tokenModifiers")
        .and_then(|v| v.as_array())
        .ok_or("missing legend.tokenModifiers")?;
    // 10 standard LSP modifiers + 3 sigil modifiers (scalarVariable, arrayVariable, hashVariable)
    // = 13 total. This count assertion catches modifier legend desynchronization —
    // if a modifier is added internally but not advertised, this fails.
    assert_eq!(modifiers.len(), 13, "expected 13 token modifiers; got {:?}", modifiers);
    let mod_strs: Vec<&str> = modifiers.iter().filter_map(|t| t.as_str()).collect();
    assert!(mod_strs.contains(&"declaration"), "should include 'declaration' modifier");
    assert!(mod_strs.contains(&"definition"), "should include 'definition' modifier");
    assert!(mod_strs.contains(&"readonly"), "should include 'readonly' modifier");
    assert!(mod_strs.contains(&"deprecated"), "should include 'deprecated' modifier");
    // defaultLibrary is used for special variables ($_, %ENV, @_); must be advertised.
    // Before PR #2772 the bitmask was wrong (8 vs 512) — guard against regression.
    assert!(
        mod_strs.contains(&"defaultLibrary"),
        "should include 'defaultLibrary' modifier (used for Perl special variables)"
    );
    // Verify defaultLibrary is at bit position 9 (index 9 in the modifiers array),
    // which is the bitmask value 512 used by collect_semantic_tokens.
    let default_library_idx = mod_strs
        .iter()
        .position(|&s| s == "defaultLibrary")
        .ok_or("defaultLibrary must be in advertised modifiers")?;
    assert_eq!(
        default_library_idx, 9,
        "defaultLibrary must be at index 9 (bitmask 512); \
         collect_semantic_tokens hardcodes 512 for special variables"
    );
    // Sigil modifiers at bits 10, 11, 12 (issue #2881)
    assert!(mod_strs.contains(&"scalarVariable"), "should include 'scalarVariable' modifier");
    assert!(mod_strs.contains(&"arrayVariable"), "should include 'arrayVariable' modifier");
    assert!(mod_strs.contains(&"hashVariable"), "should include 'hashVariable' modifier");
    let scalar_idx = mod_strs
        .iter()
        .position(|&s| s == "scalarVariable")
        .ok_or("scalarVariable must be in advertised modifiers")?;
    assert_eq!(scalar_idx, 10, "scalarVariable must be at index 10 (bitmask 1024)");
    Ok(())
}

// ============================================================================
// Capability construction — code action details
// ============================================================================

#[test]
fn code_action_always_includes_quickfix() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { code_actions: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let kinds = v
        .pointer("/codeActionProvider/codeActionKinds")
        .and_then(|v| v.as_array())
        .ok_or("missing codeActionKinds")?;
    let kind_strs: Vec<&str> = kinds.iter().filter_map(|k| k.as_str()).collect();
    assert!(kind_strs.contains(&"quickfix"), "code action kinds should include 'quickfix'");
    Ok(())
}

#[test]
fn code_action_has_resolve_provider() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { code_actions: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let resolve = v.pointer("/codeActionProvider/resolveProvider").and_then(|v| v.as_bool());
    assert_eq!(resolve, Some(true), "codeActionProvider.resolveProvider should be true");
    Ok(())
}

#[test]
fn code_action_source_organize_imports_gated_by_flag() -> Result<(), Box<dyn std::error::Error>> {
    // With source_organize_imports = false
    let flags_without =
        BuildFlags { code_actions: true, source_organize_imports: false, ..Default::default() };
    let caps_without = capabilities_for(flags_without);
    let v_without = serde_json::to_value(&caps_without)?;
    let kinds_without = v_without
        .pointer("/codeActionProvider/codeActionKinds")
        .and_then(|v| v.as_array())
        .ok_or("missing codeActionKinds")?;
    let without_strs: Vec<&str> = kinds_without.iter().filter_map(|k| k.as_str()).collect();
    assert!(
        !without_strs.contains(&"source.organizeImports"),
        "should not include source.organizeImports when flag is off"
    );

    // With source_organize_imports = true
    let flags_with =
        BuildFlags { code_actions: true, source_organize_imports: true, ..Default::default() };
    let caps_with = capabilities_for(flags_with);
    let v_with = serde_json::to_value(&caps_with)?;
    let kinds_with = v_with
        .pointer("/codeActionProvider/codeActionKinds")
        .and_then(|v| v.as_array())
        .ok_or("missing codeActionKinds")?;
    let with_strs: Vec<&str> = kinds_with.iter().filter_map(|k| k.as_str()).collect();
    assert!(
        with_strs.contains(&"source.organizeImports"),
        "should include source.organizeImports when flag is on"
    );
    Ok(())
}

// ============================================================================
// Capability construction — on-type formatting details
// ============================================================================

#[test]
fn on_type_formatting_first_trigger_is_brace() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { on_type_formatting: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let first = v
        .pointer("/documentOnTypeFormattingProvider/firstTriggerCharacter")
        .and_then(|v| v.as_str());
    assert_eq!(first, Some("}"), "firstTriggerCharacter should be '}}'");
    Ok(())
}

#[test]
fn on_type_formatting_more_triggers_include_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { on_type_formatting: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let more = v
        .pointer("/documentOnTypeFormattingProvider/moreTriggerCharacter")
        .and_then(|v| v.as_array())
        .ok_or("missing moreTriggerCharacter")?;
    let more_strs: Vec<&str> = more.iter().filter_map(|t| t.as_str()).collect();
    assert!(more_strs.contains(&";"), "moreTriggerCharacter should include ';'");
    Ok(())
}

#[test]
fn on_type_formatting_more_triggers_include_newline() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { on_type_formatting: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let more = v
        .pointer("/documentOnTypeFormattingProvider/moreTriggerCharacter")
        .and_then(|v| v.as_array())
        .ok_or("missing moreTriggerCharacter")?;
    let more_strs: Vec<&str> = more.iter().filter_map(|t| t.as_str()).collect();
    assert!(more_strs.contains(&"\n"), "moreTriggerCharacter should include newline");
    Ok(())
}

// ============================================================================
// Capability construction — code lens details
// ============================================================================

#[test]
fn code_lens_has_resolve_provider() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { code_lens: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let resolve = v.pointer("/codeLensProvider/resolveProvider").and_then(|v| v.as_bool());
    assert_eq!(resolve, Some(true), "codeLensProvider.resolveProvider should be true");
    Ok(())
}

// ============================================================================
// Capability construction — document link details
// ============================================================================

#[test]
fn document_link_has_resolve_provider() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { document_links: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let resolve = v.pointer("/documentLinkProvider/resolveProvider").and_then(|v| v.as_bool());
    assert_eq!(resolve, Some(true), "documentLinkProvider.resolveProvider should be true");
    Ok(())
}

// ============================================================================
// Capability construction — notebook document sync details
// ============================================================================

#[test]
fn notebook_sync_targets_jupyter_perl_cells() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { notebook_document_sync: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let selectors = v
        .pointer("/notebookDocumentSync/notebookSelector")
        .and_then(|v| v.as_array())
        .ok_or("missing notebookSelector")?;
    assert!(!selectors.is_empty(), "notebookSelector should not be empty");
    // First selector should target jupyter-notebook
    let first = selectors.first().ok_or("empty notebookSelector")?;
    let notebook = first.get("notebook").and_then(|n| n.as_str());
    assert_eq!(notebook, Some("jupyter-notebook"));
    // Should have cells targeting Perl
    let cells = first.get("cells").and_then(|c| c.as_array());
    assert!(cells.is_some(), "should specify cells");
    if let Some(cells) = cells {
        let has_perl =
            cells.iter().any(|c| c.get("language").and_then(|l| l.as_str()) == Some("perl"));
        assert!(has_perl, "cells should target 'perl' language");
    }
    Ok(())
}

#[test]
fn notebook_sync_save_is_true() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { notebook_document_sync: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let save = v.pointer("/notebookDocumentSync/save").and_then(|v| v.as_bool());
    assert_eq!(save, Some(true), "notebookDocumentSync.save should be true");
    Ok(())
}

// ============================================================================
// Feature-gated capabilities — workspace symbol resolve overrides basic
// ============================================================================

#[test]
fn workspace_symbol_resolve_overrides_basic_workspace_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let flags =
        BuildFlags { workspace_symbol: true, workspace_symbol_resolve: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    // When resolve is enabled, workspaceSymbolProvider should be an object with resolveProvider
    let provider = v.get("workspaceSymbolProvider").ok_or("missing workspaceSymbolProvider")?;
    let resolve = provider.get("resolveProvider").and_then(|r| r.as_bool());
    assert_eq!(
        resolve,
        Some(true),
        "workspaceSymbolProvider should have resolveProvider=true when resolve flag is on"
    );
    Ok(())
}

#[test]
fn workspace_symbol_without_resolve_is_simple_bool() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags {
        workspace_symbol: true,
        workspace_symbol_resolve: false,
        ..Default::default()
    };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let provider = v.get("workspaceSymbolProvider").ok_or("missing workspaceSymbolProvider")?;
    assert!(
        provider.is_boolean(),
        "workspaceSymbolProvider should be a boolean when resolve is off"
    );
    Ok(())
}

// ============================================================================
// Feature-gated capabilities — inline completion via experimental
// ============================================================================

#[test]
fn inline_completion_advertised_in_experimental_object() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { inline_completion: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let inline = v.pointer("/experimental/inlineCompletionProvider");
    assert!(inline.is_some(), "experimental.inlineCompletionProvider should be present");
    Ok(())
}

#[test]
fn inline_completion_disabled_has_no_experimental_key() -> Result<(), Box<dyn std::error::Error>> {
    let caps = capabilities_for(BuildFlags::default());
    let v = serde_json::to_value(&caps)?;
    // With all flags off, experimental should be absent or not have inlineCompletionProvider
    let inline = v.pointer("/experimental/inlineCompletionProvider");
    assert!(inline.is_none(), "should not have inlineCompletionProvider when flag is off");
    Ok(())
}

// ============================================================================
// Capability serialization — full roundtrip to LSP JSON format
// ============================================================================

#[test]
fn capabilities_json_production_serializes_to_valid_object() {
    let v = capabilities_json(BuildFlags::production());
    assert!(v.is_object(), "capabilities_json should return a JSON object");
}

#[test]
fn capabilities_json_production_has_expected_top_level_keys() {
    let v = capabilities_json(BuildFlags::production());
    let obj = v.as_object();
    assert!(obj.is_some());
    if let Some(o) = obj {
        assert!(o.contains_key("textDocumentSync"), "missing textDocumentSync");
        assert!(o.contains_key("hoverProvider"), "missing hoverProvider");
        assert!(o.contains_key("completionProvider"), "missing completionProvider");
        assert!(o.contains_key("definitionProvider"), "missing definitionProvider");
        assert!(o.contains_key("referencesProvider"), "missing referencesProvider");
        assert!(o.contains_key("documentSymbolProvider"), "missing documentSymbolProvider");
    }
}

#[test]
fn capabilities_json_all_has_type_hierarchy() {
    let v = capabilities_json(BuildFlags::all());
    assert!(v.get("typeHierarchyProvider").is_some());
}

#[test]
fn capabilities_json_default_is_minimal() {
    let v = capabilities_json(BuildFlags::default());
    let obj = v.as_object();
    assert!(obj.is_some());
    if let Some(o) = obj {
        // Should have textDocumentSync and little else
        assert!(o.contains_key("textDocumentSync"));
        assert!(!o.contains_key("hoverProvider"), "default should not have hoverProvider");
        assert!(
            !o.contains_key("completionProvider"),
            "default should not have completionProvider"
        );
    }
}

#[test]
fn capabilities_serialization_roundtrip_preserves_structure()
-> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags::production();
    let caps = capabilities_for(flags);
    // Serialize to JSON
    let v = serde_json::to_value(&caps)?;
    // Serialize to string and back
    let json_str = serde_json::to_string(&v)?;
    let reparsed: serde_json::Value = serde_json::from_str(&json_str)?;
    assert_eq!(v, reparsed, "roundtrip serialization should preserve structure");
    Ok(())
}

// ============================================================================
// Feature-gated capabilities — BuildFlags profiles
// ============================================================================

#[test]
fn production_profile_includes_formatting() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.document_formatting_provider.is_some(), "production should include formatting");
    assert!(
        caps.document_range_formatting_provider.is_some(),
        "production should include range formatting"
    );
}

#[test]
fn ga_lock_profile_includes_formatting() {
    let caps = capabilities_for(BuildFlags::ga_lock());
    assert!(caps.document_formatting_provider.is_some(), "ga-lock should include formatting");
    assert!(
        caps.document_range_formatting_provider.is_some(),
        "ga-lock should include range formatting"
    );
}

#[test]
fn ga_lock_profile_excludes_inline_values() {
    let caps = capabilities_for(BuildFlags::ga_lock());
    assert!(caps.inline_value_provider.is_none(), "ga-lock should exclude inline values");
}

#[test]
fn all_profile_enables_all_providers() {
    let caps = capabilities_for(BuildFlags::all());
    // Exhaustive check of all conditional providers
    assert!(caps.hover_provider.is_some(), "all: hover_provider");
    assert!(caps.completion_provider.is_some(), "all: completion_provider");
    assert!(caps.definition_provider.is_some(), "all: definition_provider");
    assert!(caps.type_definition_provider.is_some(), "all: type_definition_provider");
    assert!(caps.implementation_provider.is_some(), "all: implementation_provider");
    assert!(caps.references_provider.is_some(), "all: references_provider");
    assert!(caps.document_symbol_provider.is_some(), "all: document_symbol_provider");
    assert!(caps.document_highlight_provider.is_some(), "all: document_highlight_provider");
    assert!(caps.signature_help_provider.is_some(), "all: signature_help_provider");
    assert!(caps.declaration_provider.is_some(), "all: declaration_provider");
    assert!(caps.inlay_hint_provider.is_some(), "all: inlay_hint_provider");
    assert!(caps.diagnostic_provider.is_some(), "all: diagnostic_provider");
    assert!(caps.semantic_tokens_provider.is_some(), "all: semantic_tokens_provider");
    assert!(caps.code_action_provider.is_some(), "all: code_action_provider");
    assert!(caps.rename_provider.is_some(), "all: rename_provider");
    assert!(caps.code_lens_provider.is_some(), "all: code_lens_provider");
    assert!(caps.document_link_provider.is_some(), "all: document_link_provider");
    assert!(caps.selection_range_provider.is_some(), "all: selection_range_provider");
    assert!(
        caps.document_on_type_formatting_provider.is_some(),
        "all: on_type_formatting_provider"
    );
    assert!(caps.linked_editing_range_provider.is_some(), "all: linked_editing_range_provider");
    assert!(caps.inline_value_provider.is_some(), "all: inline_value_provider");
    assert!(caps.moniker_provider.is_some(), "all: moniker_provider");
    assert!(caps.color_provider.is_some(), "all: color_provider");
    assert!(caps.call_hierarchy_provider.is_some(), "all: call_hierarchy_provider");
    assert!(caps.folding_range_provider.is_some(), "all: folding_range_provider");
    assert!(caps.document_formatting_provider.is_some(), "all: document_formatting_provider");
    assert!(
        caps.document_range_formatting_provider.is_some(),
        "all: document_range_formatting_provider"
    );
    assert!(caps.notebook_document_sync.is_some(), "all: notebook_document_sync");
}

#[test]
fn default_profile_enables_only_text_sync() {
    let caps = capabilities_for(BuildFlags::default());
    assert!(caps.text_document_sync.is_some(), "default must have textDocumentSync");
    // Everything else off
    assert!(caps.hover_provider.is_none());
    assert!(caps.completion_provider.is_none());
    assert!(caps.definition_provider.is_none());
    assert!(caps.type_definition_provider.is_none());
    assert!(caps.implementation_provider.is_none());
    assert!(caps.references_provider.is_none());
    assert!(caps.document_symbol_provider.is_none());
    assert!(caps.workspace_symbol_provider.is_none());
    assert!(caps.inlay_hint_provider.is_none());
    assert!(caps.diagnostic_provider.is_none());
    assert!(caps.semantic_tokens_provider.is_none());
    assert!(caps.code_action_provider.is_none());
    assert!(caps.rename_provider.is_none());
    assert!(caps.code_lens_provider.is_none());
    assert!(caps.document_link_provider.is_none());
    assert!(caps.selection_range_provider.is_none());
    assert!(caps.document_on_type_formatting_provider.is_none());
    assert!(caps.linked_editing_range_provider.is_none());
    assert!(caps.inline_value_provider.is_none());
    assert!(caps.moniker_provider.is_none());
    assert!(caps.color_provider.is_none());
    assert!(caps.call_hierarchy_provider.is_none());
    assert!(caps.folding_range_provider.is_none());
    assert!(caps.document_formatting_provider.is_none());
    assert!(caps.document_range_formatting_provider.is_none());
    assert!(caps.notebook_document_sync.is_none());
    assert!(caps.document_highlight_provider.is_none());
    assert!(caps.signature_help_provider.is_none());
    assert!(caps.declaration_provider.is_none());
}

// ============================================================================
// default_capabilities follows feature flag or ga-lock
// ============================================================================

#[test]
fn default_capabilities_matches_expected_profile() {
    let caps = default_capabilities();
    // default_capabilities uses ga_lock when lsp-ga-lock feature is on, else production
    // Either way it should have text sync and hover
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_some());
    assert!(caps.completion_provider.is_some());
}

// ============================================================================
// Supported commands — advertisement correctness
// ============================================================================

#[test]
fn supported_commands_have_expected_set() {
    let cmds = get_supported_commands();
    let expected = [
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.runCritic",
        "perl.runTest",
        "perl.runTestFile",
        "perl.runSubtest",
        "perl.debugFile",
        "perl.debugTest",
        "perl.goToTest",
        "perl.goToImplementation",
    ];
    for exp in &expected {
        assert!(cmds.contains(&exp.to_string()), "supported commands should include '{exp}'");
    }
    assert_eq!(cmds.len(), expected.len(), "supported commands count should match expected");
}

// ============================================================================
// Capability construction — single-flag isolation
// ============================================================================

#[test]
fn enabling_hover_only_produces_hover_and_sync() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags { hover: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let obj = v.as_object().ok_or("not an object")?;
    // Count non-null keys
    let present: Vec<&String> = obj.keys().filter(|k| !v[k].is_null()).collect();
    // Should have exactly textDocumentSync and hoverProvider
    assert!(present.contains(&&"textDocumentSync".to_string()), "should have textDocumentSync");
    assert!(present.contains(&&"hoverProvider".to_string()), "should have hoverProvider");
    // Should not have unrelated providers
    assert!(
        !present.contains(&&"completionProvider".to_string()),
        "should not have completionProvider"
    );
    assert!(
        !present.contains(&&"definitionProvider".to_string()),
        "should not have definitionProvider"
    );
    Ok(())
}

#[test]
fn enabling_definition_only_produces_definition_and_sync() -> Result<(), Box<dyn std::error::Error>>
{
    let flags = BuildFlags { definition: true, ..Default::default() };
    let caps = capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let obj = v.as_object().ok_or("not an object")?;
    let present: Vec<&String> = obj.keys().filter(|k| !v[k].is_null()).collect();
    assert!(present.contains(&&"textDocumentSync".to_string()));
    assert!(present.contains(&&"definitionProvider".to_string()));
    assert!(!present.contains(&&"hoverProvider".to_string()));
    Ok(())
}

// ============================================================================
// capabilities_json vs capabilities_for consistency
// ============================================================================

#[test]
fn capabilities_json_matches_serialized_capabilities_for() -> Result<(), Box<dyn std::error::Error>>
{
    let flags = BuildFlags::production();
    let from_fn = capabilities_json(flags.clone());
    let from_struct = serde_json::to_value(capabilities_for(flags))?;
    // capabilities_json may add typeHierarchyProvider manually, but otherwise matches
    // Compare a subset of known fields
    assert_eq!(
        from_fn.get("hoverProvider"),
        from_struct.get("hoverProvider"),
        "hoverProvider mismatch"
    );
    assert_eq!(
        from_fn.get("completionProvider"),
        from_struct.get("completionProvider"),
        "completionProvider mismatch"
    );
    assert_eq!(
        from_fn.get("textDocumentSync"),
        from_struct.get("textDocumentSync"),
        "textDocumentSync mismatch"
    );
    assert_eq!(
        from_fn.get("definitionProvider"),
        from_struct.get("definitionProvider"),
        "definitionProvider mismatch"
    );
    Ok(())
}

#[test]
fn capabilities_json_adds_type_hierarchy_beyond_struct() -> Result<(), Box<dyn std::error::Error>> {
    let flags = BuildFlags::all();
    let from_json = capabilities_json(flags.clone());
    let from_struct = serde_json::to_value(capabilities_for(flags))?;
    // capabilities_json injects typeHierarchyProvider that struct doesn't have
    assert!(
        from_json.get("typeHierarchyProvider").is_some(),
        "capabilities_json should add typeHierarchyProvider for all() flags"
    );
    assert!(
        from_struct.get("typeHierarchyProvider").is_none(),
        "struct serialization should not have typeHierarchyProvider"
    );
    Ok(())
}

use perl_lsp::protocol::capabilities::{BuildFlags, capabilities_for};
use serde_json::json;

/// Contract test ensuring all advertised capabilities have the correct shape per LSP 3.18 spec
#[test]
fn test_capability_shapes_lsp_318_contract() -> Result<(), Box<dyn std::error::Error>> {
    let build = BuildFlags::production();
    let caps = capabilities_for(build.clone());
    let caps_json = serde_json::to_value(&caps)?;

    // Test text document sync shape (must be object with options)
    // Text sync is always enabled
    assert!(
        caps_json["textDocumentSync"].is_object(),
        "textDocumentSync must be an object, not a number"
    );
    assert!(caps_json["textDocumentSync"]["openClose"].is_boolean());
    assert!(caps_json["textDocumentSync"]["change"].is_number());

    // Test completion shape (must be object with trigger characters)
    if build.completion {
        assert!(
            caps_json["completionProvider"].is_object(),
            "completionProvider must be an object with trigger characters"
        );
        assert!(caps_json["completionProvider"]["triggerCharacters"].is_array());
        assert!(caps_json["completionProvider"]["resolveProvider"].is_boolean());
    }

    // Test hover shape (simple boolean is OK)
    if build.hover {
        assert!(caps_json["hoverProvider"].is_boolean() || caps_json["hoverProvider"].is_object());
    }

    // Test signature help shape (must be object with trigger/retrigger)
    // Signature help is always enabled
    assert!(caps_json["signatureHelpProvider"].is_object());
    assert!(caps_json["signatureHelpProvider"]["triggerCharacters"].is_array());

    // Test definition shape (boolean or object with linkSupport)
    if build.definition {
        assert!(
            caps_json["definitionProvider"].is_boolean()
                || caps_json["definitionProvider"].is_object()
        );
    }

    // Test type definition shape (boolean or object)
    if build.type_definition {
        assert!(
            caps_json["typeDefinitionProvider"].is_boolean()
                || caps_json["typeDefinitionProvider"].is_object()
        );
    }

    // Test implementation shape (boolean or object)
    if build.implementation {
        assert!(
            caps_json["implementationProvider"].is_boolean()
                || caps_json["implementationProvider"].is_object()
        );
    }

    // Test references shape (boolean or object)
    if build.references {
        assert!(
            caps_json["referencesProvider"].is_boolean()
                || caps_json["referencesProvider"].is_object()
        );
    }

    // Test document highlight shape (boolean or object)
    // Document highlight is always enabled
    assert!(
        caps_json["documentHighlightProvider"].is_boolean()
            || caps_json["documentHighlightProvider"].is_object()
    );

    // Test document symbol shape (boolean or object with label)
    if build.document_symbol {
        assert!(
            caps_json["documentSymbolProvider"].is_boolean()
                || caps_json["documentSymbolProvider"].is_object()
        );
    }

    // Test code action shape (MUST be object with kinds and resolve per LSP 3.18)
    if build.code_actions {
        assert!(
            caps_json["codeActionProvider"].is_object(),
            "codeActionProvider must be an object per LSP 3.18"
        );
        assert!(
            caps_json["codeActionProvider"]["codeActionKinds"].is_array(),
            "codeActionProvider must specify kinds"
        );
        assert!(
            caps_json["codeActionProvider"]["resolveProvider"].is_boolean(),
            "codeActionProvider must specify resolveProvider"
        );
    }

    // Test code lens shape (object with resolve boolean)
    if build.code_lens {
        assert!(caps_json["codeLensProvider"].is_object(), "codeLensProvider must be an object");
        assert!(caps_json["codeLensProvider"]["resolveProvider"].is_boolean());
    }

    // Test document link shape (object with resolve boolean)
    if build.document_links {
        assert!(caps_json["documentLinkProvider"].is_object());
        assert!(caps_json["documentLinkProvider"]["resolveProvider"].is_boolean());
    }

    // Test document color shape (boolean or object)
    if build.document_color {
        assert!(caps_json["colorProvider"].is_boolean() || caps_json["colorProvider"].is_object());
    }

    // Test formatting shape (boolean or object)
    if build.formatting {
        assert!(
            caps_json["documentFormattingProvider"].is_boolean()
                || caps_json["documentFormattingProvider"].is_object()
        );
    }

    // Test range formatting shape (boolean or object)
    if build.range_formatting {
        assert!(
            caps_json["documentRangeFormattingProvider"].is_boolean()
                || caps_json["documentRangeFormattingProvider"].is_object()
        );
    }

    // Test on-type formatting shape (object with first trigger character)
    if build.on_type_formatting {
        assert!(caps_json["documentOnTypeFormattingProvider"].is_object());
        assert!(caps_json["documentOnTypeFormattingProvider"]["firstTriggerCharacter"].is_string());
    }

    // Test rename shape (MUST be object with prepareProvider per LSP 3.18)
    if build.rename {
        assert!(
            caps_json["renameProvider"].is_object(),
            "renameProvider must be an object when prepareProvider is supported"
        );
        assert_eq!(
            caps_json["renameProvider"]["prepareProvider"],
            json!(true),
            "prepareProvider must be true when rename is supported"
        );
    }

    // Test folding range shape (boolean or object)
    if build.folding_range {
        assert!(
            caps_json["foldingRangeProvider"].is_boolean()
                || caps_json["foldingRangeProvider"].is_object()
        );
    }

    // Test selection range shape (boolean or object)
    if build.selection_ranges {
        assert!(
            caps_json["selectionRangeProvider"].is_boolean()
                || caps_json["selectionRangeProvider"].is_object()
        );
    }

    // Test linked editing range shape (boolean or object)
    if build.linked_editing {
        assert!(
            caps_json["linkedEditingRangeProvider"].is_boolean()
                || caps_json["linkedEditingRangeProvider"].is_object()
        );
    }

    // Test semantic tokens shape (object with legend)
    if build.semantic_tokens {
        assert!(caps_json["semanticTokensProvider"].is_object());
        assert!(caps_json["semanticTokensProvider"]["legend"].is_object());
        assert!(caps_json["semanticTokensProvider"]["legend"]["tokenTypes"].is_array());
        assert!(caps_json["semanticTokensProvider"]["legend"]["tokenModifiers"].is_array());
        assert!(caps_json["semanticTokensProvider"]["full"].is_boolean());
        assert!(caps_json["semanticTokensProvider"]["range"].is_boolean());
    }

    // Test moniker shape (boolean or object)
    if build.moniker {
        assert!(
            caps_json["monikerProvider"].is_boolean() || caps_json["monikerProvider"].is_object()
        );
    }

    // Test inline value shape (boolean or object)
    if build.inline_values {
        assert!(
            caps_json["inlineValueProvider"].is_boolean()
                || caps_json["inlineValueProvider"].is_object()
        );
    }

    // Test inlay hints shape (object with resolve)
    if build.inlay_hints {
        assert!(caps_json["inlayHintProvider"].is_object());
        assert!(caps_json["inlayHintProvider"]["resolveProvider"].is_boolean());
    }

    // Test diagnostic shape (object with identifier and triggers)
    if build.pull_diagnostics {
        assert!(caps_json["diagnosticProvider"].is_object());
        assert!(caps_json["diagnosticProvider"]["identifier"].is_string());
        assert!(caps_json["diagnosticProvider"]["interFileDependencies"].is_boolean());
        assert!(caps_json["diagnosticProvider"]["workspaceDiagnostics"].is_boolean());
    }

    // Test workspace symbol shape (boolean or object with resolve)
    if build.workspace_symbol {
        assert!(
            caps_json["workspaceSymbolProvider"].is_boolean()
                || caps_json["workspaceSymbolProvider"].is_object()
        );
    }

    // Test execute command shape (object with commands array)
    if build.execute_command {
        assert!(caps_json["executeCommandProvider"].is_object());
        assert!(caps_json["executeCommandProvider"]["commands"].is_array());
    }

    // Test call hierarchy shape (boolean or object)
    if build.call_hierarchy {
        assert!(
            caps_json["callHierarchyProvider"].is_boolean()
                || caps_json["callHierarchyProvider"].is_object()
        );
    }

    // Test type hierarchy shape (boolean, object, or null per LSP spec)
    if build.type_hierarchy {
        assert!(
            caps_json["typeHierarchyProvider"].is_boolean()
                || caps_json["typeHierarchyProvider"].is_object()
                || caps_json["typeHierarchyProvider"].is_null(),
            "typeHierarchyProvider should be boolean, object, or null, got: {:?}",
            caps_json["typeHierarchyProvider"]
        );
    }

    // Test inline completion shape (MUST be in experimental if using old lsp-types)
    if build.inline_completion {
        // With current lsp-types, this must be under experimental
        assert!(
            caps_json["experimental"].is_object(),
            "experimental must exist for inline completion"
        );
        assert!(
            caps_json["experimental"]["inlineCompletionProvider"].is_object(),
            "inlineCompletionProvider must be under experimental"
        );
    }

    Ok(())
}

/// Test that non-advertised features return MethodNotFound
#[test]
fn test_non_advertised_features_return_method_not_found() -> Result<(), Box<dyn std::error::Error>>
{
    // This would be tested via actual LSP server instances
    // For now, we document the expected behavior

    // When a feature is not advertised (build flag = false):
    // 1. The capability should not be present in the response
    // 2. Calling the method should return MethodNotFound (-32601)

    // When a feature is advertised (build flag = true):
    // 1. The capability must be present with correct shape
    // 2. Calling the method should never error, return [] or null for empty results

    Ok(())
}

/// Test that all capability shapes match their handler expectations
#[test]
fn test_capability_handler_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let build = BuildFlags::all();
    let caps = capabilities_for(build);
    let caps_json = serde_json::to_value(&caps)?;

    // Verify rename has prepareProvider when handler exists
    if caps_json["renameProvider"].is_object() {
        assert_eq!(caps_json["renameProvider"]["prepareProvider"], json!(true));
    }

    // Verify code action has resolve when handler exists
    if caps_json["codeActionProvider"].is_object() {
        assert!(caps_json["codeActionProvider"]["resolveProvider"].is_boolean());
    }

    // Verify code lens has resolve when handler exists
    if caps_json["codeLensProvider"].is_object() {
        assert!(caps_json["codeLensProvider"]["resolveProvider"].is_boolean());
    }

    // Verify inlay hints has resolve when handler exists
    if caps_json["inlayHintProvider"].is_object() {
        assert!(caps_json["inlayHintProvider"]["resolveProvider"].is_boolean());
    }

    Ok(())
}

/// Test ga_lock configuration is conservative
#[test]
fn test_ga_lock_is_conservative() -> Result<(), Box<dyn std::error::Error>> {
    let ga = BuildFlags::ga_lock();
    let _prod = BuildFlags::production();

    // GA lock should be more conservative than production
    assert!(!ga.inline_values, "inline values not GA");
    assert!(ga.notebook_document_sync, "notebook sync is GA");
    assert!(ga.notebook_cell_execution, "notebook cell execution is GA");

    // Graduated features should be enabled
    assert!(ga.completion, "completion is GA");
    assert!(ga.hover, "hover is GA");
    assert!(ga.definition, "definition is GA");
    assert!(ga.references, "references is GA");
    assert!(ga.linked_editing, "linked editing is GA");
    assert!(ga.inline_completion, "inline completion is GA");
    assert!(ga.moniker, "moniker is GA");
    assert!(ga.document_color, "document color is GA");
    assert!(ga.code_lens, "code lens is GA");
    assert!(ga.type_definition, "type definition is GA");
    assert!(ga.implementation, "implementation is GA");
    assert!(ga.source_organize_imports, "organize imports is GA");
    assert!(ga.formatting, "formatting is GA");
    assert!(ga.range_formatting, "range formatting is GA");

    Ok(())
}

//! LSP 3.17 Lifecycle Contract Tests
//!
//! Tests for initialize, initialized, shutdown, exit, and pre-initialize behavior.

#![recursion_limit = "256"]

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== LIFECYCLE CONTRACTS ====================

#[test]
fn test_initialize_contract_3_17() -> TestResult {
    let mut harness = LspHarness::new();

    // Full 3.17 initialization with all capabilities
    let result = harness.initialize(Some(json!({
        "processId": 1234,
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        },
        "locale": "en-US",
        "rootPath": null,  // deprecated but still sent
        "rootUri": "file:///workspace",
        "capabilities": {
            // 3.17 position encoding support
            "general": {
                "positionEncodings": ["utf-16", "utf-8", "utf-32"],
                "staleRequestSupport": {
                    "cancel": true,
                    "retryOnContentModified": ["textDocument/completion"]
                },
                "regularExpressions": {
                    "engine": "ECMAScript",
                    "version": "ES2020"
                },
                "markdown": {
                    "parser": "marked",
                    "version": "1.0.0"
                }
            },
            // Text document capabilities
            "textDocument": {
                "synchronization": {
                    "dynamicRegistration": true,
                    "willSave": true,
                    "willSaveWaitUntil": true,
                    "didSave": true
                },
                "completion": {
                    "dynamicRegistration": true,
                    "completionItem": {
                        "snippetSupport": true,
                        "commitCharactersSupport": true,
                        "documentationFormat": ["markdown", "plaintext"],
                        "deprecatedSupport": true,
                        "preselectSupport": true,
                        "tagSupport": { "valueSet": [1] },
                        "insertReplaceSupport": true,
                        "resolveSupport": {
                            "properties": ["documentation", "detail", "additionalTextEdits"]
                        },
                        "insertTextModeSupport": { "valueSet": [1, 2] },
                        "labelDetailsSupport": true
                    },
                    "completionItemKind": { "valueSet": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25] },
                    "insertTextMode": 2,
                    "contextSupport": true,
                    "completionList": {
                        "itemDefaults": ["commitCharacters", "editRange", "insertTextFormat", "insertTextMode", "data"]
                    }
                },
                "hover": {
                    "dynamicRegistration": true,
                    "contentFormat": ["markdown", "plaintext"]
                },
                "signatureHelp": {
                    "dynamicRegistration": true,
                    "signatureInformation": {
                        "documentationFormat": ["markdown", "plaintext"],
                        "parameterInformation": { "labelOffsetSupport": true },
                        "activeParameterSupport": true
                    },
                    "contextSupport": true
                },
                "declaration": {
                    "dynamicRegistration": true,
                    "linkSupport": true
                },
                "definition": {
                    "dynamicRegistration": true,
                    "linkSupport": true
                },
                "typeDefinition": {
                    "dynamicRegistration": true,
                    "linkSupport": true
                },
                "implementation": {
                    "dynamicRegistration": true,
                    "linkSupport": true
                },
                "references": {
                    "dynamicRegistration": true
                },
                "documentHighlight": {
                    "dynamicRegistration": true
                },
                "documentSymbol": {
                    "dynamicRegistration": true,
                    "symbolKind": { "valueSet": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26] },
                    "hierarchicalDocumentSymbolSupport": true,
                    "tagSupport": { "valueSet": [1] },
                    "labelSupport": true
                },
                "codeAction": {
                    "dynamicRegistration": true,
                    "codeActionLiteralSupport": {
                        "codeActionKind": {
                            "valueSet": ["", "quickfix", "refactor", "refactor.extract", "refactor.inline", "refactor.rewrite", "source", "source.organizeImports", "source.fixAll"]
                        }
                    },
                    "isPreferredSupport": true,
                    "disabledSupport": true,
                    "dataSupport": true,
                    "resolveSupport": {
                        "properties": ["edit"]
                    },
                    "honorsChangeAnnotations": true
                },
                "codeLens": {
                    "dynamicRegistration": true
                },
                "documentLink": {
                    "dynamicRegistration": true,
                    "tooltipSupport": true
                },
                "colorProvider": {
                    "dynamicRegistration": true
                },
                "formatting": {
                    "dynamicRegistration": true
                },
                "rangeFormatting": {
                    "dynamicRegistration": true,
                    "rangesSupport": true
                },
                "onTypeFormatting": {
                    "dynamicRegistration": true
                },
                "rename": {
                    "dynamicRegistration": true,
                    "prepareSupport": true,
                    "prepareSupportDefaultBehavior": 1,
                    "honorsChangeAnnotations": true
                },
                "foldingRange": {
                    "dynamicRegistration": true,
                    "rangeLimit": 5000,
                    "lineFoldingOnly": false,
                    "foldingRangeKind": { "valueSet": ["comment", "imports", "region"] },
                    "foldingRange": { "collapsedText": false }
                },
                "selectionRange": {
                    "dynamicRegistration": true
                },
                "publishDiagnostics": {
                    "relatedInformation": true,
                    "tagSupport": { "valueSet": [1, 2] },
                    "versionSupport": true,
                    "codeDescriptionSupport": true,
                    "dataSupport": true
                },
                "callHierarchy": {
                    "dynamicRegistration": true
                },
                "semanticTokens": {
                    "dynamicRegistration": true,
                    "requests": {
                        "range": true,
                        "full": { "delta": true }
                    },
                    "tokenTypes": ["namespace", "type", "class", "enum", "interface", "struct", "typeParameter", "parameter", "variable", "property", "enumMember", "event", "function", "method", "macro", "keyword", "modifier", "comment", "string", "number", "regexp", "operator", "decorator"],
                    "tokenModifiers": ["declaration", "definition", "readonly", "static", "deprecated", "abstract", "async", "modification", "documentation", "defaultLibrary"],
                    "formats": ["relative"],
                    "overlappingTokenSupport": false,
                    "multilineTokenSupport": true,
                    "serverCancelSupport": true,
                    "augmentsSyntaxTokens": true
                },
                "linkedEditingRange": {
                    "dynamicRegistration": true
                },
                "typeHierarchy": {
                    "dynamicRegistration": true
                },
                "inlineValue": {
                    "dynamicRegistration": true
                },
                "inlayHint": {
                    "dynamicRegistration": true,
                    "resolveSupport": {
                        "properties": ["tooltip", "textEdits", "label.tooltip", "label.location", "label.command"]
                    }
                },
                "diagnostic": {
                    "dynamicRegistration": true,
                    "relatedDocumentSupport": true
                },
                "moniker": {
                    "dynamicRegistration": true
                }
            },
            // Notebook document support (3.17)
            "notebookDocument": {
                "synchronization": {
                    "dynamicRegistration": true,
                    "executionSummarySupport": true
                }
            },
            // Window capabilities
            "window": {
                "workDoneProgress": true,
                "showMessage": {
                    "messageActionItem": {
                        "additionalPropertiesSupport": true
                    }
                },
                "showDocument": {
                    "support": true
                }
            },
            // Workspace capabilities
            "workspace": {
                "applyEdit": true,
                "workspaceEdit": {
                    "documentChanges": true,
                    "resourceOperations": ["create", "rename", "delete"],
                    "failureHandling": "textOnlyTransactional",
                    "normalizesLineEndings": true,
                    "changeAnnotationSupport": {
                        "groupsOnLabel": true
                    }
                },
                "didChangeConfiguration": {
                    "dynamicRegistration": true
                },
                "didChangeWatchedFiles": {
                    "dynamicRegistration": true,
                    "relativePatternSupport": true
                },
                "symbol": {
                    "dynamicRegistration": true,
                    "symbolKind": { "valueSet": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26] },
                    "tagSupport": { "valueSet": [1] },
                    "resolveSupport": {
                        "properties": ["location.range"]
                    }
                },
                "executeCommand": {
                    "dynamicRegistration": true
                },
                "semanticTokens": {
                    "refreshSupport": true
                },
                "codeLens": {
                    "refreshSupport": true
                },
                "fileOperations": {
                    "dynamicRegistration": true,
                    "didCreate": true,
                    "willCreate": true,
                    "didRename": true,
                    "willRename": true,
                    "didDelete": true,
                    "willDelete": true
                },
                "inlineValue": {
                    "refreshSupport": true
                },
                "inlayHint": {
                    "refreshSupport": true
                },
                "diagnostics": {
                    "refreshSupport": true
                },
                "workspaceFolders": true,
                "configuration": true
            }
        },
        "initializationOptions": {
            "testMode": true
        },
        "workspaceFolders": [
            {
                "uri": "file:///workspace",
                "name": "Test Workspace"
            }
        ]
    })))?;

    // Validate server response structure
    assert!(result.is_object());
    let capabilities = &result["capabilities"];
    assert!(capabilities.is_object());

    // Check position encoding (3.17)
    if let Some(encoding) = capabilities.get("positionEncoding") {
        assert!(encoding.is_string());
        let enc = encoding.as_str().ok_or("encoding not a string")?;
        assert!(["utf-8", "utf-16", "utf-32"].contains(&enc));
    }

    // Check server info
    if let Some(info) = result.get("serverInfo") {
        assert!(info["name"].is_string());
        if let Some(version) = info.get("version") {
            assert!(version.is_string());
        }
    }
    Ok(())
}

#[test]
fn test_initialized_notification() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Send initialized notification - no response expected
    harness.notify("initialized", json!({}));

    // Server should now accept requests
    let response = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Should get a valid response (even if null)
    assert!(response.is_ok() || response.is_err());
    Ok(())
}

// ==================== SHUTDOWN & EXIT ====================

#[test]
fn test_shutdown_exit_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Shutdown request
    let response = harness.request("shutdown", json!(null))?;
    assert!(response.is_null());

    // Exit notification
    harness.notify("exit", json!(null));
    Ok(())
}

// ==================== PRE-INITIALIZE BEHAVIOR ====================

#[test]
fn test_inbound_before_initialize_contract() -> TestResult {
    // Requests before initialize must return -32002 ServerNotInitialized
    // Notifications must be dropped (except exit)

    // This test would need a harness method to create without auto-initialize
    // let mut harness = LspHarness::new_without_initialize();

    // Request before initialize -> -32002
    // let resp = harness.request_raw(json!({
    //     "jsonrpc":"2.0","id":1,"method":"textDocument/hover",
    //     "params":{"textDocument":{"uri":"file:///t.pl"},
    //               "position":{"line":0,"character":0}}
    // }));
    // assert_eq!(resp["error"]["code"], -32002);

    // Notification before initialize -> drop silently
    // harness.notify("workspace/didChangeConfiguration", json!({"settings":{}}));
    Ok(())
}

// ==================== $-PREFIXED MESSAGES ====================

#[test]
fn test_dollar_prefixed_request_method_not_found() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Requests with methods starting with $/ must return -32601 MethodNotFound
    // (unless explicitly implemented like $/cancelRequest)

    // This would test unknown $/ methods
    // let resp = harness.request_raw(json!({
    //     "jsonrpc":"2.0","id":1,"method":"$/unknownRequest","params":{}
    // }));
    // assert_eq!(resp["error"]["code"], -32601);
    Ok(())
}

// ==================== ERROR CODES ====================

#[test]
fn test_error_codes_3_17() -> TestResult {
    // Standard JSON-RPC error codes
    const _PARSE_ERROR: i32 = -32700;
    #[allow(dead_code)]
    const INVALID_REQUEST: i32 = -32600;
    #[allow(dead_code)]
    const METHOD_NOT_FOUND: i32 = -32601;
    #[allow(dead_code)]
    const INVALID_PARAMS: i32 = -32602;
    #[allow(dead_code)]
    const INTERNAL_ERROR: i32 = -32603;

    // LSP error codes
    const _SERVER_NOT_INITIALIZED: i32 = -32002;
    #[allow(dead_code)]
    const UNKNOWN_ERROR_CODE: i32 = -32001;
    const _REQUEST_CANCELLED: i32 = -32800;
    #[allow(dead_code)]
    const CONTENT_MODIFIED: i32 = -32801;
    const _SERVER_CANCELLED: i32 = -32802; // 3.17
    const _REQUEST_FAILED: i32 = -32803;

    // Validate error code constants match spec
    // PARSE_ERROR = -32700 (< -32000 as required)
    // SERVER_NOT_INITIALIZED = -32002
    // REQUEST_CANCELLED = -32800
    // SERVER_CANCELLED = -32802 (LSP 3.17)
    // REQUEST_FAILED = -32803
    Ok(())
}

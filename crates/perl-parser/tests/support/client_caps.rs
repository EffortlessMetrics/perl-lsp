use serde_json::{json, Value};

/// Returns comprehensive client capabilities for testing
/// This ensures all tests use the same client configuration
#[allow(dead_code)]
pub fn full() -> Value {
    json!({
        "textDocument": {
            "completion": {
                "completionItem": {
                    "snippetSupport": true,
                    "documentationFormat": ["markdown", "plaintext"]
                }
            },
            "hover": {
                "contentFormat": ["markdown", "plaintext"]
            },
            "signatureHelp": {
                "signatureInformation": {
                    "documentationFormat": ["markdown", "plaintext"],
                    "parameterInformation": {
                        "labelOffsetSupport": true
                    }
                }
            },
            "definition": {
                "dynamicRegistration": false,
                "linkSupport": true
            },
            "references": {
                "dynamicRegistration": false
            },
            "documentHighlight": {
                "dynamicRegistration": false
            },
            "documentSymbol": {
                "dynamicRegistration": false,
                "symbolKind": {
                    "valueSet": (1..=26).collect::<Vec<_>>()
                },
                "hierarchicalDocumentSymbolSupport": true
            },
            "formatting": {
                "dynamicRegistration": false
            },
            "rangeFormatting": {
                "dynamicRegistration": false
            },
            "rename": {
                "dynamicRegistration": false,
                "prepareSupport": true
            },
            "codeAction": {
                "dynamicRegistration": false,
                "codeActionLiteralSupport": {
                    "codeActionKind": {
                        "valueSet": [
                            "quickfix",
                            "refactor",
                            "refactor.extract",
                            "refactor.inline",
                            "refactor.rewrite",
                            "source",
                            "source.organizeImports"
                        ]
                    }
                }
            },
            "codeLens": {
                "dynamicRegistration": false
            },
            "documentLink": {
                "dynamicRegistration": false
            },
            "inlayHint": {
                "dynamicRegistration": false
            },
            "diagnostic": {
                "dynamicRegistration": false,
                "relatedDocumentSupport": false
            },
            "semanticTokens": {
                "dynamicRegistration": false,
                "requests": {
                    "full": true,
                    "range": true
                },
                "tokenTypes": [
                    "namespace", "type", "class", "enum", "interface",
                    "struct", "typeParameter", "parameter", "variable",
                    "property", "enumMember", "event", "function",
                    "method", "macro", "keyword", "modifier", "comment",
                    "string", "number", "regexp", "operator"
                ],
                "tokenModifiers": [
                    "declaration", "definition", "readonly", "static",
                    "deprecated", "abstract", "async", "modification",
                    "documentation", "defaultLibrary"
                ]
            },
            "foldingRange": {
                "dynamicRegistration": false
            },
            "selectionRange": {
                "dynamicRegistration": false
            },
            "callHierarchy": {
                "dynamicRegistration": false
            },
            "typeHierarchy": {
                "dynamicRegistration": false
            },
            "onTypeFormatting": {
                "dynamicRegistration": false
            }
        },
        "workspace": {
            "workspaceFolders": true,
            "diagnostics": true,
            "symbol": {
                "dynamicRegistration": false,
                "symbolKind": {
                    "valueSet": (1..=26).collect::<Vec<_>>()
                }
            },
            "executeCommand": {
                "dynamicRegistration": false
            },
            "didChangeWatchedFiles": {
                "dynamicRegistration": false
            },
            "fileOperations": {
                "dynamicRegistration": false,
                "willRename": true,
                "didRename": true
            }
        },
        "window": {
            "workDoneProgress": true,
            "showMessage": {
                "messageActionItem": {
                    "additionalPropertiesSupport": true
                }
            }
        },
        "general": {
            "staleRequestSupport": {
                "cancel": true,
                "retryOnContentModified": ["textDocument/semanticTokens"]
            }
        }
    })
}

/// Returns minimal client capabilities for basic testing
#[allow(dead_code)]
pub fn minimal() -> Value {
    json!({
        "textDocument": {},
        "workspace": {},
        "window": {}
    })
}

/// Returns client capabilities with specific features enabled
#[allow(dead_code)]
pub fn with_features(features: &[&str]) -> Value {
    let mut caps = json!({
        "textDocument": {},
        "workspace": {},
        "window": {}
    });

    for feature in features {
        match *feature {
            "completion" => {
                caps["textDocument"]["completion"] = json!({
                    "completionItem": { "snippetSupport": true }
                });
            }
            "hover" => {
                caps["textDocument"]["hover"] = json!({
                    "contentFormat": ["markdown", "plaintext"]
                });
            }
            "diagnostics" => {
                caps["textDocument"]["diagnostic"] = json!({
                    "dynamicRegistration": false
                });
                caps["workspace"]["diagnostics"] = json!(true);
            }
            "workspace-symbols" => {
                caps["workspace"]["symbol"] = json!({
                    "dynamicRegistration": false
                });
            }
            _ => {}
        }
    }

    caps
}

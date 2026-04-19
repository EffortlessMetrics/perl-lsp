//! LSP 3.17 Notebook Document Contract Tests
//!
//! Tests for notebookDocument/didOpen, didChange, didSave, didClose,
//! and execution summary support.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== NOTEBOOK SUPPORT (3.17) ====================

#[test]
fn test_notebook_document_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(None)?;
    assert!(
        init["capabilities"]["notebookDocumentSync"].is_object(),
        "notebookDocumentSync capability should be advertised"
    );

    // didOpen notebook
    harness.notify(
        "notebookDocument/didOpen",
        json!({
            "notebookDocument": {
                "uri": "file:///test.ipynb",
                "notebookType": "jupyter-notebook",
                "version": 1,
                "cells": [
                    {
                        "kind": 2,  // Code
                        "document": "file:///test.ipynb#cell1"
                    }
                ]
            },
            "cellTextDocuments": [
                {
                    "uri": "file:///test.ipynb#cell1",
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub from_notebook_cell { return 1; }\n"
                }
            ]
        }),
    );

    let cell1_symbols = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": {
                "uri": "file:///test.ipynb#cell1"
            }
        }),
    )?;
    let cell1_symbols = cell1_symbols
        .as_array()
        .ok_or("cell1 symbols should be an array")?;
    assert!(
        cell1_symbols
            .iter()
            .any(|symbol| symbol["name"].as_str() == Some("from_notebook_cell")),
        "Expected symbol from_notebook_cell in notebook cell document symbols"
    );

    // didChange notebook
    harness.notify(
        "notebookDocument/didChange",
        json!({
            "notebookDocument": {
                "uri": "file:///test.ipynb",
                "version": 2
            },
            "change": {
                "cells": {
                    "structure": {
                        "array": {
                            "start": 0,
                            "deleteCount": 0,
                            "cells": [
                                {
                                    "kind": 2,
                                    "document": "file:///test.ipynb#cell2"
                                }
                            ]
                        },
                        "didOpen": [
                            {
                                "uri": "file:///test.ipynb#cell2",
                                "languageId": "perl",
                                "version": 1,
                                "text": "sub second_notebook_cell { return 42; }\n"
                            }
                        ]
                    }
                }
            }
        }),
    );

    let cell2_symbols = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": {
                "uri": "file:///test.ipynb#cell2"
            }
        }),
    )?;
    let cell2_symbols = cell2_symbols
        .as_array()
        .ok_or("cell2 symbols should be an array")?;
    assert!(
        cell2_symbols
            .iter()
            .any(|symbol| symbol["name"].as_str() == Some("second_notebook_cell")),
        "Expected symbol second_notebook_cell in newly opened notebook cell"
    );

    // didSave notebook
    harness.notify(
        "notebookDocument/didSave",
        json!({
            "notebookDocument": {
                "uri": "file:///test.ipynb"
            }
        }),
    );

    // didClose notebook
    harness.notify(
        "notebookDocument/didClose",
        json!({
            "notebookDocument": {
                "uri": "file:///test.ipynb"
            },
            "cellTextDocuments": [
                { "uri": "file:///test.ipynb#cell1" },
                { "uri": "file:///test.ipynb#cell2" }
            ]
        }),
    );
    Ok(())
}

#[test]
fn test_notebook_execution_summary_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(None)?;
    assert!(
        init["capabilities"]["notebookDocumentSync"].is_object(),
        "notebookDocumentSync capability should be advertised"
    );

    // didOpen notebook with a single cell
    harness.notify(
        "notebookDocument/didOpen",
        json!({
            "notebookDocument": {
                "uri": "file:///exec.ipynb",
                "notebookType": "jupyter-notebook",
                "version": 1,
                "cells": [
                    {
                        "kind": 2,
                        "document": "file:///exec.ipynb#cell1"
                    }
                ]
            },
            "cellTextDocuments": [
                {
                    "uri": "file:///exec.ipynb#cell1",
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;"
                }
            ]
        }),
    );

    // didChange with executionSummary update
    harness.notify(
        "notebookDocument/didChange",
        json!({
            "notebookDocument": {
                "uri": "file:///exec.ipynb",
                "version": 2
            },
            "change": {
                "cells": {
                    "data": [
                        {
                            "document": "file:///exec.ipynb#cell1",
                            "executionSummary": {
                                "executionOrder": 1,
                                "success": true
                            }
                        }
                    ]
                }
            }
        }),
    );

    // didClose notebook
    harness.notify(
        "notebookDocument/didClose",
        json!({
            "notebookDocument": {
                "uri": "file:///exec.ipynb"
            },
            "cellTextDocuments": [
                { "uri": "file:///exec.ipynb#cell1" }
            ]
        }),
    );

    Ok(())
}

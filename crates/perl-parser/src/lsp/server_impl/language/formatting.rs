//! Formatting handlers for code formatting features
//!
//! Handles textDocument/formatting, textDocument/rangeFormatting,
//! and textDocument/onTypeFormatting requests.

use super::super::*;
use crate::formatting::CodeFormatter;
use crate::lsp::protocol::{req_position, req_range, req_uri};

impl LspServer {
    /// Handle textDocument/onTypeFormatting request
    pub(crate) fn handle_on_type_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = req_uri(&p)?;
            let ch = p["ch"].as_str().and_then(|s| s.chars().next()).unwrap_or('\n');
            let (line, col) = req_position(&p)?;

            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            if let Some(edits) =
                crate::on_type_formatting::compute_on_type_edit(&doc.text, line, col, ch)
            {
                return Ok(Some(json!(edits)));
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle textDocument/formatting request
    pub(crate) fn handle_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            self.ensure_latest(uri, req_version)?;

            let options: FormattingOptions = serde_json::from_value(params["options"].clone())
                .unwrap_or(FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                });

            eprintln!("Formatting document: {}", uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let formatter = CodeFormatter::new();
                match formatter.format_document(&doc.text, &options) {
                    Ok(edits) => {
                        let lsp_edits: Vec<Value> = edits
                            .into_iter()
                            .map(|edit| {
                                json!({
                                    "range": {
                                        "start": {
                                            "line": edit.range.start.line,
                                            "character": edit.range.start.character,
                                        },
                                        "end": {
                                            "line": edit.range.end.line,
                                            "character": edit.range.end.character,
                                        },
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_edits)));
                    }
                    Err(e) => {
                        eprintln!("Formatting error: {}", e);
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Formatting failed: {}", e),
                            data: None,
                        });
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/rangeFormatting request
    pub(crate) fn handle_range_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let ((start_line, start_char), (end_line, end_char)) = req_range(&params)?;
            let options: FormattingOptions = serde_json::from_value(params["options"].clone())
                .unwrap_or(FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                });

            let range = crate::formatting::Range {
                start: crate::formatting::Position { line: start_line, character: start_char },
                end: crate::formatting::Position { line: end_line, character: end_char },
            };

            eprintln!("Formatting range in document: {}", uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let formatter = CodeFormatter::new();
                match formatter.format_range(&doc.text, &range, &options) {
                    Ok(edits) => {
                        let lsp_edits: Vec<Value> = edits
                            .into_iter()
                            .map(|edit| {
                                json!({
                                    "range": {
                                        "start": {
                                            "line": edit.range.start.line,
                                            "character": edit.range.start.character,
                                        },
                                        "end": {
                                            "line": edit.range.end.line,
                                            "character": edit.range.end.character,
                                        },
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_edits)));
                    }
                    Err(e) => {
                        eprintln!("Range formatting error: {}", e);
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Range formatting failed: {}", e),
                            data: None,
                        });
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }
}

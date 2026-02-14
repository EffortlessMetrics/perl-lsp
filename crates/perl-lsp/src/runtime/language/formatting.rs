//! Formatting handlers for code formatting features
//!
//! Handles textDocument/formatting, textDocument/rangeFormatting,
//! and textDocument/onTypeFormatting requests.

use super::super::*;
use crate::convert::{WirePosition, WireRange};
use crate::features::formatting::{CodeFormatter, FormattingOptions};
use crate::protocol::{invalid_params, req_position, req_range, req_uri};

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
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
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
                let config_path = {
                    let config = self.config.lock();
                    config.perltidy_config.clone()
                };

                let mut formatter = CodeFormatter::new();
                if let Some(path) = config_path {
                    formatter = formatter.with_config_path(path);
                }

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

            let range = WireRange::new(
                WirePosition::new(start_line, start_char),
                WirePosition::new(end_line, end_char),
            );

            eprintln!("Formatting range in document: {}", uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let config_path = {
                    let config = self.config.lock();
                    config.perltidy_config.clone()
                };

                let mut formatter = CodeFormatter::new();
                if let Some(path) = config_path {
                    formatter = formatter.with_config_path(path);
                }

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

    /// Handle textDocument/rangesFormatting request (LSP 3.18)
    pub(crate) fn handle_ranges_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let options: FormattingOptions = serde_json::from_value(params["options"].clone())
                .unwrap_or(FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                });

            // Parse ranges array
            let ranges_array = params
                .get("ranges")
                .and_then(|r| r.as_array())
                .ok_or_else(|| invalid_params("Missing required parameter: ranges"))?;

            if ranges_array.is_empty() {
                return Ok(Some(json!([])));
            }

            eprintln!("Formatting {} ranges in document: {}", ranges_array.len(), uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let config_path = {
                    let config = self.config.lock();
                    config.perltidy_config.clone()
                };

                let mut formatter = CodeFormatter::new();
                if let Some(path) = config_path {
                    formatter = formatter.with_config_path(path);
                }

                let mut all_edits = Vec::new();

                // Process each range
                for (idx, range_val) in ranges_array.iter().enumerate() {
                    let start_line_u64 =
                        range_val.pointer("/start/line").and_then(|v| v.as_u64()).ok_or_else(
                            || invalid_params(&format!("Missing ranges[{}].start.line", idx)),
                        )?;
                    let start_line = u32::try_from(start_line_u64).map_err(|_| {
                        invalid_params(&format!("ranges[{}].start.line exceeds u32::MAX", idx))
                    })?;

                    let start_char_u64 =
                        range_val.pointer("/start/character").and_then(|v| v.as_u64()).ok_or_else(
                            || invalid_params(&format!("Missing ranges[{}].start.character", idx)),
                        )?;
                    let start_char = u32::try_from(start_char_u64).map_err(|_| {
                        invalid_params(&format!("ranges[{}].start.character exceeds u32::MAX", idx))
                    })?;

                    let end_line_u64 =
                        range_val.pointer("/end/line").and_then(|v| v.as_u64()).ok_or_else(
                            || invalid_params(&format!("Missing ranges[{}].end.line", idx)),
                        )?;
                    let end_line = u32::try_from(end_line_u64).map_err(|_| {
                        invalid_params(&format!("ranges[{}].end.line exceeds u32::MAX", idx))
                    })?;

                    let end_char_u64 =
                        range_val.pointer("/end/character").and_then(|v| v.as_u64()).ok_or_else(
                            || invalid_params(&format!("Missing ranges[{}].end.character", idx)),
                        )?;
                    let end_char = u32::try_from(end_char_u64).map_err(|_| {
                        invalid_params(&format!("ranges[{}].end.character exceeds u32::MAX", idx))
                    })?;

                    let range = WireRange::new(
                        WirePosition::new(start_line, start_char),
                        WirePosition::new(end_line, end_char),
                    );

                    match formatter.format_range(&doc.text, &range, &options) {
                        Ok(edits) => {
                            all_edits.extend(edits);
                        }
                        Err(e) => {
                            eprintln!("Range formatting error for range {}: {}", idx, e);
                            return Err(JsonRpcError {
                                code: -32603,
                                message: format!(
                                    "Range formatting failed for range {}: {}",
                                    idx, e
                                ),
                                data: None,
                            });
                        }
                    }
                }

                let lsp_edits: Vec<Value> = all_edits
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
        }

        Ok(Some(json!([])))
    }
}

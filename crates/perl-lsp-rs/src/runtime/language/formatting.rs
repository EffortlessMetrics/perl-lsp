//! Formatting handlers for code formatting features
//!
//! Handles textDocument/formatting, textDocument/rangeFormatting,
//! and textDocument/onTypeFormatting requests.

use super::super::*;
use crate::convert::{WirePosition, WireRange};
use crate::features::formatting::{
    CodeFormatter, FormattingError, FormattingOptions, PerlTidyConfig,
};
use crate::protocol::{invalid_params, req_position, req_range, req_uri};

/// Build a `JsonRpcError` from a `FormattingError`, populating the `data` field
/// with a structured object so that VSCode / LSP clients can surface targeted
/// remediation actions (e.g. "install perltidy" vs "check Perl syntax").
fn formatting_error_to_rpc(context: &str, e: FormattingError) -> JsonRpcError {
    let error_kind = e.error_kind();
    JsonRpcError {
        code: -32603,
        message: format!("{}: {}", context, e),
        data: Some(json!({
            "error_kind": error_kind,
        })),
    }
}

fn document_not_open_error(uri: &str) -> JsonRpcError {
    JsonRpcError {
        code: INVALID_REQUEST,
        message: format!("Document not open: {}", uri),
        data: None,
    }
}

impl LspServer {
    /// Build a `PerlTidyConfig` from the current server configuration.
    fn build_perltidy_config(&self) -> PerlTidyConfig {
        let config = self.config.lock();
        PerlTidyConfig {
            maximum_line_length: config.perltidy_maximum_line_length,
            indent_columns: config.perltidy_indent_columns,
            tabs: config.perltidy_tabs,
            opening_brace_on_new_line: config.perltidy_opening_brace_on_new_line,
            cuddled_else: config.perltidy_cuddled_else,
            space_after_keyword: config.perltidy_space_after_keyword,
            add_trailing_commas: config.perltidy_add_trailing_commas,
            vertical_alignment: config.perltidy_vertical_alignment,
            block_comment_indentation: config.perltidy_block_comment_indentation,
            profile: config.perltidy_profile.clone(),
            extra_args: config.perltidy_extra_args.clone(),
            timeout_secs: config.perltidy_timeout_secs,
        }
    }
}

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

            let indent_step = p["options"]["tabSize"].as_u64().unwrap_or(4) as usize;

            if let Some(edits) = crate::on_type_formatting::compute_on_type_edit(
                &doc.text,
                line,
                col,
                ch,
                indent_step,
            ) {
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

            tracing::debug!(uri, "Formatting document");

            let documents = self.documents_guard();
            let doc =
                self.get_document(&documents, uri).ok_or_else(|| document_not_open_error(uri))?;
            let config = self.build_perltidy_config();
            let formatter = CodeFormatter::with_config(config);
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
                    tracing::warn!(error = %e, "Formatting error");
                    return Err(formatting_error_to_rpc("Formatting failed", e));
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

            tracing::debug!(uri, "Formatting range in document");

            let documents = self.documents_guard();
            let doc =
                self.get_document(&documents, uri).ok_or_else(|| document_not_open_error(uri))?;
            let config = self.build_perltidy_config();
            let formatter = CodeFormatter::with_config(config);
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
                    tracing::warn!(error = %e, "Range formatting error");
                    return Err(formatting_error_to_rpc("Range formatting failed", e));
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

            tracing::debug!(count = ranges_array.len(), uri, "Formatting ranges in document");

            let documents = self.documents_guard();
            let doc =
                self.get_document(&documents, uri).ok_or_else(|| document_not_open_error(uri))?;
            let config = self.build_perltidy_config();
            let formatter = CodeFormatter::with_config(config);
            let mut all_edits = Vec::new();

            // Process each range
            for (idx, range_val) in ranges_array.iter().enumerate() {
                let start_line_u64 =
                    range_val.pointer("/start/line").and_then(|v| v.as_u64()).ok_or_else(|| {
                        invalid_params(&format!("Missing ranges[{}].start.line", idx))
                    })?;
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

                let end_line_u64 = range_val
                    .pointer("/end/line")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| invalid_params(&format!("Missing ranges[{}].end.line", idx)))?;
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
                        tracing::warn!(idx, error = %e, "Range formatting error");
                        return Err(formatting_error_to_rpc(
                            &format!("Range formatting failed for range {}", idx),
                            e,
                        ));
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

        Ok(Some(json!([])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::formatting::FormattingError;
    use perl_tdd_support::must_some;

    #[test]
    fn formatting_error_to_rpc_not_found_has_data_field() {
        let err = FormattingError::PerltidyNotFound("command not found".to_string());
        let rpc = formatting_error_to_rpc("Formatting failed", err);
        assert_eq!(rpc.code, -32603);
        assert!(rpc.message.contains("Formatting failed"), "message should contain context prefix");
        assert!(
            rpc.message.contains("perltidy not found"),
            "message should contain the error description"
        );
        assert!(
            rpc.message.contains("cpanm Perl::Tidy"),
            "message should contain cpanm install recommendation"
        );
        let data = must_some(rpc.data);
        assert_eq!(
            data["error_kind"].as_str(),
            Some("perltidy_not_found"),
            "data.error_kind should be 'perltidy_not_found'"
        );
    }

    #[test]
    fn formatting_error_to_rpc_execution_error_has_data_field() {
        let err = FormattingError::PerltidyError("syntax error at line 3".to_string());
        let rpc = formatting_error_to_rpc("Range formatting failed", err);
        assert_eq!(rpc.code, -32603);
        assert!(rpc.message.contains("Range formatting failed"));
        assert!(
            rpc.message.contains("check Perl syntax") || rpc.message.contains("perltidy error")
        );
        let data = must_some(rpc.data);
        assert_eq!(
            data["error_kind"].as_str(),
            Some("perltidy_error"),
            "data.error_kind should be 'perltidy_error'"
        );
    }

    #[test]
    fn formatting_error_to_rpc_io_error_has_data_field() {
        let err = FormattingError::IoError("disk full".to_string());
        let rpc = formatting_error_to_rpc("Formatting failed", err);
        let data = must_some(rpc.data);
        assert_eq!(
            data["error_kind"].as_str(),
            Some("io_error"),
            "data.error_kind should be 'io_error'"
        );
    }

    #[test]
    fn document_not_open_error_uses_invalid_request_code() {
        let uri = "file:///tmp/missing.pl";
        let rpc = document_not_open_error(uri);
        assert_eq!(rpc.code, INVALID_REQUEST);
        assert_eq!(rpc.message, format!("Document not open: {uri}"));
        assert!(rpc.data.is_none(), "document-not-open error should not include data");
    }
}

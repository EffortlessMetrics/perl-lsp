use super::*;

impl LspServer {
    /// Handle didClose notification
    ///
    /// Deterministic state transition: notify coordinator of document close
    /// so it can update pending change tracking if needed.
    pub(crate) fn handle_did_close(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;

            tracing::debug!("Document closed: {}", uri);

            // Notify coordinator of pending change to track cleanup work
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(uri);
            }

            self.evict_open_document_session_state(uri);

            // Notify coordinator that cleanup is complete
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Clear diagnostics for this file using centralized notify
            if let Err(e) = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": uri,
                    "diagnostics": []
                }),
            ) {
                tracing::warn!("Failed to clear diagnostics for {}: {}", uri, e);
            }
        }

        Ok(())
    }

    /// Handle didSave notification
    pub(crate) fn handle_did_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;
            let normalized_uri = self.normalize_uri_key(uri);
            let _version = params
                .pointer("/textDocument/version")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok());

            tracing::debug!("Document saved: {}", uri);

            // Re-run diagnostics on save to catch any changes
            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, &normalized_uri) {
                if let Some(ref ast) = doc.ast {
                    // Run diagnostics, threading workspace semantic queries when available.
                    let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                    let source_path = source_path_from_uri(uri);

                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                    let diagnostics = {
                        // Attempt semantic-aware path; fall back to legacy when URI not indexed.
                        let semantic_diags = self.workspace_index().and_then(|workspace_index| {
                            workspace_index.with_semantic_queries_for_uri(
                                uri,
                                |file_id, queries| {
                                    provider.get_diagnostics_with_path_and_semantics(
                                        ast,
                                        &doc.parse_errors,
                                        &doc.text,
                                        None,
                                        &[],
                                        source_path.as_deref(),
                                        file_id,
                                        &queries,
                                    )
                                },
                            )
                        });
                        semantic_diags.unwrap_or_else(|| {
                            provider.get_diagnostics_with_path(
                                ast,
                                &doc.parse_errors,
                                &doc.text,
                                None,
                                &[],
                                source_path.as_deref(),
                            )
                        })
                    };
                    #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
                    let diagnostics = provider.get_diagnostics_with_path(
                        ast,
                        &doc.parse_errors,
                        &doc.text,
                        None,
                        &[],
                        source_path.as_deref(),
                    );

                    // Convert diagnostics
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .iter()
                        .map(|diag| {
                            let (start_line, start_char) = self.offset_to_pos16(doc, diag.range.0);
                            let (end_line, end_char) = self.offset_to_pos16(doc, diag.range.1);

                            json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "severity": match diag.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "message": diag.message,
                                "source": "perl"
                            })
                        })
                        .collect();

                    // Send diagnostics notification
                    if let Err(e) = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": uri,
                            "diagnostics": lsp_diagnostics
                        }),
                    ) {
                        tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
                    }
                }
            }

            // Optionally, trigger any post-save hooks here
            // For example: format on save, run tests, etc.
        }

        Ok(())
    }

    /// Handle willSave notification
    pub(crate) fn handle_will_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let reason = params["reason"].as_u64().unwrap_or(1); // 1 = Manual, 2 = AfterDelay, 3 = FocusOut

            tracing::debug!("Document will save: {} (reason: {})", uri, reason);

            // Pre-save validation or cleanup can be done here
            // For example: remove trailing whitespace, fix imports, etc.
        }

        Ok(())
    }

    /// Handle willSaveWaitUntil request
    pub(crate) fn handle_will_save_wait_until(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let _reason = params["reason"].as_u64().unwrap_or(1);

            tracing::debug!("Document will save wait until: {}", uri);

            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Return text edits to be applied before saving
                // For example: format document, organize imports, etc.

                // Check if we should format on save
                let config = self.config.lock();
                if config.test_runner_enabled {
                    // Using existing config field as example
                    // Could add format_on_save config option
                    let formatter = CodeFormatter::new();
                    let format_options = FormattingOptions {
                        tab_size: 4,
                        insert_spaces: true,
                        trim_trailing_whitespace: Some(true),
                        insert_final_newline: Some(true),
                        trim_final_newlines: Some(true),
                    };

                    if let Ok(edits) = formatter.format_document(&doc.text, &format_options) {
                        if !edits.is_empty() {
                            // Convert FormatTextEdit to LSP TextEdit
                            // The edits already have line/character positions
                            let lsp_edits: Vec<Value> = edits
                                .iter()
                                .map(|edit| {
                                    json!({
                                        "range": {
                                            "start": {
                                                "line": edit.range.start.line,
                                                "character": edit.range.start.character
                                            },
                                            "end": {
                                                "line": edit.range.end.line,
                                                "character": edit.range.end.character
                                            }
                                        },
                                        "newText": edit.new_text
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(lsp_edits)));
                        }
                    }
                }
            }
        }

        // Return empty array if no edits
        Ok(Some(json!([])))
    }

    /// Get the end position of a document
    pub(crate) fn get_document_end_position(&self, content: &str) -> Value {
        let lines: Vec<&str> = content.split('\n').collect();
        let last_line = lines.len().saturating_sub(1);
        let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

        json!({
            "line": last_line,
            "character": last_char
        })
    }
}

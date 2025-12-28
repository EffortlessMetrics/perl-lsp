//! Diagnostic publishing and handling
//!
//! Handles both push and pull diagnostics for the LSP server.
//! - Push diagnostics: Server-initiated via `textDocument/publishDiagnostics`
//! - Pull diagnostics: Client-initiated via `textDocument/diagnostic` and `workspace/diagnostic`

use super::*;

impl LspServer {
    /// Publish diagnostics for a document (push diagnostics)
    ///
    /// This method computes diagnostics for the given document and publishes them
    /// to the client via the `textDocument/publishDiagnostics` notification.
    /// Only publishes if the client doesn't support pull diagnostics to avoid
    /// double-flow for modern clients.
    pub(crate) fn publish_diagnostics(&self, uri: &str) {
        let documents = self.documents.lock().unwrap();
        if let Some(doc) = documents.get(uri) {
            let lsp_diagnostics: Vec<Value> = if let Some(ast) = &doc.ast {
                // Get diagnostics (already includes unused variable detection)
                let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                let mut diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                // Add Perl::Critic built-in analysis
                let built_in_analyzer = BuiltInAnalyzer::new();
                let violations = built_in_analyzer.analyze(ast, &doc.text);
                for violation in violations {
                    diagnostics.push(crate::Diagnostic {
                        range: (violation.range.start.byte, violation.range.end.byte),
                        severity: violation.severity.to_diagnostic_severity(),
                        code: Some(violation.policy),
                        message: violation.description,
                        related_information: Vec::new(),
                        tags: Vec::new(),
                    });
                }

                // Convert to LSP diagnostics
                diagnostics
                    .into_iter()
                    .map(|d| {
                        let (start_line, start_char) = self.offset_to_pos16(doc, d.range.0);
                        let (end_line, end_char) = self.offset_to_pos16(doc, d.range.1);

                        json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "severity": match d.severity {
                                InternalDiagnosticSeverity::Error => 1,
                                InternalDiagnosticSeverity::Warning => 2,
                                InternalDiagnosticSeverity::Information => 3,
                                InternalDiagnosticSeverity::Hint => 4,
                            },
                            "code": d.code,
                            "source": "perl-parser",
                            "message": d.message,
                        })
                    })
                    .collect()
            } else {
                // No AST available (parse failed completely), just report parse errors
                doc.parse_errors
                    .iter()
                    .map(|e| {
                        // Extract location and message from error enum
                        let (location, message) = match e {
                            crate::error::ParseError::UnexpectedToken {
                                location,
                                expected,
                                found,
                            } => (*location, format!("Expected {}, found {}", expected, found)),
                            crate::error::ParseError::SyntaxError { location, message } => {
                                (*location, message.clone())
                            }
                            crate::error::ParseError::UnexpectedEof => {
                                (doc.text.len(), "Unexpected end of input".to_string())
                            }
                            crate::error::ParseError::LexerError { message } => {
                                (0, message.clone())
                            }
                            _ => (0, e.to_string()),
                        };

                        // Convert byte offset to line/column
                        let (line, character) = self.offset_to_pos16(doc, location);

                        json!({
                            "range": {
                                "start": {"line": line, "character": character},
                                "end": {"line": line, "character": character + 1},
                            },
                            "severity": 1, // Error
                            "code": "parse-error",
                            "source": "perl-parser",
                            "message": message,
                        })
                    })
                    .collect()
            };

            eprintln!(
                "Publishing {} diagnostics for {} (version {})",
                lsp_diagnostics.len(),
                uri,
                doc.version
            );

            // Only publish if client doesn't support pull diagnostics
            // This avoids double-flow for modern clients
            if !self.client_supports_pull_diags.load(Ordering::Relaxed) {
                // Send diagnostics notification with version
                // This ensures diagnostics are cleared when all errors are fixed
                let _ = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "version": doc.version,
                        "diagnostics": lsp_diagnostics
                    }),
                );
            }
        }
    }

    /// Handle textDocument/diagnostic request (pull diagnostics - LSP 3.17)
    ///
    /// Returns diagnostics for a single document. Supports result caching via
    /// `previousResultId` to return "unchanged" when the document hasn't changed.
    pub(super) fn handle_document_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let previous_result_id = params["previousResultId"].as_str().map(|s| s.to_string());

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Get diagnostics from the existing provider
                if let Some(ast) = &doc.ast {
                    let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                    let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                    // Generate a result ID based on content
                    let result_id = format!("{:x}", md5::compute(&doc.text));

                    // If the result ID matches the previous one, return unchanged
                    if let Some(prev_id) = previous_result_id {
                        if prev_id == result_id {
                            return Ok(Some(json!({
                                "kind": "unchanged",
                                "resultId": prev_id
                            })));
                        }
                    }

                    // Convert to LSP diagnostics
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .into_iter()
                        .enumerate()
                        .map(|(j, d)| {
                            // Cooperative yield every 32 items
                            if j & 0x1f == 0 {
                                std::thread::yield_now();
                            }
                            let start_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
                            let end_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);
                            json!({
                                "range": {
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    },
                                },
                                "severity": match d.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "source": "perl-lsp",
                                "message": d.message,
                            })
                        })
                        .collect();

                    return Ok(Some(json!({
                        "kind": "full",
                        "resultId": result_id,
                        "items": lsp_diagnostics
                    })));
                }
            }
        }

        // Return empty diagnostics if document not found
        Ok(Some(json!({
            "kind": "full",
            "items": []
        })))
    }

    /// Handle workspace/diagnostic request (LSP 3.17 pull diagnostics)
    ///
    /// Returns diagnostics for all open documents in the workspace. Supports
    /// incremental updates via `previousResultIds` to return "unchanged" for
    /// documents that haven't changed.
    pub(super) fn handle_workspace_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let previous_result_ids = if let Some(params) = &params {
            if let Some(ids) = params["previousResultIds"].as_array() {
                ids.iter()
                    .filter_map(|item| {
                        let uri = item["uri"].as_str()?;
                        let id = item["value"].as_str()?;
                        Some((uri.to_string(), id.to_string()))
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut items = Vec::new();

        // Collect document snapshots without holding lock
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock().unwrap();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (i, (uri_str, doc)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            // Check if we have a previous result ID for this document
            let prev_id =
                previous_result_ids.iter().find(|(u, _)| u == uri_str).map(|(_, id)| id.clone());

            if let Some(ast) = &doc.ast {
                let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                // Generate result ID
                let result_id = format!("{:x}", md5::compute(&doc.text));

                // Check if unchanged
                let report = if let Some(prev) = prev_id {
                    if prev == result_id {
                        json!({
                            "uri": uri_str,
                            "version": doc.version,
                            "kind": "unchanged",
                            "resultId": prev
                        })
                    } else {
                        // Convert diagnostics
                        let lsp_diagnostics: Vec<Value> = diagnostics
                            .into_iter()
                            .enumerate()
                            .map(|(j, d)| {
                                // Cooperative yield every 32 items
                                if j & 0x1f == 0 {
                                    std::thread::yield_now();
                                }
                                let start_pos =
                                    doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
                                let end_pos =
                                    doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);
                                json!({
                                    "range": {
                                        "start": {
                                            "line": start_pos.0,
                                            "character": start_pos.1,
                                        },
                                        "end": {
                                            "line": end_pos.0,
                                            "character": end_pos.1,
                                        },
                                    },
                                    "severity": match d.severity {
                                        InternalDiagnosticSeverity::Error => 1,
                                        InternalDiagnosticSeverity::Warning => 2,
                                        InternalDiagnosticSeverity::Information => 3,
                                        InternalDiagnosticSeverity::Hint => 4,
                                    },
                                    "source": "perl-lsp",
                                    "message": d.message,
                                })
                            })
                            .collect();

                        json!({
                            "uri": uri_str,
                            "version": doc.version,
                            "kind": "full",
                            "resultId": result_id,
                            "items": lsp_diagnostics
                        })
                    }
                } else {
                    // No previous result, return full
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .into_iter()
                        .enumerate()
                        .map(|(j, d)| {
                            // Cooperative yield every 32 items
                            if j & 0x1f == 0 {
                                std::thread::yield_now();
                            }
                            let start_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
                            let end_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);
                            json!({
                                "range": {
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    },
                                },
                                "severity": match d.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "source": "perl-lsp",
                                "message": d.message,
                            })
                        })
                        .collect();

                    json!({
                        "uri": uri_str,
                        "version": doc.version,
                        "kind": "full",
                        "resultId": result_id,
                        "items": lsp_diagnostics
                    })
                };

                items.push(report);
            }
        }

        Ok(Some(json!({ "items": items })))
    }
}

//! Diagnostic publishing and handling
//!
//! Handles both push and pull diagnostics for the LSP server.
//! - Push diagnostics: Server-initiated via `textDocument/publishDiagnostics`
//! - Pull diagnostics: Client-initiated via `textDocument/diagnostic` and `workspace/diagnostic`

use super::*;
use crate::features::diagnostics::{
    Diagnostic as InternalDiagnostic, DiagnosticTag as InternalDiagnosticTag,
};
use perl_diagnostics_codes::DiagnosticCode;

impl LspServer {
    /// Convert internal diagnostic tags to LSP tag values
    ///
    /// Maps internal `DiagnosticTag` variants to their LSP numeric equivalents:
    /// - Unnecessary → 1
    /// - Deprecated → 2
    fn diagnostic_tags_to_lsp(tags: &[InternalDiagnosticTag]) -> Vec<i32> {
        tags.iter()
            .map(|t| match t {
                InternalDiagnosticTag::Unnecessary => 1,
                InternalDiagnosticTag::Deprecated => 2,
            })
            .collect()
    }

    /// Generate markdown-formatted diagnostic message (LSP 3.18)
    ///
    /// Creates a rich markdown representation of a diagnostic that includes
    /// the error code (if available) and formatted message content. This is
    /// used when the client supports `textDocument.diagnostic.markupMessageSupport`.
    ///
    /// # Arguments
    ///
    /// * `code` - Optional diagnostic code (e.g., "PL001", "PC001")
    /// * `message` - The diagnostic message text
    ///
    /// # Returns
    ///
    /// A markdown-formatted string with the diagnostic information
    fn generate_diagnostic_markdown(&self, code: Option<&str>, message: &str) -> String {
        if let Some(c) = code { format!("**{}**: {}", c, message) } else { message.to_string() }
    }

    /// Publish diagnostics for a document (push diagnostics)
    ///
    /// Computes and publishes diagnostics for a Perl document including syntax
    /// errors, semantic issues, and Perl::Critic-style code quality checks.
    /// Uses push-based notification model for backward compatibility with LSP 3.16 clients.
    ///
    /// # LSP Protocol
    ///
    /// Notification: `textDocument/publishDiagnostics`
    /// Capability: `textDocument.publishDiagnostics`
    ///
    /// # Arguments
    ///
    /// * `uri` - Document URI to compute diagnostics for
    ///
    /// # Diagnostics Sources
    ///
    /// - Parse errors from Perl parser
    /// - Unused variable warnings from scope analysis
    /// - Perl::Critic built-in policy violations
    /// - External perlcritic violations (opt-in via config)
    /// - Semantic errors from type inference
    ///
    /// # Performance
    ///
    /// Only publishes if client doesn't support pull diagnostics to avoid
    /// double-flow for modern LSP 3.17+ clients.
    pub(crate) fn publish_diagnostics(&self, uri: &str) {
        let documents = self.documents.lock();
        let normalized_uri = self.normalize_uri_key(uri);
        if let Some(doc) = documents.get(&normalized_uri).or_else(|| documents.get(uri)) {
            let lsp_diagnostics: Vec<Value> = if let Some(ast) = &doc.ast {
                // Get diagnostics (already includes unused variable detection)
                let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                let mut diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                // Add Perl::Critic built-in analysis
                let built_in_analyzer = BuiltInAnalyzer::new();
                let violations = built_in_analyzer.analyze(ast, &doc.text);
                for violation in violations {
                    diagnostics.push(builtin_violation_to_diagnostic(&violation));
                }

                // Add external perlcritic diagnostics (opt-in)
                self.collect_external_perlcritic_diagnostics(uri, &doc.text, &mut diagnostics);

                // Add dead code diagnostics from workspace-wide symbol analysis
                #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                {
                    if let Some(workspace_index) = self.workspace_index() {
                        let dead_code_diags = perl_lsp_diagnostics::detect_dead_code(
                            &workspace_index,
                            uri,
                            &doc.text,
                            &doc.line_starts,
                        );
                        diagnostics.extend(dead_code_diags);
                    }
                }

                // Convert to LSP diagnostics
                diagnostics
                    .into_iter()
                    .map(|d| {
                        let (start_line, start_char) = self.offset_to_pos16(doc, d.range.0);
                        let (end_line, end_char) = self.offset_to_pos16(doc, d.range.1);

                        let mut diag = json!({
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
                        });
                        if !d.tags.is_empty() {
                            diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                        }
                        diag
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
                            "code": DiagnosticCode::ParseError.as_str(),
                            "source": "perl-parser",
                            "message": message,
                        })
                    })
                    .collect()
            };

            eprintln!(
                "Publishing {} diagnostics for {} (version {}, tier: {})",
                lsp_diagnostics.len(),
                normalized_uri,
                doc.version,
                doc.degradation_tier
            );

            // Only publish if client doesn't support pull diagnostics
            // This avoids double-flow for modern clients
            if !self.client_supports_pull_diags.load(Ordering::Relaxed) {
                // Send diagnostics notification with version
                // This ensures diagnostics are cleared when all errors are fixed
                if let Err(e) = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "version": doc.version,
                        "diagnostics": lsp_diagnostics
                    }),
                ) {
                    eprintln!("Failed to publish diagnostics for {}: {}", uri, e);
                }
            }
        }
    }

    /// Handle textDocument/diagnostic request (pull diagnostics - LSP 3.17)
    ///
    /// Computes diagnostics for a single document using the pull-based model
    /// introduced in LSP 3.17. Supports efficient incremental updates via
    /// content-based result IDs to avoid re-sending unchanged diagnostics.
    ///
    /// # LSP Protocol
    ///
    /// Request: `textDocument/diagnostic`
    /// Response: `DocumentDiagnosticReport`
    /// Capability: `textDocument.diagnostic`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters containing document URI and optional previousResultId
    ///
    /// # Returns
    ///
    /// DocumentDiagnosticReport with kind "unchanged" or "full" depending on content changes
    ///
    /// # Caching Strategy
    ///
    /// Uses MD5 hash of document content as result ID for efficient change detection.
    /// Returns "unchanged" response when content hash matches previousResultId.
    pub(super) fn handle_document_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let previous_result_id = params["previousResultId"].as_str().map(|s| s.to_string());

            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Get diagnostics from the existing provider
                if let Some(ast) = &doc.ast {
                    let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                    let mut diagnostics =
                        provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                    // Add external perlcritic diagnostics (opt-in)
                    self.collect_external_perlcritic_diagnostics(uri, &doc.text, &mut diagnostics);

                    // Add dead code diagnostics from workspace-wide symbol analysis
                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                    {
                        if let Some(workspace_index) = self.workspace_index() {
                            let dead_code_diags = perl_lsp_diagnostics::detect_dead_code(
                                &workspace_index,
                                uri,
                                &doc.text,
                                &doc.line_starts,
                            );
                            diagnostics.extend(dead_code_diags);
                        }
                    }

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
                            let mut diag = json!({
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
                                "code": d.code.clone(),
                                "source": "perl-lsp",
                                "message": d.message.clone(),
                            });

                            // Add diagnostic tags (e.g., Unnecessary, Deprecated)
                            if !d.tags.is_empty() {
                                diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                            }

                            // Add markdown content if client supports it (LSP 3.18)
                            if self.client_capabilities.lock().markup_message_support {
                                let markdown = self
                                    .generate_diagnostic_markdown(d.code.as_deref(), &d.message);
                                diag["data"] = json!({
                                    "messageMarkup": {
                                        "kind": "markdown",
                                        "value": markdown
                                    }
                                });
                            }

                            diag
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
    /// Computes diagnostics for all open documents in the workspace using the
    /// pull-based model. Provides efficient batch processing with incremental
    /// updates via content-based result IDs.
    ///
    /// # LSP Protocol
    ///
    /// Request: `workspace/diagnostic`
    /// Response: `WorkspaceDiagnosticReport`
    /// Capability: `workspace.diagnostics`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters with optional previousResultIds map
    ///
    /// # Returns
    ///
    /// WorkspaceDiagnosticReport containing document diagnostic reports with
    /// "unchanged" or "full" kind per document based on content changes
    ///
    /// # Performance
    ///
    /// - Cooperative yielding every 8 documents for responsiveness
    /// - MD5-based content hashing for efficient change detection
    /// - Lock-free document snapshot to avoid blocking other requests
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
            let documents = self.documents.lock();
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
                let mut diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                // Add external perlcritic diagnostics (opt-in)
                self.collect_external_perlcritic_diagnostics(uri_str, &doc.text, &mut diagnostics);

                // Add dead code diagnostics from workspace-wide symbol analysis
                #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                {
                    if let Some(workspace_index) = self.workspace_index() {
                        let dead_code_diags = perl_lsp_diagnostics::detect_dead_code(
                            &workspace_index,
                            uri_str,
                            &doc.text,
                            &doc.line_starts,
                        );
                        diagnostics.extend(dead_code_diags);
                    }
                }

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
                                let mut diag = json!({
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
                                    "code": d.code.clone(),
                                    "source": "perl-lsp",
                                    "message": d.message,
                                });
                                if !d.tags.is_empty() {
                                    diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                                }
                                diag
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
                            let mut diag = json!({
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
                                "code": d.code,
                                "source": "perl-lsp",
                                "message": d.message,
                            });
                            if !d.tags.is_empty() {
                                diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                            }
                            diag
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

    /// Collect external perlcritic diagnostics if the feature is enabled.
    ///
    /// Checks the `perlcritic_enabled` config flag and whether `perlcritic` is
    /// installed on the system. If both conditions are met, runs perlcritic on
    /// the file and appends violations with severity mapped from Perl::Critic's
    /// 1-5 scale to LSP severity levels (Brutal/Cruel -> Error, Harsh ->
    /// Warning, Stern/Gentle -> Information).
    ///
    /// Silently skips if perlcritic is not installed or the URI is not a file.
    /// The `doc_text` parameter is used to convert perlcritic's line/column
    /// positions into byte offsets for the internal diagnostic range.
    #[cfg(not(target_arch = "wasm32"))]
    fn collect_external_perlcritic_diagnostics(
        &self,
        uri: &str,
        doc_text: &str,
        diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
        // Check config: perlcritic must be explicitly enabled (opt-in)
        let (enabled, severity, profile) = {
            let cfg = self.config.lock();
            (cfg.perlcritic_enabled, cfg.perlcritic_severity, cfg.perlcritic_profile.clone())
        };
        if !enabled {
            return;
        }

        // Convert URI to file system path; skip non-file URIs
        let file_path = match url::Url::parse(uri).ok().and_then(|u| u.to_file_path().ok()) {
            Some(p) => p,
            None => return,
        };

        // Silently skip if perlcritic is not installed
        if !crate::execute_command::command_exists("perlcritic") {
            return;
        }

        // Auto-discover .perlcriticrc in the file's directory if no profile is configured
        let resolved_profile = profile.or_else(|| {
            file_path.parent().and_then(|dir| {
                let candidate = dir.join(".perlcriticrc");
                if candidate.exists() { candidate.to_str().map(|s| s.to_string()) } else { None }
            })
        });

        let critic_config = crate::perl_critic::CriticConfig {
            severity,
            profile: resolved_profile,
            ..crate::perl_critic::CriticConfig::default()
        };
        let mut analyzer = crate::perl_critic::CriticAnalyzer::with_os_runtime(critic_config);

        match analyzer.analyze_file(&file_path) {
            Ok(violations) => {
                for v in violations {
                    // Map Perl::Critic severity (1-5) to LSP DiagnosticSeverity:
                    // Brutal(1)/Cruel(2) -> Error, Harsh(3) -> Warning,
                    // Stern(4)/Gentle(5) -> Information
                    let internal_severity = match v.severity {
                        crate::perl_critic::Severity::Brutal
                        | crate::perl_critic::Severity::Cruel => InternalDiagnosticSeverity::Error,
                        crate::perl_critic::Severity::Harsh => InternalDiagnosticSeverity::Warning,
                        crate::perl_critic::Severity::Stern
                        | crate::perl_critic::Severity::Gentle => {
                            InternalDiagnosticSeverity::Information
                        }
                    };

                    // Convert 0-indexed line/column from CriticAnalyzer to byte offsets.
                    let line_0 = v.range.start.line;
                    let col_0 = v.range.start.column;
                    let start_byte = position_to_offset(doc_text, line_0, col_0).unwrap_or(0);
                    let end_byte =
                        position_to_offset(doc_text, v.range.end.line, v.range.end.column)
                            .unwrap_or(start_byte.saturating_add(1));

                    diagnostics.push(InternalDiagnostic {
                        range: (start_byte, end_byte),
                        severity: internal_severity,
                        code: Some(format!("PC:{}", v.policy)),
                        message: v.description,
                        related_information: Vec::new(),
                        tags: Vec::new(),
                        suggestion: None,
                    });
                }
            }
            Err(e) => {
                eprintln!("perlcritic failed for {}: {}", uri, e);
            }
        }
    }

    /// No-op stub for WASM targets where subprocess execution is unavailable.
    #[cfg(target_arch = "wasm32")]
    fn collect_external_perlcritic_diagnostics(
        &self,
        _uri: &str,
        _doc_text: &str,
        _diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
    }
}

/// Convert a built-in analyzer violation to an internal diagnostic.
fn builtin_violation_to_diagnostic(
    violation: &crate::perl_critic::Violation,
) -> InternalDiagnostic {
    let lsp_severity = violation.severity.to_diagnostic_severity();
    let internal_severity = match lsp_severity {
        lsp_types::DiagnosticSeverity::ERROR => InternalDiagnosticSeverity::Error,
        lsp_types::DiagnosticSeverity::WARNING => InternalDiagnosticSeverity::Warning,
        lsp_types::DiagnosticSeverity::INFORMATION => InternalDiagnosticSeverity::Information,
        lsp_types::DiagnosticSeverity::HINT => InternalDiagnosticSeverity::Hint,
        _ => InternalDiagnosticSeverity::Hint,
    };
    InternalDiagnostic {
        range: (violation.range.start.byte, violation.range.end.byte),
        severity: internal_severity,
        code: Some(violation.policy.clone()),
        message: violation.description.clone(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

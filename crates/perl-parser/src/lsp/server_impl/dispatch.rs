//! Request dispatch and routing
//!
//! Routes incoming JSON-RPC requests to appropriate handlers.
//! This module contains the main request dispatcher and cancellation handling.

use super::*;

/// Enhanced cancelled response with provider context and performance tracking
pub(super) fn enhanced_cancelled_response(
    token: &PerlLspCancellationToken,
    cleanup_context: Option<&ProviderCleanupContext>,
) -> JsonRpcResponse {
    let provider_name =
        if let Some(context) = cleanup_context { &context.provider_type } else { token.provider() };

    let method_name = provider_name.split('/').next_back().unwrap_or(provider_name);
    let message = format!("Request cancelled - {} provider", method_name);

    let mut data = json!({
        "provider": provider_name,
        "request_id": token.request_id(),
        "timestamp": token.timestamp()
    });

    // Add performance tracking
    let elapsed_ms = token.elapsed().as_millis() as u64;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("latency_ms".to_string(), json!(elapsed_ms));
    }

    // Add cleanup context if available
    if let Some(context) = cleanup_context {
        if let Some(obj) = data.as_object_mut() {
            obj.insert(
                "cancelled_at_ms".to_string(),
                json!(context.cancelled_at.elapsed().as_millis() as u64),
            );

            if let Some(params) = &context.request_params {
                obj.insert("original_params".to_string(), params.clone());
            }
        }
    }

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(token.request_id().clone()),
        result: None,
        error: Some(JsonRpcError { code: REQUEST_CANCELLED, message, data: Some(data) }),
    }
}

/// Macro for early cancellation check in dispatcher arms
macro_rules! early_cancel_or {
    ($self:ident, $id:expr, $handler:expr) => {{
        if let Some(ref _rid) = $id {
            if $self.is_cancelled(_rid) {
                $self.cancel_clear(_rid);
                return Some(cancelled_response(_rid));
            }
        }
        $handler
    }};
}
pub(crate) use early_cancel_or;

impl LspServer {
    /// Handle a JSON-RPC request
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // Handle $/cancelRequest notification with enhanced context processing
        if request.method == "$/cancelRequest" {
            if let Some(params) = request.params.as_ref() {
                if let Some(idv) = params.get("id").cloned() {
                    // Enhanced cancellation with provider context
                    let start_time = Instant::now();

                    // Use global registry for enhanced cancellation
                    if let Ok(_cleanup_context) = GLOBAL_CANCELLATION_REGISTRY.cancel_request(&idv)
                    {
                        let latency = start_time.elapsed();

                        // Log performance metrics for AC12 validation
                        eprintln!(
                            "Enhanced cancellation processed in {:?} for request {:?}",
                            latency, idv
                        );

                        // Validate performance requirements (<50ms end-to-end response time)
                        if latency.as_millis() > 50 {
                            eprintln!("WARNING: Cancellation latency exceeded 50ms: {:?}", latency);
                        }

                        // Optional: Send response with enhanced context for client tracking
                        // (Note: $/cancelRequest is typically a notification, but enhanced context
                        // can be useful for debugging and performance analysis)
                    }

                    // Fallback to legacy cancellation for compatibility
                    self.cancel_mark(&idv);
                }
            }
            return None; // Notifications don't get responses
        }

        // Optimized cancellation setup - batch operations for better performance
        if let Some(ref request_id) = id {
            // Fast path: Check for immediate cancellation before expensive setup
            if self.is_cancelled(request_id) {
                return Some(cancelled_response(request_id));
            }

            // Only register cancellation token for potentially long-running operations
            let needs_cancellation = matches!(
                request.method.as_str(),
                "textDocument/completion"
                    | "textDocument/hover"
                    | "textDocument/definition"
                    | "textDocument/references"
                    | "textDocument/documentSymbol"
                    | "textDocument/codeAction"
                    | "textDocument/formatting"
                    | "textDocument/rename"
                    | "workspace/symbol"
                    | "callHierarchy/incomingCalls"
                    | "callHierarchy/outgoingCalls"
            );

            if needs_cancellation {
                let token =
                    PerlLspCancellationToken::new(request_id.clone(), request.method.clone());
                let cleanup_context =
                    ProviderCleanupContext::new(request.method.clone(), request.params.clone());

                // Batch registration to reduce lock overhead
                let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token);
                let _ = GLOBAL_CANCELLATION_REGISTRY.register_cleanup(request_id, cleanup_context);

                // Quick cancellation check after registration
                if GLOBAL_CANCELLATION_REGISTRY.is_cancelled(request_id) {
                    if let Some(token) = GLOBAL_CANCELLATION_REGISTRY.get_token(request_id) {
                        let cleanup_context =
                            GLOBAL_CANCELLATION_REGISTRY.cancel_request(request_id).ok().flatten();
                        return Some(enhanced_cancelled_response(&token, cleanup_context.as_ref()));
                    }
                    return Some(cancelled_response(request_id));
                }
            }
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "initialized" => {
                self.initialized = true;
                eprintln!("Server initialized");

                // Register file watchers for Perl files only if client supports it
                if self.client_capabilities.dynamic_registration_support {
                    self.register_file_watchers_async();
                }

                // Send index-ready notification
                let _ = self.send_index_ready_notification();

                Ok(None)
            }
            // All other requests require initialization
            _ if !self.initialized && request.method != "shutdown" && request.method != "exit" => {
                Err(JsonRpcError {
                    code: -32002, // ServerNotInitialized per LSP spec
                    message: "Server not initialized".to_string(),
                    data: None,
                })
            }
            "shutdown" => {
                // Clear any pending cancelled requests on shutdown
                self.cancelled.lock().unwrap().clear();
                Ok(Some(json!(null)))
            }
            "textDocument/didOpen" => match self.handle_did_open(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/didChange" => {
                // Use incremental version if available
                #[cfg(feature = "incremental")]
                let result = if std::env::var("PERL_LSP_INCREMENTAL").is_ok() {
                    self.handle_did_change_incremental(request.params)
                } else {
                    self.handle_did_change(request.params)
                };
                #[cfg(not(feature = "incremental"))]
                let result = self.handle_did_change(request.params);
                match result {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "textDocument/didClose" => match self.handle_did_close(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/didSave" => match self.handle_did_save(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/willSave" => match self.handle_will_save(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/willSaveWaitUntil" => self.handle_will_save_wait_until(request.params),
            "textDocument/completion" => early_cancel_or!(self, id, {
                self.handle_completion_cancellable(request.params, id.as_ref())
            }),
            "textDocument/hover" => early_cancel_or!(self, id, {
                self.handle_hover_cancellable(request.params, id.as_ref())
            }),
            "textDocument/signatureHelp" => self.handle_signature_help(request.params),
            "textDocument/definition" => early_cancel_or!(self, id, {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_definition(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_definition_cancellable(request.params, id.as_ref()),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_definition_cancellable(request.params, id.as_ref())
                        .or_else(|_| self.on_definition(json!({})).map(Some))
                }
            }),
            "textDocument/declaration" => self.handle_declaration(request.params),
            "textDocument/references" => early_cancel_or!(self, id, {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_references(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_references(request.params),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_references(request.params)
                        .or_else(|_| self.on_references(json!({})).map(Some))
                }
            }),
            "textDocument/documentHighlight" => self.handle_document_highlight(request.params),
            "textDocument/prepareTypeHierarchy" => {
                self.handle_prepare_type_hierarchy(request.params)
            }
            "typeHierarchy/prepare" => {
                // Alias for deprecated/alternate method string
                self.handle_prepare_type_hierarchy(request.params)
            }
            "typeHierarchy/supertypes" => self.handle_type_hierarchy_supertypes(request.params),
            "typeHierarchy/subtypes" => self.handle_type_hierarchy_subtypes(request.params),
            "textDocument/diagnostic" => self.handle_document_diagnostic(request.params),
            "workspace/diagnostic" => {
                early_cancel_or!(self, id, self.handle_workspace_diagnostic(request.params))
            }
            "textDocument/prepareRename" => self.handle_prepare_rename(request.params),
            // GA contract: not supported in v0.8.3
            // PR 3: Wire workspace/symbol to use the index
            "workspace/symbol" => {
                #[cfg(feature = "workspace")]
                let result = self.handle_workspace_symbols_v2(request.params);
                #[cfg(not(feature = "workspace"))]
                let result = self.handle_workspace_symbols(request.params);
                early_cancel_or!(self, id, result)
            }
            "workspace/symbol/resolve" => self.handle_workspace_symbol_resolve(request.params),

            "textDocument/rename" => self.handle_rename_workspace(request.params),
            "textDocument/codeAction" => self.handle_code_action(request.params),
            "codeAction/resolve" => self.handle_code_action_resolve(request.params),
            // PR 6: Semantic tokens
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(request.params),
            // PR 7: Inlay hints
            "textDocument/inlayHint" => {
                early_cancel_or!(self, id, self.handle_inlay_hints(request.params))
            }
            // PR 8: Document links
            "textDocument/documentLink" => self.handle_document_links(request.params),
            // PR 8: Selection ranges
            "textDocument/selectionRange" => self.handle_selection_range(request.params),
            // PR 9: On-type formatting
            "textDocument/onTypeFormatting" => self.handle_on_type_formatting(request.params),
            // Code lens support
            "textDocument/codeLens" => self.handle_code_lens(request.params),
            "codeLens/resolve" => self.handle_code_lens_resolve(request.params),
            // Linked editing ranges
            "textDocument/linkedEditingRange" => self.handle_linked_editing_range(request.params),
            // Inline completion
            "textDocument/inlineCompletion" => self.handle_inline_completion(request.params),
            // Inline values for debugging
            "textDocument/inlineValue" => self.handle_inline_value(request.params),
            // Monikers
            "textDocument/moniker" => self.handle_moniker(request.params),
            // Document colors
            "textDocument/documentColor" => self.handle_document_color(request.params),
            "textDocument/colorPresentation" => self.handle_color_presentation(request.params),
            // Semantic tokens range
            "textDocument/semanticTokens/range" => {
                self.handle_semantic_tokens_range(request.params)
            }
            // GA contract: these methods remain unsupported in v0.8.3
            "workspace/executeCommand" => self.handle_execute_command(request.params),
            "textDocument/typeDefinition" => self.handle_type_definition(request.params),
            "textDocument/implementation" => self.handle_implementation(request.params),
            "textDocument/documentSymbol" => {
                eprintln!("Processing documentSymbol request");
                let result = self.handle_document_symbol(request.params);
                eprintln!("DocumentSymbol result: {:?}", result.is_ok());
                result
            }
            "textDocument/foldingRange" => {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_folding_range(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_folding_range(request.params),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_folding_range(request.params)
                        .or_else(|_| self.on_folding_range(json!({})).map(Some))
                }
            }
            "textDocument/formatting" => self.handle_formatting(request.params),
            "textDocument/rangeFormatting" => self.handle_range_formatting(request.params),
            "textDocument/prepareCallHierarchy" => {
                self.handle_prepare_call_hierarchy(request.params)
            }
            "callHierarchy/incomingCalls" => self.handle_incoming_calls(request.params),
            "callHierarchy/outgoingCalls" => self.handle_outgoing_calls(request.params),
            "experimental/testDiscovery" => self.handle_test_discovery(request.params),
            "workspace/configuration" => self.handle_configuration(request.params),
            "workspace/didChangeWatchedFiles" => {
                self.handle_did_change_watched_files(request.params)
            }
            "workspace/didChangeWorkspaceFolders" => {
                match self.handle_did_change_workspace_folders(request.params) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "workspace/willRenameFiles" => self.handle_will_rename_files(request.params),
            "workspace/didDeleteFiles" => self.handle_did_delete_files(request.params),
            "workspace/applyEdit" => self.handle_apply_edit(request.params),
            // Test-specific slow operation for cancellation testing
            // This is available in all builds but only used by tests
            "$/test/slowOperation" => {
                // Optional server-side timeout for internal cancellation testing
                let timeout = request
                    .params
                    .as_ref()
                    .and_then(|p| p.get("serverTimeoutMs"))
                    .and_then(|v| v.as_u64())
                    .map(Duration::from_millis);
                let start = Instant::now();

                // Check for cancellation periodically during the slow operation
                // Total time: 20 * 50ms = 1 second
                for i in 0..20 {
                    std::thread::sleep(Duration::from_millis(50));
                    if let Some(ref id) = id {
                        if self.is_cancelled(id) {
                            eprintln!("Operation cancelled at iteration {}", i);
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(id.clone()),
                                result: None,
                                error: Some(request_cancelled_error()),
                            });
                        }

                        if let Some(to) = timeout {
                            if start.elapsed() >= to {
                                eprintln!("Server-side timeout at iteration {}", i);
                                return Some(JsonRpcResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: Some(id.clone()),
                                    result: None,
                                    error: Some(server_cancelled_error()),
                                });
                            }
                        }
                    }
                }
                eprintln!("Slow operation completed without cancellation");
                Ok(Some(json!({"status": "completed", "iterations": 20})))
            }
            _ => {
                eprintln!("Method not implemented: {}", request.method);
                // Enhanced error response with comprehensive context
                Err(enhanced_error(
                    METHOD_NOT_FOUND,
                    &format!("Method '{}' not found or not supported", request.method),
                    "method_not_found",
                    Some(&request.method),
                ))
            }
        };

        // Clean up cancellation token after request processing
        if let Some(ref request_id) = id {
            GLOBAL_CANCELLATION_REGISTRY.remove_request(request_id);
        }

        // Check for enhanced cancellation with provider context
        if let Some(ref request_id) = id {
            if let Some(token) = GLOBAL_CANCELLATION_REGISTRY.get_token(request_id) {
                if token.is_cancelled() {
                    let cleanup_context =
                        GLOBAL_CANCELLATION_REGISTRY.cancel_request(request_id).ok().flatten();
                    return Some(enhanced_cancelled_response(&token, cleanup_context.as_ref()));
                }
            }
        }

        match result {
            Ok(Some(result)) => {
                eprintln!("Sending successful response for request {}", request.method);
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(result),
                    error: None,
                })
            }
            Ok(None) => {
                eprintln!("Request {} is a notification, no response", request.method);
                None // Notification, no response
            }
            Err(error) => {
                eprintln!("Sending error response for request {}: {:?}", request.method, error);
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(error),
                })
            }
        }
    }
}

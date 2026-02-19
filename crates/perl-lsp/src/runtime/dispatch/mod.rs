//! Request dispatch and routing for the LSP server
//!
//! This module implements the JSON-RPC request/response routing layer for the Perl LSP server.
//! It handles incoming requests, dispatches them to appropriate handlers, and manages
//! cancellation tokens for responsive user experience.
//!
//! # Architecture
//!
//! The dispatch layer is organized into focused submodules:
//!
//! - **text_document**: Handlers for document-level operations (completion, hover, definition, etc.)
//! - **workspace**: Handlers for workspace-level operations (symbols, configuration, file events)
//! - **lifecycle**: Handlers for server lifecycle (initialize, shutdown, exit)
//! - **cancellation**: Request cancellation support with provider cleanup context
//! - **experimental**: Experimental features and test endpoints
//!
//! # Request Flow
//!
//! 1. Request arrives via JSON-RPC transport
//! 2. Cancellation token registered for long-running operations
//! 3. Method string matched to handler in `handle_request`
//! 4. Handler invoked with params and optional request ID
//! 5. Response returned (or None for notifications)
//! 6. Cancellation token cleaned up
//!
//! # Cancellation Support
//!
//! Long-running operations (completion, references, workspace symbols) support LSP cancellation:
//!
//! - `$/cancelRequest` notifications mark requests as cancelled
//! - Handlers periodically check cancellation state
//! - Enhanced cancellation includes provider cleanup context for resource management
//! - Performance target: <50ms cancellation response time
//!
//! # Performance Characteristics
//!
//! - **Dispatch overhead**: <1ms for method routing
//! - **Cancellation setup**: <5ms for token registration
//! - **Response serialization**: <10ms for typical responses
//!
//! # Error Handling
//!
//! - ServerNotInitialized (-32002): Returned for requests before initialization
//! - MethodNotFound (-32601): Returned for unknown/unsupported methods
//! - Enhanced error responses include method context for debugging

mod cancellation;
mod experimental;
mod lifecycle;
mod text_document;
mod workspace;

pub(crate) use cancellation::early_cancel_or;
pub(crate) use cancellation::enhanced_cancelled_response;

use super::*;
use crate::cancellation::{
    GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken, ProviderCleanupContext,
};
use std::time::Instant;

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
                return Some(cancelled_response_with_method(request_id, &request.method));
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
                    return Some(cancelled_response_with_method(request_id, &request.method));
                }
            }
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize_dispatch(request.params),
            "initialized" => self.handle_initialized_dispatch(),
            // All other requests require initialization
            _ if !self.initialized && request.method != "shutdown" && request.method != "exit" => {
                Err(JsonRpcError {
                    code: -32002, // ServerNotInitialized per LSP spec
                    message: "Server not initialized".to_string(),
                    data: None,
                })
            }
            "shutdown" => self.handle_shutdown_dispatch(),
            "exit" => self.handle_exit_dispatch(),
            "textDocument/didOpen" => self.handle_did_open_dispatch(request.params),
            "textDocument/didChange" => self.handle_did_change_dispatch(request.params),
            "textDocument/didClose" => self.handle_did_close_dispatch(request.params),
            "textDocument/didSave" => self.handle_did_save_dispatch(request.params),
            "textDocument/willSave" => self.handle_will_save_dispatch(request.params),
            "textDocument/willSaveWaitUntil" => {
                self.handle_will_save_wait_until_dispatch(request.params)
            }
            "notebookDocument/didOpen" => self.handle_notebook_did_open_dispatch(request.params),
            "notebookDocument/didChange" => {
                self.handle_notebook_did_change_dispatch(request.params)
            }
            "notebookDocument/didSave" => self.handle_notebook_did_save_dispatch(request.params),
            "notebookDocument/didClose" => self.handle_notebook_did_close_dispatch(request.params),
            "textDocument/completion" => early_cancel_or!(self, id, "textDocument/completion", {
                self.handle_completion_cancellable_dispatch(request.params, id.as_ref())
            }),
            "completionItem/resolve" => self.handle_completion_resolve_dispatch(request.params),
            "textDocument/hover" => early_cancel_or!(self, id, "textDocument/hover", {
                self.handle_hover_cancellable_dispatch(request.params, id.as_ref())
            }),
            "textDocument/signatureHelp" => self.handle_signature_help_dispatch(request.params),
            "textDocument/definition" => early_cancel_or!(self, id, "textDocument/definition", {
                self.handle_definition_cancellable_dispatch(request.params, id.as_ref())
            }),
            "textDocument/declaration" => self.handle_declaration_dispatch(request.params),
            "textDocument/references" => early_cancel_or!(self, id, "textDocument/references", {
                self.handle_references_dispatch(request.params)
            }),
            "textDocument/documentHighlight" => {
                self.handle_document_highlight_dispatch(request.params)
            }
            "textDocument/prepareTypeHierarchy" => {
                self.handle_prepare_type_hierarchy_dispatch(request.params)
            }
            "typeHierarchy/prepare" => {
                // Alias for deprecated/alternate method string
                self.handle_prepare_type_hierarchy_dispatch(request.params)
            }
            "typeHierarchy/supertypes" => {
                self.handle_type_hierarchy_supertypes_dispatch(request.params)
            }
            "typeHierarchy/subtypes" => {
                self.handle_type_hierarchy_subtypes_dispatch(request.params)
            }
            "textDocument/diagnostic" => self.handle_document_diagnostic_dispatch(request.params),
            "workspace/diagnostic" => early_cancel_or!(
                self,
                id,
                "workspace/diagnostic",
                self.handle_workspace_diagnostic_dispatch(request.params)
            ),
            "textDocument/prepareRename" => self.handle_prepare_rename_dispatch(request.params),
            "workspace/symbol" => early_cancel_or!(self, id, "workspace/symbol", {
                self.handle_workspace_symbols_dispatch(request.params)
            }),
            "workspace/symbol/resolve" => {
                self.handle_workspace_symbol_resolve_dispatch(request.params)
            }
            "textDocument/rename" => self.handle_rename_workspace_dispatch(request.params),
            "textDocument/codeAction" => self.handle_code_action_dispatch(request.params),
            "codeAction/resolve" => self.handle_code_action_resolve_dispatch(request.params),
            "textDocument/semanticTokens/full" => {
                self.handle_semantic_tokens_dispatch(request.params)
            }
            "textDocument/inlayHint" => early_cancel_or!(
                self,
                id,
                "textDocument/inlayHint",
                self.handle_inlay_hints_dispatch(request.params)
            ),
            "inlayHint/resolve" => self.handle_inlay_hint_resolve_dispatch(request.params),
            "textDocument/documentLink" => self.handle_document_links_dispatch(request.params),
            "documentLink/resolve" => self.handle_document_link_resolve_dispatch(request.params),
            "textDocument/selectionRange" => self.handle_selection_range_dispatch(request.params),
            "textDocument/onTypeFormatting" => {
                self.handle_on_type_formatting_dispatch(request.params)
            }
            "textDocument/codeLens" => self.handle_code_lens_dispatch(request.params),
            "codeLens/resolve" => self.handle_code_lens_resolve_dispatch(request.params),
            "textDocument/linkedEditingRange" => {
                self.handle_linked_editing_range_dispatch(request.params)
            }
            "textDocument/inlineCompletion" => {
                self.handle_inline_completion_dispatch(request.params)
            }
            "textDocument/inlineValue" => self.handle_inline_value_dispatch(request.params),
            "textDocument/moniker" => self.handle_moniker_dispatch(request.params),
            "textDocument/documentColor" => self.handle_document_color_dispatch(request.params),
            "textDocument/colorPresentation" => {
                self.handle_color_presentation_dispatch(request.params)
            }
            "textDocument/semanticTokens/range" => {
                self.handle_semantic_tokens_range_dispatch(request.params)
            }
            "workspace/executeCommand" => self.handle_execute_command_dispatch(request.params),
            "textDocument/typeDefinition" => self.handle_type_definition_dispatch(request.params),
            "textDocument/implementation" => self.handle_implementation_dispatch(request.params),
            "textDocument/documentSymbol" => self.handle_document_symbol_dispatch(request.params),
            "textDocument/foldingRange" => self.handle_folding_range_dispatch(request.params),
            "textDocument/formatting" => self.handle_formatting_dispatch(request.params),
            "textDocument/rangeFormatting" => self.handle_range_formatting_dispatch(request.params),
            "textDocument/rangesFormatting" => {
                self.handle_ranges_formatting_dispatch(request.params)
            }
            "textDocument/prepareCallHierarchy" => {
                self.handle_prepare_call_hierarchy_dispatch(request.params)
            }
            "callHierarchy/incomingCalls" => early_cancel_or!(
                self,
                id,
                "callHierarchy/incomingCalls",
                self.handle_incoming_calls_dispatch(request.params)
            ),
            "callHierarchy/outgoingCalls" => early_cancel_or!(
                self,
                id,
                "callHierarchy/outgoingCalls",
                self.handle_outgoing_calls_dispatch(request.params)
            ),
            "experimental/testDiscovery" => self.handle_test_discovery_dispatch(request.params),
            "workspace/configuration" => self.handle_configuration_dispatch(request.params),
            "workspace/didChangeWatchedFiles" => {
                self.handle_did_change_watched_files_dispatch(request.params)
            }
            "workspace/didChangeWorkspaceFolders" => {
                self.handle_did_change_workspace_folders_dispatch(request.params)
            }
            "workspace/didChangeConfiguration" => {
                self.handle_did_change_configuration_dispatch(request.params)
            }
            "window/workDoneProgress/cancel" => {
                self.handle_progress_cancel_dispatch(request.params)
            }
            "workspace/willRenameFiles" => self.handle_will_rename_files_dispatch(request.params),
            "workspace/didRenameFiles" => self.handle_did_rename_files_dispatch(request.params),
            "workspace/willDeleteFiles" => self.handle_will_delete_files_dispatch(request.params),
            "workspace/didDeleteFiles" => self.handle_did_delete_files_dispatch(request.params),
            "workspace/willCreateFiles" => self.handle_will_create_files_dispatch(request.params),
            "workspace/didCreateFiles" => self.handle_did_create_files_dispatch(request.params),
            "workspace/applyEdit" => self.handle_apply_edit_dispatch(request.params),
            "workspace/textDocumentContent" => {
                self.handle_text_document_content_dispatch(request.params)
            }
            "$/setTrace" => self.handle_set_trace_dispatch(request.params),
            "$/test/slowOperation" => self.handle_slow_operation_dispatch(&id, request.params),
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

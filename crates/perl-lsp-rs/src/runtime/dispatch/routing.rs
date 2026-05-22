//! Method routing for JSON-RPC requests.
//!
//! This module owns the method-to-handler table. Preflight checks and response
//! rendering live in sibling modules so routing remains focused on dispatch.

use super::super::*;
use super::response::RoutedResponse;

impl LspServer {
    pub(super) fn route_request(
        &self,
        request: JsonRpcRequest,
        id: Option<Value>,
        should_respond: bool,
    ) -> RoutedResponse {
        let method = request.method.clone();
        let result = match method.as_str() {
            "initialize" => self.handle_initialize_dispatch(request.params),
            "initialized" => self.handle_initialized_dispatch(),
            // Compatibility: some lightweight clients send `initialize` and then
            // immediately issue requests without an explicit `initialized` notification.
            // Accept those requests once `initialize` has completed successfully.
            _ if !self.initialize_requested.load(Ordering::Acquire)
                && method != "shutdown"
                && method != "exit" =>
            {
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
            "textDocument/completion" => {
                return self.route_cancellable(id, method, should_respond, |request_id| {
                    self.handle_completion_cancellable_dispatch(request.params, request_id)
                });
            }
            "completionItem/resolve" => self.handle_completion_resolve_dispatch(request.params),
            "textDocument/hover" => {
                return self.route_cancellable(id, method, should_respond, |request_id| {
                    self.handle_hover_cancellable_dispatch(request.params, request_id)
                });
            }
            "textDocument/signatureHelp" => self.handle_signature_help_dispatch(request.params),
            "textDocument/definition" => {
                return self.route_cancellable(id, method, should_respond, |request_id| {
                    self.handle_definition_cancellable_dispatch(request.params, request_id)
                });
            }
            "textDocument/declaration" => self.handle_declaration_dispatch(request.params),
            "textDocument/references" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_references_dispatch(request.params)
                });
            }
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
            "workspace/diagnostic" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_workspace_diagnostic_dispatch(request.params)
                });
            }
            "textDocument/prepareRename" => self.handle_prepare_rename_dispatch(request.params),
            "workspace/symbol" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_workspace_symbols_dispatch(request.params)
                });
            }
            "workspace/symbol/resolve" => {
                self.handle_workspace_symbol_resolve_dispatch(request.params)
            }
            "textDocument/rename" => self.handle_rename_workspace_dispatch(request.params),
            "textDocument/codeAction" => self.handle_code_action_dispatch(request.params),
            "codeAction/resolve" => self.handle_code_action_resolve_dispatch(request.params),
            "textDocument/semanticTokens/full" => {
                self.handle_semantic_tokens_dispatch(request.params)
            }
            "textDocument/inlayHint" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_inlay_hints_dispatch(request.params)
                });
            }
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
            "textDocument/perlInlineCompletionStream" => {
                self.handle_streaming_inline_completion_dispatch(request.params)
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
            "callHierarchy/incomingCalls" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_incoming_calls_dispatch(request.params)
                });
            }
            "callHierarchy/outgoingCalls" => {
                return self.route_cancellable(id, method, should_respond, |_| {
                    self.handle_outgoing_calls_dispatch(request.params)
                });
            }
            "perl/showAst" => self.handle_show_ast_dispatch(request.params),
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
            "$/perl-lsp/clientResponse" => {
                self.handle_client_response(request.params);
                Ok(None)
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
                tracing::debug!(method = %method, "Method not implemented");
                // Enhanced error response with comprehensive context
                Err(enhanced_error(
                    METHOD_NOT_FOUND,
                    &format!("Method '{}' not found or not supported", method),
                    "method_not_found",
                    Some(&method),
                ))
            }
        };

        self.record_live_provider_decision_trace(&method, &result);
        RoutedResponse::Handler { id, method, should_respond, result }
    }

    fn route_cancellable<F>(
        &self,
        id: Option<Value>,
        method: String,
        should_respond: bool,
        handler: F,
    ) -> RoutedResponse
    where
        F: FnOnce(Option<&Value>) -> Result<Option<Value>, JsonRpcError>,
    {
        if let Some(request_id) = id.as_ref()
            && let Some(typed_id) = JsonRpcId::from_value(request_id)
            && self.is_cancelled(&typed_id)
        {
            self.cancel_clear(&typed_id);
            return RoutedResponse::Immediate(cancelled_response_with_method(request_id, &method));
        }

        let result = handler(id.as_ref());
        self.record_live_provider_decision_trace(&method, &result);
        RoutedResponse::Handler { id, method, should_respond, result }
    }
}

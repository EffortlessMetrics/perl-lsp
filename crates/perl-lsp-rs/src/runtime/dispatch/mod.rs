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
//! 3. Method string matched to handler in `routing::route_request`
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

mod ast_explorer;
mod cancellation;
mod experimental;
mod lifecycle;
mod preflight;
mod request_cancellation;
mod response;
mod routing;
mod text_document;
mod workspace;

pub(crate) use cancellation::enhanced_cancelled_response;

use super::*;

impl LspServer {
    /// Handle a JSON-RPC request
    pub fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let context = preflight::RequestContext::from_request(&request);

        match preflight::prepare_request(self, &request, &context) {
            preflight::PreflightOutcome::Continue => {}
            preflight::PreflightOutcome::NotificationHandled => return None,
            preflight::PreflightOutcome::Respond(response) => return Some(response),
        }

        let routed = self.route_request(request, context.id.clone(), context.should_respond);
        response::finalize_response(context.id.as_ref(), routed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: i64, method: &str, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(JsonRpcId::Integer(id)),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn requests_are_accepted_after_initialize_even_without_initialized_notification() {
        let server = LspServer::new();

        let before_initialize = server
            .handle_request(request(1, "custom/unknown", None))
            .and_then(|response| response.error)
            .map(|error| error.code);
        assert_eq!(before_initialize, Some(-32002));

        let initialize = server.handle_request(request(2, "initialize", Some(json!({}))));
        assert!(
            initialize.as_ref().is_some_and(|response| response.error.is_none()),
            "initialize request should succeed"
        );

        let after_initialize = server
            .handle_request(request(3, "custom/unknown", None))
            .and_then(|response| response.error)
            .map(|error| error.code);
        assert_eq!(after_initialize, Some(-32601));
    }
}

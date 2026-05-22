//! JSON-RPC response construction for dispatched requests.

use super::super::*;
use super::request_cancellation::finalize_cancellation_state;

pub(super) fn finalize_response(
    request_id: Option<&Value>,
    routed: RoutedResponse,
) -> Option<JsonRpcResponse> {
    match routed {
        RoutedResponse::Immediate(response) => Some(response),
        RoutedResponse::Handler { id, method, should_respond, result } => {
            // Check for enhanced cancellation with provider context before cleanup.
            // This preserves cancelled responses for requests that are interrupted while
            // handlers are already running.
            if let Some(cancelled) = finalize_cancellation_state(request_id) {
                return Some(cancelled);
            }

            build_response(id, &method, should_respond, result)
        }
    }
}

pub(super) enum RoutedResponse {
    Immediate(JsonRpcResponse),
    Handler {
        id: Option<Value>,
        method: String,
        should_respond: bool,
        result: Result<Option<Value>, JsonRpcError>,
    },
}

fn build_response(
    id: Option<Value>,
    method: &str,
    should_respond: bool,
    result: Result<Option<Value>, JsonRpcError>,
) -> Option<JsonRpcResponse> {
    let id = id.as_ref().and_then(JsonRpcId::from_value);
    match result {
        Ok(Some(result)) if should_respond => {
            tracing::trace!(method = %method, "Sending successful response");
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: Some(result),
                error: None,
            })
        }
        Ok(Some(_)) => {
            tracing::trace!(method = %method, "Request is a notification (id missing), no response");
            None
        }
        Ok(None) => {
            tracing::trace!(method = %method, "Request is a notification, no response");
            None
        }
        Err(error) if should_respond => {
            tracing::debug!(method = %method, error = ?error, "Sending error response");
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            })
        }
        Err(error) => {
            tracing::debug!(method = %method, error = ?error, "Suppressed error response for notification request");
            None
        }
    }
}

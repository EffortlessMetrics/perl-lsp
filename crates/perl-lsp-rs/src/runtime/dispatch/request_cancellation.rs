use super::*;
use crate::cancellation::{
    GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken, ProviderCleanupContext,
};
use crate::protocol::JsonRpcId;
use std::time::Instant;

pub(super) fn handle_cancel_notification(server: &LspServer, request: &JsonRpcRequest) -> bool {
    if request.method != "$/cancelRequest" {
        return false;
    }

    if let Some(params) = request.params.as_ref()
        && let Some(idv) = params.get("id")
        && let Some(typed_id) = JsonRpcId::try_from_value(idv)
    {
        let start_time = Instant::now();
        if let Ok(_cleanup_context) = GLOBAL_CANCELLATION_REGISTRY.cancel_request(&typed_id) {
            let latency = start_time.elapsed();
            tracing::debug!(latency = ?latency, request = %typed_id, "Enhanced cancellation processed");
            if latency.as_millis() > 50 {
                tracing::warn!(latency = ?latency, "Cancellation latency exceeded 50ms");
            }
        }
        server.cancel_mark(&typed_id);
    }

    true
}

pub(super) fn register_request_cancellation(
    server: &LspServer,
    request_id: Option<&Value>,
    request: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let request_id = request_id?;

    // Convert the raw Value request ID to a typed JsonRpcId at the boundary.
    // If the value isn't a valid request ID shape, skip cancellation tracking.
    let typed_id = JsonRpcId::try_from_value(request_id)?;

    if server.is_cancelled(&typed_id) {
        return Some(cancelled_response_with_method(request_id, &request.method));
    }

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
            | "textDocument/inlayHint"
    );

    if !needs_cancellation {
        return None;
    }

    let token = PerlLspCancellationToken::new(typed_id.clone(), request.method.clone());
    let cleanup_context =
        ProviderCleanupContext::new(request.method.clone(), request.params.clone());

    if let Err(e) = GLOBAL_CANCELLATION_REGISTRY.register_token(token) {
        tracing::trace!(error = %e, "cancellation: failed to register token");
    }
    if let Err(e) = GLOBAL_CANCELLATION_REGISTRY.register_cleanup(&typed_id, cleanup_context) {
        tracing::trace!(error = %e, "cancellation: failed to register cleanup");
    }

    if GLOBAL_CANCELLATION_REGISTRY.is_cancelled(&typed_id) {
        if let Some(token) = GLOBAL_CANCELLATION_REGISTRY.get_token(&typed_id) {
            let cleanup_context = GLOBAL_CANCELLATION_REGISTRY
                .cancel_request(&typed_id)
                .map_err(|e| {
                    tracing::trace!(error = %e, "cancellation: failed to cancel request (early)");
                })
                .ok()
                .flatten();
            return Some(enhanced_cancelled_response(&token, cleanup_context.as_ref()));
        }
        return Some(cancelled_response_with_method(request_id, &request.method));
    }

    None
}

pub(super) fn finalize_cancellation_state(request_id: Option<&Value>) -> Option<JsonRpcResponse> {
    let request_id = request_id?;
    // Convert at the boundary; if conversion fails, skip registry interaction.
    let typed_id = JsonRpcId::try_from_value(request_id)?;

    if let Some(token) = GLOBAL_CANCELLATION_REGISTRY.get_token(&typed_id)
        && token.is_cancelled()
    {
        let cleanup_context = GLOBAL_CANCELLATION_REGISTRY
            .cancel_request(&typed_id)
            .map_err(|e| {
                tracing::trace!(error = %e, "cancellation: failed to cancel request (post-dispatch)");
            })
            .ok()
            .flatten();
        GLOBAL_CANCELLATION_REGISTRY.remove_request(&typed_id);
        return Some(enhanced_cancelled_response(&token, cleanup_context.as_ref()));
    }

    GLOBAL_CANCELLATION_REGISTRY.remove_request(&typed_id);
    None
}

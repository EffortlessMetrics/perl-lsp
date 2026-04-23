//! Cancellation handling for LSP requests
//!
//! Provides enhanced cancellation responses and early cancellation checking macros.

use super::super::*;
use crate::cancellation::{PerlLspCancellationToken, ProviderCleanupContext};
use serde_json::json;

/// Enhanced cancelled response with provider context and performance tracking
pub fn enhanced_cancelled_response(
    token: &PerlLspCancellationToken,
    cleanup_context: Option<&ProviderCleanupContext>,
) -> JsonRpcResponse {
    let provider_name =
        if let Some(context) = cleanup_context { &context.provider_type } else { token.provider() };

    let method_name = provider_name.split('/').next_back().unwrap_or_default();
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
/// Takes the method name to include provider context in cancellation responses
macro_rules! early_cancel_or {
    ($self:ident, $id:expr, $method:expr, $handler:expr) => {{
        if let Some(ref _rid) = $id {
            if $self.is_cancelled(_rid) {
                $self.cancel_clear(_rid);
                return Some(cancelled_response_with_method(_rid, $method));
            }
        }
        $handler
    }};
}
pub(crate) use early_cancel_or;

//! Request preflight checks before routing.
//!
//! Keeps cancellation registration and compatibility initialization separate from
//! method routing and response construction.

use super::super::*;
use super::request_cancellation::{handle_cancel_notification, register_request_cancellation};

pub(super) struct RequestContext {
    pub(super) id: Option<Value>,
    pub(super) should_respond: bool,
}

impl RequestContext {
    pub(super) fn from_request(request: &JsonRpcRequest) -> Self {
        let id = request.id.as_ref().map(JsonRpcId::to_value);
        let should_respond = id.is_some();

        Self { id, should_respond }
    }
}

pub(super) enum PreflightOutcome {
    Continue,
    NotificationHandled,
    Respond(JsonRpcResponse),
}

pub(super) fn prepare_request(
    server: &LspServer,
    request: &JsonRpcRequest,
    context: &RequestContext,
) -> PreflightOutcome {
    if handle_cancel_notification(server, request) {
        return PreflightOutcome::NotificationHandled;
    }

    if let Some(cancelled) = register_request_cancellation(server, context.id.as_ref(), request) {
        return PreflightOutcome::Respond(cancelled);
    }

    auto_initialize_for_compat(server, request);

    PreflightOutcome::Continue
}

fn auto_initialize_for_compat(server: &LspServer, request: &JsonRpcRequest) {
    if !server.initialized.load(Ordering::Acquire)
        && server.initialize_requested.load(Ordering::Acquire)
        && !is_lifecycle_method(&request.method)
    {
        server.auto_initialize_for_compat(&request.method);
    }
}

fn is_lifecycle_method(method: &str) -> bool {
    matches!(method, "initialize" | "initialized" | "shutdown" | "exit")
}

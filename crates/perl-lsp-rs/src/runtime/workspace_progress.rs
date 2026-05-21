//! Workspace indexing progress and readiness notifications.

#[cfg(feature = "workspace")]
use super::outbound::OutboundSender;
#[cfg(feature = "workspace")]
use super::types::ServerRequestId;
#[cfg(feature = "workspace")]
use serde_json::json;

#[cfg(feature = "workspace")]
const WORKSPACE_INDEX_PROGRESS_TOKEN: &str = "workspace-index";

#[cfg(feature = "workspace")]
pub(super) fn send_index_ready_notification(outbound: &OutboundSender, ready: bool) {
    if let Err(e) = outbound.send_notification("perl-lsp/index-ready", json!({ "ready": ready })) {
        tracing::warn!(error = %e, "Failed to send index-ready notification");
    }
}

#[cfg(feature = "workspace")]
pub(super) fn send_progress_create(outbound: &OutboundSender, request_id: ServerRequestId) {
    if let Err(e) = outbound.send_request(
        request_id,
        "window/workDoneProgress/create",
        json!({ "token": WORKSPACE_INDEX_PROGRESS_TOKEN }),
    ) {
        tracing::warn!(error = %e, "Failed to send workDoneProgress/create");
    }
}

#[cfg(feature = "workspace")]
pub(super) fn send_progress_begin(outbound: &OutboundSender) {
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "begin",
                "title": "Indexing workspace",
                "cancellable": false,
                "percentage": 0
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress begin");
    }
}

#[cfg(feature = "workspace")]
pub(super) fn send_progress_report(outbound: &OutboundSender, indexed: usize, total: usize) {
    let percentage = (indexed * 100).checked_div(total).unwrap_or(0).min(99) as u32;
    let message = format!("Indexed {} of {} files", indexed, total);
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "report",
                "message": message,
                "percentage": percentage
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress report");
    }
}

#[cfg(feature = "workspace")]
pub(super) fn send_progress_end(outbound: &OutboundSender, message: &str) {
    if let Err(e) = outbound.send_notification(
        "$/progress",
        json!({
            "token": WORKSPACE_INDEX_PROGRESS_TOKEN,
            "value": {
                "kind": "end",
                "message": message
            }
        }),
    ) {
        tracing::warn!(error = %e, "Failed to send progress end");
    }
}

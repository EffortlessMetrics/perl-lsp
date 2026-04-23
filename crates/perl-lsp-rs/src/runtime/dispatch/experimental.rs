//! Experimental LSP request handlers
//!
//! Wraps experimental and test-only LSP requests.

use super::super::*;
use crate::protocol::{request_cancelled_error, server_cancelled_error};
use serde_json::json;
use std::time::{Duration, Instant};

impl LspServer {
    /// Handle test discovery request
    pub(super) fn handle_test_discovery_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_test_discovery(params)
    }

    /// Handle slow operation test request
    pub(super) fn handle_slow_operation_dispatch(
        &self,
        id: &Option<Value>,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Optional server-side timeout for internal cancellation testing
        let timeout = params
            .as_ref()
            .and_then(|p| p.get("serverTimeoutMs"))
            .and_then(|v| v.as_u64())
            .map(Duration::from_millis);
        let start = Instant::now();

        // Check for cancellation periodically during the slow operation
        // Total time: 20 * 50ms = 1 second
        for i in 0..20 {
            std::thread::sleep(Duration::from_millis(50));
            if let Some(id) = id {
                if self.is_cancelled(id) {
                    tracing::debug!(iteration = i, "Operation cancelled");
                    return Ok(Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null,
                        "error": request_cancelled_error()
                    })));
                }

                if let Some(to) = timeout {
                    if start.elapsed() >= to {
                        tracing::debug!(iteration = i, "Server-side timeout");
                        return Err(server_cancelled_error());
                    }
                }
            }
        }
        tracing::debug!("Slow operation completed without cancellation");
        Ok(Some(json!({"status": "completed", "iterations": 20})))
    }
}

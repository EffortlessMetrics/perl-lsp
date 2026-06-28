//! DAP Capability Advertising Tests
//!
//! Tests that verify capabilities are correctly advertised in the initialize response.
//! Prevents regression of capability/handler mismatches where capabilities are hardcoded
//! false while handlers are implemented and routed (e.g., #1663 Phase 0).

#[cfg(feature = "dap-phase2")]
mod capability_tests {
    use anyhow::Result;
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use serde_json::Value;
    use std::sync::mpsc::channel;

    fn create_test_adapter() -> DebugAdapter {
        let (tx, _rx) = channel();
        let mut adapter = DebugAdapter::new();
        adapter.set_event_sender(tx);
        adapter
    }

    fn extract_initialize_response(msg: DapMessage) -> Result<Value> {
        match msg {
            DapMessage::Response { success, command, body, .. } => {
                if command == "initialize" && success {
                    body.ok_or_else(|| anyhow::anyhow!("initialize response missing body"))
                } else {
                    anyhow::bail!("initialize response not successful")
                }
            }
            _ => anyhow::bail!("expected initialize response"),
        }
    }

    /// Verify that supportsRestartFrame capability is advertised as true.
    /// Corresponds to handler: crates/perl-dap/src/debug_adapter/execution.rs:574
    /// Related issue: #1663 Phase 0
    #[tokio::test]
    async fn test_supports_restart_frame_advertised() -> Result<()> {
        let mut adapter = create_test_adapter();
        let init = adapter.handle_request(1, "initialize", None);
        let caps = extract_initialize_response(init)?;

        let supports_restart_frame = caps
            .get("supportsRestartFrame")
            .ok_or_else(|| anyhow::anyhow!("missing supportsRestartFrame capability"))?;

        assert_eq!(
            supports_restart_frame, true,
            "supportsRestartFrame must be advertised as true when handler exists"
        );
        Ok(())
    }

    /// Verify that supportsStepInTargetsRequest capability is advertised as true.
    /// Corresponds to handler: crates/perl-dap/src/debug_adapter/execution.rs:486
    /// Related issue: #1663 Phase 0
    #[tokio::test]
    async fn test_supports_step_in_targets_request_advertised() -> Result<()> {
        let mut adapter = create_test_adapter();
        let init = adapter.handle_request(1, "initialize", None);
        let caps = extract_initialize_response(init)?;

        let supports_step_in_targets = caps
            .get("supportsStepInTargetsRequest")
            .ok_or_else(|| anyhow::anyhow!("missing supportsStepInTargetsRequest capability"))?;

        assert_eq!(
            supports_step_in_targets, true,
            "supportsStepInTargetsRequest must be advertised as true when handler exists"
        );
        Ok(())
    }

    /// Verify that supportsTerminateThreadsRequest capability is advertised as true.
    /// Corresponds to handler: crates/perl-dap/src/debug_adapter/execution.rs:593
    /// Related issue: #1663 Phase 0
    #[tokio::test]
    async fn test_supports_terminate_threads_request_advertised() -> Result<()> {
        let mut adapter = create_test_adapter();
        let init = adapter.handle_request(1, "initialize", None);
        let caps = extract_initialize_response(init)?;

        let supports_terminate_threads = caps
            .get("supportsTerminateThreadsRequest")
            .ok_or_else(|| anyhow::anyhow!("missing supportsTerminateThreadsRequest capability"))?;

        assert_eq!(
            supports_terminate_threads, true,
            "supportsTerminateThreadsRequest must be advertised as true when handler exists"
        );
        Ok(())
    }

    /// Verify all three critical capabilities are advertised together.
    /// Prevents regression of capability/handler mismatches.
    #[tokio::test]
    async fn test_all_three_capabilities_advertised() -> Result<()> {
        let mut adapter = create_test_adapter();
        let init = adapter.handle_request(1, "initialize", None);
        let caps = extract_initialize_response(init)?;

        let restart_frame = caps
            .get("supportsRestartFrame")
            .ok_or_else(|| anyhow::anyhow!("missing supportsRestartFrame"))?;
        let step_in_targets = caps
            .get("supportsStepInTargetsRequest")
            .ok_or_else(|| anyhow::anyhow!("missing supportsStepInTargetsRequest"))?;
        let terminate_threads = caps
            .get("supportsTerminateThreadsRequest")
            .ok_or_else(|| anyhow::anyhow!("missing supportsTerminateThreadsRequest"))?;

        assert_eq!(restart_frame, true, "supportsRestartFrame should be true");
        assert_eq!(step_in_targets, true, "supportsStepInTargetsRequest should be true");
        assert_eq!(terminate_threads, true, "supportsTerminateThreadsRequest should be true");

        Ok(())
    }
}

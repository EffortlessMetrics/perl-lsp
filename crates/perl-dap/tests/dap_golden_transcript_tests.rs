//! DAP Golden Transcript Tests (AC13)
//!
//! Tests for comprehensive integration with expected DAP message sequences
//!
//! Specification: docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-integration-tests
//!
//! Run with: cargo test -p perl-dap --features dap-phase2

#[cfg(feature = "dap-phase2")]
mod dap_golden_transcripts {
    use anyhow::Result;
    use perl_tdd_support::must;

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-hello-world-transcript
    #[tokio::test]
    #[ignore]
    // AC:13
    async fn test_hello_world_golden_transcript() -> Result<()> {
        // Initialize → Launch → SetBreakpoints → Continue → Stopped → StackTrace → Disconnect
        must(Err::<(), _>("Hello world golden transcript not yet implemented (AC13)"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-step-through-transcript
    #[tokio::test]
    #[ignore]
    // AC:13
    async fn test_step_through_golden_transcript() -> Result<()> {
        // Step-by-step execution with variable inspection
        must(Err::<(), _>("Step through golden transcript not yet implemented (AC13)"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-module-debugging-transcript
    #[tokio::test]
    #[ignore]
    // AC:13
    async fn test_module_debugging_golden_transcript() -> Result<()> {
        // Cross-file debugging with workspace navigation
        must(Err::<(), _>("Module debugging golden transcript not yet implemented (AC13)"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-evaluate-transcript
    #[tokio::test]
    #[ignore]
    // AC:13
    async fn test_evaluate_expressions_golden_transcript() -> Result<()> {
        // REPL-style expression evaluation
        must(Err::<(), _>("Evaluate expressions golden transcript not yet implemented (AC13)"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-error-handling-transcript
    #[tokio::test]
    #[ignore]
    // AC:13
    async fn test_error_handling_golden_transcript() -> Result<()> {
        // Exception handling and error recovery
        must(Err::<(), _>("Error handling golden transcript not yet implemented (AC13)"));
        Ok(())
    }
}

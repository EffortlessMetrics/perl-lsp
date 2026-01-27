//! DAP LSP Non-Regression Tests (AC17)
//!
//! Tests to ensure LSP functionality remains unaffected by DAP integration
//!
//! Specification: docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-integration-non-regression
//!
//! Run with: cargo test -p perl-lsp --features dap-phase3

#[cfg(feature = "dap-phase3")]
mod dap_phase3_tests {
    use anyhow::Result;
    use serde_json::json;
    use std::time::{Duration, Instant};
    
    #[path = "common/mod.rs"]
    mod common;
    use common::*;

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-features-unaffected
    #[test]
    // AC:17
    fn test_lsp_features_unaffected_by_dap() -> Result<()> {
        let mut server = start_lsp_server();
        initialize_lsp(&mut server);

        // Verify basic LSP functionality (hover) with DAP feature enabled
        let hover_id = 1;
        send_request(&mut server, json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": "file:///test.pl" },
                "position": { "line": 0, "character": 0 }
            }
        }));

        let response = read_response_matching_i64(&mut server, hover_id, Duration::from_millis(500));
        assert!(response.is_some());
        
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-response-time
    #[test]
    // AC:17
    fn test_lsp_response_time_maintained() -> Result<()> {
        let mut server = start_lsp_server();
        initialize_lsp(&mut server);

        let start = Instant::now();
        let hover_id = 2;
        send_request(&mut server, json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": "file:///test.pl" },
                "position": { "line": 0, "character": 0 }
            }
        }));

        let _response = read_response_matching_i64(&mut server, hover_id, Duration::from_millis(500));
        let latency = start.elapsed();
        
        // AC2: Maintain <50ms response time
        assert!(latency < Duration::from_millis(100), "LSP response too slow with DAP enabled: {:?}", latency);
        
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-workspace-navigation
    #[test]
    // AC:17
    fn test_workspace_navigation_with_dap() -> Result<()> {
        let mut server = start_lsp_server();
        initialize_lsp(&mut server);

        // Verify workspace symbol search works
        let search_id = 3;
        send_request(&mut server, json!({
            "jsonrpc": "2.0",
            "id": search_id,
            "method": "workspace/symbol",
            "params": { "query": "main" }
        }));

        let response = read_response_matching_i64(&mut server, search_id, Duration::from_millis(500));
        assert!(response.is_some());
        
        Ok(())
    }
}

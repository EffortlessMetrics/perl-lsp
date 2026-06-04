//! Protocol-shape coverage for the in-process LSP E2E harness.

mod support;

use serde_json::json;
use std::time::Duration;
use support::LspHarness;

#[test]
fn harness_preserves_string_request_ids_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new_without_initialize();
    harness.initialize_ready("file:///workspace", None)?;

    let response = harness.request_raw_preserving_id_with_timeout(
        json!({
            "jsonrpc": "2.0",
            "id": "e2e-string-id-1",
            "method": "workspace/symbol",
            "params": { "query": "" }
        }),
        Duration::from_secs(2),
    );

    assert_eq!(
        response.get("id"),
        Some(&json!("e2e-string-id-1")),
        "LSP responses must echo caller-supplied string IDs: {response:#}"
    );
    assert!(
        response.get("error").is_none(),
        "workspace/symbol with a string ID should complete without protocol error: {response:#}"
    );
    assert!(
        response.get("result").is_some(),
        "successful string-ID response should include a result envelope: {response:#}"
    );

    Ok(())
}

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn initialize_advertises_standard_inline_completion_provider() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;
    assert_eq!(init.pointer("/capabilities/inlineCompletionProvider"), Some(&json!({})));
    Ok(())
}

#[test]
fn initialize_does_not_put_inline_completion_provider_under_experimental() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;
    assert!(init.pointer("/capabilities/experimental/inlineCompletionProvider").is_none());
    Ok(())
}

#[test]
fn initialize_preserves_perl_inline_completion_stream_experimental_flag() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({})))?;
    assert_eq!(
        init.pointer("/capabilities/experimental/perlInlineCompletionStream"),
        Some(&json!(true))
    );
    Ok(())
}

#[test]
fn initialized_registers_inline_completion_when_dynamic_registration_supported() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;
    harness.notify("initialized", json!({}));

    let requests = harness.drain_server_requests(500);
    let reg = requests.into_iter().find(|request| {
        request.get("method") == Some(&json!("client/registerCapability"))
            && request.pointer("/params/registrations").and_then(|r| r.as_array()).is_some_and(
                |regs| {
                    regs.iter().any(|entry| {
                        entry.get("method") == Some(&json!("textDocument/inlineCompletion"))
                    })
                },
            )
    });

    let reg = reg.ok_or("expected inline completion client/registerCapability")?;
    let id = reg.get("id").and_then(|v| v.as_i64()).ok_or("id must be integer")?;
    assert!((1..=i32::MAX as i64).contains(&id));
    Ok(())
}

#[test]
fn inline_completion_guardrails() {
    let snap = include_str!("snapshots/lsp_cap_snap__server_capabilities_full_client.snap");
    assert!(
        !snap.contains("inlineCompletionProvider:")
            || !snap.contains("experimental:\n  inlineCompletionProvider"),
        "snapshot must not advertise experimental.inlineCompletionProvider"
    );

    let watchers_src = include_str!("../src/runtime/lifecycle/watchers.rs");
    assert!(
        !watchers_src.contains("self.outbound.send_request"),
        "lifecycle code must not call self.outbound.send_request directly"
    );
    assert!(
        !watchers_src.contains("UNIX_EPOCH") && !watchers_src.contains("as_millis"),
        "registration code must not generate request IDs from timestamps"
    );
}

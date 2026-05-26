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
    let init = harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

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

    let requests = harness.drain_server_requests(500);
    let request = requests
        .into_iter()
        .find(|request| {
            request.get("method") == Some(&json!("client/registerCapability"))
                && request.pointer("/params/registrations").and_then(|r| r.as_array()).is_some_and(
                    |registrations| {
                        registrations.iter().any(|entry| {
                            entry.get("method") == Some(&json!("textDocument/inlineCompletion"))
                        })
                    },
                )
        })
        .ok_or("expected inline completion client/registerCapability")?;

    assert_eq!(
        request.pointer("/params/registrations/0/id"),
        Some(&json!("perl-inlineCompletion"))
    );
    let id = request.get("id").and_then(|v| v.as_i64()).ok_or("request id must be integer")?;
    assert!((1..=i64::from(i32::MAX)).contains(&id));
    Ok(())
}

#[test]
fn disabled_inline_completion_removes_static_and_experimental_capabilities() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize_with_init_options(
        Some(json!({
            "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
        })),
        json!({"disabledFeatures": ["lsp.inline_completion"]}),
    )?;

    assert!(init.pointer("/capabilities/inlineCompletionProvider").is_none());
    assert!(init.pointer("/capabilities/experimental/perlInlineCompletionStream").is_none());

    let requests = harness.drain_server_requests(500);
    let has_inline_registration = requests.iter().any(|request| {
        request.get("method") == Some(&json!("client/registerCapability"))
            && request.pointer("/params/registrations").and_then(|r| r.as_array()).is_some_and(
                |registrations| {
                    registrations.iter().any(|entry| {
                        entry.get("method") == Some(&json!("textDocument/inlineCompletion"))
                    })
                },
            )
    });
    assert!(!has_inline_registration);

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

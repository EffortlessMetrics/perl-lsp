mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn initialize_advertises_standard_inline_completion_provider() -> TestResult {
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(json!({
        "textDocument": {"inlineCompletion": {"dynamicRegistration": true}}
    })))?;

    assert_eq!(init_result.pointer("/capabilities/inlineCompletionProvider"), Some(&json!({})));
    Ok(())
}

#[test]
fn initialize_does_not_put_inline_completion_provider_under_experimental() -> TestResult {
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(json!({
        "textDocument": {"inlineCompletion": {"dynamicRegistration": true}}
    })))?;

    assert!(init_result.pointer("/capabilities/experimental/inlineCompletionProvider").is_none());
    Ok(())
}

#[test]
fn initialize_preserves_perl_inline_completion_stream_flag() -> TestResult {
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(json!({
        "textDocument": {"inlineCompletion": {"dynamicRegistration": true}}
    })))?;

    assert_eq!(
        init_result.pointer("/capabilities/experimental/perlInlineCompletionStream"),
        Some(&json!(true))
    );
    Ok(())
}

#[test]
fn initialized_registers_inline_completion_when_dynamic_registration_supported() -> TestResult {
    let mut harness = LspHarness::new();
    let _ = harness.initialize(Some(json!({
        "textDocument": {"inlineCompletion": {"dynamicRegistration": true}}
    })))?;

    let requests = harness.drain_server_requests(500);
    let request = requests
        .into_iter()
        .find(|req| {
            req.get("method") == Some(&json!("client/registerCapability"))
                && req.pointer("/params/registrations/0/method")
                    == Some(&json!("textDocument/inlineCompletion"))
        })
        .ok_or("missing inline completion dynamic registration")?;

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
    let init_result = harness.initialize_with_init_options(
        Some(json!({
            "textDocument": {"inlineCompletion": {"dynamicRegistration": true}}
        })),
        json!({"disabledFeatures": ["lsp.inline_completion"]}),
    )?;

    assert!(init_result.pointer("/capabilities/inlineCompletionProvider").is_none());
    assert!(init_result.pointer("/capabilities/experimental/perlInlineCompletionStream").is_none());

    let requests = harness.drain_server_requests(500);
    let has_inline_registration = requests.iter().any(|req| {
        req.get("method") == Some(&json!("client/registerCapability"))
            && req.pointer("/params/registrations/0/method")
                == Some(&json!("textDocument/inlineCompletion"))
    });
    assert!(!has_inline_registration);

    Ok(())
}

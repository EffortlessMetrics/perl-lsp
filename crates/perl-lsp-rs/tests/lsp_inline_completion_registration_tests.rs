mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

#[test]
fn initialize_advertises_standard_inline_completion_provider() {
    let mut harness = LspHarness::new_without_initialize();
    let init = harness
        .initialize(Some(json!({
            "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
        })))
        .expect("initialize should succeed");

    assert_eq!(init.pointer("/capabilities/inlineCompletionProvider"), Some(&json!({})));
}

#[test]
fn initialize_does_not_put_inline_completion_provider_under_experimental() {
    let mut harness = LspHarness::new_without_initialize();
    let init =
        harness.initialize(Some(json!({ "textDocument": { "inlineCompletion": {} } }))).unwrap();
    assert!(init.pointer("/capabilities/experimental/inlineCompletionProvider").is_none());
}

#[test]
fn initialize_preserves_perl_inline_completion_stream_experimental_flag() {
    let mut harness = LspHarness::new_without_initialize();
    let init =
        harness.initialize(Some(json!({ "textDocument": { "inlineCompletion": {} } }))).unwrap();
    assert_eq!(
        init.pointer("/capabilities/experimental/perlInlineCompletionStream"),
        Some(&json!(true))
    );
}

#[test]
fn initialized_registers_inline_completion_when_dynamic_registration_supported() {
    let mut harness = LspHarness::new_without_initialize();
    harness
        .initialize(Some(json!({
            "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
        })))
        .unwrap();

    let requests = harness.drain_server_requests(500);
    let inline_reg = requests.iter().find(|req| {
        req.get("method") == Some(&json!("client/registerCapability"))
            && req.pointer("/params/registrations").and_then(|v| v.as_array()).is_some_and(|regs| {
                regs.iter()
                    .any(|reg| reg.get("method") == Some(&json!("textDocument/inlineCompletion")))
            })
    });

    let req = inline_reg.expect("expected inline completion client/registerCapability request");
    let id = req.get("id").and_then(|v| v.as_i64()).expect("request id must be integer");
    assert!((1..=i32::MAX as i64).contains(&id));
}

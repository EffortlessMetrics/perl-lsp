mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn initialize_static_advertises_inline_completion_when_dynamic_not_supported() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({
        "textDocument": {}
    })))?;

    assert_eq!(init.pointer("/capabilities/inlineCompletionProvider"), Some(&json!({})));
    assert_no_inline_completion_registration(harness.drain_server_requests(200));
    Ok(())
}

#[test]
fn initialize_static_advertises_inline_completion_when_dynamic_registration_false() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": false } }
    })))?;

    assert_eq!(init.pointer("/capabilities/inlineCompletionProvider"), Some(&json!({})));
    assert_no_inline_completion_registration(harness.drain_server_requests(200));
    Ok(())
}

#[test]
fn initialize_dynamic_client_does_not_static_advertise_inline_completion() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    assert!(init.pointer("/capabilities/inlineCompletionProvider").is_none());
    Ok(())
}

#[test]
fn initialize_never_puts_inline_completion_provider_under_experimental() -> TestResult {
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

    let (request, registration) =
        find_inline_completion_registration(harness.drain_server_requests(500))
            .ok_or("expected inline completion client/registerCapability")?;

    assert_eq!(registration.get("method"), Some(&json!("textDocument/inlineCompletion")));
    assert_eq!(registration.get("id"), Some(&json!("perl-inlineCompletion")));
    let selector = registration
        .pointer("/registerOptions/documentSelector")
        .and_then(Value::as_array)
        .ok_or("inline completion registration must include documentSelector")?;
    assert!(selector.contains(&json!({ "language": "perl" })));
    assert!(selector.contains(&json!({ "language": "perl5" })));
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
fn lsp4ij_dynamic_inline_completion_with_trigger_context_returns_deterministic_items() -> TestResult
{
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_lsp4ij.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion(&mut harness, uri, 0, 4)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(!items.is_empty(), "expected deterministic inline completion items");
    assert!(
        items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))),
        "expected deterministic strict; suggestion, got: {items:?}"
    );
    Ok(())
}

#[test]
fn inline_completion_invoked_trigger_returns_deterministic_items() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_invoked.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion_with_trigger_kind(&mut harness, uri, 0, 4, 1)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(!items.is_empty(), "invoked trigger must return deterministic items");
    assert!(
        items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))),
        "expected deterministic strict; suggestion for invoked trigger, got: {items:?}"
    );
    Ok(())
}

#[test]
fn inline_completion_selected_completion_info_matching_text_returns_same_range() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_selected_completion_info.pl";
    harness.open(uri, "use ")?;

    let selected_range = json!({
        "start": { "line": 0, "character": 4 },
        "end": { "line": 0, "character": 4 }
    });
    let result = request_inline_completion_with_context(
        &mut harness,
        uri,
        0,
        4,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": selected_range.clone(),
                "text": "strict"
            }
        }),
    )?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    let strict = items
        .iter()
        .find(|item| item.get("insertText") == Some(&json!("strict;")))
        .ok_or("selectedCompletionInfo text strict must return the extending strict; item")?;
    assert_eq!(
        strict.get("range"),
        Some(&selected_range),
        "matching selectedCompletionInfo items must use the selected completion range"
    );
    assert_eq!(strict.pointer("/range/start/line"), strict.pointer("/range/end/line"));
    Ok(())
}

#[test]
fn inline_completion_selected_completion_info_text_mismatch_returns_empty() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_selected_completion_text_mismatch.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion_with_context(
        &mut harness,
        uri,
        0,
        4,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 4 }
                },
                "text": "strictlyDifferent"
            }
        }),
    )?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(items.is_empty(), "non-extending selectedCompletionInfo text must return empty");
    Ok(())
}

#[test]
fn inline_completion_selected_completion_info_range_mismatch_returns_empty() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_selected_completion_range_mismatch.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion_with_context(
        &mut harness,
        uri,
        0,
        4,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 3 }
                },
                "text": "strict"
            }
        }),
    )?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(items.is_empty(), "selectedCompletionInfo range mismatch must return empty");
    Ok(())
}

#[test]
fn inline_completion_selected_completion_info_multiline_range_returns_empty() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_selected_completion_multiline_range.pl";
    harness.open(uri, "use \n")?;

    let result = request_inline_completion_with_context(
        &mut harness,
        uri,
        0,
        4,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 1, "character": 0 }
                },
                "text": "strict"
            }
        }),
    )?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(items.is_empty(), "inline completion ranges must not span multiple lines");
    Ok(())
}

#[test]
fn inline_completion_items_use_string_insert_text() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_insert_text_shape.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion(&mut harness, uri, 0, 4)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(!items.is_empty(), "expected deterministic inline completion items");
    assert!(
        items.iter().all(|item| item.get("insertText").is_some_and(Value::is_string)),
        "InlineCompletionItem.insertText must be a string or valid StringValue object"
    );
    Ok(())
}

#[test]
fn inline_completion_without_context_remains_permissive() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_legacy_no_context.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion_without_context(&mut harness, uri, 0, 4)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(
        items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))),
        "legacy no-context request remains permissive behavior, got: {items:?}"
    );
    Ok(())
}

#[test]
fn static_inline_completion_returns_deterministic_items() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": {}
    })))?;

    let uri = "file:///inline_static.pl";
    harness.open(uri, "use ")?;

    let result = request_inline_completion(&mut harness, uri, 0, 4)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(!items.is_empty(), "expected deterministic inline completion items");
    assert!(
        items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))),
        "expected deterministic strict; suggestion, got: {items:?}"
    );
    Ok(())
}

#[test]
fn inline_completion_unsupported_position_returns_empty_items() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
    })))?;

    let uri = "file:///inline_lsp4ij_neutral.pl";
    harness.open(uri, "my $name = \"World\";")?;

    let result = request_inline_completion(&mut harness, uri, 0, 11)?;
    let items = result
        .get("items")
        .and_then(Value::as_array)
        .ok_or("inline completion result must contain items array")?;

    assert!(items.is_empty(), "unsupported position must not emit noisy suggestions");
    Ok(())
}

#[test]
fn disabled_inline_completion_rejects_runtime_request() -> TestResult {
    let mut harness = LspHarness::new();
    let init = harness.initialize_with_init_options(
        Some(json!({
            "textDocument": { "inlineCompletion": { "dynamicRegistration": true } }
        })),
        json!({"disabledFeatures": ["lsp.inline_completion"]}),
    )?;

    assert!(init.pointer("/capabilities/inlineCompletionProvider").is_none());
    assert!(init.pointer("/capabilities/experimental/perlInlineCompletionStream").is_none());

    let uri = "file:///inline_disabled.pl";
    harness.open(uri, "use ")?;
    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/inlineCompletion",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        }
    }));

    assert_eq!(response.pointer("/error/code"), Some(&json!(-32601)));
    assert_eq!(
        response.pointer("/error/message"),
        Some(&json!("Method not advertised in server capabilities"))
    );
    Ok(())
}

#[cfg(feature = "expose_lsp_test_api")]
#[test]
fn lsp4ij_inline_completion_dynamic_registration_shape_is_parsed() -> TestResult {
    let server = perl_lsp::LspServer::new();
    server.test_handle_initialize_dispatch(Some(json!({
        "capabilities": {
            "textDocument": {
                "inlineCompletion": {
                    "dynamicRegistration": true
                }
            }
        }
    })))?;

    let caps = server.test_client_capabilities();
    assert!(caps.inline_completion_support);
    assert!(caps.inline_completion_dynamic_registration_support);
    Ok(())
}

#[test]
fn inline_completion_guardrails() -> TestResult {
    let minimal_snap =
        include_str!("snapshots/lsp_cap_snap__server_capabilities_minimal_client.snap");
    let full_snap = include_str!("snapshots/lsp_cap_snap__server_capabilities_full_client.snap");
    let production_json = include_str!("snapshots/production_capabilities.json");
    let ga_lock_json = include_str!("snapshots/ga_lock_capabilities.json");
    let all_json = include_str!("snapshots/all_capabilities.json");
    for snapshot in [minimal_snap, full_snap] {
        assert!(
            !snapshot.contains("experimental:\n  inlineCompletionProvider"),
            "snapshot must not advertise experimental.inlineCompletionProvider"
        );
    }
    for snapshot in [production_json, ga_lock_json, all_json] {
        assert!(
            !snapshot.contains("\"experimental\": {\n    \"inlineCompletionProvider\""),
            "capability JSON snapshot must not advertise experimental.inlineCompletionProvider"
        );
    }

    let lifecycle_caps = include_str!("../src/runtime/lifecycle/capabilities.rs");
    assert!(
        !lifecycle_caps.contains(
            "capabilities[\"experimental\"] = json!({\n            \"perlInlineCompletionStream\": true\n        });"
        ),
        "initialize must merge experimental fields instead of overwriting them"
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
    let inline_registration = watchers_src
        .split("pub(crate) fn register_inline_completion_if_needed")
        .nth(1)
        .ok_or("inline completion registration function must exist")?;
    assert!(
        !inline_registration.contains(".dynamic_registration_support")
            && !inline_registration.contains("caps.dynamic_registration_support")
            && !inline_registration
                .contains("client_capabilities.lock().dynamic_registration_support"),
        "inline-completion dynamic registration must not be gated by file-watcher dynamic registration"
    );
    Ok(())
}

fn find_inline_completion_registration(
    requests: Vec<serde_json::Value>,
) -> Option<(serde_json::Value, serde_json::Value)> {
    for request in requests {
        if request.get("method") != Some(&json!("client/registerCapability")) {
            continue;
        }

        let registration = request
            .pointer("/params/registrations")
            .and_then(Value::as_array)
            .and_then(|registrations| {
                registrations.iter().find(|entry| {
                    entry.get("method") == Some(&json!("textDocument/inlineCompletion"))
                })
            })
            .cloned();

        if let Some(registration) = registration {
            return Some((request, registration));
        }
    }

    None
}

fn assert_no_inline_completion_registration(requests: Vec<serde_json::Value>) {
    assert!(
        find_inline_completion_registration(requests).is_none(),
        "static inline completion clients must not also receive dynamic registration"
    );
}

fn request_inline_completion(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
) -> Result<Value, String> {
    request_inline_completion_with_trigger_kind(harness, uri, line, character, 2)
}

fn request_inline_completion_with_trigger_kind(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
    trigger_kind: u8,
) -> Result<Value, String> {
    request_inline_completion_with_context(
        harness,
        uri,
        line,
        character,
        json!({ "triggerKind": trigger_kind }),
    )
}

fn request_inline_completion_with_context(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
    context: Value,
) -> Result<Value, String> {
    harness.request(
        "textDocument/inlineCompletion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": context
        }),
    )
}

fn request_inline_completion_without_context(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
) -> Result<Value, String> {
    harness.request(
        "textDocument/inlineCompletion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )
}

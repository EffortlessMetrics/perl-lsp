//! Integration tests for the streaming inline completion protocol.
//!
//! Validates the `textDocument/perlInlineCompletionStream` custom request,
//! including `$/progress` emission, session management, and fallback behavior.
//!
//! Run with:
//! ```bash
//! RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --features expose_lsp_test_api \
//!     -- streaming --test-threads=2
//! ```

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper: initialize server with default capabilities.
fn init_harness() -> Result<LspHarness, String> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    Ok(harness)
}

/// Helper: enable AI streaming completion via didChangeConfiguration.
fn enable_ai_streaming(harness: &mut LspHarness) {
    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "aiCompletion": {
                        "enabled": true,
                        "streaming": {
                            "enabled": true
                        }
                    }
                }
            }
        }),
    );
    // Give the server time to process the configuration change.
    std::thread::sleep(Duration::from_millis(50));
}

/// Helper: enable AI completion but disable streaming specifically.
fn enable_ai_disable_streaming(harness: &mut LspHarness) {
    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "aiCompletion": {
                        "enabled": true,
                        "streaming": {
                            "enabled": false
                        }
                    }
                }
            }
        }),
    );
    std::thread::sleep(Duration::from_millis(50));
}

// ==================== Streaming with AI enabled ====================

/// The happy path: AI+streaming enabled, partialResultToken present.
/// The handler should return `null` and emit a `$/progress` notification.
#[test]
fn streaming_completion_returns_null_and_emits_progress() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///streaming_test.pl";
    harness.open(uri, "use strict;\nmy $obj = Package->")?;

    // Drain any startup notifications (diagnostics, etc.)
    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    let result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 19 },
            "partialResultToken": "stream-token-1"
        }),
    )?;

    // The handler returns null -- all data is sent via $/progress.
    assert!(
        result.is_null(),
        "expected null response for streaming request, got: {result}"
    );

    // Verify that a $/progress notification was emitted.
    let progress_notifications = harness.drain_notifications(Some("$/progress"), 500);
    let matching: Vec<_> = progress_notifications
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("stream-token-1"))
        .collect();

    assert!(
        !matching.is_empty(),
        "expected at least one $/progress notification with token 'stream-token-1', \
         got {} total progress notifications",
        progress_notifications.len()
    );

    // Validate the progress payload structure.
    let progress = matching[0];
    let value = &progress["params"]["value"];
    assert_eq!(
        value["kind"].as_str(),
        Some("perlInlineCompletionStream"),
        "progress kind must be 'perlInlineCompletionStream'"
    );
    assert!(
        value.get("sessionId").is_some(),
        "progress must contain a sessionId"
    );
    assert!(
        value.get("sequence").is_some(),
        "progress must contain a sequence number"
    );
    assert_eq!(
        value["isFinal"].as_bool(),
        Some(true),
        "current implementation emits a single final progress"
    );
    assert!(
        value.get("items").is_some(),
        "progress must contain an items array"
    );

    Ok(())
}

/// Verify the progress session ID format and that the sequence starts at 0.
#[test]
fn streaming_completion_progress_has_valid_session_and_sequence() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///session_test.pl";
    harness.open(uri, "sub foo {\n    \n}")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    let _result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 4 },
            "partialResultToken": "sess-check-token"
        }),
    )?;

    let progress_notifications = harness.drain_notifications(Some("$/progress"), 500);
    let matching: Vec<_> = progress_notifications
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("sess-check-token"))
        .collect();

    assert!(!matching.is_empty(), "expected progress notification");

    let value = &matching[0]["params"]["value"];
    let session_id = value["sessionId"]
        .as_str()
        .ok_or("sessionId should be a string")?;
    assert!(
        session_id.starts_with("sess-"),
        "session ID should start with 'sess-', got: {session_id}"
    );

    let sequence = value["sequence"].as_u64();
    assert_eq!(sequence, Some(0), "first progress sequence should be 0");

    Ok(())
}

// ==================== Fallback: AI disabled ====================

/// When AI completion is disabled, the streaming request should fall back
/// to the one-shot inline completion handler and return items directly.
#[test]
fn streaming_completion_without_ai_falls_back_to_one_shot() -> TestResult {
    let mut harness = init_harness()?;
    // AI is disabled by default; do NOT call enable_ai_streaming.

    let uri = "file:///fallback_ai_disabled.pl";
    harness.open(uri, "my $obj = Package->")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    let result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 0, "character": 19 },
            "partialResultToken": "fallback-token-1"
        }),
    )?;

    // With AI disabled, the handler falls back to one-shot inline completion,
    // which returns an items array (not null).
    let items = result
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("expected items array in fallback response")?;
    assert!(
        !items.is_empty(),
        "one-shot fallback should return completions for 'Package->'"
    );
    assert_eq!(
        items[0]["insertText"].as_str(),
        Some("new()"),
        "expected 'new()' from deterministic one-shot handler"
    );

    // No $/progress should have been emitted for this request.
    let progress = harness.drain_notifications(Some("$/progress"), 200);
    let matching: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("fallback-token-1"))
        .collect();
    assert!(
        matching.is_empty(),
        "no progress notifications expected when AI is disabled"
    );

    Ok(())
}

/// When AI is enabled but streaming specifically is disabled, the streaming
/// request should also fall back to one-shot.
#[test]
fn streaming_completion_with_streaming_disabled_falls_back() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_disable_streaming(&mut harness);

    let uri = "file:///fallback_streaming_disabled.pl";
    harness.open(uri, "my $obj = Package->")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    let result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 0, "character": 19 },
            "partialResultToken": "stream-disabled-token"
        }),
    )?;

    // Falls back to one-shot -- returns items, not null.
    let items = result
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("expected items array in fallback response")?;
    assert!(
        !items.is_empty(),
        "one-shot fallback should return completions"
    );

    Ok(())
}

// ==================== Fallback: no partialResultToken ====================

/// When the client omits partialResultToken, the handler must fall back to
/// one-shot inline completion regardless of AI config.
#[test]
fn streaming_completion_without_partial_result_token_falls_back() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///no_token.pl";
    harness.open(uri, "my $obj = Package->")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    // Omit partialResultToken entirely.
    let result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 0, "character": 19 }
        }),
    )?;

    // Without a token, falls back to one-shot -- returns items.
    let items = result
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("expected items array when partialResultToken is missing")?;
    assert!(
        !items.is_empty(),
        "one-shot fallback should return completions"
    );

    Ok(())
}

// ==================== Session cancellation ====================

/// Sending two streaming requests for the same position should cancel the
/// first session. Verify the server handles this without error and both
/// return null.
#[test]
fn streaming_completion_second_request_cancels_first_session() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///cancel_test.pl";
    harness.open(uri, "use strict;\nmy $x = ")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    // First request
    let result1 = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 9 },
            "partialResultToken": "cancel-token-1"
        }),
    )?;
    assert!(result1.is_null(), "first streaming response should be null");

    // Second request at same position -- cancels the first session.
    let result2 = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 9 },
            "partialResultToken": "cancel-token-2"
        }),
    )?;
    assert!(
        result2.is_null(),
        "second streaming response should be null"
    );

    // Both should have emitted progress, but with different session IDs.
    let progress = harness.drain_notifications(Some("$/progress"), 500);
    let token1_progress: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("cancel-token-1"))
        .collect();
    let token2_progress: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("cancel-token-2"))
        .collect();

    assert!(
        !token1_progress.is_empty(),
        "first request should emit progress"
    );
    assert!(
        !token2_progress.is_empty(),
        "second request should emit progress"
    );

    // Verify different session IDs.
    let sid1 = token1_progress[0]
        .pointer("/params/value/sessionId")
        .and_then(|v| v.as_str());
    let sid2 = token2_progress[0]
        .pointer("/params/value/sessionId")
        .and_then(|v| v.as_str());
    assert_ne!(
        sid1, sid2,
        "two requests at the same position should produce different session IDs"
    );

    Ok(())
}

// ==================== URI cancellation ====================

/// After closing a document, subsequent streaming requests for that URI
/// should return null without crashing.
#[test]
fn streaming_completion_on_closed_doc_returns_null() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///closed_doc.pl";
    harness.open(uri, "use strict;\nmy $x = 1;\n")?;
    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    // Close the document
    harness.close(uri)?;
    std::thread::sleep(Duration::from_millis(50));

    // Request streaming on the now-closed document.
    let result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 5 },
            "partialResultToken": "closed-doc-token"
        }),
    )?;

    // Should gracefully return null (document not found).
    assert!(
        result.is_null(),
        "streaming on closed doc should return null"
    );

    Ok(())
}

// ==================== Missing params ====================

/// Sending the streaming request without params should return an error.
#[test]
fn streaming_completion_missing_params_returns_error() -> TestResult {
    let mut harness = init_harness()?;

    // Send request without valid textDocument params.
    // The harness request method wraps params, but we can send malformed params.
    let result = harness.request("textDocument/perlInlineCompletionStream", json!({}));

    // Should return an error (missing textDocument.uri).
    assert!(
        result.is_err(),
        "streaming request with empty params should error"
    );

    Ok(())
}

// ==================== Capability advertisement ====================

/// The server should advertise `perlInlineCompletionStream` in experimental
/// capabilities after initialization.
#[test]
fn streaming_completion_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(None)?;

    let experimental = init_result
        .pointer("/capabilities/experimental")
        .ok_or("expected experimental capabilities")?;
    assert_eq!(
        experimental.get("perlInlineCompletionStream"),
        Some(&json!(true)),
        "server should advertise perlInlineCompletionStream capability"
    );

    Ok(())
}

// ==================== Progress payload schema ====================

/// Validate the full schema of the progress notification payload.
#[test]
fn streaming_completion_progress_schema_validation() -> TestResult {
    let mut harness = init_harness()?;
    enable_ai_streaming(&mut harness);

    let uri = "file:///schema_test.pl";
    harness.open(uri, "#!/usr/bin/perl\nuse strict;\n")?;

    harness.wait_for_idle(Duration::from_millis(200));
    let _ = harness.drain_notifications(None, 100);

    let _result = harness.request(
        "textDocument/perlInlineCompletionStream",
        json!({
            "textDocument": { "uri": uri, "version": 1 },
            "position": { "line": 1, "character": 11 },
            "partialResultToken": "schema-token"
        }),
    )?;

    let progress = harness.drain_notifications(Some("$/progress"), 500);
    let matching: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("schema-token"))
        .collect();

    assert!(!matching.is_empty(), "expected progress notification");

    let notif = matching[0];

    // Top-level: method must be $/progress
    assert_eq!(
        notif["method"].as_str(),
        Some("$/progress"),
        "notification method must be $/progress"
    );

    // params.token must match the request's partialResultToken
    assert_eq!(
        notif.pointer("/params/token").and_then(|v| v.as_str()),
        Some("schema-token"),
        "token must match partialResultToken"
    );

    // params.value must be present
    let value = &notif["params"]["value"];
    assert!(!value.is_null(), "value must be present");

    // Required fields in value
    let required_fields = ["kind", "sessionId", "sequence", "isFinal", "items"];
    for field in &required_fields {
        assert!(
            value.get(field).is_some(),
            "progress value must contain '{field}'"
        );
    }

    // Type checks
    assert!(value["kind"].is_string(), "kind must be a string");
    assert!(value["sessionId"].is_string(), "sessionId must be a string");
    assert!(value["sequence"].is_number(), "sequence must be a number");
    assert!(value["isFinal"].is_boolean(), "isFinal must be a boolean");
    assert!(value["items"].is_array(), "items must be an array");

    Ok(())
}

// Mock streaming-backend coverage includes:
// 1. Multiple intermediate $/progress notifications with increasing sequence numbers
// 2. Each chunk's cumulative text in the items array
// 3. Cancellation mid-stream via session cancel-previous semantics
// 4. Backend error propagation and graceful termination

#[cfg(feature = "expose_lsp_test_api")]
mod mock_streaming_completion_tests {
    use parking_lot::Mutex;
    use perl_content_length_framing::ContentLengthFramer;
    use perl_lsp::{JsonRpcRequest, LspServer};
    use serde_json::{Value, json};
    use std::io::Write;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    struct TestOutputCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl TestOutputCapture {
        fn new() -> Self {
            Self {
                buffer: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn messages(&self) -> Vec<Value> {
            let bytes = self.buffer.lock().clone();
            parse_jsonrpc_frames(&bytes)
        }
    }

    impl Write for TestOutputCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn parse_jsonrpc_frames(bytes: &[u8]) -> Vec<Value> {
        let mut framer = ContentLengthFramer::new();
        let mut messages = Vec::new();
        framer.push(bytes);
        while let Ok(Some(message)) = framer.try_next() {
            if let Ok(value) = serde_json::from_slice::<Value>(&message) {
                messages.push(value);
            }
        }
        messages
    }

    fn wait_for_progress_messages(
        capture: &TestOutputCapture,
        token: &str,
        timeout: Duration,
    ) -> Vec<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let messages = capture.messages();
            let matching: Vec<_> = messages
                .into_iter()
                .filter(|msg| {
                    msg.get("method").and_then(|v| v.as_str()) == Some("$/progress")
                        && msg.pointer("/params/token").and_then(|v| v.as_str()) == Some(token)
                })
                .collect();
            if !matching.is_empty() || Instant::now() >= deadline {
                return matching;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn create_server() -> (LspServer, TestOutputCapture) {
        let capture = TestOutputCapture::new();
        let output = Box::new(capture.clone()) as Box<dyn Write + Send>;
        let server = LspServer::with_output(Arc::new(Mutex::new(output)));

        let init_request = JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: Some(json!({
                "processId": std::process::id(),
                "rootUri": "file:///workspace",
                "capabilities": {
                    "textDocument": {
                        "inlineCompletion": { "dynamicRegistration": false },
                    }
                }
            })),
        };
        let _ = server.handle_request(init_request);

        let initialized = JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: None,
            method: "initialized".into(),
            params: Some(json!({})),
        };
        let _ = server.handle_request(initialized);

        server.test_configure_ai_completion(true, true);
        let config_request = JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: None,
            method: "workspace/didChangeConfiguration".into(),
            params: Some(json!({
                "settings": {
                    "perl": {
                        "aiCompletion": {
                            "enabled": true,
                            "streaming": {
                                "enabled": true
                            }
                        }
                    }
                }
            })),
        };
        let _ = server.handle_request(config_request);

        (server, capture)
    }

    fn open_doc(server: &LspServer, uri: &str, text: &str) {
        let _ = server.handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: None,
            method: "textDocument/didOpen".into(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text,
                }
            })),
        });
    }

    fn request_streaming_completion(server: &LspServer, uri: &str, token: &str) -> Value {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "textDocument/perlInlineCompletionStream".into(),
            params: Some(json!({
                "textDocument": { "uri": uri, "version": 1 },
                "position": { "line": 0, "character": 19 },
                "partialResultToken": token,
            })),
        };

        server
            .handle_request(request)
            .and_then(|response| response.result)
            .unwrap_or(json!(null))
    }

    struct MockChunkBackend {
        chunks: Vec<&'static str>,
        delays_ms: Vec<u64>,
    }

    impl perl_lsp_inline_completion::InlineCompletionBackend for MockChunkBackend {
        fn stream(
            &self,
            _req: &perl_lsp_inline_completion::BackendRequest,
            sink: &mut dyn FnMut(
                perl_lsp_inline_completion::StreamChunk,
            ) -> perl_lsp_inline_completion::StreamControl,
        ) -> Result<(), perl_lsp_inline_completion::BackendError> {
            for (idx, chunk) in self.chunks.iter().enumerate() {
                if let Some(delay_ms) = self.delays_ms.get(idx) {
                    if *delay_ms > 0 {
                        thread::sleep(Duration::from_millis(*delay_ms));
                    }
                }
                let is_final = idx + 1 == self.chunks.len();
                let _ = sink(perl_lsp_inline_completion::StreamChunk {
                    text: (*chunk).to_string(),
                    is_final,
                });
            }
            Ok(())
        }
    }

    struct MockErrorChunkBackend;

    impl perl_lsp_inline_completion::InlineCompletionBackend for MockErrorChunkBackend {
        fn stream(
            &self,
            _req: &perl_lsp_inline_completion::BackendRequest,
            sink: &mut dyn FnMut(
                perl_lsp_inline_completion::StreamChunk,
            ) -> perl_lsp_inline_completion::StreamControl,
        ) -> Result<(), perl_lsp_inline_completion::BackendError> {
            let _ = sink(perl_lsp_inline_completion::StreamChunk {
                text: "find_".to_string(),
                is_final: false,
            });
            Err(perl_lsp_inline_completion::BackendError::Provider(
                "mock stream error".into(),
            ))
        }
    }

    #[test]
    fn streaming_completion_mock_backend_cumulative_chunks() {
        let (server, capture) = create_server();

        let backend = MockChunkBackend {
            chunks: vec!["fi", "find_", "find_user($id)"],
            delays_ms: vec![0, 0, 0],
        };
        server.test_install_ai_backend(Some(Arc::new(backend)));

        let uri = "file:///streaming-mock-chunks.pl";
        open_doc(&server, uri, "my $obj = Package->");
        let result = request_streaming_completion(&server, uri, "stream-mock-1");
        assert!(result.is_null());

        let progress =
            wait_for_progress_messages(&capture, "stream-mock-1", Duration::from_millis(500));
        assert_eq!(progress.len(), 3);

        let expected = ["fi", "find_", "find_user($id)"];
        let mut last_sequence = None;
        for idx in 0..progress.len() {
            let value = &progress[idx]["params"]["value"];
            let sequence = value["sequence"]
                .as_u64()
                .expect("progress should include sequence");
            if let Some(previous) = last_sequence {
                assert!(sequence > previous);
            } else {
                assert_eq!(sequence, 0);
            }
            last_sequence = Some(sequence);
            assert_eq!(value["items"][0]["insertText"], expected[idx]);
        }
    }

    #[test]
    fn streaming_completion_mock_backend_cancel_previous_isolation() {
        let (server, capture) = create_server();
        let server = Arc::new(server);

        let backend = MockChunkBackend {
            chunks: vec!["fi", "find_", "find_user($id)"],
            delays_ms: vec![0, 120, 120],
        };
        server.test_install_ai_backend(Some(Arc::new(backend)));

        let uri = "file:///streaming-mock-cancel.pl";
        open_doc(&server, uri, "my $obj = Package->");

        let first = {
            let server = Arc::clone(&server);
            let uri = uri.to_string();
            thread::spawn(move || {
                request_streaming_completion(&server, &uri, "stream-cancel-old");
            })
        };

        thread::sleep(Duration::from_millis(30));
        let new_result = request_streaming_completion(&server, uri, "stream-cancel-new");
        assert!(new_result.is_null());

        first
            .join()
            .expect("first streaming request thread panicked");

        thread::sleep(Duration::from_millis(250));
        let old_progress =
            wait_for_progress_messages(&capture, "stream-cancel-old", Duration::from_millis(300));
        let new_progress =
            wait_for_progress_messages(&capture, "stream-cancel-new", Duration::from_millis(300));

        assert_eq!(old_progress.len(), 1);
        assert!(!new_progress.is_empty());
    }

    #[test]
    fn streaming_completion_mock_backend_error_sends_final_progress() {
        let (server, capture) = create_server();
        server.test_install_ai_backend(Some(Arc::new(MockErrorChunkBackend)));

        let uri = "file:///streaming-mock-error.pl";
        open_doc(&server, uri, "my $obj = Package->");

        let result = request_streaming_completion(&server, uri, "stream-error-1");
        assert!(result.is_null());

        let progress =
            wait_for_progress_messages(&capture, "stream-error-1", Duration::from_millis(500));
        assert!(!progress.is_empty());
        assert_eq!(
            progress[0]["params"]["value"]["items"][0]["insertText"],
            "find_"
        );
        let final_progress = progress
            .last()
            .expect("error path should emit at least one progress frame");
        assert!(
            final_progress
                .pointer("/params/value/isFinal")
                .and_then(Value::as_bool)
                .is_some_and(|is_final| is_final),
            "error path should emit a final progress frame"
        );
        assert_eq!(
            final_progress["params"]["value"]["items"][0]["insertText"], "find_",
            "error path should preserve final cumulative text"
        );
        assert!(
            final_progress["params"]["value"]["sequence"]
                .as_u64()
                .is_some(),
            "final progress frame should carry sequence"
        );
    }
}

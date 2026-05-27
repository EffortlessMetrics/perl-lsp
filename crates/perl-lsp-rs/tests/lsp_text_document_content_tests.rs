//! Wire-contract tests for LSP 3.18 `workspace/textDocumentContent`.

mod support;

use parking_lot::Mutex;
use perl_lsp::LspServer;
use perl_lsp::protocol::{INVALID_PARAMS, INVALID_REQUEST};
use perl_lsp_rs_core::transport::framing::ContentLengthFramer;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::lsp_harness::LspHarness;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Default)]
struct OutputCapture {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl OutputCapture {
    fn messages(&self) -> TestResult<Vec<Value>> {
        let bytes = self.buffer.lock().clone();
        let mut framer = ContentLengthFramer::new();
        framer.push(&bytes);

        let mut messages = Vec::new();
        while let Some(body) = framer.try_next()? {
            messages.push(serde_json::from_slice::<Value>(&body)?);
        }
        Ok(messages)
    }
}

impl Write for OutputCapture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn initialized_harness() -> Result<LspHarness, String> {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready("file:///workspace", Some(json!({})))?;
    Ok(harness)
}

fn text_document_content_response(params: Option<Value>) -> Result<Value, String> {
    let mut harness = initialized_harness()?;
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "workspace/textDocumentContent"
    });
    if let Some(params) = params {
        request["params"] = params;
    }
    Ok(harness.request_raw(request))
}

fn assert_error_code(response: &Value, expected_code: i32) -> TestResult {
    let code = response
        .pointer("/error/code")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("expected JSON-RPC error code in response: {response}"))?;
    assert_eq!(code, i64::from(expected_code), "response: {response}");
    Ok(())
}

fn wait_for_method(output: &OutputCapture, method: &str) -> TestResult<Value> {
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        let messages = output.messages()?;
        if let Some(message) = messages
            .into_iter()
            .find(|message| message.get("method").and_then(Value::as_str) == Some(method))
        {
            return Ok(message);
        }

        if Instant::now() >= deadline {
            return Err(format!("method {method} was not emitted").into());
        }

        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn initialize_advertises_perldoc_text_document_content_scheme() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let init = harness.initialize_ready("file:///workspace", Some(json!({})))?;
    let schemes = init
        .pointer("/capabilities/workspace/textDocumentContent/schemes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing workspace.textDocumentContent.schemes: {init}"))?;
    let names: Vec<&str> = schemes.iter().filter_map(Value::as_str).collect();

    assert_eq!(names, vec!["perldoc"]);
    Ok(())
}

#[test]
fn text_document_content_missing_params_returns_invalid_params() -> TestResult {
    let response = text_document_content_response(None)?;
    assert_error_code(&response, INVALID_PARAMS)
}

#[test]
fn text_document_content_missing_uri_returns_invalid_params() -> TestResult {
    let response = text_document_content_response(Some(json!({})))?;
    assert_error_code(&response, INVALID_PARAMS)
}

#[test]
fn text_document_content_invalid_uri_returns_invalid_params() -> TestResult {
    let response = text_document_content_response(Some(json!({ "uri": "not a uri" })))?;
    assert_error_code(&response, INVALID_PARAMS)
}

#[test]
fn text_document_content_unsupported_scheme_returns_deterministic_error() -> TestResult {
    let response =
        text_document_content_response(Some(json!({ "uri": "unsupported://some/path" })))?;
    assert_error_code(&response, INVALID_REQUEST)?;
    let message = response
        .pointer("/error/message")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing error message in response: {response}"))?;
    assert!(
        message.contains("Unsupported URI scheme or content not found"),
        "unexpected unsupported-scheme error: {message}"
    );
    Ok(())
}

#[test]
fn text_document_content_perldoc_strict_returns_text_or_explicit_unavailable_error() -> TestResult {
    let response = text_document_content_response(Some(json!({ "uri": "perldoc://strict" })))?;

    if response.get("error").is_some() {
        assert_error_code(&response, INVALID_REQUEST)?;
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("missing error message in response: {response}"))?;
        assert!(
            message.contains("Unsupported URI scheme or content not found"),
            "perldoc unavailable path should be explicit, got: {message}"
        );
        return Ok(());
    }

    let text = response
        .pointer("/result/text")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("workspace/textDocumentContent missing result.text: {response}"))?;
    assert!(!text.trim().is_empty(), "perldoc text must not be empty");
    assert!(text.to_ascii_lowercase().contains("strict"), "strict perldoc should mention strict");
    Ok(())
}

#[test]
fn text_document_content_refresh_uses_bounded_server_request_id() -> TestResult {
    let output = OutputCapture::default();
    let server = LspServer::with_output(Arc::new(Mutex::new(
        Box::new(output.clone()) as Box<dyn Write + Send>
    )));

    server.request_text_document_content_refresh("perldoc://strict")?;

    let request = wait_for_method(&output, "workspace/textDocumentContent/refresh")?;

    let id = request
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("refresh request missing integer id: {request}"))?;
    assert!((1..=i64::from(i32::MAX)).contains(&id), "request id out of bounds: {id}");
    assert_eq!(request.pointer("/params/uri"), Some(&json!("perldoc://strict")));
    Ok(())
}

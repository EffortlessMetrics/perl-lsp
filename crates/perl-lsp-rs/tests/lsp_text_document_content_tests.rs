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
    assert!(text.contains("perldoc://warnings"), "strict virtual perldoc should link to warnings");
    Ok(())
}

#[test]
fn text_document_content_perldoc_warnings_links_back_to_strict_or_unavailable() -> TestResult {
    let response = text_document_content_response(Some(json!({ "uri": "perldoc://warnings" })))?;

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
    assert!(
        text.to_ascii_lowercase().contains("warnings"),
        "warnings perldoc should mention warnings"
    );
    assert!(text.contains("perldoc://strict"), "warnings virtual perldoc should link to strict");
    Ok(())
}

#[test]
fn text_document_content_perldoc_local_module_prefers_workspace_pod() -> TestResult {
    let (mut harness, _workspace) = LspHarness::with_workspace(&[(
        "lib/Local/VirtualDoc.pm",
        r#"package Local::VirtualDoc;

=head1 NAME

Local::VirtualDoc - workspace virtual docs

=head1 SYNOPSIS

use Local::VirtualDoc;

=head1 DESCRIPTION

Local POD served from the workspace module file.
See also L<Local::Dependency>, L<Local::Dependency>, L<Local::Helper>, and L<Local::VirtualDoc>.
Ignore local sections such as L</reset> and labeled targets such as L<helper|Local::Skipped>.

=head2 reset

Reset the local virtual document fixture.

=cut

1;
"#,
    )])?;

    let result = harness.request(
        "workspace/textDocumentContent",
        json!({ "uri": "perldoc://Local::VirtualDoc" }),
    )?;
    let text = result
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("workspace/textDocumentContent missing result.text: {result}"))?;

    assert!(
        text.contains("Workspace virtual perldoc"),
        "local module should use workspace POD, got: {text}"
    );
    assert!(text.contains("Module: Local::VirtualDoc"), "module heading missing: {text}");
    assert!(
        text.contains("Local::VirtualDoc - workspace virtual docs"),
        "NAME POD missing: {text}"
    );
    assert!(
        text.contains("Local POD served from the workspace module file."),
        "DESCRIPTION POD missing: {text}"
    );
    assert!(
        text.contains(
            "Related virtual perldoc:\n- perldoc://Local::Dependency\n- perldoc://Local::Helper"
        ),
        "workspace POD module links should become sorted virtual perldoc links: {text}"
    );
    assert!(
        !text.contains("perldoc://Local::VirtualDoc"),
        "workspace POD virtual content should ignore self-links: {text}"
    );
    assert!(
        !text.contains("perldoc://Local::Skipped") && !text.contains("perldoc:///reset"),
        "workspace POD virtual content should ignore non-simple POD targets: {text}"
    );
    assert!(
        text.contains("METHOD reset\nReset the local virtual document fixture."),
        "head2 method POD missing: {text}"
    );
    Ok(())
}

#[test]
fn text_document_content_related_workspace_perldoc_links_resolve() -> TestResult {
    let (mut harness, _workspace) = LspHarness::with_workspace(&[
        (
            "lib/Local/VirtualDoc.pm",
            r#"package Local::VirtualDoc;

=head1 NAME

Local::VirtualDoc - source docs

=head1 DESCRIPTION

See L<Local::Dependency>, L<Local::Dependency>, L<Local::Helper>, and L<Local::VirtualDoc>.
Ignore malformed or non-module targets: L<display|Local::Skipped>, L</section>, L<https://example.invalid>, L<Local::>.

=cut

1;
"#,
        ),
        (
            "lib/Local/Dependency.pm",
            r#"package Local::Dependency;

=head1 NAME

Local::Dependency - dependency docs

=head1 DESCRIPTION

Dependency docs are served from the linked workspace module.

=cut

1;
"#,
        ),
        (
            "lib/Local/Helper.pm",
            r#"package Local::Helper;

=head1 NAME

Local::Helper - helper docs

=head1 DESCRIPTION

Helper docs are served from the linked workspace module.

=cut

1;
"#,
        ),
    ])?;

    let source = harness.request(
        "workspace/textDocumentContent",
        json!({ "uri": "perldoc://Local::VirtualDoc" }),
    )?;
    let source_text = source.get("text").and_then(Value::as_str).ok_or_else(|| {
        format!("workspace/textDocumentContent missing source result.text: {source}")
    })?;

    assert!(
        source_text.contains(
            "Related virtual perldoc:\n- perldoc://Local::Dependency\n- perldoc://Local::Helper"
        ),
        "source workspace POD should expose sorted related virtual links: {source_text}"
    );
    assert!(
        !source_text.contains("perldoc://Local::VirtualDoc")
            && !source_text.contains("perldoc://Local::Skipped")
            && !source_text.contains("perldoc:///section")
            && !source_text.contains("perldoc://Local::>"),
        "source workspace POD should not expose self or non-simple links: {source_text}"
    );

    for (module, name, description) in [
        (
            "Local::Dependency",
            "Local::Dependency - dependency docs",
            "Dependency docs are served from the linked workspace module.",
        ),
        (
            "Local::Helper",
            "Local::Helper - helper docs",
            "Helper docs are served from the linked workspace module.",
        ),
    ] {
        let result = harness.request(
            "workspace/textDocumentContent",
            json!({ "uri": format!("perldoc://{module}") }),
        )?;
        let text = result.get("text").and_then(Value::as_str).ok_or_else(|| {
            format!("workspace/textDocumentContent missing linked result.text: {result}")
        })?;

        assert!(text.contains(&format!("Module: {module}")));
        assert!(text.contains(name));
        assert!(text.contains(description));
    }

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

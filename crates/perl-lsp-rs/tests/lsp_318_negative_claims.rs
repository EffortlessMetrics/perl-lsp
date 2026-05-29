//! Negative claim tests for optional LSP 3.18 surfaces.
//!
//! These tests do not implement new protocol features. They lock the current
//! support boundary: optional 3.18 structures must be absent unless the server
//! has explicit capability handling and wire tests for them.

mod support;

use parking_lot::Mutex;
use perl_lsp::LspServer;
use perl_lsp::protocol::METHOD_NOT_FOUND;
use perl_lsp::server::MessageType;
use perl_lsp_rs_core::runtime::tuning::RuntimeTuning;
use perl_lsp_rs_core::transport::framing::ContentLengthFramer;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
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

#[test]
fn initialize_does_not_advertise_unimplemented_318_capabilities() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let init = harness.initialize_ready("file:///workspace", Some(json!({})))?;
    let caps = init
        .get("capabilities")
        .ok_or_else(|| format!("initialize response missing capabilities: {init}"))?;

    assert_absent(caps, "/semanticTokensProvider/full/delta")?;
    assert_absent(caps, "/documentRangesFormattingProvider")?;
    assert_absent(caps, "/experimental/inlineCompletionProvider")?;
    assert_absent(caps, "/codeActionProvider/documentation")?;
    assert_absent(caps, "/completionProvider/applyKind")?;
    assert_absent(caps, "/workspace/foldingRange")?;

    Ok(())
}

#[test]
fn code_action_documentation_advertised_when_supported() -> TestResult {
    let mut harness = LspHarness::new_raw();
    let init = harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "textDocument": {
                "codeAction": {
                    "documentationSupport": true
                }
            }
        })),
    )?;
    let docs = init
        .pointer("/capabilities/codeActionProvider/documentation")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("expected CodeActionOptions.documentation in initialize: {init}"))?;
    assert_eq!(docs.len(), 3, "server should document quickfix, refactor, and source.fixAll kinds");

    let execute_commands = init
        .pointer("/capabilities/executeCommandProvider/commands")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("expected executeCommandProvider.commands in initialize: {init}"))?;
    assert!(
        execute_commands
            .iter()
            .any(|command| command.as_str() == Some("perl.explainProviderDecision")),
        "code-action documentation commands must use an advertised command: {init}"
    );

    for expected_kind in ["quickfix", "refactor", "source.fixAll"] {
        let doc = docs
            .iter()
            .find(|doc| doc.get("kind").and_then(Value::as_str) == Some(expected_kind))
            .ok_or_else(|| format!("missing documentation entry for {expected_kind}: {docs:?}"))?;
        let command = doc
            .get("command")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("documentation entry missing command object: {doc}"))?;
        assert_eq!(
            command.get("command").and_then(Value::as_str),
            Some("perl.explainProviderDecision"),
            "documentation entry must point at the existing provider-decision command: {doc}"
        );
        assert!(
            command.get("title").and_then(Value::as_str).is_some_and(|title| !title.is_empty()),
            "documentation command must have a user-visible title: {doc}"
        );
        let argument = command
            .get("arguments")
            .and_then(Value::as_array)
            .and_then(|args| args.first())
            .ok_or_else(|| format!("documentation command missing argument payload: {doc}"))?;
        assert!(
            argument.get("provider").and_then(Value::as_str).is_some(),
            "provider-decision documentation command must include provider context: {doc}"
        );
    }
    Ok(())
}

#[test]
fn semantic_tokens_delta_request_returns_method_not_found() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready("file:///workspace", Some(json!({})))?;
    harness.open("file:///test.pl", "use strict;\n")?;

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 318,
        "method": "textDocument/semanticTokens/full/delta",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "previousResultId": "stale-result"
        }
    }));

    assert_error_code(&response, METHOD_NOT_FOUND)
}

#[test]
fn completion_response_does_not_emit_apply_kind_without_client_support() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready("file:///workspace", Some(json!({})))?;
    harness.open("file:///test.pl", "sub alpha {}\nal")?;

    let completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 2 },
            "context": { "triggerKind": 1 }
        }),
    )?;

    assert_no_key(&completion, "applyKind")?;
    if let Some(item_defaults) = completion.get("itemDefaults") {
        assert!(
            item_defaults.get("data").is_none(),
            "CompletionList.itemDefaults.data is not claimed without explicit support: {completion}"
        );
    }
    Ok(())
}

#[test]
fn code_action_and_workspace_edit_responses_do_not_emit_optional_318_shapes() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "$undefined")?;

    let actions = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 10 }
            },
            "context": {
                "diagnostics": [],
                "only": ["quickfix", "refactor"],
                "triggerKind": 1
            }
        }),
    )?;

    assert_no_key(&actions, "documentation")?;
    assert_no_key(&actions, "tags")?;
    assert_no_workspace_edit_metadata(&actions)?;
    assert_no_key(&actions, "snippet")?;
    assert_no_command_tooltip(&actions)?;

    let mut rename_harness = LspHarness::new();
    rename_harness.initialize(None)?;
    rename_harness.open("file:///rename.pl", "my $old = 1;\n$old++;")?;

    let rename = rename_harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": "file:///rename.pl" },
            "position": { "line": 0, "character": 4 },
            "newName": "new"
        }),
    )?;

    assert_no_workspace_edit_metadata(&rename)?;
    assert_no_key(&rename, "snippet")?;
    Ok(())
}

#[test]
fn code_actions_do_not_emit_llm_generated_tags_for_deterministic_actions() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "textDocument": {
                "codeAction": {
                    "tagSupport": {
                        "valueSet": [1]
                    }
                }
            }
        })),
    )?;
    harness.open("file:///deterministic-actions.pl", "print \"hi\";\n")?;

    let actions = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": "file:///deterministic-actions.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": {
                "diagnostics": [],
                "only": ["quickfix", "refactor", "source"],
                "triggerKind": 1
            }
        }),
    )?;

    let action_items =
        actions.as_array().ok_or_else(|| format!("expected code action array: {actions}"))?;
    assert!(
        action_items.iter().any(|action| action.get("title").is_some()),
        "test must exercise at least one deterministic action: {actions}"
    );
    assert_no_key(&actions, "tags")?;
    Ok(())
}

#[test]
fn code_action_resolve_does_not_echo_tags_without_client_support() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready("file:///workspace", Some(json!({})))?;

    let resolved = harness.request(
        "codeAction/resolve",
        json!({
            "title": "client supplied generated action",
            "kind": "quickfix",
            "tags": [1]
        }),
    )?;

    assert_no_key(&resolved, "tags")?;
    Ok(())
}

#[test]
fn code_action_pragmas_emit_snippet_text_edits_when_supported() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "workspace": {
                "workspaceEdit": {
                    "documentChanges": true,
                    "snippetEditSupport": true
                }
            }
        })),
    )?;
    harness.open("file:///snippet.pl", "print \"hi\";\n")?;

    let actions = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": "file:///snippet.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": {
                "diagnostics": [],
                "only": ["quickfix"],
                "triggerKind": 1
            }
        }),
    )?;

    let action_items =
        actions.as_array().ok_or_else(|| format!("expected code action array: {actions}"))?;
    let strict_actions: Vec<&Value> = action_items
        .iter()
        .filter(|action| action.get("title").and_then(Value::as_str) == Some("Add use strict;"))
        .collect();
    assert_eq!(
        strict_actions.len(),
        1,
        "expected exactly one strict pragma code action: {actions}"
    );
    let strict_action = strict_actions
        .first()
        .copied()
        .ok_or_else(|| format!("strict pragma code action missing after count check: {actions}"))?;
    assert_absent(strict_action, "/edit/changes")?;

    let document_change = strict_action
        .pointer("/edit/documentChanges")
        .and_then(Value::as_array)
        .and_then(|changes| changes.first())
        .ok_or_else(|| {
            format!("expected snippet action to use documentChanges: {strict_action}")
        })?;
    assert_eq!(
        document_change.pointer("/textDocument/uri").and_then(Value::as_str),
        Some("file:///snippet.pl"),
        "snippet action should target the current document: {strict_action}"
    );
    assert_eq!(
        document_change.pointer("/textDocument/version").and_then(Value::as_i64),
        Some(1),
        "snippet action should carry the current document version: {strict_action}"
    );

    let edit = document_change
        .get("edits")
        .and_then(Value::as_array)
        .and_then(|edits| edits.first())
        .ok_or_else(|| format!("expected SnippetTextEdit in document change: {strict_action}"))?;
    assert_absent(edit, "/newText")?;
    assert_eq!(
        edit.pointer("/snippet/kind").and_then(Value::as_str),
        Some("snippet"),
        "SnippetTextEdit must use a StringValue snippet kind: {edit}"
    );
    assert_eq!(
        edit.pointer("/snippet/value").and_then(Value::as_str),
        Some("use strict;\n"),
        "pragma snippet should preserve the plain fallback text: {edit}"
    );
    Ok(())
}

#[test]
fn code_action_pragmas_require_document_changes_for_snippet_text_edits() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "workspace": {
                "workspaceEdit": {
                    "snippetEditSupport": true
                }
            }
        })),
    )?;
    harness.open("file:///snippet-fallback.pl", "print \"hi\";\n")?;

    let actions = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": "file:///snippet-fallback.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": {
                "diagnostics": [],
                "only": ["quickfix"],
                "triggerKind": 1
            }
        }),
    )?;

    assert_no_key(&actions, "snippet")?;
    let action_items =
        actions.as_array().ok_or_else(|| format!("expected code action array: {actions}"))?;
    let strict_actions: Vec<&Value> = action_items
        .iter()
        .filter(|action| action.get("title").and_then(Value::as_str) == Some("Add use strict;"))
        .collect();
    assert_eq!(
        strict_actions.len(),
        1,
        "expected exactly one strict pragma code action: {actions}"
    );
    let strict_action = strict_actions
        .first()
        .copied()
        .ok_or_else(|| format!("strict pragma code action missing after count check: {actions}"))?;
    assert!(
        strict_action.pointer("/edit/changes").is_some(),
        "clients without documentChanges support must keep plain WorkspaceEdit.changes: {strict_action}"
    );
    Ok(())
}

#[test]
fn diagnostics_keep_plain_string_messages_without_markup_support() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready("file:///workspace", Some(json!({})))?;
    harness.open("file:///test.pl", "use strict\nmy $x = ;\n")?;

    let report = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
    )?;

    assert_plain_message_values(&report)?;
    Ok(())
}

#[test]
fn dynamic_file_watcher_registration_uses_string_globs_not_relative_patterns() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::normal_defaults());
    harness.initialize(Some(json!({
        "workspace": {
            "didChangeWatchedFiles": {
                "dynamicRegistration": true
            }
        }
    })))?;

    let requests = harness.drain_server_requests(250);
    let registration = registration_for_method(&requests, "workspace/didChangeWatchedFiles")
        .ok_or("expected file watcher registration")?;
    let watchers = registration
        .pointer("/registerOptions/watchers")
        .and_then(Value::as_array)
        .ok_or("watcher registration missing watchers")?;

    for watcher in watchers {
        let glob_pattern = watcher
            .get("globPattern")
            .ok_or_else(|| format!("watcher missing globPattern: {watcher}"))?;
        assert!(
            glob_pattern.is_string(),
            "clients without relativePatternSupport must keep string file watcher globs: {watcher}"
        );
        assert!(
            glob_pattern.get("baseUri").is_none(),
            "relative-pattern baseUri must be absent without relativePatternSupport: {watcher}"
        );
    }
    Ok(())
}

#[test]
fn folding_range_refresh_is_not_sent_without_client_support() -> TestResult {
    let output = OutputCapture::default();
    let server = LspServer::with_output(Arc::new(Mutex::new(
        Box::new(output.clone()) as Box<dyn Write + Send>
    )));

    server.request_folding_range_refresh()?;
    std::thread::sleep(Duration::from_millis(50));

    let messages = output.messages()?;
    assert!(
        messages.is_empty(),
        "workspace/foldingRange/refresh must not be sent without refreshSupport: {messages:?}"
    );
    Ok(())
}

#[test]
fn window_message_type_does_not_emit_debug_level() -> TestResult {
    let output = OutputCapture::default();
    let server = LspServer::with_output(Arc::new(Mutex::new(
        Box::new(output.clone()) as Box<dyn Write + Send>
    )));

    server.log_message(MessageType::Info, "info")?;
    server.show_message(MessageType::Warning, "warning")?;
    std::thread::sleep(Duration::from_millis(50));

    for message in output.messages()? {
        if matches!(
            message.get("method").and_then(Value::as_str),
            Some("window/logMessage" | "window/showMessage")
        ) {
            let typ = message
                .pointer("/params/type")
                .and_then(Value::as_i64)
                .ok_or_else(|| format!("window message missing numeric type: {message}"))?;
            assert_ne!(typ, 5, "normal runtime paths must not emit debug messages: {message}");
        }
    }
    Ok(())
}

#[test]
fn markdown_surfaces_do_not_emit_trusted_commands_or_theme_icons_without_support() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "textDocument": {
                "completion": {
                    "completionItem": {
                        "documentationFormat": ["markdown", "plaintext"],
                        "snippetSupport": true
                    }
                },
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                },
                "signatureHelp": {
                    "signatureInformation": {
                        "documentationFormat": ["markdown", "plaintext"],
                        "parameterInformation": {
                            "labelOffsetSupport": true
                        }
                    }
                }
            }
        })),
    )?;

    harness.open("file:///markdown.pl", "my $rx = qr/foo\\d+/;\nprint(\"value\");\npri")?;

    let hover = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///markdown.pl" },
            "position": { "line": 0, "character": 13 }
        }),
    )?;
    assert_no_trusted_markdown_affordances(&hover)?;
    assert!(
        value_contains_markdown_kind(&hover),
        "expected regex hover to exercise a markdown surface: {hover}"
    );

    let completion = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///markdown.pl" },
            "position": { "line": 2, "character": 3 },
            "context": { "triggerKind": 1 }
        }),
    )?;
    assert_no_trusted_markdown_affordances(&completion)?;

    let print_item = find_completion_item(&completion, "print")?;
    let resolved_print = harness.request("completionItem/resolve", print_item.clone())?;
    assert_no_trusted_markdown_affordances(&resolved_print)?;
    assert!(
        value_contains_markdown_kind(&resolved_print),
        "expected completion resolve to exercise markdown documentation: {resolved_print}"
    );

    let signature = harness.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": "file:///markdown.pl" },
            "position": { "line": 1, "character": 7 }
        }),
    )?;
    assert_no_trusted_markdown_affordances(&signature)?;

    let perldoc = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 3180,
        "method": "workspace/textDocumentContent",
        "params": { "uri": "perldoc://strict" }
    }));
    if perldoc.get("error").is_none() {
        assert_no_trusted_markdown_affordances(&perldoc)?;
    }

    Ok(())
}

fn assert_absent(value: &Value, pointer: &str) -> TestResult {
    assert!(value.pointer(pointer).is_none(), "{pointer} must be absent from {value}");
    Ok(())
}

fn assert_error_code(response: &Value, expected_code: i32) -> TestResult {
    let code = response
        .pointer("/error/code")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("expected JSON-RPC error code in response: {response}"))?;
    assert_eq!(code, i64::from(expected_code), "response: {response}");
    Ok(())
}

fn assert_no_key(value: &Value, key: &str) -> TestResult {
    let mut paths = Vec::new();
    collect_key_paths(value, key, "$", &mut paths);
    assert!(paths.is_empty(), "key '{key}' must be absent; found at {}", paths.join(", "));
    Ok(())
}

fn assert_no_workspace_edit_metadata(value: &Value) -> TestResult {
    assert_no_key(value, "metadata")
}

fn collect_key_paths(value: &Value, key: &str, path: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (name, child) in map {
                let child_path = format!("{path}.{name}");
                if name == key {
                    paths.push(child_path.clone());
                }
                collect_key_paths(child, key, &child_path, paths);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                collect_key_paths(child, key, &format!("{path}[{idx}]"), paths);
            }
        }
        _ => {}
    }
}

fn assert_no_command_tooltip(value: &Value) -> TestResult {
    let mut paths = Vec::new();
    collect_command_tooltip_paths(value, "$", &mut paths);
    assert!(
        paths.is_empty(),
        "Command.tooltip outside CodeLens command objects is not claimed; found command objects with tooltip at {}",
        paths.join(", ")
    );
    Ok(())
}

fn collect_command_tooltip_paths(value: &Value, path: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if map.get("command").is_some() && map.get("tooltip").is_some() {
                paths.push(path.to_string());
            }
            for (name, child) in map {
                collect_command_tooltip_paths(child, &format!("{path}.{name}"), paths);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                collect_command_tooltip_paths(child, &format!("{path}[{idx}]"), paths);
            }
        }
        _ => {}
    }
}

fn assert_plain_message_values(value: &Value) -> TestResult {
    let mut non_string_paths = Vec::new();
    collect_non_string_message_paths(value, "$", &mut non_string_paths);
    assert!(
        non_string_paths.is_empty(),
        "Diagnostic.message MarkupContent must be gated by markupMessageSupport; non-string message values at {}",
        non_string_paths.join(", ")
    );
    Ok(())
}

fn collect_non_string_message_paths(value: &Value, path: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (name, child) in map {
                let child_path = format!("{path}.{name}");
                if name == "message" && !child.is_string() {
                    paths.push(child_path.clone());
                }
                collect_non_string_message_paths(child, &child_path, paths);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                collect_non_string_message_paths(child, &format!("{path}[{idx}]"), paths);
            }
        }
        _ => {}
    }
}

fn assert_no_trusted_markdown_affordances(value: &Value) -> TestResult {
    let mut paths = Vec::new();
    collect_trusted_markdown_affordance_paths(value, "$", &mut paths);
    assert!(
        paths.is_empty(),
        "trusted markdown command links and theme icons are not claimed; found at {}",
        paths.join(", ")
    );
    Ok(())
}

fn collect_trusted_markdown_affordance_paths(value: &Value, path: &str, paths: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if text.to_ascii_lowercase().contains("command:") || text.contains("$(") {
                paths.push(path.to_string());
            }
        }
        Value::Object(map) => {
            for (name, child) in map {
                collect_trusted_markdown_affordance_paths(child, &format!("{path}.{name}"), paths);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                collect_trusted_markdown_affordance_paths(child, &format!("{path}[{idx}]"), paths);
            }
        }
        _ => {}
    }
}

fn value_contains_markdown_kind(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.get("kind").and_then(Value::as_str) == Some("markdown")
                || map.values().any(value_contains_markdown_kind)
        }
        Value::Array(items) => items.iter().any(value_contains_markdown_kind),
        _ => false,
    }
}

fn find_completion_item(completion: &Value, label: &str) -> TestResult<Value> {
    let items = completion
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| completion.as_array())
        .ok_or_else(|| format!("completion response missing items: {completion}"))?;

    items
        .iter()
        .find(|item| item.get("label").and_then(Value::as_str) == Some(label))
        .cloned()
        .ok_or_else(|| format!("completion response missing {label} item: {completion}").into())
}

fn registration_for_method<'a>(requests: &'a [Value], method: &str) -> Option<&'a Value> {
    requests.iter().find_map(|request| {
        if request.get("method").and_then(Value::as_str) != Some("client/registerCapability") {
            return None;
        }

        request.pointer("/params/registrations").and_then(Value::as_array).and_then(
            |registrations| {
                registrations
                    .iter()
                    .find(|entry| entry.get("method").and_then(Value::as_str) == Some(method))
            },
        )
    })
}

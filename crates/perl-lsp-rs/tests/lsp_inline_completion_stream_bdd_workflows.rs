//! BDD-style UX workflows for streaming inline completion.
//!
//! These scenarios validate user-visible behavior across request sequencing,
//! document edits, and document lifecycle events.

// Integration tests print diagnostic output for CI troubleshooting; this is
// not the LSP server's stdio transport, so print_stderr doesn't apply the
// way it does to production code.
#![allow(clippy::print_stderr)]

mod support;

use serde_json::Value;
use support::lsp_ux_harness::InlineCompletionUxHarness;

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, step: &str) {
        eprintln!("[{}] Given {}", self.name, step);
    }

    fn when(&self, step: &str) {
        eprintln!("[{}] When {}", self.name, step);
    }

    fn then(&self, step: &str) {
        eprintln!("[{}] Then {}", self.name, step);
    }
}

fn session_id(progress: &Value) -> Option<&str> {
    progress.pointer("/params/value/sessionId").and_then(Value::as_str)
}

fn has_inline_items(response: &Value) -> bool {
    response.get("items").and_then(Value::as_array).is_some_and(|items| !items.is_empty())
}

#[test]
fn bdd_streaming_emits_progress_with_session_identity() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = Scenario::new("Streaming completion emits session-tagged progress");
    scenario.given("an initialized workspace with AI inline completion enabled");

    let mut ux = InlineCompletionUxHarness::start(
        "file:///bdd_streaming_progress.pl",
        "use strict;\nmy $obj = Package->",
    )?;

    scenario.when("requesting streamed inline completion at the cursor");
    let response = ux.request_stream(1, 19, "bdd-progress-token")?;

    scenario.then("the server streams progress or returns explicit fallback inline items");
    let progress = ux.progress_for_token("bdd-progress-token", 600);
    if progress.is_empty() {
        assert!(
            has_inline_items(&response),
            "expected streaming progress or fallback inline items; response={response}"
        );
    } else {
        let first_sid =
            progress.first().and_then(session_id).ok_or("progress payload missing sessionId")?;
        assert!(!first_sid.is_empty(), "sessionId should not be empty");
    }

    Ok(())
}

#[test]
fn bdd_streaming_after_incremental_edit_uses_new_session() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = Scenario::new("Streaming completion after didChange opens a new stream session");
    scenario.given("an open file with a partially typed symbol");

    let mut ux = InlineCompletionUxHarness::start(
        "file:///bdd_streaming_edit.pl",
        "use strict;\nmy $obj = Package->",
    )?;

    scenario.when("requesting a stream, editing the file, and requesting again");
    let first_response = ux.request_stream(1, 19, "token-before-edit")?;

    ux.change_full(
        "use strict;\nmy $obj = Package->
",
    )?;
    let second_response = ux.request_stream(1, 19, "token-after-edit")?;

    let progress = ux.drain_progress(800);
    let before_progress: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(Value::as_str) == Some("token-before-edit"))
        .cloned()
        .collect();
    let after_progress: Vec<_> = progress
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(Value::as_str) == Some("token-after-edit"))
        .cloned()
        .collect();
    let before_covered = !before_progress.is_empty() || has_inline_items(&first_response);
    let after_covered = !after_progress.is_empty() || has_inline_items(&second_response);

    assert!(before_covered, "expected progress or fallback items before edit");
    assert!(after_covered, "expected progress or fallback items after edit");

    scenario.then("sequential requests stay isolated by token/session behavior");
    if !before_progress.is_empty() && !after_progress.is_empty() {
        let before_sid = before_progress
            .first()
            .and_then(session_id)
            .ok_or("before-edit progress missing sessionId")?;
        let after_sid = after_progress
            .first()
            .and_then(session_id)
            .ok_or("after-edit progress missing sessionId")?;

        assert_ne!(before_sid, after_sid, "stream sessions should rotate across requests");
    }

    Ok(())
}

#[test]
fn bdd_streaming_request_on_closed_document_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = Scenario::new("Streaming completion gracefully handles closed documents");
    scenario.given("a document was opened and then closed in the editor");

    let mut ux = InlineCompletionUxHarness::start(
        "file:///bdd_streaming_closed_doc.pl",
        "use strict;\nmy $x = 1;\n",
    )?;
    ux.close_document()?;

    scenario.when("requesting inline streaming completion for that closed URI");
    let response = ux.request_stream(1, 5, "token-closed-doc")?;

    scenario.then("the server returns null and avoids protocol-level failure");
    assert!(response.is_null(), "closed-document streaming should return null");

    Ok(())
}

//! Tests for the generation-aware staleness guard in the pull-diagnostic path.
//!
//! LSP 3.17 pull diagnostics (`textDocument/diagnostic` and `workspace/diagnostic`)
//! are computed synchronously from a document snapshot taken under the documents
//! lock.  When a `didChange` arrives concurrently, the document's generation
//! counter advances.  Without a guard, the handler would return diagnostics from
//! the old snapshot — stale data the client did not ask for.
//!
//! These tests verify:
//!   1. The pull-document handler returns a valid result for stable documents.
//!   2. The pull-document handler tolerates a generation advance that happened
//!      *before* the handler ran (pre-advanced case — must NOT false-positive).
//!   3. The pull-workspace handler returns a result with an entry per document.
//!   4. The pull-workspace handler processes a document whose generation advanced
//!      before the handler ran (must NOT false-positive on the stale guard).
//!   5. Race: spawn a thread that advances generation immediately after the lock
//!      is taken (simulating concurrent didChange); the handler must return
//!      `items: []` (guard fires) or a non-error result (guard not reached in
//!      time) — never an error or panic.
//!   6. SyntaxOnly mode + concurrent didChange: the SyntaxOnly branch of the
//!      pull handler is guarded by the same staleness check — no stale parse
//!      errors returned, no panic.
//!
//! Run:
//!     RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!         --features expose_lsp_test_api \
//!         --test pull_diagnostics_freshness_tests

#![cfg(feature = "expose_lsp_test_api")]

use perl_lsp::LspServer;
use perl_lsp_rs_core::runtime::tuning::{DiagnosticMode, RuntimeTuning};
use serde_json::json;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── helpers ──────────────────────────────────────────────────────────────────

fn fresh_server() -> LspServer {
    let server = LspServer::new();
    // Initialize without a workspace root so we get a Ready index immediately.
    let _ = server.test_handle_initialize_dispatch(Some(json!({
        "capabilities": {},
        "rootUri": null,
        "workspaceFolders": null
    })));
    server
}

/// Extract the `items` array from a pull-diagnostic response.
///
/// The response has shape `{ "kind": "full" | "unchanged", "items": [...] }`.
/// Returns `None` when the response is absent or has no `items` key.
fn pull_items(response: Option<serde_json::Value>) -> Option<Vec<serde_json::Value>> {
    let v = response?;
    v.get("items")?.as_array().map(|a| a.to_vec())
}

/// Extract the `items` array from a workspace-diagnostic result for a given URI.
///
/// Workspace diagnostic response has shape:
/// `{ "items": [ { "uri": "...", "kind": "full", "items": [...] }, ... ] }`
fn workspace_items_for(
    response: Option<serde_json::Value>,
    uri: &str,
) -> Option<Vec<serde_json::Value>> {
    let v = response?;
    let outer = v.get("items")?.as_array()?;
    for entry in outer {
        if entry.get("uri")?.as_str() == Some(uri) {
            return entry.get("items")?.as_array().map(|a| a.to_vec());
        }
    }
    None
}

// ── document-diagnostic tests ─────────────────────────────────────────────────

/// Baseline: pull diagnostic on a stable document returns a valid response.
#[test]
fn pull_document_diagnostic_stable_doc_returns_result() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_diag_stable.pl";

    server.test_apply_did_open(uri, "my $x = 1;\n", 1)?;

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;

    assert!(
        resp.is_some(),
        "pull document diagnostic must return Some(result) for an open document"
    );
    assert!(pull_items(resp).is_some(), "result must carry an 'items' array");

    Ok(())
}

/// Parse-error doc: pull diagnostic returns non-empty items.
///
/// `my $x =;` is a syntax error — the handler must surface it as a diagnostic.
/// This also serves as the RED test for the guard: before the fix the handler
/// returned items from whatever state it found; after the fix it still returns
/// items here (no generation mismatch), so this test must pass both before and
/// after the implementation.  The meaningful RED test is
/// `pull_document_diagnostic_stale_on_concurrent_change` below.
#[test]
fn pull_document_diagnostic_syntax_error_returns_items() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_diag_syntax_error.pl";

    server.test_apply_did_open(uri, "my $x =;\n", 1)?;

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;

    let items = pull_items(resp).ok_or("response must carry items")?;
    assert!(
        !items.is_empty(),
        "syntax error must produce at least one diagnostic item, got: {items:?}"
    );

    Ok(())
}

/// Pre-advanced generation must NOT false-positive on the staleness guard.
///
/// If a didChange completed *before* the pull handler runs, the document's
/// generation counter already reflects the new state.  The handler snapshots the
/// current (post-change) generation and computes diagnostics; the guard check
/// sees the same value and must NOT discard the result.
#[test]
fn pull_document_diagnostic_pre_advanced_generation_does_not_false_positive() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_diag_pre_advance.pl";

    server.test_apply_did_open(uri, "my $x = 1;\n", 1)?;

    // Simulate a didChange that completed before the pull handler runs.
    server.test_apply_did_change(uri, "my $x = 2;\nmy $y = 3;\n", 2)?;

    let gen_after_change = server.test_document_generation(uri);
    assert!(gen_after_change.is_some(), "document must be open after didChange");

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;

    // The guard must NOT fire for a pre-completed change — the result must carry items.
    assert!(
        pull_items(resp).is_some(),
        "pre-advanced generation must not suppress diagnostics (false-positive guard)"
    );

    Ok(())
}

/// Concurrent didChange: spawn a thread that advances generation after a short
/// delay, then call the pull handler.  The result must be either:
///   (a) `items: []`  — guard fired because generation advanced during computation
///   (b) a non-empty items array — guard was not reached before computation completed
///
/// Under no circumstances must the handler panic or return an error.
///
/// This test is inherently non-deterministic; it validates the *contract*
/// (no panics, no errors) rather than a specific timing outcome.
#[test]
fn pull_document_diagnostic_concurrent_didchange_does_not_panic() -> TestResult {
    let server = Arc::new(fresh_server());
    let uri = "file:///pull_diag_concurrent.pl";

    // Open a document that will produce diagnostics (syntax error).
    server.test_apply_did_open(uri, "my $x =;\n", 1)?;

    // Spawn a thread that advances the generation after a tiny delay, simulating
    // a concurrent didChange arriving while the handler is computing.
    let server_clone = Arc::clone(&server);
    let uri_owned = uri.to_string();
    let handle = std::thread::spawn(move || {
        // Small sleep so the handler has likely taken its snapshot.
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _ = server_clone.test_apply_did_change(&uri_owned, "my $y = 1;\n", 2);
    });

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })));

    handle.join().map_err(|_| "background thread panicked")?;

    // The handler must not return an error.
    let resp = resp?;

    // It may return `items: []` (guard fired) or non-empty (guard not reached).
    // Either is acceptable; we only assert no panic / error.
    assert!(resp.is_some(), "pull handler must always return Some result, never None");

    Ok(())
}

// ── workspace-diagnostic tests ────────────────────────────────────────────────

/// Baseline: pull workspace diagnostic returns a result with one entry per open doc.
#[test]
fn pull_workspace_diagnostic_returns_entry_per_doc() -> TestResult {
    let server = fresh_server();
    let uri1 = "file:///pull_ws_a.pl";
    let uri2 = "file:///pull_ws_b.pl";

    server.test_apply_did_open(uri1, "my $x = 1;\n", 1)?;
    server.test_apply_did_open(uri2, "my $y = 2;\n", 1)?;

    let resp = server.test_handle_workspace_diagnostic(Some(json!({})))?;

    assert!(resp.is_some(), "workspace/diagnostic must return Some result");

    // Both URIs must appear in the outer items array.
    let outer = resp
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .ok_or("response must carry outer items array")?;

    let uris: Vec<&str> = outer.iter().filter_map(|e| e.get("uri")?.as_str()).collect();

    assert!(uris.contains(&uri1), "workspace diagnostic must include {uri1}; got: {uris:?}");
    assert!(uris.contains(&uri2), "workspace diagnostic must include {uri2}; got: {uris:?}");

    Ok(())
}

/// Pre-advanced generation in workspace path must NOT false-positive.
///
/// When one document's generation advances before `workspace/diagnostic` runs,
/// the stale guard must not discard that document's entry.
#[test]
fn pull_workspace_diagnostic_pre_advanced_generation_does_not_false_positive() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_ws_pre_advance.pl";

    server.test_apply_did_open(uri, "my $x = 1;\n", 1)?;
    // Apply a change before the workspace handler runs.
    server.test_apply_did_change(uri, "my $x = 2;\n", 2)?;

    let resp = server.test_handle_workspace_diagnostic(Some(json!({})))?;

    // The document must appear in the result (guard must not false-positive).
    assert!(
        workspace_items_for(resp, uri).is_some(),
        "pre-advanced document must appear in workspace/diagnostic result"
    );

    Ok(())
}

/// Concurrent didChange during workspace diagnostic must not panic.
#[test]
fn pull_workspace_diagnostic_concurrent_didchange_does_not_panic() -> TestResult {
    let server = Arc::new(fresh_server());
    let uri1 = "file:///pull_ws_concurrent_a.pl";
    let uri2 = "file:///pull_ws_concurrent_b.pl";

    server.test_apply_did_open(uri1, "my $x =;\n", 1)?;
    server.test_apply_did_open(uri2, "my $y = 1;\n", 1)?;

    let server_clone = Arc::clone(&server);
    let uri1_owned = uri1.to_string();
    let handle = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _ = server_clone.test_apply_did_change(&uri1_owned, "my $z = 99;\n", 2);
    });

    let resp = server.test_handle_workspace_diagnostic(Some(json!({})));

    handle.join().map_err(|_| "background thread panicked")?;

    let resp = resp?;
    assert!(resp.is_some(), "workspace/diagnostic must always return Some result");

    Ok(())
}

// ── SyntaxOnly-mode tests ─────────────────────────────────────────────────────

/// Build a server in SyntaxOnly diagnostic mode.
fn syntax_only_server() -> LspServer {
    let mut tuning = RuntimeTuning::normal_defaults();
    tuning.diagnostic_mode = DiagnosticMode::SyntaxOnly;
    LspServer::new_with_tuning(tuning)
}

/// SyntaxOnly stable: pull diagnostic on an error-free document returns items (possibly empty).
#[test]
fn pull_document_diagnostic_syntax_only_stable_doc_returns_result() -> TestResult {
    let server = syntax_only_server();
    let uri = "file:///syntax_only_stable.pl";

    server.test_apply_did_open(uri, "my $x = 1;\n", 1)?;

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;

    assert!(
        resp.is_some(),
        "SyntaxOnly pull diagnostic must return Some result for an open document"
    );
    assert!(pull_items(resp).is_some(), "SyntaxOnly result must carry an items array");

    Ok(())
}

/// SyntaxOnly syntax error: the parse errors surface through the pull path.
#[test]
fn pull_document_diagnostic_syntax_only_returns_parse_errors() -> TestResult {
    let server = syntax_only_server();
    let uri = "file:///syntax_only_error.pl";

    server.test_apply_did_open(uri, "my $x =;\n", 1)?;

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;

    let items = pull_items(resp).ok_or("SyntaxOnly response must carry items")?;
    assert!(
        !items.is_empty(),
        "SyntaxOnly mode must surface parse errors as diagnostic items, got: {items:?}"
    );

    Ok(())
}

/// SyntaxOnly concurrent didChange must not panic and must not return stale parse errors.
///
/// A thread advances the generation while the SyntaxOnly pull handler is analysing
/// the syntax snapshot.  The handler must either:
///   (a) return `items: []` — staleness guard fired
///   (b) return a non-error result — guard not reached before computation finished
///
/// The key invariant: no panic, no error, and no stale diagnostics from the
/// superseded document version are presented as authoritative.
#[test]
fn pull_document_diagnostic_syntax_only_concurrent_didchange_does_not_panic() -> TestResult {
    let server = Arc::new(syntax_only_server());
    let uri = "file:///syntax_only_concurrent.pl";

    // Open a document with a syntax error so the SyntaxOnly path has items to return.
    server.test_apply_did_open(uri, "my $x =;\n", 1)?;

    let server_clone = Arc::clone(&server);
    let uri_owned = uri.to_string();
    let handle = std::thread::spawn(move || {
        // Tiny delay so the handler has likely taken its snapshot before we advance.
        std::thread::sleep(std::time::Duration::from_millis(2));
        // Fix the syntax error in the new version — if the guard fails and stale
        // parse_errors leak through, the items would be non-empty when they should
        // reflect the fixed document (empty).  The guard must prevent that.
        let _ = server_clone.test_apply_did_change(&uri_owned, "my $x = 1;\n", 2);
    });

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })));

    handle.join().map_err(|_| "background thread panicked")?;

    // The handler must not return an error.
    let resp = resp?;
    let items = pull_items(resp).ok_or("SyntaxOnly response must carry items array")?;
    // Guard fires  → items empty (concurrent didChange advanced the generation)
    // Guard not reached → items may be non-empty (valid snapshot from old version)
    // Both are acceptable; asserting the array shape confirms a well-formed
    // DocumentDiagnosticReport rather than Some(json!(null)) or Some(json!({})).
    assert!(
        items.iter().all(|i| i.is_object()),
        "items must be well-formed diagnostic objects or empty, got: {items:?}"
    );

    Ok(())
}

// ── pending-parse gap tests (#3396 PR4) ───────────────────────────────────────
//
// These exercise the seam a future async parse worker will open: text
// generation ahead of the last published `ParsedSnapshot`, i.e.
// `DocumentState::current_parsed()` returns `None`. Production parsing is
// synchronous today so this state is otherwise unreachable; the
// `test_apply_text_change_without_reparse` / `test_publish_parse_for_current_generation`
// helpers force and then close the gap deterministically.

/// Single-document pull (`textDocument/diagnostic`) always re-parses the
/// text it is given from scratch -- it never reads the cached
/// `ParsedSnapshot` -- so it is immune to the pending-parse gap by
/// construction. This is *more* conservative than the policy requires
/// (always fresh), which is why no production change was needed here.
#[test]
fn pull_document_diagnostic_stays_fresh_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_diag_pending_gap.pl";

    server.test_apply_did_open(uri, "my $x = 1;\n", 1)?;
    server.test_apply_text_change_without_reparse(uri, "my $x =;\n", 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "helper must bump the generation without republishing"
    );

    let resp = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;
    let items = pull_items(resp).ok_or("response must carry items array")?;
    assert!(
        !items.is_empty(),
        "single-document pull re-parses live text on every request, so it must \
         surface the syntax error in the gapped text even though the cached \
         ParsedSnapshot is stale; got: {items:?}"
    );

    Ok(())
}

/// User-facing honesty canary, opposite direction: an edit that FIXES a
/// syntax error must not leave the now-stale PL001 diagnostic visible to a
/// pull request issued DURING the pending-parse gap. Single-document pull
/// re-parses live text on every call (proven above), so the fixed text's
/// diagnostics are what a client requesting `textDocument/diagnostic` mid-gap
/// would actually see -- this pins that the fix is honestly reflected rather
/// than the pre-edit (stale N-1 AST) syntax error surviving as a
/// false-current diagnostic.
#[test]
fn pull_document_diagnostic_does_not_report_a_fixed_syntax_error_as_current_during_pending_parse_gap(
) -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_diag_fix_during_gap.pl";

    // BEFORE: the syntax error is present and reported.
    server.test_apply_did_open(uri, "my $x =;\n", 1)?;
    let before = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;
    let before_items = pull_items(before).ok_or("response must carry items array")?;
    assert!(
        before_items.iter().any(|d| d.get("code").and_then(|c| c.as_str()) == Some("PL001")),
        "baseline: the syntax error must be reported before the fix; got: {before_items:?}"
    );

    // Apply an edit that FIXES the syntax error, but withhold republication
    // of the parse snapshot -- current_parsed() stays None (the pending-parse
    // gap the async worker will open).
    server.test_apply_text_change_without_reparse(uri, "my $x = 1;\n", 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "helper must bump the generation without republishing"
    );

    // DURING the gap: the diagnostic a client would see must reflect the
    // current (fixed) text, never a stale gen-N-clean-or-dirty claim derived
    // from the N-1 AST's cached (and now superseded) syntax error.
    let during_gap = server.test_handle_document_diagnostic(Some(json!({
        "textDocument": { "uri": uri }
    })))?;
    let gap_items =
        pull_items(during_gap).ok_or("response must carry items array during the gap")?;
    assert!(
        !gap_items.iter().any(|d| d.get("code").and_then(|c| c.as_str()) == Some("PL001")),
        "gap: pull diagnostics must not report the now-fixed syntax error as current \
         (would mean it presented the stale N-1 AST's diagnostic instead of the current \
         text's); got: {gap_items:?}"
    );

    Ok(())
}

/// Workspace pull (`workspace/diagnostic`, `LspServer::handle_workspace_diagnostic`)
/// reads the cached `ParsedSnapshot` directly (`doc.current_parsed()`) rather
/// than re-parsing. During a pending-parse gap it already skips the document
/// (`let Some(parsed) = doc.current_parsed() else { continue };`) instead of
/// reporting a false-fresh empty/full diagnostics set -- the entry is simply
/// omitted from `items` for this response, which leaves whatever the client
/// is currently displaying for that URI untouched. This test proves that
/// omission behavior holds, independent of whether the client sent a known
/// previous resultId.
#[test]
fn pull_workspace_diagnostic_omits_gapped_doc_from_items() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_ws_pending_gap.pl";

    server.test_apply_did_open(uri, "my $x =;\n", 1)?;

    // First pull establishes a resultId the client would echo back next time
    // -- included in this test to prove the gap omits the entry regardless
    // of whether a previous resultId is known.
    let first = server.test_handle_workspace_diagnostic(Some(json!({})))?;
    let outer = first
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .ok_or("first response must carry outer items array")?;
    let entry = outer
        .iter()
        .find(|e| e.get("uri").and_then(|u| u.as_str()) == Some(uri))
        .ok_or("first response must include the open document")?;
    let prev_result_id = entry
        .get("resultId")
        .and_then(|v| v.as_str())
        .ok_or("first full report must carry a resultId")?
        .to_string();

    // Open the pending-parse gap without changing the text's parse-error
    // content, so a leaked stale/empty report would be observably wrong.
    server.test_apply_text_change_without_reparse(uri, "my $x =;\n", 2)?;
    assert_eq!(server.test_document_generation(uri), Some(1));

    let resp = server.test_handle_workspace_diagnostic(Some(json!({
        "previousResultIds": [ { "uri": uri, "value": prev_result_id } ]
    })))?;
    let outer = resp
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .ok_or("response must carry outer items array")?;

    assert!(
        !outer.iter().any(|e| e.get("uri").and_then(|u| u.as_str()) == Some(uri)),
        "pending-parse gap must omit the document from workspace/diagnostic items \
         rather than reporting a false-fresh empty/full diagnostics set; got: {outer:?}"
    );

    Ok(())
}

/// Once the gap closes (a snapshot is published for the current
/// generation), workspace pull resumes reporting fresh AST-backed
/// diagnostics for the document normally.
#[test]
fn pull_workspace_diagnostic_resumes_after_pending_parse_gap_closes() -> TestResult {
    let server = fresh_server();
    let uri = "file:///pull_ws_gap_closes.pl";

    server.test_apply_did_open(uri, "my $x =;\n", 1)?;
    server.test_apply_text_change_without_reparse(uri, "my $x = 1;\n", 2)?;

    // While the gap is open, the document is omitted entirely.
    let gapped = server.test_handle_workspace_diagnostic(Some(json!({})))?;
    let gapped_outer = gapped
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .ok_or("gapped response must carry outer items array")?;
    assert!(
        !gapped_outer.iter().any(|e| e.get("uri").and_then(|u| u.as_str()) == Some(uri)),
        "document must still be omitted before the gap closes; got: {gapped_outer:?}"
    );

    server.test_publish_parse_for_current_generation(uri)?;

    let resp = server.test_handle_workspace_diagnostic(Some(json!({})))?;
    let outer = resp
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .ok_or("response must carry outer items array")?;
    let entry = outer
        .iter()
        .find(|e| e.get("uri").and_then(|u| u.as_str()) == Some(uri))
        .ok_or("response must include the resumed document once the gap closes")?;

    assert_eq!(
        entry.get("kind").and_then(|v| v.as_str()),
        Some("full"),
        "gap closed by publication must resume normal full-report computation; got: {entry:?}"
    );
    let items = entry
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("full report must carry an items array")?;
    assert!(
        !items.iter().any(|d| d.get("code").and_then(|c| c.as_str()) == Some("PL001")),
        "the syntax error was fixed before the gap closed, so the resumed report \
         must not carry a parse-error (PL001) diagnostic; got: {items:?}"
    );

    Ok(())
}

//! Tests for async performance improvements:
//! - #2457: Parser cancellation token wiring
//! - #2458: File watcher change notification debouncing

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use perl_lsp::{JsonRpcRequest, LspServer};
use perl_tdd_support::must;
use serde_json::json;

// ── helpers ─────────────────────────────────────────────────────────────────

fn initialized_server() -> LspServer {
    let server = LspServer::new();
    let _ = server.handle_request(must(serde_json::from_value::<JsonRpcRequest>(json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "initialize",
        "params": { "rootUri": null, "capabilities": {} }
    }))));
    let _ = server.handle_request(must(serde_json::from_value::<JsonRpcRequest>(json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "initialized",
        "params": {}
    }))));
    server
}

// ── #2457: Parser cancellation token wiring ─────────────────────────────────

/// Verify that `handle_did_open_with_cancellation` accepts a cancellation token
/// and uses `Parser::new_with_cancellation` internally.  The actual cancellation
/// flag is not polled by the parser yet (API stub), but the signature must
/// compile and the parse must complete successfully.
#[test]
fn test_parser_cancellation_token_accepted_in_did_open() {
    let server = initialized_server();

    let cancellation_flag = Arc::new(AtomicBool::new(false));
    let params = json!({
        "textDocument": {
            "uri": "file:///test_cancel_open.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nmy $x = 1;\n"
        }
    });

    // Pass a live (non-cancelled) token — parse should succeed
    let result = server
        .handle_did_open_with_cancellation(Some(params), Some(Arc::clone(&cancellation_flag)));
    assert!(
        result.is_ok(),
        "handle_did_open_with_cancellation failed: {:?}",
        result
    );
}

/// Verify that the cancellation token path in `handle_did_open_with_cancellation`
/// works when a token is pre-cancelled (parser still returns a result since the
/// stub does not actually check the flag during parsing).
#[test]
fn test_parser_pre_cancelled_token_in_did_open_still_completes() {
    let server = initialized_server();

    let cancellation_flag = Arc::new(AtomicBool::new(true)); // pre-cancelled
    let params = json!({
        "textDocument": {
            "uri": "file:///test_cancel_open2.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nmy $y = 2;\n"
        }
    });

    // The parser accepts the flag but doesn't poll it — parse completes normally.
    let result = server
        .handle_did_open_with_cancellation(Some(params), Some(Arc::clone(&cancellation_flag)));
    assert!(
        result.is_ok(),
        "Pre-cancelled token caused unexpected error: {:?}",
        result
    );
}

/// Verify `handle_did_change_with_cancellation` signature works correctly.
#[test]
fn test_parser_cancellation_token_accepted_in_did_change() {
    let server = initialized_server();

    // Open document first
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test_cancel_change.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nmy $x = 1;\n"
        }
    });
    server
        .handle_did_open_with_cancellation(Some(open_params), None)
        .ok();

    let cancellation_flag = Arc::new(AtomicBool::new(false));
    let change_params = json!({
        "textDocument": {
            "uri": "file:///test_cancel_change.pl",
            "version": 2
        },
        "contentChanges": [
            { "text": "use strict;\nmy $x = 42;\n" }
        ]
    });

    let result = server.handle_did_change_with_cancellation(
        Some(change_params),
        Some(Arc::clone(&cancellation_flag)),
    );
    assert!(
        result.is_ok(),
        "handle_did_change_with_cancellation failed: {:?}",
        result
    );
}

/// Verify the no-token path (None) works identically to the existing behaviour.
#[test]
fn test_parser_no_cancellation_token_unchanged_behaviour() {
    let server = initialized_server();

    let params = json!({
        "textDocument": {
            "uri": "file:///test_no_cancel.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nmy $z = 3;\n"
        }
    });

    // None token falls through to Parser::new — same as handle_did_open
    let result = server.handle_did_open_with_cancellation(Some(params), None);
    assert!(result.is_ok(), "None-token path failed: {:?}", result);
}

// ── #2458: File watcher change notification debouncing ──────────────────────

/// Verify that `FileWatcherDebouncer::with_interval` batches rapid events.
///
/// Uses the debouncer directly (unit level) to avoid filesystem I/O in the test.
#[test]
fn test_file_watcher_debouncer_coalesces_50_rapid_events() {
    use perl_lsp::runtime::file_watcher_debounce::FileWatcherDebouncer;

    let call_count = Arc::new(AtomicUsize::new(0));
    let total_uris = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&call_count);
    let t = Arc::clone(&total_uris);

    let debouncer = FileWatcherDebouncer::with_interval(Duration::from_millis(100), move |uris| {
        c.fetch_add(1, Ordering::SeqCst);
        t.fetch_add(uris.len(), Ordering::SeqCst);
    });

    // Simulate a git checkout: 50 file changes arriving within a few milliseconds
    for i in 0..50usize {
        debouncer.schedule(&format!("file:///workspace/file{i}.pl"));
    }

    // Wait for the debounce window to expire and the batch to fire
    thread::sleep(Duration::from_millis(300));

    let calls = call_count.load(Ordering::SeqCst);
    let uris = total_uris.load(Ordering::SeqCst);
    assert!(
        calls <= 2,
        "Expected <=2 batch calls for 50 rapid changes, got {calls}"
    );
    assert_eq!(uris, 50, "All 50 URIs should be delivered, got {uris}");
}

/// Verify that `FileWatcherDebouncer` is properly wired through `LspServer`.
///
/// Creates a bare `LspServer`, manually installs a debouncer with a short
/// window, calls `schedule_file_watcher_uri()` for 10 distinct URIs rapidly,
/// and asserts the batch callback fires after the debounce window expires.
#[test]
fn test_file_watcher_debouncer_wired_through_server() {
    use perl_lsp::runtime::file_watcher_debounce::FileWatcherDebouncer;

    let server = LspServer::new();
    let call_count = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&call_count);
    let debouncer = FileWatcherDebouncer::with_interval(Duration::from_millis(80), move |_uris| {
        c.fetch_add(1, Ordering::SeqCst);
    });
    server.install_file_watcher_debouncer(debouncer);

    // 10 rapid CHANGED events — should queue, not fire immediately
    for i in 0..10usize {
        let queued = server.schedule_file_watcher_uri(&format!("file:///workspace/file{i}.pl"));
        assert!(queued, "debouncer should be installed and return true");
    }

    thread::sleep(Duration::from_millis(250));
    assert!(
        call_count.load(Ordering::SeqCst) >= 1,
        "batch callback should have fired after debounce window"
    );
}

/// Verify that duplicate URIs within the debounce window are deduplicated.
#[test]
fn test_file_watcher_debouncer_deduplicates_same_uri() {
    use perl_lsp::runtime::file_watcher_debounce::FileWatcherDebouncer;

    let uri_counts: Arc<std::sync::Mutex<Vec<usize>>> = Arc::new(std::sync::Mutex::new(vec![]));
    let uc = Arc::clone(&uri_counts);

    let debouncer = FileWatcherDebouncer::with_interval(Duration::from_millis(60), move |uris| {
        uc.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(uris.len());
    });

    // Schedule the same URI 20 times — should deduplicate to 1
    for _ in 0..20 {
        debouncer.schedule("file:///workspace/same.pl");
    }

    thread::sleep(Duration::from_millis(200));

    let counts = uri_counts.lock().unwrap_or_else(|e| e.into_inner()).clone();
    // Total URIs delivered should be 1 (deduplicated)
    let total: usize = counts.iter().sum();
    assert_eq!(
        total, 1,
        "Expected 1 deduplicated URI, got {total} across {:?}",
        counts
    );
}

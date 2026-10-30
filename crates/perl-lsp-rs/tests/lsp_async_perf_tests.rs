//! Tests for async performance improvements:
//! - #2457: Parser cancellation token wiring
//! - #2458: File watcher change notification debouncing

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

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

fn wait_for_debounce_counts(
    uri_counts: &Arc<std::sync::Mutex<Vec<usize>>>,
    timeout: Duration,
) -> Vec<usize> {
    let deadline = Instant::now() + timeout;
    loop {
        let counts = uri_counts.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if !counts.is_empty() || Instant::now() >= deadline {
            return counts;
        }
        thread::sleep(Duration::from_millis(10));
    }
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
    assert!(result.is_ok(), "handle_did_open_with_cancellation failed: {:?}", result);
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
    assert!(result.is_ok(), "Pre-cancelled token caused unexpected error: {:?}", result);
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
    server.handle_did_open_with_cancellation(Some(open_params), None).ok();

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
    assert!(result.is_ok(), "handle_did_change_with_cancellation failed: {:?}", result);
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

/// Verify live debouncer delivery for 50 independent URI changes.
/// The controlled-clock unit test checks batching for equal deadlines.
#[test]
fn test_file_watcher_debouncer_delivers_50_rapid_events() {
    use perl_lsp::runtime::file_watcher_debounce::FileWatcherDebouncer;
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    let debouncer = FileWatcherDebouncer::with_interval(Duration::from_millis(100), move |uris| {
        let _ = tx.send(uris);
    });
    let mut expected: Vec<_> = (0..50).map(|i| format!("file:///workspace/file{i}.pl")).collect();
    for uri in &expected {
        debouncer.schedule(uri);
    }

    let deadline = Instant::now() + Duration::from_millis(300);
    let mut actual = Vec::new();
    while actual.len() < expected.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let batch =
            rx.recv_timeout(remaining).expect("all 50 URI events must be delivered within 300ms");
        actual.extend(batch);
    }
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected, "each scheduled URI must be delivered exactly once");
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
        uc.lock().unwrap_or_else(|e| e.into_inner()).push(uris.len());
    });

    // Schedule the same URI 20 times — should deduplicate to 1
    for _ in 0..20 {
        debouncer.schedule("file:///workspace/same.pl");
    }

    let counts = wait_for_debounce_counts(&uri_counts, Duration::from_secs(1));
    // Total URIs delivered should be 1 (deduplicated)
    let total: usize = counts.iter().sum();
    assert_eq!(total, 1, "Expected 1 deduplicated URI, got {total} across {:?}", counts);
}

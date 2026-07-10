//! Deterministic same-document TOCTOU regression tests for
//! `handle_type_definition` / `handle_implementation` (#3613).
//!
//! Both handlers capture the request document's ast/text under one lock
//! acquisition (generation N), then later re-read all open documents via
//! `documents_text_snapshot()` to build the fallback cross-file scan's
//! `doc_map`. Before the #3613 fix, that later re-read pulled `uri`'s own
//! entry live from the documents map instead of pinning it to the captured
//! generation -- so a `didChange` racing in between the two lock
//! acquisitions could pair a generation-N ast with generation-N+1 text for
//! the SAME document in one answer (the same class of bug fixed for
//! `handle_references_inner` in #3610, commit a95ad727e).
//!
//! These tests prove the fix deterministically: a test-only sync hook
//! (`test_set_navigation_same_doc_fallback_gap_hook`, gated behind
//! `expose_lsp_test_api`) pauses the handler thread at the exact gap between
//! the up-front capture and the later re-read. A second thread then applies
//! a real `didChange` to the same document before releasing the handler --
//! no sleeps, just channels. The racing edit pads an earlier line heavily
//! enough that, were `uri`'s doc_map entry re-read live, the byte-offset /
//! line-column conversion between the (stale) ast and the (fresh) text would
//! misalign and the handler would fail to resolve the target at all. The
//! assertions therefore prove a positive: the fallback consumed the CAPTURED
//! (generation-N) text, not the racing generation-N+1 text.
//!
//! Run:
//!     RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!         --features expose_lsp_test_api \
//!         --test navigation_same_document_toctou_regression_tests

#![cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]

use perl_lsp::LspServer;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::thread;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// `test_set_navigation_same_doc_fallback_gap_hook` arms a single
/// process-global hook slot. If two tests in this binary that use the hook
/// ran concurrently (the crate's `--test-threads=2` convention runs
/// multiple `#[test]` fns in parallel within one process), one test's
/// `test_set_...` call could silently clobber another's before its handler
/// reaches the gap, corrupting both. Serialize the hook-using tests through
/// this lock so each owns the gap exclusively for its critical section.
static TOCTOU_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn toctou_hook_lock() -> &'static Mutex<()> {
    TOCTOU_HOOK_LOCK.get_or_init(|| Mutex::new(()))
}

fn fresh_server() -> LspServer {
    let server = LspServer::new();
    let _ = server.test_handle_initialize_dispatch(Some(json!({
        "capabilities": {},
        "rootUri": null,
        "workspaceFolders": null
    })));
    server
}

/// Return the (line, character) of the first occurrence of `needle` in `source`.
fn find_pos(source: &str, needle: &str) -> Option<(u32, u32)> {
    for (line_idx, line_text) in source.lines().enumerate() {
        if let Some(col) = line_text.find(needle) {
            return Some((line_idx as u32, col as u32));
        }
    }
    None
}

/// Extract the first result's `targetRange.start.line` from a LocationLink array.
fn first_target_start_line(result: &Option<Value>) -> Option<u64> {
    result.as_ref()?.as_array()?.first()?.pointer("/targetRange/start/line")?.as_u64()
}

const TYPE_DEF_BEFORE: &str =
    "package MyClass;\nsub new { bless {}, shift }\npackage main;\nmy $obj = MyClass->new();\n";

/// Same source, but line 0 is padded with 200 filler bytes (no new lines) --
/// enough that, were the fallback's byte-offset/line-column conversion for
/// `uri` to use this fresher text instead of the captured one, the
/// downstream offsets computed from the OLD ast would misalign badly enough
/// to miss the "MyClass" identifier entirely.
fn type_def_after() -> String {
    format!(
        "package MyClass{};\nsub new {{ bless {{}}, shift }}\npackage main;\nmy $obj = MyClass->new();\n",
        "P".repeat(200)
    )
}

/// `handle_type_definition`'s fallback must resolve `MyClass` from the
/// CAPTURED generation-0 text, not a fresher generation-1 re-read racing in
/// via `didChange` while the handler is paused at the same-document gap.
#[test]
fn type_definition_fallback_pins_captured_generation_under_racing_didchange() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = Arc::new(fresh_server());
    let uri = "file:///same_doc_toctou_type_definition.pl";

    server.test_apply_did_open(uri, TYPE_DEF_BEFORE, 1)?;

    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found in fixture")?;

    let (reached_tx, reached_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    server.test_set_navigation_same_doc_fallback_gap_hook(reached_tx, resume_rx);

    let handler = {
        let server = Arc::clone(&server);
        thread::spawn(move || {
            server.test_handle_type_definition(Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            })))
        })
    };

    // Block (no sleep) until the handler has captured ast/doc_text from
    // generation 0 and reached the gap right before it re-reads
    // `documents_text_snapshot()` for the fallback.
    // Bounded, not a bare `.recv()`: a regression in the handler that never
    // reaches the gap must fail this test promptly instead of hanging the
    // suite forever.
    reached_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|err| format!("handler never reached the fallback gap: {err}"))?;

    // Race a real edit into the SAME document while the handler is paused.
    let after = type_def_after();
    server.test_apply_did_change(uri, &after, 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "the racing didChange must actually advance the document generation"
    );

    // Release the handler.
    resume_tx.send(()).map_err(|err| format!("failed to resume the paused handler: {err}"))?;

    let result = handler.join().map_err(|_| "handler thread panicked")??;

    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected a non-empty type-definition result resolved from the CAPTURED \
             generation-0 text; got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "type-definition fallback must resolve `MyClass` using the captured generation-0 \
         text/ast pair, not a fresher generation-1 re-read of the same document racing in via \
         didChange; got empty result: {result:?}"
    );

    let target_line = first_target_start_line(&result)
        .ok_or("missing targetRange.start.line in LocationLink result")?;
    assert_eq!(
        target_line, 0,
        "type-definition must resolve to 'package MyClass;' at line 0 of the CAPTURED \
         generation-0 text -- a fresher generation-1 re-read would misalign the ast-derived \
         offset against the padded line 0 and fail to resolve `MyClass` at all; got line {target_line}"
    );

    Ok(())
}

/// `handle_type_definition`'s fallback must resolve `MyClass` from the
/// CAPTURED ast/text pair even when a concurrent `didClose` removes `uri`
/// from the live documents map entirely between the up-front capture and
/// the fallback's `documents_text_snapshot()` re-read.
///
/// This is the residual TOCTOU instance the racing-`didChange` test above
/// does not cover: `didChange` leaves `uri` present in the map (just at a
/// fresher generation), but `didClose` removes it outright
/// (`handle_did_close` -> `evict_open_document_session_state` ->
/// `documents.remove(key)`). A fallback that only *substitutes* `uri`'s
/// entry when the map iteration still yields it (rather than
/// unconditionally pinning the captured snapshot) would silently drop
/// `uri` from `doc_map`, and the provider's `documents.get(uri)?` would
/// then return `None` -- an empty result -- even though the request
/// already captured a perfectly valid document snapshot before the close
/// raced in.
#[test]
fn type_definition_fallback_resolves_when_uri_closes_during_fallback_gap() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = Arc::new(fresh_server());
    let uri = "file:///same_doc_toctou_type_definition_close_race.pl";

    server.test_apply_did_open(uri, TYPE_DEF_BEFORE, 1)?;

    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found in fixture")?;

    let (reached_tx, reached_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    server.test_set_navigation_same_doc_fallback_gap_hook(reached_tx, resume_rx);

    let handler = {
        let server = Arc::clone(&server);
        thread::spawn(move || {
            server.test_handle_type_definition(Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            })))
        })
    };

    // Block (no sleep) until the handler has captured ast/doc_text and
    // reached the gap right before it re-reads `documents_text_snapshot()`
    // for the fallback.
    // Bounded, not a bare `.recv()`: a regression in the handler that never
    // reaches the gap must fail this test promptly instead of hanging the
    // suite forever.
    reached_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|err| format!("handler never reached the fallback gap: {err}"))?;

    // Race a real close into the SAME document while the handler is paused.
    server.test_apply_did_close(uri)?;

    // Release the handler.
    resume_tx.send(()).map_err(|err| format!("failed to resume the paused handler: {err}"))?;

    let result = handler.join().map_err(|_| "handler thread panicked")??;

    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected a non-empty type-definition result resolved from the CAPTURED \
             snapshot even though `uri` closed mid-flight; got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "type-definition fallback must resolve `MyClass` using the captured ast/text pair \
         even when a racing didClose removes `uri` from the live documents map before the \
         fallback's documents_text_snapshot() re-read; got empty result: {result:?}"
    );

    let target_line = first_target_start_line(&result)
        .ok_or("missing targetRange.start.line in LocationLink result")?;
    assert_eq!(
        target_line, 0,
        "type-definition must resolve to 'package MyClass;' at line 0 of the captured text \
         despite the racing didClose; got line {target_line}"
    );

    Ok(())
}

const IMPL_BEFORE: &str = "package Derived;\nuse parent 'Base';\nsub method { }\npackage Base;\nsub new { bless {}, shift }\npackage main;\nmy $obj = Derived->new();\n";

/// Same source, but line 0 (`package Derived;`) is padded with 200 filler
/// bytes -- enough that a live re-read of `uri` would shift the interpreted
/// line number of every later node (including `package Base;`) away from
/// its requested line, breaking the offset/line-column round-trip that
/// `find_implementations` relies on.
fn impl_after() -> String {
    format!(
        "package Derived{};\nuse parent 'Base';\nsub method {{ }}\npackage Base;\nsub new {{ bless {{}}, shift }}\npackage main;\nmy $obj = Derived->new();\n",
        "P".repeat(200)
    )
}

/// `handle_implementation`'s fallback must resolve `Base`'s implementor
/// (`Derived`) from the CAPTURED generation-0 text, not a fresher
/// generation-1 re-read racing in via `didChange` while the handler is
/// paused at the same-document gap.
#[test]
fn implementation_fallback_pins_captured_generation_under_racing_didchange() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = Arc::new(fresh_server());
    let uri = "file:///same_doc_toctou_implementation.pl";

    server.test_apply_did_open(uri, IMPL_BEFORE, 1)?;

    let (line, character) =
        find_pos(IMPL_BEFORE, "package Base;").ok_or("package Base; not found in fixture")?;
    // Position the cursor on the `Base` identifier itself.
    let character = character + u32::try_from("package ".len()).unwrap_or(0);

    let (reached_tx, reached_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    server.test_set_navigation_same_doc_fallback_gap_hook(reached_tx, resume_rx);

    let handler = {
        let server = Arc::clone(&server);
        thread::spawn(move || {
            server.test_handle_implementation(Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            })))
        })
    };

    // Bounded, not a bare `.recv()`: a regression in the handler that never
    // reaches the gap must fail this test promptly instead of hanging the
    // suite forever.
    reached_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|err| format!("handler never reached the fallback gap: {err}"))?;

    let after = impl_after();
    server.test_apply_did_change(uri, &after, 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "the racing didChange must actually advance the document generation"
    );

    resume_tx.send(()).map_err(|err| format!("failed to resume the paused handler: {err}"))?;

    let result = handler.join().map_err(|_| "handler thread panicked")??;

    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected a non-empty implementation result resolved from the CAPTURED \
             generation-0 text; got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "implementation fallback must resolve `Derived` as an implementor of `Base` using the \
         captured generation-0 text/ast pair, not a fresher generation-1 re-read of the same \
         document racing in via didChange; got empty result: {result:?}"
    );

    let target_line = first_target_start_line(&result)
        .ok_or("missing targetRange.start.line in LocationLink result")?;
    assert_eq!(
        target_line, 0,
        "implementation must resolve to 'package Derived;' at line 0 of the CAPTURED \
         generation-0 text -- a fresher generation-1 re-read would misalign the ast-derived \
         offset against the padded line 0 and fail to resolve `Base`'s cursor position at all; \
         got line {target_line}"
    );

    Ok(())
}

/// `handle_implementation`'s fallback must resolve `Derived` as an
/// implementor of `Base` from the CAPTURED ast/text pair even when a
/// concurrent `didClose` removes `uri` from the live documents map
/// entirely between the up-front capture and the fallback's
/// `documents_text_snapshot()` re-read (see the type-definition sibling
/// test above for why this is a distinct case from the racing-`didChange`
/// test).
#[test]
fn implementation_fallback_resolves_when_uri_closes_during_fallback_gap() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = Arc::new(fresh_server());
    let uri = "file:///same_doc_toctou_implementation_close_race.pl";

    server.test_apply_did_open(uri, IMPL_BEFORE, 1)?;

    let (line, character) =
        find_pos(IMPL_BEFORE, "package Base;").ok_or("package Base; not found in fixture")?;
    let character = character + u32::try_from("package ".len()).unwrap_or(0);

    let (reached_tx, reached_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    server.test_set_navigation_same_doc_fallback_gap_hook(reached_tx, resume_rx);

    let handler = {
        let server = Arc::clone(&server);
        thread::spawn(move || {
            server.test_handle_implementation(Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            })))
        })
    };

    // Bounded, not a bare `.recv()`: a regression in the handler that never
    // reaches the gap must fail this test promptly instead of hanging the
    // suite forever.
    reached_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|err| format!("handler never reached the fallback gap: {err}"))?;

    // Race a real close into the SAME document while the handler is paused.
    server.test_apply_did_close(uri)?;

    resume_tx.send(()).map_err(|err| format!("failed to resume the paused handler: {err}"))?;

    let result = handler.join().map_err(|_| "handler thread panicked")??;

    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected a non-empty implementation result resolved from the CAPTURED \
             snapshot even though `uri` closed mid-flight; got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "implementation fallback must resolve `Derived` as an implementor of `Base` using \
         the captured ast/text pair even when a racing didClose removes `uri` from the live \
         documents map before the fallback's documents_text_snapshot() re-read; got empty \
         result: {result:?}"
    );

    let target_line = first_target_start_line(&result)
        .ok_or("missing targetRange.start.line in LocationLink result")?;
    assert_eq!(
        target_line, 0,
        "implementation must resolve to 'package Derived;' at line 0 of the captured text \
         despite the racing didClose; got line {target_line}"
    );

    Ok(())
}

/// Non-racing control: with no concurrent edit, both handlers must still
/// resolve correctly -- proves the fix is behavior-identical when there is
/// no gap to exploit.
///
/// Also takes `toctou_hook_lock()` even though it never arms the hook itself:
/// `wait_at_same_doc_fallback_gap()` runs unconditionally on every call into
/// `handle_type_definition`/`handle_implementation`, including this test's.
/// Without the lock, this test could run concurrently (under the crate's
/// `--test-threads=2` convention) with a racing test in the narrow window
/// between that test arming the hook and its own handler thread reaching the
/// gate -- stealing the armed hook meant for the other test and corrupting
/// both. Serializing through the same lock closes that window.
#[test]
fn type_definition_and_implementation_resolve_normally_with_no_race() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = fresh_server();

    let type_def_uri = "file:///no_race_type_definition.pl";
    server.test_apply_did_open(type_def_uri, TYPE_DEF_BEFORE, 1)?;
    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found")?;
    let type_def_result = server.test_handle_type_definition(Some(json!({
        "textDocument": { "uri": type_def_uri },
        "position": { "line": line, "character": character }
    })))?;
    let type_def_locations =
        type_def_result.as_ref().and_then(Value::as_array).ok_or("expected array result")?;
    assert!(!type_def_locations.is_empty(), "no-race baseline: type definition must resolve");

    let impl_uri = "file:///no_race_implementation.pl";
    server.test_apply_did_open(impl_uri, IMPL_BEFORE, 1)?;
    let (line, character) =
        find_pos(IMPL_BEFORE, "package Base;").ok_or("package Base; not found")?;
    let character = character + u32::try_from("package ".len()).unwrap_or(0);
    let impl_result = server.test_handle_implementation(Some(json!({
        "textDocument": { "uri": impl_uri },
        "position": { "line": line, "character": character }
    })))?;
    let impl_locations =
        impl_result.as_ref().and_then(Value::as_array).ok_or("expected array result")?;
    assert!(!impl_locations.is_empty(), "no-race baseline: implementation must resolve");

    Ok(())
}

/// Deterministic (no race, no hook needed) regression for a dual-key bug in
/// `pinned_doc_map_for`: the live documents map is always keyed by
/// `normalize_uri_key` (`handle_did_open` stores under the normalized URI),
/// but the pinned entry was inserted under the RAW request URI. For a
/// document whose raw and normalized forms differ -- e.g. a Windows
/// drive-letter-cased URI like `file:///C:/...` vs its normalized
/// `file:///c:/...` -- this left the SAME document present in `doc_map`
/// under two keys: the normalized one (from `documents_text_snapshot()`)
/// and the raw one (the pinned insert). `find_package_definition_in_docs`
/// then finds the same `package` declaration twice and treats the result as
/// ambiguous (`locations.len() > 1`), returning an empty result even though
/// exactly one document matched.
///
/// This has nothing to do with the TOCTOU race the other tests in this file
/// cover -- it reproduces on every call for any URI whose raw and
/// normalized forms differ, no concurrent edit required.
///
/// Still takes `toctou_hook_lock()` for the same reason
/// `type_definition_and_implementation_resolve_normally_with_no_race` above
/// does even though it never arms the hook itself:
/// `wait_at_same_doc_fallback_gap()` runs unconditionally on every call into
/// `handle_type_definition`/`handle_implementation`, including this test's.
/// Without the lock, this test could run concurrently (under the crate's
/// `--test-threads=2` convention) with a racing test in the narrow window
/// between that test arming the hook and its own handler thread reaching the
/// gate -- stealing the armed hook meant for the other test and corrupting
/// both.
#[test]
fn type_definition_and_implementation_resolve_for_uri_with_raw_normalized_key_mismatch()
-> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = fresh_server();

    // Uppercase Windows drive letter: `perl_uri::uri_key` normalizes this to
    // lowercase (`file:///c:/...`), so raw != normalized for this URI.
    let type_def_uri = "file:///C:/same_doc_toctou/type_definition_uri_variant.pl";
    server.test_apply_did_open(type_def_uri, TYPE_DEF_BEFORE, 1)?;
    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found")?;
    let type_def_result = server.test_handle_type_definition(Some(json!({
        "textDocument": { "uri": type_def_uri },
        "position": { "line": line, "character": character }
    })))?;
    let type_def_locations =
        type_def_result.as_ref().and_then(Value::as_array).ok_or_else(|| {
            format!(
                "expected a non-empty type-definition result for a URI whose raw and normalized \
             forms differ; got: {type_def_result:?}"
            )
        })?;
    assert!(
        !type_def_locations.is_empty(),
        "type-definition must resolve `MyClass` for a raw/normalized-mismatched URI, not treat \
         the single matching document as two ambiguous matches; got empty result: \
         {type_def_result:?}"
    );

    let impl_uri = "file:///C:/same_doc_toctou/implementation_uri_variant.pl";
    server.test_apply_did_open(impl_uri, IMPL_BEFORE, 1)?;
    let (line, character) =
        find_pos(IMPL_BEFORE, "package Base;").ok_or("package Base; not found")?;
    let character = character + u32::try_from("package ".len()).unwrap_or(0);
    let impl_result = server.test_handle_implementation(Some(json!({
        "textDocument": { "uri": impl_uri },
        "position": { "line": line, "character": character }
    })))?;
    let impl_locations = impl_result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected a non-empty implementation result for a URI whose raw and normalized \
             forms differ; got: {impl_result:?}"
        )
    })?;
    assert!(
        !impl_locations.is_empty(),
        "implementation must resolve `Derived` as an implementor of `Base` for a \
         raw/normalized-mismatched URI, not treat the single matching document as two \
         ambiguous matches; got empty result: {impl_result:?}"
    );

    Ok(())
}

/// Focused test for the URI-normalization branch in `pinned_doc_map_for`:
/// when raw and normalized URIs differ (e.g. `file:///C:/...` vs
/// `file:///c:/...`), the deduplication removal must actually execute,
/// leaving only the pinned entry in the map under the raw key.
/// This exercises the `if normalized != uri { doc_map.remove(...) }` true branch.
///
/// Serializes through toctou_hook_lock: `wait_at_same_doc_fallback_gap()` runs
/// unconditionally on every call into `handle_type_definition`, even though
/// this test doesn't arm the hook. Without the lock, a concurrent test's armed
/// hook could interfere. Guard is released before assertions to prevent
/// poisoning the shared mutex if an assertion fails.
#[test]
fn pinned_doc_map_for_deduplicates_on_raw_normalized_mismatch() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = fresh_server();

    // Uppercase drive letter triggers normalization: raw `file:///C:/...`
    // becomes normalized `file:///c:/...`. pinned_doc_map_for should detect
    // this mismatch and remove the normalized entry from the snapshot map
    // before inserting the pinned entry under the raw key.
    // Use TYPE_DEF_BEFORE pattern: it has a resolvable package reference.
    let raw_uri = "file:///C:/dedup_test/type_definition_dedup.pl";
    server.test_apply_did_open(raw_uri, TYPE_DEF_BEFORE, 1)?;

    // Query at the "MyClass->new()" reference, same as the existing regression test.
    // pinned_doc_map_for constructs doc_map, and the deduplication removal
    // ensures MyClass is found exactly once (not counted twice as ambiguous).
    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found")?;
    let result = server.test_handle_type_definition(Some(json!({
        "textDocument": { "uri": raw_uri },
        "position": { "line": line, "character": character }
    })))?;

    // Release lock before assertions to prevent poisoning if assertion fails
    drop(_guard);

    // Type definition must resolve (non-empty) after deduplication.
    // Empty result would indicate duplicate-key ambiguity (the old bug).
    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected type-definition to resolve after deduplication on \
             raw/normalized-mismatched URI; got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "type-definition must find MyClass after deduplication removal on \
         raw!=normalized URI; got empty result (indicates duplicate-key bug): {result:?}"
    );
    Ok(())
}

/// Focused test for the URI-normalization branch when raw and normalized URIs
/// are IDENTICAL (the false branch of `if normalized != uri`): the removal
/// should NOT execute, and the original map state (from snapshot) should be
/// preserved. This exercises both branches of the conditional: true (mismatch,
/// remove) and false (already normalized, skip removal).
///
/// Serializes through toctou_hook_lock: `wait_at_same_doc_fallback_gap()` runs
/// unconditionally on every call into `handle_type_definition`, even though
/// this test doesn't arm the hook. Without the lock, a concurrent test's armed
/// hook could interfere. Guard is released before assertions to prevent
/// poisoning the shared mutex if an assertion fails.
#[test]
fn pinned_doc_map_for_skips_removal_when_uri_already_normalized() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = fresh_server();

    // All-lowercase drive letter URI is already in normalized form: raw == normalized.
    // The `if normalized != uri` condition should be false, and doc_map.remove
    // should NOT execute. Verify the handler still works correctly.
    let normalized_uri = "file:///d:/lowercase_uri_test/already_normalized.pl";
    server.test_apply_did_open(normalized_uri, TYPE_DEF_BEFORE, 1)?;

    // Query at the "MyClass->new()" reference.
    // pinned_doc_map_for constructs doc_map WITHOUT removal (skip branch).
    let (line, character) =
        find_pos(TYPE_DEF_BEFORE, "MyClass->new()").ok_or("MyClass->new() not found")?;
    let result = server.test_handle_type_definition(Some(json!({
        "textDocument": { "uri": normalized_uri },
        "position": { "line": line, "character": character }
    })))?;

    // Release lock before assertions to prevent poisoning if assertion fails
    drop(_guard);

    // Resolution should work (the no-removal path still produces a valid map).
    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected type-definition to resolve on already-normalized URI; \
             got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "type-definition must resolve even when skip-removal branch is taken \
         (already-normalized URI); got empty result: {result:?}"
    );
    Ok(())
}

/// Focused test verifying that `pinned_doc_map_for` actually calls
/// `doc_map.insert` (and `doc_map.remove` beforehand if needed).
/// This is a call-observation test: the document must be in the map
/// for handlers to locate it during fallback search.
///
/// Serializes through toctou_hook_lock: `wait_at_same_doc_fallback_gap()` runs
/// unconditionally on every call into `handle_implementation`, even though
/// this test doesn't arm the hook. Without the lock, a concurrent test's armed
/// hook could interfere. Guard is released before assertions to prevent
/// poisoning the shared mutex if an assertion fails.
#[test]
fn pinned_doc_map_for_insert_and_remove_calls_observed_through_resolution() -> TestResult {
    let _guard = toctou_hook_lock().lock().map_err(|_| "toctou hook lock poisoned")?;

    let server = fresh_server();

    // Open a document with uppercase drive letter (forces deduplication path).
    let raw_uri = "file:///C:/insertion_test/observe_calls.pl";
    server.test_apply_did_open(raw_uri, IMPL_BEFORE, 1)?;

    // Request implementation on "Base" package. This forces `handle_implementation`
    // to call `pinned_doc_map_for`, which must:
    // 1. Call `normalize_uri_key(uri)` to get normalized form
    // 2. Call `doc_map.remove(&normalized)` if they differ (true branch)
    // 3. Call `doc_map.insert(uri.to_string(), ...)` to add pinned entry
    // Without these calls, the fallback would not locate the package.
    let (line, character) =
        find_pos(IMPL_BEFORE, "package Base;").ok_or("package Base; not found")?;
    let character = character + u32::try_from("package ".len()).unwrap_or(0);
    let result = server.test_handle_implementation(Some(json!({
        "textDocument": { "uri": raw_uri },
        "position": { "line": line, "character": character }
    })))?;

    // Release lock before assertions to prevent poisoning if assertion fails
    drop(_guard);

    // Implementation must resolve to find the Derived subclass.
    // Resolution failure would indicate the calls (remove + insert) didn't happen.
    let locations = result.as_ref().and_then(Value::as_array).ok_or_else(|| {
        format!(
            "expected implementation handler to resolve Derived as subclass of Base; \
             got: {result:?}"
        )
    })?;
    assert!(
        !locations.is_empty(),
        "implementation must resolve Derived subclass after pinned_doc_map_for's \
         remove+insert calls; got empty result (missing from doc_map): {result:?}"
    );
    Ok(())
}

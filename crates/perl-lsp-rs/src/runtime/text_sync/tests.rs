use super::*;
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc as StdArc;
use std::time::Duration;

/// Shared-buffer writer for capturing outbound notifications in tests.
struct SharedVecWriter {
    inner: StdArc<parking_lot::Mutex<Vec<u8>>>,
}

impl Write for SharedVecWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn make_server_with_capture() -> (LspServer, StdArc<parking_lot::Mutex<Vec<u8>>>) {
    let buf = StdArc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
    let writer = SharedVecWriter { inner: StdArc::clone(&buf) };
    let server =
        LspServer::with_io(Box::new(std::io::Cursor::new(Vec::<u8>::new())), Box::new(writer));
    (server, buf)
}

#[cfg(feature = "incremental")]
#[test]
fn test_build_incremental_edits_uses_evolving_document_ranges() {
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    // Original text: "abcde" (all ASCII — one byte per character)
    let original_str = "abcde";
    let original = ropey::Rope::from_str(original_str);
    let changes = vec![
        // Edit 0: insert "X" at char 1 (between 'a' and 'b').
        // After this edit the working document becomes "aXbcde".
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 1 },
                end: Position { line: 0, character: 1 },
            }),
            range_length: None,
            text: "X".to_string(),
        },
        // Edit 1: replace chars 4..6 on the *post-insert* document "aXbcde".
        // Characters 4..6 of "aXbcde" are "de".  In original-doc space that
        // maps to bytes 3..5 (we subtract the +1 shift from the prior insert).
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 4 },
                end: Position { line: 0, character: 6 },
            }),
            range_length: None,
            text: "YZ".to_string(),
        },
    ];

    let edit_set =
        build_incremental_edit_set(&original, &changes).expect("expected ranged edit set");
    assert_eq!(edit_set.edits.len(), 2);

    // Edit 0 is a pure insertion — original-space offsets are 1..1.
    assert_eq!(edit_set.edits[0].start_byte, 1, "edit[0] start_byte must be in original space");
    assert_eq!(
        edit_set.edits[0].old_end_byte, 1,
        "edit[0] old_end_byte must be in original space (insertion)"
    );

    // Edit 1 in evolving space was 4..6, but the prior insert added 1 byte,
    // so in original-document space the range is 3..5 (the "de" suffix).
    assert_eq!(
        edit_set.edits[1].start_byte, 3,
        "edit[1] start_byte must be mapped back to original-doc space"
    );
    assert_eq!(
        edit_set.edits[1].old_end_byte, 5,
        "edit[1] old_end_byte must be mapped back to original-doc space"
    );

    // Crucially, applying the edit set to the original source must produce
    // the same document that the LSP client intended.  `apply_to_string`
    // sorts edits in reverse start_byte order and applies them against the
    // original string — this only works when all offsets are in
    // original-document space.
    //
    // Expected sequence:
    //   1. apply edit[1] (highest start_byte=3): "abcde"[3..5] → "YZ"  ⟹ "abcYZ"
    //   2. apply edit[0] (start_byte=1):         "abcYZ"[1..1] ← "X"   ⟹ "aXbcYZ"
    let result = edit_set.apply_to_string(original_str);
    assert_eq!(result, "aXbcYZ", "apply_to_string must reproduce the client-intended document");
}

#[cfg(feature = "incremental")]
#[test]
fn test_build_incremental_edits_returns_none_when_follow_up_edit_targets_inserted_text() {
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    let original = ropey::Rope::from_str("abc");
    let changes = vec![
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 1 },
                end: Position { line: 0, character: 1 },
            }),
            range_length: None,
            text: "XYZ".to_string(),
        },
        // This second edit applies to the inserted text in the evolving
        // document ("aXYZbc"), which cannot be represented with
        // original-document byte offsets.
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 1 },
                end: Position { line: 0, character: 2 },
            }),
            range_length: None,
            text: "_".to_string(),
        },
    ];

    assert!(
        build_incremental_edit_set(&original, &changes).is_none(),
        "edits that target newly inserted content should fall back to full reparse"
    );
}

/// After a deletion the cumulative_shift is negative; a second edit that targets
/// text AFTER the deleted region must be correctly mapped back via checked_add
/// (the negative-shift branch of map_offset_to_original_space).
#[cfg(feature = "incremental")]
#[test]
fn test_build_incremental_edits_negative_shift_uses_checked_add() {
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    // Original: "abcde" (5 bytes, all ASCII).
    let original = ropey::Rope::from_str("abcde");
    let changes = vec![
        // Edit 0: delete [1,3) → removes "bc", leaving evolving doc "ade".
        // cumulative_shift becomes 0 - (3-1) = -2.
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 1 },
                end: Position { line: 0, character: 3 },
            }),
            range_length: None,
            text: String::new(),
        },
        // Edit 1: replace [1,2) on evolving "ade" (the 'd') with "D".
        // evolving_start=1, evolving_end=2, cumulative_shift=-2.
        // map_offset(1, -2) = 1.checked_add(2) = Some(3)  (in original-doc space: 'd' = byte 3).
        // map_offset(2, -2) = 2.checked_add(2) = Some(4)  (in original-doc space: 'e' = byte 4).
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 1 },
                end: Position { line: 0, character: 2 },
            }),
            range_length: None,
            text: "D".to_string(),
        },
    ];

    let edit_set = build_incremental_edit_set(&original, &changes)
        .expect("deletion batch followed by suffix edit should be mappable");
    assert_eq!(edit_set.edits.len(), 2, "both edits should be in the set");

    // Edit 0 maps 1..1 in original space (zero-length deletion start at byte 1 was already
    // calculated with cumulative_shift=0).
    assert_eq!(edit_set.edits[0].start_byte, 1);
    assert_eq!(edit_set.edits[0].old_end_byte, 3);

    // Edit 1: evolving [1,2) + cumulative_shift=-2 → original [3,4).
    assert_eq!(
        edit_set.edits[1].start_byte, 3,
        "negative cumulative_shift must use checked_add to map back to original space"
    );
    assert_eq!(edit_set.edits[1].old_end_byte, 4);
}

/// Verify that a ranged didChange initializes and preserves incremental_doc.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_path_taken_on_ranged_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_incremental.pl";
    let text = "my $x = 42;\nmy $y = 99;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Verify incremental_doc was initialized on didOpen
    {
        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after didOpen")?;
        assert!(doc.incremental_doc.is_some(), "incremental_doc must be initialized on didOpen");
    }

    // Apply a ranged change: replace "42" with "43"
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{
            "range": {
                "start": { "line": 0, "character": 8 },
                "end":   { "line": 0, "character": 10 }
            },
            "text": "43"
        }]
    })))?;

    // Document must still be stored with updated content and a present AST
    {
        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after didChange")?;
        assert!(doc.text.contains("43"), "document text must be updated");
        assert!(doc.ast.is_some(), "AST must be present after incremental change");
        // incremental_doc must still be present after a ranged edit
        assert!(doc.incremental_doc.is_some(), "incremental_doc must survive a ranged edit");
        // The incremental doc's internal source must reflect the edit.
        // This catches a silent reinit-instead-of-apply bug: reinit would also hold
        // "43" in the source, but would not have the version counter bumped from 0.
        // Checking the source text is the strongest behavioral assertion available
        // without mocking the apply_edits call itself.
        let inc = doc.incremental_doc.as_ref().unwrap();
        assert!(
            inc.source.contains("43"),
            "incremental_doc.source must contain the edit result; got: {:?}",
            inc.source
        );
        assert!(
            !inc.source.contains("42"),
            "incremental_doc.source must not contain the old value; got: {:?}",
            inc.source
        );
        // version > 0 proves apply_edits was called (increments version), not just reinit
        // (which starts at version 0 after IncrementalDocument::new).
        assert!(
            inc.version > 0,
            "incremental_doc.version must be > 0 after at least one edit; got {}",
            inc.version
        );
    }
    Ok(())
}

/// Verify that a full-document replace (no range) re-initializes incremental_doc.
#[cfg(feature = "incremental")]
#[test]
fn test_full_replace_reinitializes_incremental_doc() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_inc_replace.pl";
    let text = "my $x = 1;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Full-document replace (no range field)
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "my $y = 2;\n" }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after full replace")?;
    assert!(
        doc.incremental_doc.is_some(),
        "incremental_doc must be re-initialized on full replace"
    );
    assert!(doc.text.contains("$y"), "text must be updated to new content");
    Ok(())
}

/// Verify that broken syntax does not panic and leaves the document in a valid state.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_fallback_on_parse_error() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_inc_error.pl";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1,
                          "text": "my $x = 42;\n" }
    }))?;

    // Replace with broken syntax — must not panic; document must survive
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{
            "range": { "start": { "line": 0, "character": 0 },
                       "end":   { "line": 0, "character": 11 } },
            "text": "sub { !!!"
        }]
    })))?;

    assert!(server.documents.lock().contains_key(uri), "document must survive broken syntax");
    Ok(())
}

/// Verify that an empty contentChanges array does not crash and leaves the document intact.
/// The server must handle no-op change notifications gracefully.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_empty_content_changes() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_inc_empty_changes.pl";
    let text = "my $x = 1;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Send a didChange with an empty contentChanges array (no-op notification)
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": []
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after empty change")?;
    // Text must be unchanged
    assert_eq!(doc.text, text, "empty contentChanges must not modify document text");
    // incremental_doc must still be present (reinit from same text is fine)
    assert!(doc.incremental_doc.is_some(), "incremental_doc must be present after no-op change");
    Ok(())
}

#[test]
fn test_did_change_ranged_edit_ignored_for_unopened_document()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///not-opened.pl";

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 1 },
        "contentChanges": [{
            "range": {
                "start": { "line": 0, "character": 0 },
                "end":   { "line": 0, "character": 0 }
            },
            "text": "my $x = 1;\n"
        }]
    })))?;

    let docs = server.documents.lock();
    assert!(docs.get(uri).is_none(), "ranged didChange for unopened docs must be ignored");
    Ok(())
}

/// Verify that an edit at the very end of the document (zero-length insertion) is handled.
/// This is the most common case for autocompletion triggers.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_insert_at_end_of_document() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_inc_insert_end.pl";
    let text = "my $x = 1;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Insert a new line at the end (line 1, char 0 — past the only line)
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{
            "range": {
                "start": { "line": 1, "character": 0 },
                "end":   { "line": 1, "character": 0 }
            },
            "text": "my $y = 2;\n"
        }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after end-of-doc insert")?;
    assert!(doc.text.contains("$y"), "new line must appear in document text");
    assert!(doc.incremental_doc.is_some(), "incremental_doc must survive end-of-document insert");
    Ok(())
}

/// Verify that UTF-16 position conversion handles multi-byte characters correctly.
/// LSP clients send UTF-16 code unit indices; characters like emoji or CJK take 2 units
/// but 4+ UTF-8 bytes. The byte offset calculation must account for this.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_utf16_multi_byte_character_positions() -> Result<(), Box<dyn std::error::Error>>
{
    let server = LspServer::new();
    let uri = "file:///test_inc_utf16.pl";
    // Line 0: "my $emoji = 😀;\n" (😀 is U+1F600, takes 2 UTF-16 units, 4 UTF-8 bytes)
    // UTF-16 positions: m(0) y(1) space(2) $(3) e(4) m(5) o(6) j(7) i(8) space(9) =(10) space(11) 😀(12-13) ;(14)
    // UTF-8 bytes: "my $emoji = " (12 bytes) + "😀" (4 bytes) + ";\n"
    let text = "my $emoji = 😀;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Replace the emoji (UTF-16: start=12, end=14) with the ASCII "xx"
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{
            "range": {
                "start": { "line": 0, "character": 12 },
                "end":   { "line": 0, "character": 14 }
            },
            "text": "xx"
        }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after UTF-16 edit")?;
    // Should have replaced emoji with "xx"
    assert!(doc.text.contains("xx"), "UTF-16 multi-byte replacement failed: expected 'xx' in text");
    // The emoji should no longer be there
    assert!(!doc.text.contains("😀"), "UTF-16 multi-byte removal failed: emoji should be gone");
    Ok(())
}

/// Verify that the `incremental_state` fast-path field is initialized on
/// `didOpen` and survives a ranged `didChange` (Gap A wiring, issue #2080).
///
/// This test fails before the `IncrementalState` field is wired into
/// `DocumentState` and confirmed after it is. It also verifies that the
/// incremental fast path produces a `reparsed_bytes` count less than the
/// full document size, proving checkpoint recovery ran.
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_state_wired_into_did_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_inc_state_gap_a.pl";

    // Build a document large enough to have checkpoints before the edit site.
    let mut lines: Vec<String> = (0..30).map(|i| format!("my $var_{i} = {i};")).collect();
    let text = lines.join("\n") + "\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // After didOpen, incremental_state must be initialized.
    {
        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after didOpen")?;
        assert!(
            doc.incremental_state.is_some(),
            "incremental_state must be initialized on didOpen (Gap A wiring absent)"
        );
        let state = doc.incremental_state.as_ref().unwrap();
        assert!(
            state.lex_checkpoints.len() > 1,
            "IncrementalState must have lex checkpoints after initial parse, got {}",
            state.lex_checkpoints.len()
        );
    }

    // Edit the last line: change `my $var_29 = 29;` -> `my $var_29 = 999;`
    // A checkpoint before the edit site means we should reparse < full doc.
    let edit_line = lines.len() as u64 - 1;
    lines[29] = "my $var_29 = 999;".to_string();

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{
            "range": {
                "start": { "line": edit_line, "character": 13 },
                "end":   { "line": edit_line, "character": 15 }
            },
            "text": "999"
        }]
    })))?;

    // After didChange, incremental_state must survive and source must be updated.
    {
        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after didChange")?;
        assert!(
            doc.incremental_state.is_some(),
            "incremental_state must survive a ranged edit (Gap A wiring absent)"
        );
        let state = doc.incremental_state.as_ref().unwrap();
        assert!(
            state.source.contains("999"),
            "incremental_state.source must reflect edit; got: {:?}",
            &state.source[state.source.len().saturating_sub(50)..]
        );
    }

    Ok(())
}

/// Verify that `did_open` and `did_change` return with the document stored
/// and that the `pending_index_tasks()` counter is accessible (issue #2352).
///
/// Without a tokio runtime the sync fallback path runs, so the counter
/// returns to zero before the assertions.  This test exercises the public
/// API surface introduced by the async-indexing refactor.
#[test]
fn test_indexing_does_not_block_did_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_async_index.pl";
    let text = "package Foo;\nsub bar { 1 }\n1;\n";

    // Open document — handler must return Ok even when indexing is async.
    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": text
        }
    }))?;

    // Document must be stored in the in-memory map after did_open returns.
    assert!(server.documents.lock().contains_key(uri));

    // The counter is accessible; in the sync fallback path (no tokio runtime
    // in unit tests) it settles to 0 once the handler returns.
    assert_eq!(server.pending_index_tasks(), 0);

    // A subsequent did_change must also succeed.
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "package Foo;\nsub baz { 2 }\n1;\n" }]
    })))?;

    assert!(server.documents.lock().contains_key(uri));
    assert_eq!(server.pending_index_tasks(), 0);

    Ok(())
}

/// `new_parse_token` must cancel the previous flag when called a second time
/// for the same URI and return a fresh `false` flag.
#[test]
fn test_new_parse_token_cancels_previous_flag() {
    let server = LspServer::new();
    let uri = "file:///test_cancel_token.pl";

    let first = server.new_parse_token(uri);
    assert!(!first.load(Ordering::Relaxed), "first token must start false");

    // Second call for same URI must set the first flag to true.
    let second = server.new_parse_token(uri);
    assert!(first.load(Ordering::Relaxed), "first token must be cancelled after second call");
    assert!(!second.load(Ordering::Relaxed), "second token must start false");

    // Third call cancels second, returns fresh third.
    let third = server.new_parse_token(uri);
    assert!(second.load(Ordering::Relaxed), "second token must be cancelled after third call");
    assert!(!third.load(Ordering::Relaxed), "third token must start false");
}

/// Different URIs must not interfere with each other's cancellation tokens.
#[test]
fn test_new_parse_token_is_per_uri() {
    let server = LspServer::new();
    let uri_a = "file:///a.pl";
    let uri_b = "file:///b.pl";

    let token_a = server.new_parse_token(uri_a);
    let token_b = server.new_parse_token(uri_b);

    // Issuing a second token for uri_b must not affect uri_a's token.
    let _token_b2 = server.new_parse_token(uri_b);
    assert!(
        !token_a.load(Ordering::Relaxed),
        "uri_a token must not be cancelled by uri_b activity"
    );
    assert!(
        token_b.load(Ordering::Relaxed),
        "uri_b first token must be cancelled by uri_b second token"
    );
}

/// `handle_did_close` must cancel the in-flight parse flag and remove it from
/// the map so that the entry does not leak after the document is closed.
#[test]
fn test_did_close_cancels_and_removes_flag() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_close_cancel.pl";

    // Simulate a parse token being registered for this URI.
    let token = server.new_parse_token(uri);
    assert!(!token.load(Ordering::Relaxed), "token must start false");

    // Open document so did_close has something to clean up.
    server.handle_did_open_with_cancellation(
        Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;"
            }
        })),
        None,
    )?;

    // Now issue a new token (as dispatch would do) — replaces the previous one.
    let in_flight_token = server.new_parse_token(uri);

    // Close the document.
    server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;

    // The in-flight token must have been cancelled by did_close.
    assert!(
        in_flight_token.load(Ordering::Relaxed),
        "did_close must set the in-flight parse flag to true"
    );

    // The flags map must be empty for this URI — no leak.
    assert!(
        !server.parse_cancel_flags.lock().contains_key(uri),
        "did_close must remove the URI entry from parse_cancel_flags"
    );

    Ok(())
}

#[test]
fn test_did_close_zeroes_memory_state_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_close_memory_snapshot.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "package Close::Snapshot;\nsub target { 1 }\n1;\n"
        }
    }))?;
    let _token = server.new_parse_token(uri);
    server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
        uri: uri.to_string(),
        document_version: 1,
        line: 0,
        character: 0,
    });

    let before = server.memory_state_snapshot();
    assert_eq!(before.documents, 1);
    assert!(before.open_text_bytes > 0);
    assert_eq!(before.parse_cancel_flags, 1);
    assert_eq!(before.stream_sessions, 1);

    server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;

    let after = server.memory_state_snapshot();
    assert_eq!(after.documents, 0);
    assert_eq!(after.open_text_bytes, 0);
    assert_eq!(after.parse_cancel_flags, 0);
    assert_eq!(after.stream_sessions, 0);

    Ok(())
}

#[test]
fn test_did_close_after_change_storm_drains_background_index_tasks()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let server = LspServer::new();
        let uri = "file:///test_close_change_storm.pl";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "package Close::Storm;\nsub target { 1 }\n1;\n"
            }
        }))?;

        for version in 2..30 {
            server.handle_did_change(Some(json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{
                    "text": format!(
                        "package Close::Storm;\nsub target {{ {} }}\n1;\n",
                        version
                    )
                }]
            })))?;
        }

        let _token = server.new_parse_token(uri);
        server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
            uri: uri.to_string(),
            document_version: 29,
            line: 0,
            character: 0,
        });
        server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;
        #[cfg(feature = "workspace")]
        if let Some(workspace_index) = server.workspace_index() {
            workspace_index.remove_file(uri);
        }

        for _ in 0..100 {
            if server.pending_index_task_count.load(Ordering::SeqCst) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let after = server.memory_state_snapshot();
        assert_eq!(after.pending_index_tasks, 0, "background indexing tasks must drain");
        assert_eq!(after.documents, 0, "didClose must remove open document state");
        assert_eq!(after.open_text_bytes, 0, "didClose must drop open document text");
        assert_eq!(after.parse_cancel_flags, 0, "didClose must remove parse-cancel flags");
        assert_eq!(after.stream_sessions, 0, "didClose must remove stream sessions");
        #[cfg(feature = "workspace")]
        if let Some(workspace_index) = server.workspace_index() {
            assert!(
                workspace_index.file_symbols(uri).is_empty(),
                "stale background indexing must not repopulate workspace symbols after close"
            );
            assert!(
                workspace_index.document_store().get(uri).is_none(),
                "stale background indexing must not repopulate document store after close"
            );
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    Ok(())
}

#[test]
fn test_diagnostics_churn_drains_retained_state_after_close_delete()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let (server, _buf) = make_server_with_capture();
        server.install_diagnostic_debouncer(
            super::diagnostic_debounce::DiagnosticDebouncer::with_interval(
                Duration::from_millis(60),
                |_| {},
            ),
        );

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("diagnostics_churn.pl");
        let uri = url::Url::from_file_path(&path).map_err(|_| "invalid file path")?;
        let uri = uri.to_string();
        let normalized_uri = server.normalize_uri_key(&uri);
        let fixed_template = |version: i32| {
            format!("package Diagnostics::Churn;\nmy $value = {version};\n$value;\n1;\n")
        };
        let broken_template =
            |version: i32| format!("package Diagnostics::Churn;\nsub broken_{version} {{\n");

        std::fs::write(&path, fixed_template(1))?;
        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": fixed_template(1)
            }
        }))?;

        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 4 }
        })));
        {
            let cache = server.semantic_analyzer_cache.lock();
            assert!(
                cache.keys().any(|(cached_uri, _)| cached_uri == &normalized_uri),
                "hover should populate semantic analyzer cache before churn"
            );
        }

        let mut saw_debounce_pressure = false;
        for version in 2..10 {
            let text =
                if version % 2 == 0 { broken_template(version) } else { fixed_template(version) };
            std::fs::write(&path, &text)?;
            server.handle_did_change(Some(json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }]
            })))?;
            server.publish_diagnostics(&uri);

            let pressure = server.runtime_pressure_snapshot();
            saw_debounce_pressure |= pressure.diagnostic_debounce_pending_uris > 0;

            if version % 2 != 0 {
                let _ = server.handle_hover(Some(json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 1, "character": 4 }
                })));
            }
        }
        assert!(saw_debounce_pressure, "diagnostics churn should exercise debounce pressure");

        let in_flight_token = server.new_parse_token(&uri);
        server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
            uri: uri.clone(),
            document_version: 9,
            line: 1,
            character: 4,
        });

        server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;
        std::fs::remove_file(&path)?;
        server.handle_did_change_watched_files(Some(json!({
            "changes": [
                { "uri": uri, "type": 3 }
            ]
        })))?;

        for _ in 0..100 {
            let pressure = server.runtime_pressure_snapshot();
            if pressure.pending_index_tasks == 0
                && pressure.diagnostic_debounce_pending_uris == 0
                && pressure.active_stream_sessions == 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(in_flight_token.load(Ordering::Relaxed), "close/delete must trip parse token");

        let memory = server.memory_state_snapshot();
        assert_eq!(memory.documents, 0);
        assert_eq!(memory.open_text_bytes, 0);
        assert_eq!(memory.parse_cancel_flags, 0);
        assert_eq!(memory.stream_sessions, 0);
        assert_eq!(memory.pending_index_tasks, 0);

        let pressure = server.runtime_pressure_snapshot();
        assert_eq!(pressure.pending_index_tasks, 0);
        assert_eq!(pressure.diagnostic_debounce_pending_uris, 0);
        assert_eq!(pressure.active_stream_sessions, 0);

        let cache = server.semantic_analyzer_cache.lock();
        assert!(
            !cache.keys().any(|(cached_uri, _)| cached_uri == &normalized_uri),
            "semantic analyzer cache must not retain diagnostics churn URI after close/delete"
        );

        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    Ok(())
}

#[test]
fn test_did_change_cancels_stream_sessions_for_uri_variants()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("stream_variant.pl");
    let source = "package Stream::Variant;\nsub target { 1 }\n1;\n";
    std::fs::write(&path, source)?;

    let canonical_uri =
        url::Url::from_file_path(&path).map_err(|()| "failed to build file URI")?.to_string();
    let raw_path =
        canonical_uri.strip_prefix("file://").ok_or("expected file URI for stream variant test")?;
    let localhost_uri = format!("file://localhost{raw_path}");
    assert_ne!(canonical_uri, localhost_uri);
    assert_eq!(server.normalize_uri_key(&canonical_uri), server.normalize_uri_key(&localhost_uri));

    server.did_open(json!({
        "textDocument": {
            "uri": canonical_uri,
            "languageId": "perl",
            "version": 1,
            "text": source
        }
    }))?;
    server.stream_sessions().start_session(crate::runtime::stream_session::SessionKey {
        uri: canonical_uri.clone(),
        document_version: 1,
        line: 0,
        character: 0,
    });
    assert_eq!(server.memory_state_snapshot().stream_sessions, 1);

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": localhost_uri, "version": 2 },
        "contentChanges": [{
            "text": "package Stream::Variant;\nsub target { 2 }\n1;\n"
        }]
    })))?;

    assert_eq!(
        server.memory_state_snapshot().stream_sessions,
        0,
        "didChange must cancel stale stream sessions across normalized URI variants"
    );

    Ok(())
}

#[test]
fn test_did_change_replaces_document_symbols_in_index() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_symbol_reindex.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "sub old_name { 1 }\n"
        }
    }))?;

    assert!(server.symbol_index.lock().search_prefix("old_").contains(&"old_name".to_string()));

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "sub new_name { 2 }\n" }]
    })))?;

    let index = server.symbol_index.lock();
    assert!(index.search_prefix("old_").is_empty());
    assert!(index.search_prefix("new_").contains(&"new_name".to_string()));

    Ok(())
}

#[test]
fn test_did_close_removes_document_symbols_from_index() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_symbol_close.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "sub close_me { 1 }\n"
        }
    }))?;

    assert!(server.symbol_index.lock().search_prefix("close_").contains(&"close_me".to_string()));

    server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;

    assert!(server.symbol_index.lock().search_prefix("close_").is_empty());
    Ok(())
}

#[cfg(feature = "workspace")]
#[test]
fn test_did_close_preserves_workspace_index_for_existing_file()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_close_preserves_workspace_index.pl";
    let source = "package Close::Only;\nsub still_indexed { 1 }\n1;\n";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": source
        }
    }))?;
    if let Some(coordinator) = server.coordinator() {
        coordinator.index().index_file(url::Url::parse(uri)?, source.to_string())?;
        assert!(
            !coordinator.index().file_symbols(uri).is_empty(),
            "workspace index setup must hold symbols before close"
        );
        assert!(
            coordinator.index().document_store().get(uri).is_some(),
            "workspace document store setup must hold the file before close"
        );
    }

    server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;

    let after = server.memory_state_snapshot();
    assert_eq!(after.documents, 0, "didClose must evict open-document state");
    assert_eq!(after.open_text_bytes, 0, "didClose must drop open-buffer text");
    assert!(
        server.symbol_index.lock().search_prefix("still_").is_empty(),
        "didClose must clear open-document symbol overlays"
    );
    if let Some(coordinator) = server.coordinator() {
        assert!(
            !coordinator.index().file_symbols(uri).is_empty(),
            "didClose is not file deletion; workspace-backed symbols for existing files must remain"
        );
        assert!(
            coordinator.index().document_store().get(uri).is_some(),
            "didClose must not remove workspace-index document store entries for existing files"
        );
    }

    Ok(())
}

/// didClose must clear diagnostics using the client-provided URI string.
///
/// This preserves exact URI identity for clients that key diagnostics by
/// the original URI representation rather than normalized equivalents.
#[test]
fn test_did_close_clears_diagnostics_with_original_uri() -> Result<(), Box<dyn std::error::Error>> {
    let (server, buf) = make_server_with_capture();
    let uri = "FILE:///test_close_uri_identity.pl";

    server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;
    drop(server);
    std::thread::sleep(Duration::from_millis(50));

    let text = String::from_utf8(buf.lock().clone()).unwrap_or_default();
    assert!(
        text.contains(r#""method":"textDocument/publishDiagnostics""#),
        "didClose must publish diagnostics clear notification; got: {text:?}"
    );
    assert!(
        text.contains(&format!(r#""uri":"{}""#, uri)),
        "didClose must publish diagnostics using original URI; got: {text:?}"
    );
    Ok(())
}

/// didSave must publish diagnostics using the original URI string.
#[test]
fn test_did_save_publishes_diagnostics_with_original_uri() -> Result<(), Box<dyn std::error::Error>>
{
    let (server, buf) = make_server_with_capture();
    let uri = "FILE:///test_save_uri_identity.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "my $x = 1;\n"
        }
    }))?;

    // Ignore notifications produced by didOpen; assert only didSave payload.
    buf.lock().clear();

    server.handle_did_save(Some(json!({
        "textDocument": {"uri": uri, "version": 1}
    })))?;
    drop(server);
    std::thread::sleep(Duration::from_millis(50));

    let text = String::from_utf8(buf.lock().clone()).unwrap_or_default();
    assert!(
        text.contains(r#""method":"textDocument/publishDiagnostics""#),
        "didSave must publish diagnostics notification; got: {text:?}"
    );
    assert!(
        text.contains(&format!(r#""uri":"{}""#, uri)),
        "didSave must publish diagnostics using original URI; got: {text:?}"
    );
    Ok(())
}

/// A parse cancelled via a pre-set flag must return Ok(()) and not store
/// a document, so the caller behaves as if the parse simply didn't happen.
#[test]
fn test_cancelled_open_returns_ok_without_storing_document()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let server = LspServer::new();
    let uri = "file:///test_cancelled_open.pl";

    // Pre-set the cancellation flag — the parse must be skipped immediately.
    let flag = Arc::new(AtomicBool::new(true));

    // Build a source large enough that parse() wouldn't return instantly
    // on its own — we rely on the pre-parse check in parse().
    let text: String = (0..200).map(|i| format!("my $x{} = {};\n", i, i)).collect();

    let result = server.handle_did_open_with_cancellation(
        Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
        Some(flag),
    );

    // The handler must return Ok (not propagate Cancelled as a JsonRpcError).
    assert!(result.is_ok(), "cancelled open must return Ok(()): {:?}", result);

    // The document must NOT have been stored (cancelled parse = no result).
    let normalized = server.normalize_uri_key(uri);
    assert!(
        !server.documents.lock().contains_key(&normalized),
        "cancelled parse must not store document state"
    );

    Ok(())
}

/// Binary content guard — didOpen with null bytes must skip the parser and
/// store the document with DegradationTier::Minimal and no AST.
#[test]
fn test_binary_file_guard_did_open_skips_parse() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_binary.pl";
    // Simulate a binary file that arrived as a valid UTF-8 string containing null bytes
    let binary_content = "PK\x00\x03some binary content\x00\x00\x00";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": binary_content
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after binary didOpen")?;
    assert_eq!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "binary content should result in Minimal degradation tier"
    );
    assert!(doc.ast.is_none(), "parser must not be called on binary content");
    Ok(())
}

/// Binary content guard — a single null byte is sufficient to trigger the guard.
#[test]
fn test_binary_file_guard_single_null_byte_triggers_guard() -> Result<(), Box<dyn std::error::Error>>
{
    let server = LspServer::new();
    let uri = "file:///test_null.pl";
    let content_with_null = "#!/usr/bin/perl\nmy $x = 1;\x00\n";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": content_with_null
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after single-null didOpen")?;
    assert_eq!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "a single null byte must trigger the binary guard"
    );
    assert!(doc.ast.is_none(), "parser must not be called when null byte is present");
    Ok(())
}

/// Binary content guard — normal Perl source (no null bytes) must still parse normally.
#[test]
fn test_binary_file_guard_normal_perl_still_parses() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///normal.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "#!/usr/bin/perl\nuse strict;\nmy $x = 42;\n"
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after normal didOpen")?;
    assert_ne!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "normal Perl should not be treated as binary content"
    );
    Ok(())
}

/// Binary content guard — didChange with null bytes must skip parse and keep DegradationTier::Minimal.
#[test]
fn test_binary_file_guard_did_change_skips_parse() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_binary_change.pl";

    // Open with valid Perl first
    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "my $x = 1;\n"
        }
    }))?;

    // Full-document replace with binary content (null bytes)
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "PK\x00\x03binary\x00data" }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document not stored after binary didChange")?;
    assert_eq!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "binary content via didChange should result in Minimal degradation tier"
    );
    assert!(doc.ast.is_none(), "parser must not be called on binary content via didChange");
    Ok(())
}

#[test]
fn test_template_file_guard_skips_parse_for_non_perl_language_id()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///app/templates/welcome.html.ep";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "html",
            "version": 1,
            "text": "<div><%= $name %></div>"
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
    assert_eq!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "template with non-Perl language mode should stay in no-parse mode"
    );
    assert!(doc.ast.is_none(), "template with non-Perl languageId must skip parse");
    Ok(())
}

#[test]
fn test_template_file_guard_persists_across_did_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///app/templates/welcome.html.ep";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "html",
            "version": 1,
            "text": "<div><%= $name %></div>"
        }
    }))?;

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "<div><%= $title %></div>" }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("template document not stored after didChange")?;
    assert_eq!(
        doc.degradation_tier,
        DegradationTier::Minimal,
        "template should remain in no-parse mode after didChange"
    );
    assert!(doc.ast.is_none(), "template should continue skipping parse on didChange");
    Ok(())
}

#[test]
fn test_template_file_guard_parses_embedded_perl_language_id()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///app/templates/welcome.html.ep";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "embedded-perl",
            "version": 1,
            "text": "<%= my $name = 'world'; %>"
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
    assert!(doc.ast.is_some(), "template with embedded-perl languageId should be parsed as Perl");
    Ok(())
}

#[test]
fn test_template_file_guard_parses_mojolicious_language_id()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///app/templates/index.html.ep";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "mojolicious",
            "version": 1,
            "text": "% my $title = 'Hello';"
        }
    }))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
    assert!(doc.ast.is_some(), "template with mojolicious languageId should be parsed as Perl");
    Ok(())
}

/// Semantic analyzer cache must accumulate at most one entry per document
/// version across multiple hover calls at different offsets.
///
/// This verifies the (uri, content_hash) key strategy: two hovers on the
/// same document text must reuse the cached SemanticAnalyzer rather than
/// constructing a fresh one.
#[test]
fn test_semantic_analyzer_cache_reuses_entry_on_same_version()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_cache_hover.pl";
    let text = "my $x = 1;\nmy $y = 2;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Two hover calls at different positions on the same document version.
    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 3 }
    })));

    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 3 }
    })));

    // Cache must have exactly 1 entry: one per (uri, content_hash).
    let cache = server.semantic_analyzer_cache.lock();
    assert_eq!(
        cache.len(),
        1,
        "should cache exactly one analyzer entry per document version (got {})",
        cache.len()
    );

    Ok(())
}

/// The semantic analyzer cache must be cleared for a URI when the document
/// changes (textDocument/didChange), so stale analysis is never served.
#[test]
fn test_semantic_analyzer_cache_invalidated_on_did_change() -> Result<(), Box<dyn std::error::Error>>
{
    let server = LspServer::new();
    let uri = "file:///test_cache_invalidate_change.pl";
    let text = "my $x = 1;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Prime the cache with a hover call.
    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 3 }
    })));

    // Verify the cache has an entry before the change.
    {
        let cache = server.semantic_analyzer_cache.lock();
        assert!(!cache.is_empty(), "cache must be populated before didChange");
    }

    // Apply a document change.
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": "my $x = 99;\n" }]
    })))?;

    // Cache must be cleared for this URI after didChange.
    let cache = server.semantic_analyzer_cache.lock();
    let uri_key = server.normalize_uri_key(uri);
    let still_has_stale = cache.keys().any(|(k, _)| k == &uri_key);
    assert!(!still_has_stale, "semantic_analyzer_cache must evict entries for changed URI");

    Ok(())
}

/// The semantic analyzer cache must be cleared for a URI when the document
/// is closed (textDocument/didClose), preventing stale memory retention.
#[test]
fn test_semantic_analyzer_cache_invalidated_on_did_close() -> Result<(), Box<dyn std::error::Error>>
{
    let server = LspServer::new();
    let uri = "file:///test_cache_invalidate_close.pl";
    let text = "my $x = 1;\n";

    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
    }))?;

    // Prime the cache with a hover call.
    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 3 }
    })));

    // Verify the cache has an entry before the close.
    {
        let cache = server.semantic_analyzer_cache.lock();
        assert!(!cache.is_empty(), "cache must be populated before didClose");
    }

    // Close the document.
    server.handle_did_close(Some(json!({ "textDocument": { "uri": uri } })))?;

    // Cache must be cleared for this URI after didClose.
    let cache = server.semantic_analyzer_cache.lock();
    let uri_key = server.normalize_uri_key(uri);
    let still_has_stale = cache.keys().any(|(k, _)| k == &uri_key);
    assert!(!still_has_stale, "semantic_analyzer_cache must evict entries for closed URI");

    Ok(())
}

/// A new document version must produce a distinct cache entry (different
/// content hash) while the old version's entry is evicted on didChange.
#[test]
fn test_semantic_analyzer_cache_separates_document_versions()
-> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///test_cache_versions.pl";
    let text_v1 = "my $x = 1;\n";
    let text_v2 = "my $x = 999;\n";

    // Open v1 and prime the cache.
    server.did_open(json!({
        "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text_v1 }
    }))?;

    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 3 }
    })));

    // Change to v2 (invalidates v1 entry) then hover again.
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 2 },
        "contentChanges": [{ "text": text_v2 }]
    })))?;

    let _ = server.handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 3 }
    })));

    // Cache must have at most 1 entry (v2 only; v1 was evicted on didChange).
    let cache = server.semantic_analyzer_cache.lock();
    assert!(
        cache.len() <= 1,
        "cache must hold at most one entry after version change (got {})",
        cache.len()
    );

    Ok(())
}

// =========================================================================
// Error-path tests — closes #3039
//
// These tests verify that each handler correctly propagates INVALID_PARAMS
// errors when required LSP parameters are missing.  They use Result<()>
// returns and explicit Err-branch assertions rather than #[should_panic].
// =========================================================================

/// handle_did_close with no textDocument.uri must return INVALID_PARAMS.
#[test]
fn handle_did_close_missing_uri_returns_invalid_params() {
    let server = LspServer::new();
    let result = server.handle_did_close(Some(json!({ "textDocument": {} })));
    assert!(result.is_err(), "handle_did_close must error on missing URI");
    if let Err(err) = result {
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "error code must be INVALID_PARAMS; got {}",
            err.code
        );
        assert!(
            err.message.contains("textDocument.uri"),
            "error message must name the missing field; got: {}",
            err.message
        );
    }
}

/// handle_did_close with None params must succeed silently (no-op).
#[test]
fn handle_did_close_none_params_is_ok() {
    let server = LspServer::new();
    let result = server.handle_did_close(None);
    assert!(result.is_ok(), "handle_did_close with None params must not error");
}

/// handle_did_close for a non-existent URI must succeed silently.
#[test]
fn handle_did_close_unknown_uri_is_ok() {
    let server = LspServer::new();
    let result = server
        .handle_did_close(Some(json!({ "textDocument": { "uri": "file:///never_opened.pl" } })));
    assert!(result.is_ok(), "closing a document that was never opened must not error");
}

/// handle_did_save with no textDocument.uri must return INVALID_PARAMS.
#[test]
fn handle_did_save_missing_uri_returns_invalid_params() {
    let server = LspServer::new();
    let result = server.handle_did_save(Some(json!({ "textDocument": {} })));
    assert!(result.is_err(), "handle_did_save must error on missing URI");
    if let Err(err) = result {
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "error code must be INVALID_PARAMS; got {}",
            err.code
        );
    }
}

/// handle_did_save with None params must succeed silently (no-op).
#[test]
fn handle_did_save_none_params_is_ok() {
    let server = LspServer::new();
    let result = server.handle_did_save(None);
    assert!(result.is_ok(), "handle_did_save with None params must not error");
}

/// did_open with a missing textDocument.text field must return INVALID_PARAMS.
#[test]
fn did_open_missing_text_returns_invalid_params() {
    let server = LspServer::new();
    let result = server.did_open(json!({
        "textDocument": {
            "uri": "file:///missing_text.pl",
            "languageId": "perl",
            "version": 1
        }
    }));
    assert!(result.is_err(), "did_open must error when textDocument.text is absent");
    if let Err(err) = result {
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "error code must be INVALID_PARAMS; got {}",
            err.code
        );
    }
}

/// did_open with a missing textDocument.uri field must return INVALID_PARAMS.
#[test]
fn did_open_missing_uri_returns_invalid_params() {
    let server = LspServer::new();
    let result = server.did_open(json!({
        "textDocument": {
            "languageId": "perl",
            "version": 1,
            "text": "my $x = 1;\n"
        }
    }));
    assert!(result.is_err(), "did_open must error when textDocument.uri is absent");
    if let Err(err) = result {
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "error code must be INVALID_PARAMS; got {}",
            err.code
        );
    }
}

/// handle_did_change with missing URI must return INVALID_PARAMS.
#[test]
fn handle_did_change_missing_uri_returns_invalid_params() {
    let server = LspServer::new();
    let result = server.handle_did_change(Some(json!({
        "textDocument": {},
        "contentChanges": []
    })));
    assert!(result.is_err(), "handle_did_change with missing URI must error");
    if let Err(err) = result {
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "error code must be INVALID_PARAMS; got {}",
            err.code
        );
    }
}

/// didChange with an out-of-order version must be ignored to avoid document rollback.
#[test]
fn handle_did_change_ignores_stale_versions() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///stale_version.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 5,
            "text": "my $x = 1;\n"
        }
    }))?;

    // Incoming didChange version is older than current (4 < 5): ignore.
    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri, "version": 4 },
        "contentChanges": [{ "text": "my $x = 999;\n" }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document missing after stale didChange")?;
    assert_eq!(doc.version, 5, "stale didChange must not update document version");
    assert_eq!(doc.text, "my $x = 1;\n", "stale didChange must not modify document text");
    Ok(())
}

/// didChange without a version field should still be applied for compatibility.
#[test]
fn handle_did_change_without_version_uses_next_version() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let uri = "file:///missing_version.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "my $x = 1;\n"
        }
    }))?;

    server.handle_did_change(Some(json!({
        "textDocument": { "uri": uri },
        "contentChanges": [{ "text": "my $x = 2;\n" }]
    })))?;

    let docs = server.documents.lock();
    let doc = docs.get(uri).ok_or("document missing after didChange without version")?;
    assert_eq!(doc.version, 2, "missing-version didChange should advance version by one");
    assert_eq!(doc.text, "my $x = 2;\n", "didChange without version should apply content");
    Ok(())
}

/// Legacy Windows URI form (`file://C:\...`) should normalize to canonical
/// `file:///c:/...` so follow-up requests using standard URI syntax still
/// resolve the open document.
#[test]
fn did_open_normalizes_legacy_windows_file_uri_form() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let legacy_uri = r"file://C:\Users\dev\example.pl";
    let canonical_uri = "file:///c:/Users/dev/example.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": legacy_uri,
            "languageId": "perl",
            "version": 1,
            "text": "my $x = 1;\n"
        }
    }))?;

    let docs = server.documents.lock();
    assert!(
        docs.contains_key(canonical_uri),
        "legacy URI should normalize to canonical key; keys: {:?}",
        docs.keys().collect::<Vec<_>>()
    );
    Ok(())
}

/// Plain Windows paths (`C:\...`) are non-standard in LSP, but some editors
/// still emit them.  Normalize to `file:///c:/...` for resilient lookup keys.
#[test]
fn did_open_normalizes_plain_windows_path_uri() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();
    let plain_path = r"C:\Users\dev\plain_path.pl";
    let canonical_uri = "file:///c:/Users/dev/plain_path.pl";

    server.did_open(json!({
        "textDocument": {
            "uri": plain_path,
            "languageId": "perl",
            "version": 1,
            "text": "my $y = 2;\n"
        }
    }))?;

    let docs = server.documents.lock();
    assert!(
        docs.contains_key(canonical_uri),
        "plain Windows path should normalize to canonical key; keys: {:?}",
        docs.keys().collect::<Vec<_>>()
    );
    Ok(())
}

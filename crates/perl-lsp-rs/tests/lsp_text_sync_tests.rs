//! Incremental text sync tests
//!
//! Tests for `textdoc::apply_changes()` covering multi-edit batches,
//! multi-line replacements, non-ASCII/UTF-16 boundaries, and full document replace.

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use perl_lsp::textdoc::{Doc, PosEnc, apply_changes};
use ropey::Rope;

/// Two incremental edits in a single didChange notification
#[test]
fn test_two_edits_in_one_did_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Doc { rope: Rope::from_str("hello world\n"), version: 1 };

    // Edit 1: replace "hello" with "hi"
    // Edit 2: replace "world" with "earth" (positions shift after edit 1)
    let changes = vec![
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            }),
            range_length: None,
            text: "hi".to_string(),
        },
        // After edit 1, doc is "hi world\n"
        // "world" is now at chars [3, 8)
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 3 },
                end: Position { line: 0, character: 8 },
            }),
            range_length: None,
            text: "earth".to_string(),
        },
    ];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(doc.rope.to_string(), "hi earth\n", "Both edits should be applied sequentially");

    Ok(())
}

/// Multi-line replacement: replace lines 1-2 with a single new line
#[test]
fn test_multi_line_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Doc { rope: Rope::from_str("line0\nline1\nline2\nline3\n"), version: 1 };

    // Replace lines 1-2 (inclusive) with a single replacement line
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 1, character: 0 },
            end: Position { line: 3, character: 0 },
        }),
        range_length: None,
        text: "replaced\n".to_string(),
    }];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(
        doc.rope.to_string(),
        "line0\nreplaced\nline3\n",
        "Two lines should collapse into one replacement"
    );

    Ok(())
}

/// Non-ASCII / UTF-16 boundary: edit after emoji using UTF-16 positions
#[test]
fn test_edit_after_emoji_utf16() -> Result<(), Box<dyn std::error::Error>> {
    // "ab\u{1F600}cd\n" - emoji is 2 UTF-16 code units
    let mut doc = Doc { rope: Rope::from_str("ab\u{1F600}cd\n"), version: 1 };

    // In UTF-16: 'a'=0, 'b'=1, emoji=[2,4), 'c'=4, 'd'=5
    // Replace "cd" (characters 4-6 in UTF-16)
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 4 },
            end: Position { line: 0, character: 6 },
        }),
        range_length: None,
        text: "XY".to_string(),
    }];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(
        doc.rope.to_string(),
        "ab\u{1F600}XY\n",
        "Edit after emoji should use correct UTF-16 offsets"
    );

    Ok(())
}

/// Full document replace: change event without range replaces entire content
#[test]
fn test_full_document_replace() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Doc { rope: Rope::from_str("old content\nwith multiple\nlines\n"), version: 1 };

    // No range = full document replacement
    let changes = vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "brand new content\n".to_string(),
    }];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(
        doc.rope.to_string(),
        "brand new content\n",
        "Full document should be replaced when no range is given"
    );

    Ok(())
}

/// Accented characters: edit after multi-byte UTF-8 chars that are single UTF-16 units
#[test]
fn test_edit_after_accented_chars() -> Result<(), Box<dyn std::error::Error>> {
    // e-acute is 2 bytes in UTF-8 but 1 UTF-16 code unit
    let mut doc = Doc { rope: Rope::from_str("\u{00E9}\u{00E9}ab\n"), version: 1 };

    // In UTF-16: e-acute=0, e-acute=1, 'a'=2, 'b'=3
    // Replace "ab" at [2,4)
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 2 },
            end: Position { line: 0, character: 4 },
        }),
        range_length: None,
        text: "ZZ".to_string(),
    }];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(
        doc.rope.to_string(),
        "\u{00E9}\u{00E9}ZZ\n",
        "Accented chars are 1 UTF-16 unit each"
    );

    Ok(())
}

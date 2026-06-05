use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use perl_lsp::textdoc::{Doc, PosEnc, apply_changes, safe_range_mapping};
use ropey::Rope;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn malformed_did_change_range_is_rejected_for_incremental_mapping() -> TestResult {
    let rope = Rope::from_str("my $x = 1;\n");
    let reversed = Range {
        start: Position { line: 0, character: 8 },
        end: Position { line: 0, character: 3 },
    };

    let mapping = safe_range_mapping(&rope, &reversed, PosEnc::Utf16);
    assert!(mapping.is_none(), "reversed ranges must not map into parser incremental edits");
    Ok(())
}

#[test]
fn multibyte_boundary_edit_is_rejected_for_incremental_mapping() -> TestResult {
    let rope = Rope::from_str("hi 😀x\n");

    let split_surrogate = Range {
        start: Position { line: 0, character: 4 },
        end: Position { line: 0, character: 5 },
    };

    let mapping = safe_range_mapping(&rope, &split_surrogate, PosEnc::Utf16);
    assert!(
        mapping.is_none(),
        "ranges that split a UTF-16 surrogate pair must degrade conservatively"
    );
    Ok(())
}

#[test]
fn full_document_replacement_event_is_conservative_by_definition() -> TestResult {
    let mut doc = Doc { rope: Rope::from_str("old\n"), version: 1 };
    let full_replace = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "new\n".to_string(),
    };

    apply_changes(&mut doc, &[full_replace], PosEnc::Utf16);
    assert_eq!(doc.rope.to_string(), "new\n");
    Ok(())
}

#[test]
fn malformed_ranges_do_not_panic_or_corrupt_following_changes() -> TestResult {
    let mut doc = Doc { rope: Rope::from_str("my $x = 1;\n"), version: 1 };

    let malformed = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 9 },
            end: Position { line: 0, character: 2 },
        }),
        range_length: None,
        text: "BROKEN".to_string(),
    };

    let valid = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 8 },
            end: Position { line: 0, character: 9 },
        }),
        range_length: None,
        text: "2".to_string(),
    };

    apply_changes(&mut doc, &[malformed, valid], PosEnc::Utf16);
    assert_eq!(doc.rope.to_string(), "my $x = 2;\n");
    Ok(())
}

#[test]
fn valid_utf16_range_maps_chars_and_bytes_for_incremental_parser() -> TestResult {
    let rope = Rope::from_str("my $rocket = \"🚀\";\n");
    let range = Range {
        start: Position { line: 0, character: 14 },
        end: Position { line: 0, character: 16 },
    };

    let mapping = safe_range_mapping(&rope, &range, PosEnc::Utf16)
        .ok_or("emoji range should map when both endpoints are UTF-16 aligned")?;

    assert_eq!(mapping.start_byte, 14, "emoji starts after ASCII prefix bytes");
    assert_eq!(mapping.end_byte, 18, "emoji occupies four UTF-8 bytes");
    assert_eq!(mapping.end_char - mapping.start_char, 1, "emoji is one rope char");
    Ok(())
}

#[test]
fn crlf_line_end_range_maps_before_line_separator() -> TestResult {
    let rope = Rope::from_str("my $x = 1;\r\nmy $y = 2;\r\n");
    let range = Range {
        start: Position { line: 0, character: 10 },
        end: Position { line: 0, character: 10 },
    };

    let mapping = safe_range_mapping(&rope, &range, PosEnc::Utf16)
        .ok_or("end-of-line insertion before CRLF should map exactly")?;

    assert_eq!(mapping.start_byte, 10, "mapping should stop before the CRLF bytes");
    assert_eq!(mapping.end_byte, 10, "zero-width insertion should preserve end byte");
    Ok(())
}

#[test]
fn utf8_range_inside_multibyte_character_is_rejected_for_incremental_mapping() -> TestResult {
    let rope = Rope::from_str("my $name = \"éclair\";\n");
    let split_utf8 = Range {
        start: Position { line: 0, character: 13 },
        end: Position { line: 0, character: 14 },
    };

    let mapping = safe_range_mapping(&rope, &split_utf8, PosEnc::Utf8);

    assert!(
        mapping.is_none(),
        "ranges that split a multi-byte UTF-8 scalar must use the full reparse path"
    );
    Ok(())
}

#[test]
fn sequential_changes_can_replace_crlf_line_and_then_append() -> TestResult {
    let mut doc = Doc { rope: Rope::from_str("my $x = 1;\r\nprint $x;\r\n"), version: 1 };
    let changes = vec![
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 8 },
                end: Position { line: 0, character: 9 },
            }),
            range_length: None,
            text: "2".to_string(),
        },
        TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 1, character: 9 },
                end: Position { line: 1, character: 9 },
            }),
            range_length: None,
            text: " # updated".to_string(),
        },
    ];

    apply_changes(&mut doc, &changes, PosEnc::Utf16);

    assert_eq!(doc.rope.to_string(), "my $x = 2;\r\nprint $x; # updated\r\n");
    Ok(())
}

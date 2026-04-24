use super::*;

#[test]
fn malformed_range_order_degrades_to_full_document_span_for_parser_mapping() {
    let rope = Rope::from_str("my $x = 1;\n");
    let malformed = Range {
        start: Position { line: 0, character: 8 },
        end: Position { line: 0, character: 2 },
    };

    assert_eq!(range_to_chars(&rope, &malformed, PosEnc::Utf16), (0, rope.len_chars()));
    assert_eq!(range_to_bytes(&rope, &malformed, PosEnc::Utf16), (0, rope.len_bytes()));
}

#[test]
fn multibyte_boundary_split_degrades_to_full_document_span_for_parser_mapping() {
    let rope = Rope::from_str("say \"😀\";\n");
    // Emoji starts at UTF-16 unit 5 and spans [5,7). Character 6 is inside the pair.
    let ambiguous = Range {
        start: Position { line: 0, character: 6 },
        end: Position { line: 0, character: 7 },
    };

    assert_eq!(range_to_chars(&rope, &ambiguous, PosEnc::Utf16), (0, rope.len_chars()));
    assert_eq!(range_to_bytes(&rope, &ambiguous, PosEnc::Utf16), (0, rope.len_bytes()));
}

#[test]
fn full_document_replace_event_remains_supported() {
    let mut doc = Doc { rope: Rope::from_str("old\n"), version: 7 };
    let replace = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "new\ncontent\n".to_string(),
    };

    apply_changes(&mut doc, &[replace], PosEnc::Utf16);
    assert_eq!(doc.rope.to_string(), "new\ncontent\n");
}

#[test]
fn malformed_ranged_change_does_not_panic_or_corrupt_textdoc_state() {
    let mut doc = Doc { rope: Rope::from_str("abc😀def\n"), version: 3 };
    let original = doc.rope.to_string();

    let malformed_change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position { line: 0, character: 1000 },
            end: Position { line: 0, character: 1 },
        }),
        range_length: None,
        text: "X".to_string(),
    };

    apply_changes(&mut doc, &[malformed_change], PosEnc::Utf16);

    // Ranged apply path remains lossy/clamped and should keep the rope valid.
    assert!(!doc.rope.to_string().is_empty());
    assert_ne!(doc.rope.len_chars(), 0);
    assert_eq!(doc.rope.to_string(), original);
}

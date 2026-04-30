use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use perl_lsp::textdoc::{Doc, PosEnc, apply_changes, safe_range_mapping};
use ropey::Rope;

#[test]
fn malformed_did_change_range_is_rejected_for_incremental_mapping()
-> Result<(), Box<dyn std::error::Error>> {
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
fn multibyte_boundary_edit_is_rejected_for_incremental_mapping()
-> Result<(), Box<dyn std::error::Error>> {
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
fn full_document_replacement_event_is_conservative_by_definition()
-> Result<(), Box<dyn std::error::Error>> {
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
fn malformed_ranges_do_not_panic_or_corrupt_following_changes()
-> Result<(), Box<dyn std::error::Error>> {
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

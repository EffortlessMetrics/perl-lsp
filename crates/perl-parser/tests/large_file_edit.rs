use perl_parser::textdoc::{Doc, PosEnc, apply_changes};
use ropey::Rope;
use lsp_types::TextDocumentContentChangeEvent;

/// Ensure that applying edits on large files remains efficient and accurate
#[test]
fn large_file_edit() {
    let initial = "a".repeat(100_000);
    let mut doc = Doc { rope: Rope::from_str(&initial), version: 1 };
    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "b".repeat(100_000),
    };
    apply_changes(&mut doc, &[change], PosEnc::Utf16);
    assert_eq!(doc.rope.len_bytes(), 100_000);
    assert_eq!(doc.rope.to_string().chars().next().unwrap(), 'b');
}

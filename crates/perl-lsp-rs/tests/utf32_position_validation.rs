use lsp_types::Position;
use perl_lsp::textdoc::{PosEnc, byte_to_lsp_pos, lsp_pos_to_byte};
use ropey::Rope;

#[test]
fn utf32_position_counts_scalars_for_non_bmp_chars() {
    let rope = Rope::from_str("hi \u{1F600}x");

    // UTF-32 counts scalar values, so 'x' is at scalar column 4.
    let pos = Position { line: 0, character: 4 };
    let byte = lsp_pos_to_byte(&rope, pos, PosEnc::Utf32);

    assert_eq!(byte, 7);
}

#[test]
fn utf32_roundtrip_byte_to_position_to_byte() {
    let rope = Rope::from_str("a\u{1F600}b");
    let byte = 5; // 'b'

    let pos = byte_to_lsp_pos(&rope, byte, PosEnc::Utf32);
    assert_eq!(pos.character, 2);

    let back = lsp_pos_to_byte(&rope, pos, PosEnc::Utf32);
    assert_eq!(back, byte);
}

use perl_symbol::cursor::{byte_offset_utf16, token_under_cursor};

#[test]
fn utf16_to_byte_and_token_lookup_agree_for_ascii() {
    let text = "my $count = 1;\n";
    let col = 5;
    let byte = byte_offset_utf16("my $count = 1;", col);
    assert_eq!(byte, col);
    assert_eq!(token_under_cursor(text, 0, col), Some("$count".to_string()));
}

#[test]
fn utf16_to_byte_and_token_lookup_agree_with_surrogate_pair_prefix() {
    let text = "😀 $count = 1;\n";
    // Cursor on 'c' in "$count". UTF-16: 😀 is two units, then space + '$' + 'c'.
    let col_utf16 = 5;
    let byte = byte_offset_utf16("😀 $count = 1;", col_utf16);
    assert_eq!(byte, 7);
    assert_eq!(token_under_cursor(text, 0, col_utf16), Some("$count".to_string()));
}

#[test]
fn token_under_cursor_returns_none_at_line_end() {
    let text = "use Demo::Worker;\n";
    let col_utf16 = "use Demo::Worker;".encode_utf16().count();
    assert_eq!(token_under_cursor(text, 0, col_utf16), None);
}

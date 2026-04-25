use perl_symbol::cursor::{
    byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position, token_under_cursor,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn utf16_byte_parity_for_scalar_identifier() -> Result<()> {
    let line = "my $value = 1;";
    let text = format!("{line}\n");

    let col_utf16 = line.find('v').ok_or("missing fixture cursor")?;
    let byte_pos = byte_offset_utf16(line, col_utf16);

    let token = token_under_cursor(&text, 0, col_utf16).ok_or("missing token")?;
    let (name, _kind) = extract_symbol_from_source(byte_pos, line).ok_or("missing symbol")?;
    let (start, end) = get_symbol_range_at_position(byte_pos, line).ok_or("missing range")?;

    assert_eq!(token, "$value");
    assert_eq!(name, "value");
    assert_eq!(&line[start..end], "$value");
    Ok(())
}

#[test]
fn utf16_byte_parity_for_unicode_prefix_and_module_token() -> Result<()> {
    let line = "😀 use Demo::Worker;";
    let text = format!("{line}\n");

    let demo_byte = line.find("Demo").ok_or("missing fixture cursor")?;
    let col_utf16 = line[..demo_byte].encode_utf16().count();
    let byte_pos = byte_offset_utf16(line, col_utf16);

    let token = token_under_cursor(&text, 0, col_utf16).ok_or("missing module token")?;
    let (name, _kind) = extract_symbol_from_source(byte_pos, line).ok_or("missing symbol")?;

    assert_eq!(token, "Demo::Worker");
    assert_eq!(name, "Demo");
    Ok(())
}

/// `get_symbol_range_at_position` with cursor in the MIDDLE of an identifier
/// must return the full sigil+name range — not just the suffix from cursor.
/// The shared scanner's `include_leading_identifier` walk-back is what makes
/// this work; this test locks in that behavior.
#[test]
fn range_cursor_in_middle_of_identifier_returns_full_sigil_range() -> Result<()> {
    let line = "print $total;";
    // Position the cursor on 'o' (middle of "total"), NOT the first char.
    let byte_pos = line.find("otal").ok_or("missing 'otal' in fixture")?;
    let (start, end) = get_symbol_range_at_position(byte_pos, line).ok_or("expected Some range")?;
    // Full range must cover "$total", not just "otal".
    assert_eq!(&line[start..end], "$total", "range should span the full sigil+name");
    Ok(())
}

/// Cursor positioned on a 4-byte UTF-8 code point (emoji) must return None
/// for all three cursor extraction functions — no panic, no OOB access.
#[test]
fn cursor_on_multibyte_emoji_byte_returns_none() {
    // "😀" encodes as 4 bytes: F0 9F 98 80
    let line = "😀foo";
    // Byte 0 is the first byte of the emoji (0xF0).
    assert!(
        extract_symbol_from_source(0, line).is_none(),
        "cursor on leading byte of emoji must not produce a symbol"
    );
    // get_symbol_range_at_position falls back to Some((pos, pos)) for non-ident bytes;
    // verify it returns the empty-range sentinel, not None.
    assert_eq!(
        get_symbol_range_at_position(0, line),
        Some((0, 0)),
        "non-ident byte should return empty sentinel range"
    );
    assert!(
        token_under_cursor(&format!("{line}\n"), 0, 0).is_none(),
        "token_under_cursor on emoji column 0 must return None"
    );
}

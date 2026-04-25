use perl_symbol::cursor::{
    byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position, token_under_cursor,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn utf16_byte_parity_for_scalar_identifier() -> Result<()> {
    // All chars before 'v' are ASCII so byte offset == UTF-16 column.
    // The real multi-byte divergence is exercised by `utf16_byte_parity_for_unicode_prefix_and_module_token`.
    let line = "my $value = 1;";
    let text = format!("{line}\n");

    // find() returns a byte offset; for ASCII that equals the UTF-16 column.
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

/// Cursor placed in the middle of an identifier (not at the first char) must
/// still extract only the trailing portion for `extract_symbol_from_source`
/// (which does not walk backward) but the full range for
/// `get_symbol_range_at_position` (which does walk backward).
#[test]
fn cursor_in_middle_of_identifier_extracts_trailing_name_only() -> Result<()> {
    // Cursor on 'l' (index 2 within "value", byte 6 in the line "$value")
    let line = "$value";
    let byte_pos = line.find('l').ok_or("missing 'l'")?;

    let (name, kind) = extract_symbol_from_source(byte_pos, line).ok_or("missing symbol")?;
    // extract_symbol_from_source starts from the cursor, not the sigil, so
    // it returns the trailing portion "lue".
    assert_eq!(name, "lue");
    assert_eq!(kind, perl_symbol::cursor::CursorSymbolKind::Subroutine);

    let (start, end) = get_symbol_range_at_position(byte_pos, line).ok_or("missing range")?;
    // range walks backward to the sigil so the full token is returned.
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

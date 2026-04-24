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

use perl_symbol::cursor::{
    extract_symbol_from_source, get_symbol_range_at_position, token_under_cursor,
};
use perl_tdd_support::must_some;

#[test]
fn token_under_cursor_matches_byte_scanner_for_scalar_symbol() -> Result<(), String> {
    let text = "my $value = 1;\n";
    let line = text.lines().next().ok_or_else(|| "missing line".to_string())?;
    let byte_pos = line.find('v').ok_or_else(|| "missing symbol".to_string())?;

    let from_line = must_some(token_under_cursor(text, 0, byte_pos));
    let (name, _) = must_some(extract_symbol_from_source(byte_pos, line));
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, line));

    assert_eq!(from_line, "$value");
    assert_eq!(&line[start..end], "$value");
    assert_eq!(name, "value");
    Ok(())
}

#[test]
fn utf16_column_conversion_keeps_parity_for_non_ascii_prefix() -> Result<(), String> {
    let text = "😀 $alpha = 1;\n";
    let line = text.lines().next().ok_or_else(|| "missing line".to_string())?;
    let byte_pos = line.find('a').ok_or_else(|| "missing symbol".to_string())?;

    // col 4 = emoji surrogate pair (2 UTF-16 units) + space + '$'
    let from_utf16 = must_some(token_under_cursor(text, 0, 4));
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, line));

    assert_eq!(from_utf16, "$alpha");
    assert_eq!(&line[start..end], "$alpha");
    Ok(())
}

#[test]
fn module_token_parity_between_utf16_and_byte_range() -> Result<(), String> {
    let text = "use Demo::Worker;\n";
    let line = text.lines().next().ok_or_else(|| "missing line".to_string())?;
    let byte_pos = line.find("Demo").ok_or_else(|| "missing token".to_string())? + 2;

    let utf16_token = must_some(token_under_cursor(text, 0, 8));

    // For module names, range scanner is identifier-oriented and stops at ':'
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, line));
    assert_eq!(utf16_token, "Demo::Worker");
    assert_eq!(&line[start..end], "Demo");
    Ok(())
}

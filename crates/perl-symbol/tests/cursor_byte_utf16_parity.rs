use perl_symbol::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position,
    token_under_cursor,
};
use perl_tdd_support::must_some;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn cursor_byte_range_and_extract_share_span_on_scalar_symbol() -> Result<()> {
    let line = "my $total_count = 1;";
    let pos = line.find("count").ok_or("missing count")?;

    let (name, kind) = must_some(extract_symbol_from_source(pos, line));
    let (start, end) = must_some(get_symbol_range_at_position(pos, line));

    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(name, "total_count");
    assert_eq!(&line[start..end], "$total_count");
    Ok(())
}

#[test]
fn token_under_cursor_matches_byte_scanner_for_ascii_module_names() -> Result<()> {
    let line = "use Demo::Worker;";
    let col_utf16 = line.find("Worker").ok_or("missing Worker")?;

    let token_utf16 = must_some(token_under_cursor(&(line.to_string() + "\n"), 0, col_utf16));
    let byte_pos = byte_offset_utf16(line, col_utf16);
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, line));

    // range scanner excludes module separators by design, token scanner includes them.
    assert_eq!(token_utf16, "Demo::Worker");
    assert_eq!(&line[start..end], "Worker");
    Ok(())
}

#[test]
fn utf16_to_byte_parity_holds_with_non_ascii_prefix() -> Result<()> {
    let line = "😀 $value = 1;";
    let utf16_col = 4; // after emoji+space+sigil -> cursor on 'v'
    let byte_pos = byte_offset_utf16(line, utf16_col);

    let (name, kind) = must_some(extract_symbol_from_source(byte_pos, line));
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, line));
    let token = must_some(token_under_cursor(&(line.to_string() + "\n"), 0, utf16_col));

    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(name, "value");
    assert_eq!(&line[start..end], "$value");
    assert_eq!(token, "$value");
    Ok(())
}

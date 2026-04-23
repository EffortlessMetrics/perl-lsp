use perl_parser_core::line_index::LineIndex;

#[test]
fn line_index_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new(String::new());
    // offset 0 should map to line 0, col 0
    let (line, col) = index.offset_to_position(0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
    Ok(())
}

#[test]
fn line_index_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("hello".to_string());
    let (line, col) = index.offset_to_position(0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);

    // "hello" last char at offset 4
    let (line2, col2) = index.offset_to_position(4);
    assert_eq!(line2, 0);
    assert_eq!(col2, 4);
    Ok(())
}

#[test]
fn line_index_multiple_lines() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("a\nb\nc".to_string());
    // "b" is at offset 2
    let (line, col) = index.offset_to_position(2);
    assert_eq!(line, 1);
    assert_eq!(col, 0);

    // "c" is at offset 4
    let (line2, col2) = index.offset_to_position(4);
    assert_eq!(line2, 2);
    assert_eq!(col2, 0);
    Ok(())
}

#[test]
fn line_index_position_to_offset_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\ndef\nghi".to_string();
    let index = LineIndex::new(text);

    // "def" starts at line 1, col 0 → offset 4
    let offset = index.position_to_offset(1, 0);
    assert_eq!(offset, Some(4));

    let (line, col) = index.offset_to_position(4);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
    Ok(())
}

#[test]
fn line_index_range() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abc\ndef".to_string());
    let (start, end) = index.range(0, 4);
    assert_eq!(start, (0, 0)); // "a" at line 0 col 0
    assert_eq!(end, (1, 0)); // "d" at line 1 col 0
    Ok(())
}

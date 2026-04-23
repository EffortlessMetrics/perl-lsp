use perl_parser_core::PositionMapper;

#[test]
fn is_empty_true() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("");
    assert!(mapper.is_empty());
    assert_eq!(mapper.len_bytes(), 0);
    Ok(())
}

#[test]
fn is_empty_false() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("a");
    assert!(!mapper.is_empty());
    assert_eq!(mapper.len_bytes(), 1);
    Ok(())
}

#[test]
fn char_to_lsp_pos_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello\nworld";
    let mapper = PositionMapper::new(source);
    // char 0 = 'h' on line 0, col 0
    let pos = mapper.char_to_lsp_pos(0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 0);
    // char 6 = 'w' on line 1, col 0
    let pos2 = mapper.char_to_lsp_pos(6);
    assert_eq!(pos2.line, 1);
    assert_eq!(pos2.character, 0);
    Ok(())
}

#[test]
fn lsp_pos_to_char_and_back() -> Result<(), Box<dyn std::error::Error>> {
    let source = "abc\ndefgh";
    let mapper = PositionMapper::new(source);
    // byte 4 = 'd' => line 1, col 0
    let pos = mapper.byte_to_lsp_pos(4);
    if let Some(char_idx) = mapper.lsp_pos_to_char(pos) {
        let roundtrip = mapper.char_to_lsp_pos(char_idx);
        assert_eq!(roundtrip.line, pos.line);
        assert_eq!(roundtrip.character, pos.character);
    }
    Ok(())
}

#[test]
fn out_of_bounds_byte_position() -> Result<(), Box<dyn std::error::Error>> {
    let source = "short";
    let mapper = PositionMapper::new(source);
    // byte offset beyond source length should clamp
    let pos = mapper.byte_to_lsp_pos(1000);
    // Should not crash, returns clamped position
    assert!(pos.line <= 1);
    Ok(())
}

#[test]
fn apply_edit_insert() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("hello world");
    mapper.apply_edit(5, 5, " beautiful");
    assert_eq!(mapper.text(), "hello beautiful world");
    Ok(())
}

#[test]
fn apply_edit_delete() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("hello beautiful world");
    mapper.apply_edit(5, 15, "");
    assert_eq!(mapper.text(), "hello world");
    Ok(())
}

#[test]
fn apply_edit_replace() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("hello world");
    mapper.apply_edit(6, 11, "earth");
    assert_eq!(mapper.text(), "hello earth");
    Ok(())
}

#[test]
fn slice_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let source = "abcdefghij";
    let mapper = PositionMapper::new(source);
    assert_eq!(mapper.slice(0, 3), "abc");
    assert_eq!(mapper.slice(7, 10), "hij");
    // Entire string
    assert_eq!(mapper.slice(0, 10), "abcdefghij");
    // Empty slice
    assert_eq!(mapper.slice(5, 5), "");
    Ok(())
}

#[test]
fn update_replaces_entirely() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("old content");
    mapper.update("new content\nwith lines");
    assert_eq!(mapper.text(), "new content\nwith lines");
    assert_eq!(mapper.len_lines(), 2);
    Ok(())
}

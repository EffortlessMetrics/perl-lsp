use perl_parser_core::{
    LineEnding,
    // Position mapping
    PositionMapper,
};
use perl_tdd_support::must_some;

#[test]
fn mapper_empty_text() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("");
    assert_eq!(mapper.len_bytes(), 0);
    assert!(mapper.is_empty());
    Ok(())
}

#[test]
fn mapper_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("hello world");
    assert_eq!(mapper.len_bytes(), 11);
    assert!(!mapper.is_empty());
    assert_eq!(mapper.len_lines(), 1);
    Ok(())
}

#[test]
fn mapper_multi_line_lf() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("line1\nline2\nline3");
    assert_eq!(mapper.len_lines(), 3);
    assert_eq!(mapper.line_ending(), LineEnding::Lf);
    Ok(())
}

#[test]
fn mapper_crlf_detection() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("line1\r\nline2\r\n");
    assert_eq!(mapper.line_ending(), LineEnding::CrLf);
    Ok(())
}

#[test]
fn mapper_byte_to_lsp_pos_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let text = "my $x = 42;\nmy $y = 99;";
    let mapper = PositionMapper::new(text);

    // Byte 0 should map to line 0, char 0
    let pos = mapper.byte_to_lsp_pos(0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 0);

    // Roundtrip: lsp_pos -> byte -> lsp_pos
    let byte = must_some(mapper.lsp_pos_to_byte(pos));
    assert_eq!(byte, 0);
    Ok(())
}

#[test]
fn mapper_second_line_position() -> Result<(), Box<dyn std::error::Error>> {
    let text = "line1\nline2";
    let mapper = PositionMapper::new(text);

    // "line2" starts at byte 6
    let pos = mapper.byte_to_lsp_pos(6);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 0);
    Ok(())
}

#[test]
fn mapper_text_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let original = "my $x = 42;\nmy $y = 99;";
    let mapper = PositionMapper::new(original);
    assert_eq!(mapper.text(), original);
    Ok(())
}

#[test]
fn mapper_slice() -> Result<(), Box<dyn std::error::Error>> {
    let mapper = PositionMapper::new("hello world");
    let slice = mapper.slice(0, 5);
    assert_eq!(slice, "hello");
    Ok(())
}

#[test]
fn mapper_update_replaces_content() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("old text");
    mapper.update("new text");
    assert_eq!(mapper.text(), "new text");
    Ok(())
}

#[test]
fn mapper_apply_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut mapper = PositionMapper::new("hello world");
    // Replace "world" (bytes 6..11) with "rust"
    mapper.apply_edit(6, 11, "rust");
    assert_eq!(mapper.text(), "hello rust");
    Ok(())
}

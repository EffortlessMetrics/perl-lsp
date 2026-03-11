use perl_line_index::LineIndex;

#[test]
fn roundtrip_line_and_column() {
    let index = LineIndex::new("abc\ndef\nxyz");
    let (line, col) = index.byte_to_position(4);
    assert_eq!((line, col), (1, 0));
    assert_eq!(index.position_to_byte(line, col), Some(4));
}

#[test]
fn out_of_bounds_line_returns_none() {
    let index = LineIndex::new("one\ntwo");
    assert_eq!(index.position_to_byte(10, 0), None);
}

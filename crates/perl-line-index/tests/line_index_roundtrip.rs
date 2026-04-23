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

#[test]
fn checked_position_to_byte_rejects_out_of_range_column() {
    let index = LineIndex::new("one\ntwo");
    assert_eq!(index.position_to_byte_checked(0, 3), Some(3));
    assert_eq!(index.position_to_byte_checked(0, 5), None);
}

#[test]
fn checked_position_to_byte_accepts_terminal_empty_line() {
    let index = LineIndex::new("one\n");
    assert_eq!(index.position_to_byte_checked(1, 0), Some(4));
    assert_eq!(index.position_to_byte_checked(1, 1), None);
}

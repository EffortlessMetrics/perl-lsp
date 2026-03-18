use perl_text_line::{
    is_identifier_byte, is_keyword_boundary, line_bounds_at, skip_ascii_whitespace,
};

#[test]
fn returns_full_range_for_single_line() {
    let line = "use Foo::Bar;";
    assert_eq!(line_bounds_at(line, 0), (0, line.len()));
    assert_eq!(line_bounds_at(line, line.len()), (0, line.len()));
}

#[test]
fn returns_local_line_when_cursor_is_on_multiline_text() {
    let source = "package Demo;\nuse Foo::Bar;\nmy $x;\n";
    let cursor = source.find("Foo::Bar").unwrap_or(0);
    let (start, end) = line_bounds_at(source, cursor);

    assert_eq!(&source[start..end], "use Foo::Bar;");
}

#[test]
fn treats_keyword_as_bounded_by_whitespace_or_punctuation() {
    let line = "package Foo; use My::Module; # trailing";
    let bytes = line.as_bytes();
    let use_idx = line.find("use").unwrap_or(0);
    let pkg_idx = line.find("package").unwrap_or(0);
    assert!(is_keyword_boundary(bytes, use_idx, 3));
    assert!(!is_keyword_boundary(bytes, pkg_idx, 3));
}

#[test]
fn skip_ascii_whitespace_advances_over_spaces_and_tabs() {
    let bytes = " \t  use".as_bytes();
    let idx = skip_ascii_whitespace(bytes, 0);
    assert_eq!(idx, 4);
    assert!(is_identifier_byte(b'u'));
}

#[test]
fn treats_newline_as_ascii_whitespace_in_skip() {
    let bytes = "\n \t".as_bytes();
    let idx = skip_ascii_whitespace(bytes, 0);
    assert_eq!(idx, 3);
}

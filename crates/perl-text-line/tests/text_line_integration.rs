use perl_module::token_parser::parse_module_token;
use perl_text_line::{
    is_identifier_byte, is_keyword_boundary, line_bounds_at, skip_ascii_whitespace,
};

#[test]
fn integration_with_module_reference_style_scanning_keeps_ranges_stable() {
    let source = "package Demo;\nuse Foo::Bar qw(Thing);\n";
    let cursor = source.find("Foo::Bar").unwrap_or(0) + 4;
    let (line_start, line_end) = line_bounds_at(source, cursor);
    let line = &source[line_start..line_end];
    let bytes = line.as_bytes();

    let use_start = line.find("use").unwrap_or(0);
    assert!(is_keyword_boundary(bytes, use_start, 3));
    let token_start = skip_ascii_whitespace(bytes, use_start + 3);
    let token = parse_module_token(line, token_start);
    assert!(token.is_some(), "module token should be parsed");
    let token = token.unwrap_or(perl_module::token_core::ModuleTokenSpan { start: 0, end: 0 });
    let token_text = &line[token.start..token.end];

    assert_eq!(token_text, "Foo::Bar");
    assert_eq!(&line[token_start..token.end], token_text);
    assert!(is_identifier_byte(token_text.as_bytes()[0]));
}

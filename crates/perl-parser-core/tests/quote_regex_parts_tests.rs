use perl_parser_core::syntax::quote::extract_regex_parts;

#[test]
fn strips_m_prefix_for_symbol_delimiter() {
    let (pattern, body, modifiers) = extract_regex_parts("m/foo/i");
    assert_eq!(pattern, "/foo/");
    assert_eq!(body, "foo");
    assert_eq!(modifiers, "i");
}

#[test]
fn does_not_strip_m_prefix_for_alphabetic_second_char() {
    let (pattern, _, _) = extract_regex_parts("match");
    assert!(pattern.starts_with('m'));
}

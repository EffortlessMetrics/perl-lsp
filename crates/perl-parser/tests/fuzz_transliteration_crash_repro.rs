use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/");

    assert_eq!(search, "abc");
    assert_eq!(replace, "xyz");
    assert_eq!(modifiers, "");
}

#[test]
fn fuzz_transliteration_regression_suite() {
    let test_cases = [
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")),
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr /a\\//x\\//", ("a\\/", "x\\/", "")),
        ("tr/🦀/🐪/r", ("🦀", "🐪", "r")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_eq!((search.as_str(), replace.as_str(), modifiers.as_str()), expected, "{input}");
    }
}

#[test]
fn malformed_transliteration_never_panics() {
    let malformed_inputs =
        ["tr", "trabc", "tr/unterminated", "tr{abc}{xyz", "tr{abc}xyz}", "y\\abc\\xyz\\"];

    for input in malformed_inputs {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "extract_transliteration_parts panicked for {input}");
    }
}

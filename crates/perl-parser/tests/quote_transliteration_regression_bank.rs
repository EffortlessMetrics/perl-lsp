use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn transliteration_regression_bank_delimiter_edge_cases() {
    let cases = [
        ("tr|||", ("", "", "")),
        ("tr///", ("", "", "")),
        ("tr,a,b,", ("a", "b", "")),
        ("y<>", ("", "", "")),
        ("tr<ab><xy>cd", ("ab", "xy", "cd")),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!((actual.0.as_str(), actual.1.as_str(), actual.2.as_str()), expected, "{input}");
    }
}

#[test]
fn transliteration_regression_bank_malformed_never_panics() {
    let malformed = [
        "tr",
        "y",
        "tr ",
        "trabc",
        "tr{",
        "tr{abc}",
        "tr/abc",
        "tr/abc/",
        "tr{abc}{def",
        "y[abc]def",
    ];

    for input in malformed {
        let parsed = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(parsed.is_ok(), "panicked for malformed transliteration: {input}");
    }
}

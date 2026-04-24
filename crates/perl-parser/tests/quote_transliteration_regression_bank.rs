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

#[test]
fn transliteration_regression_bank_escaped_delimiter_in_search() {
    // Backslash-escaped delimiter inside search body must be preserved, not treated as closing.
    let cases = [("tr/a\\/b/c/", ("a\\/b", "c", "")), ("tr/x\\/y\\/z/w/", ("x\\/y\\/z", "w", ""))];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!((actual.0.as_str(), actual.1.as_str(), actual.2.as_str()), expected, "{input}");
    }
}

#[test]
fn transliteration_regression_bank_unicode_content() {
    // Unicode characters inside search/replace bodies must not confuse byte-offset arithmetic.
    let cases = [
        ("tr/\u{03B1}/\u{03B2}/", ("\u{03B1}", "\u{03B2}", "")),
        ("tr/\u{00E9}t\u{00E9}/abc/d", ("\u{00E9}t\u{00E9}", "abc", "d")),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!((actual.0.as_str(), actual.1.as_str(), actual.2.as_str()), expected, "{input}");
    }
}

#[test]
fn transliteration_regression_bank_empty_bodies_with_modifiers() {
    // tr with empty search and replace but valid modifiers should extract them correctly.
    let cases =
        [("tr///cds", ("", "", "cds")), ("tr|||rs", ("", "", "rs")), ("y<><>d", ("", "", "d"))];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!((actual.0.as_str(), actual.1.as_str(), actual.2.as_str()), expected, "{input}");
    }
}

use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn transliteration_regression_bank() {
    let cases = [
        ("tr/a\\/b/c\\/d/", ("a\\/b", "c\\/d", "")),
        ("tr/🦀αβ/🐪γδ/cdsr", ("🦀αβ", "🐪γδ", "cdsr")),
        ("tr///", ("", "", "")),
        ("y{}{}r", ("", "", "r")),
        ("tr{abc}{xyz}cdsr", ("abc", "xyz", "cdsr")),
        ("tr[abc]{xyz}d", ("abc", "xyz", "d")),
        ("tr{abc{def}}{xyz{uvw}}r", ("abc{def}", "xyz{uvw}", "r")),
        ("tr/abc/xyz/z", ("abc", "xyz", "")),
        ("tr/abc/xyz/1", ("abc", "xyz", "")),
        ("tr foo bar baz", ("", "", "")),
        ("tr/abc", ("abc", "", "")),
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr<ab\\>c><xy\\>z>", ("ab\\>c", "xy\\>z", "")),
        ("y /abc/xyz/r", ("abc", "xyz", "r")),
    ];

    for (input, expected) in cases {
        let parsed = extract_transliteration_parts(input);
        assert_eq!(
            (parsed.0.as_str(), parsed.1.as_str(), parsed.2.as_str()),
            expected,
            "failed case: {input}"
        );
    }
}

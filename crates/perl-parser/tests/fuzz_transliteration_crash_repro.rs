use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let input = "tr/abc/xyz/";
    let (search, replace, modifiers) = extract_transliteration_parts(input);

    assert_eq!(search.as_str(), "abc", "Search pattern incorrect");
    assert_eq!(replace.as_str(), "xyz", "Replace pattern incorrect");
    assert_eq!(modifiers.as_str(), "", "Modifiers incorrect");
}

#[test]
fn transliteration_regression_bank() {
    let test_cases = [
        // Non-paired delimiters: tr/// and y///
        ("tr/a/b/", ("a", "b", "")),
        ("tr/a/b/cds", ("a", "b", "cds")),
        ("y/abc/xyz/r", ("abc", "xyz", "r")),
        ("y/x/y/g", ("x", "y", "")), // invalid transliteration modifier is filtered out
        // Paired delimiters, including mixed replacement delimiter
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc]{xyz}rs", ("abc", "xyz", "rs")),
        ("y(a[b]c){x[y]z}c", ("a[b]c", "x[y]z", "c")),
        // Delimiter edge cases
        ("tr!a!b!r", ("a", "b", "r")),
        ("tr<ab><xy>z", ("ab", "xy", "")), // invalid modifier z filtered out
        ("tr /a/b/d", ("a", "b", "d")),
        // Malformed-but-nonpanicking cases
        ("tr{abc}", ("abc", "", "")),
        ("tr{abc}d", ("abc", "", "")), // no replacement delimiter => no modifiers
        ("tr/abc", ("abc", "", "")),
        ("trabc", ("", "", "")), // invalid delimiter after operator
        ("y", ("", "", "")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "mismatch for input {input}");
    }
}

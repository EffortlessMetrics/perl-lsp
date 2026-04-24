use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz/"),
        ("abc".to_string(), "xyz".to_string(), "".to_string())
    );
}

#[test]
fn fuzz_transliteration_regression_suite() {
    let test_cases = [
        // tr/// and y///
        ("tr/abc/xyz/", ("abc", "xyz", "")),
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/ds", ("a", "b", "ds")),
        ("y/x/y/g", ("x", "y", "")), // invalid modifier is filtered out
        // paired delimiter forms
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc]{xyz}cr", ("abc", "xyz", "cr")),
        ("y(foo)(bar)s", ("foo", "bar", "s")),
        // delimiter edge cases
        ("tr#abc#xyz#cdr", ("abc", "xyz", "cdr")),
        ("tr /abc/xyz/r", ("abc", "xyz", "r")),
        ("tr/a\\/b/c\\/d/", ("a\\/b", "c\\/d", "")),
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed transliteration case: {input}");
    }
}

#[test]
fn malformed_transliteration_is_nonpanicking() {
    let malformed_cases = [
        "tr",
        "trabc",
        "tr/abc",
        "tr/abc/",
        "tr/abc/xyz",
        "tr{abc}",
        "tr{abc}{",
        "tr{abc}xyz",
        "y",
        "y ",
    ];

    for input in malformed_cases {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "panicked for malformed transliteration: {input}");
    }
}

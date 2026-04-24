//! Regression coverage for fuzz-discovered transliteration crashes/misparses.

use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() {
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/");
    assert_eq!((search.as_str(), replace.as_str(), modifiers.as_str()), ("abc", "xyz", ""));
}

#[test]
fn fuzz_transliteration_regression_suite() {
    let cases = [
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("tr/a\\/b/c\\/d/", ("a\\/b", "c\\/d", "")),
        ("tr/αβγ/ΑΒΓ/", ("αβγ", "ΑΒΓ", "")),
        ("tr//xyz/", ("", "xyz", "")),
        ("tr/abc//", ("abc", "", "")),
        ("tr/abc/xyz/z", ("abc", "xyz", "")),
        ("tr/abc/xyz/1", ("abc", "xyz", "")),
        ("tr/abc/xyz", ("abc", "xyz", "")),
        ("tr{abc}{xyz}r", ("abc", "xyz", "r")),
        ("tr{abc}[xyz]cds", ("abc", "xyz", "cds")),
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_eq!(
            (search.as_str(), replace.as_str(), modifiers.as_str()),
            expected,
            "failed for input `{input}`"
        );
    }
}

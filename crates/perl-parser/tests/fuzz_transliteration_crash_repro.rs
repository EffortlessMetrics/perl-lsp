/// Minimal reproduction case for transliteration parsing bug discovered in fuzz testing.
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
fn transliteration_regression_suite_non_paired_and_aliases() {
    let test_cases = [
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")), // g is not a transliteration modifier
        ("tr /a-z/A-Z/cds", ("a-z", "A-Z", "cds")),
        ("y#foo#bar#rs", ("foo", "bar", "rs")),
        ("tr/a/b", ("a", "b", "")), // malformed (no final delimiter), no panic
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed for input {input}");
    }
}

#[test]
fn transliteration_regression_suite_paired_and_malformed() {
    let test_cases = [
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc]{xyz}r", ("abc", "xyz", "r")),
        ("y(a(b)c)(x(y)z)s", ("a(b)c", "x(y)z", "s")),
        ("tr{abc}{xyz", ("abc", "xyz", "")), // malformed replacement close, no panic
        ("tr{abc}xyz", ("abc", "", "")),     // missing replacement delimiter
        ("tr", ("", "", "")),                // obvious malformed input
    ];

    for (input, expected) in test_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed for input {input}");
    }
}

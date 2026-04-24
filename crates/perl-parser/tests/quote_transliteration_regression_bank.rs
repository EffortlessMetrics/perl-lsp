use perl_parser::quote_parser::{
    TransliterationError, extract_transliteration_parts, extract_transliteration_parts_strict,
};

#[test]
fn transliteration_regression_bank() {
    let cases = [
        ("tr/a\\/b/x\\/y/", ("a\\/b", "x\\/y", "")),
        ("tr/æøå/ÆØÅ/d", ("æøå", "ÆØÅ", "d")),
        ("tr//foo/", ("", "foo", "")),
        ("tr/foo//", ("foo", "", "")),
        ("tr{abc}{xyz}cdsr", ("abc", "xyz", "cdsr")),
        ("y[abc]{xyz}r", ("abc", "xyz", "r")),
        ("tr/abc/xyz/zz", ("abc", "xyz", "")),
        ("tr/abc/xyz/cz", ("abc", "xyz", "c")),
        ("tr /a-z/A-Z/s", ("a-z", "A-Z", "s")),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!((actual.0.as_str(), actual.1.as_str(), actual.2.as_str()), expected, "{input}",);
    }
}

#[test]
fn transliteration_all_four_paired_delimiter_kinds() {
    // tr<>, tr{}, tr[], tr() — all four ASCII bracket pairs must work
    let cases = [
        ("tr<abc><xyz>", ("abc", "xyz", "")),
        ("tr(abc)(xyz)s", ("abc", "xyz", "s")),
        ("tr[abc][xyz]d", ("abc", "xyz", "d")),
        ("tr{abc}{xyz}r", ("abc", "xyz", "r")),
        // Nested brackets inside search/replace
        ("tr{a{b}c}{x{y}z}", ("a{b}c", "x{y}z", "")),
        ("tr<a<b>c><x<y>z>", ("a<b>c", "x<y>z", "")),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!(
            (actual.0.as_str(), actual.1.as_str(), actual.2.as_str()),
            expected,
            "{input}",
        );
    }
}

#[test]
fn strict_parser_rejects_invalid_delimiter_and_modifier() {
    let invalid_delimiter = extract_transliteration_parts_strict("trabc");
    assert_eq!(invalid_delimiter, Err(TransliterationError::InvalidDelimiter('a')));

    let invalid_modifier = extract_transliteration_parts_strict("tr/a/b/z");
    assert_eq!(invalid_modifier, Err(TransliterationError::InvalidModifier('z')));
}

#[test]
fn malformed_closures_do_not_smear_modifiers_or_panic() {
    let cases = [
        ("tr/abc/xyz", ("abc", "xyz", "")),
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr{abc}xyz}", ("abc", "", "")),
    ];

    for (input, expected) in cases {
        let unwind_result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(unwind_result.is_ok(), "panic on malformed transliteration input: {input}");
        let parsed = match unwind_result {
            Ok(parts) => parts,
            Err(_) => continue,
        };
        assert_eq!((parsed.0.as_str(), parsed.1.as_str(), parsed.2.as_str()), expected, "{input}",);
    }
}

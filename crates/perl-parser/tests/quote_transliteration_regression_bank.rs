use perl_parser::quote_parser::{
    extract_transliteration_parts, extract_transliteration_parts_strict,
};

#[test]
fn transliteration_regression_bank_core_cases() {
    let cases = [
        ("tr /a\\/b/c\\/d/", ("a\\/b", "c\\/d", "")),
        ("tr/αβγ/ΑΒΓ/r", ("αβγ", "ΑΒΓ", "r")),
        ("tr//x/", ("", "x", "")),
        ("tr/a//d", ("a", "", "d")),
        ("tr{abc}{xyz}cdsr", ("abc", "xyz", "cdsr")),
    ];

    for (input, expected) in cases {
        let parsed = extract_transliteration_parts(input);
        assert_eq!(
            (parsed.0.as_str(), parsed.1.as_str(), parsed.2.as_str()),
            expected,
            "regression in transliteration parser for input: {}",
            input
        );
    }
}

#[test]
fn transliteration_strict_rejects_invalid_forms() {
    let invalid_cases = ["trabc", "tr a", "tr/abc/xyz/z", "y/abc/xyz/9", "tr/abc", "tr{abc}{xyz"];

    for input in invalid_cases {
        assert!(
            extract_transliteration_parts_strict(input).is_err(),
            "strict parser should reject malformed transliteration: {}",
            input
        );
    }
}

#[test]
fn transliteration_strict_accepts_valid_modifiers()
-> Result<(), perl_parser::quote_parser::TransliterationError> {
    for modifier in ['c', 'd', 's', 'r'] {
        let input = format!("tr/a/b/{}", modifier);
        let parsed = extract_transliteration_parts_strict(&input)?;
        assert_eq!(parsed.2, modifier.to_string());
    }
    Ok(())
}

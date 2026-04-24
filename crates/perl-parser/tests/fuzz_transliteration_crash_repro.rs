use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn minimal_transliteration_crash_repro() -> Result<(), Box<dyn std::error::Error>> {
    let (search, replace, modifiers) = extract_transliteration_parts("tr/abc/xyz/");
    assert_eq!((search.as_str(), replace.as_str(), modifiers.as_str()), ("abc", "xyz", ""));
    Ok(())
}

#[test]
fn transliteration_regression_bank() -> Result<(), Box<dyn std::error::Error>> {
    // tr/// and y/// coverage
    let slash_cases = [
        ("tr/abc/xyz/", ("abc", "xyz", "")),
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")),
        ("tr /left/right/c", ("left", "right", "c")),
    ];

    for (input, expected) in slash_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_eq!(
            (search.as_str(), replace.as_str(), modifiers.as_str()),
            expected,
            "slash-delimited transliteration parse mismatch for `{input}`"
        );
    }

    // Paired delimiter forms, including mixed paired delimiters.
    let paired_cases = [
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("tr[abc][xyz]s", ("abc", "xyz", "s")),
        ("tr(abc)<xyz>r", ("abc", "xyz", "r")),
        ("y<a-z>{A-Z}cd", ("a-z", "A-Z", "cd")),
    ];

    for (input, expected) in paired_cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_eq!(
            (search.as_str(), replace.as_str(), modifiers.as_str()),
            expected,
            "paired transliteration parse mismatch for `{input}`"
        );
    }

    // Delimiter edge cases and malformed-but-nonpanicking cases.
    let malformed_cases = [
        ("tr/abc/xyz", ("abc", "xyz", "")),
        ("tr{abc}{xyz", ("abc", "xyz", "")),
        ("tr{abc}xyz", ("abc", "", "")),
        ("trabc", ("", "", "")),
        ("tr /abc/xyz/qd", ("abc", "xyz", "d")),
        ("y", ("", "", "")),
    ];

    for (input, expected) in malformed_cases {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "parser panicked for malformed input `{input}`");
        let (search, replace, modifiers) = match result {
            Ok(parts) => parts,
            Err(_) => return Err("unexpected transliteration panic".into()),
        };
        assert_eq!(
            (search.as_str(), replace.as_str(), modifiers.as_str()),
            expected,
            "malformed transliteration parse mismatch for `{input}`"
        );
    }

    Ok(())
}

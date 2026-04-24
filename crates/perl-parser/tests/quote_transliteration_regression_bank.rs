use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn quote_transliteration_regression_bank_valid_cases() {
    let cases = [
        ("tr/a\\/b/c\\/d/", ("a\\/b".to_string(), "c\\/d".to_string(), String::new())),
        ("tr/🦀π/🐪λ/r", ("🦀π".to_string(), "🐪λ".to_string(), "r".to_string())),
        ("tr///", (String::new(), String::new(), String::new())),
        (
            "tr{abc}/xyz/cdsr",
            ("abc".to_string(), "xyz".to_string(), "cdsr".to_string()),
        ),
        ("y{abc}{xyz}d", ("abc".to_string(), "xyz".to_string(), "d".to_string())),
    ];

    for (input, expected) in cases {
        let actual = extract_transliteration_parts(input);
        assert_eq!(actual, expected, "unexpected parse for {input}");
    }
}

#[test]
fn quote_transliteration_regression_bank_malformed_inputs_are_non_panicking() {
    let malformed =
        ["tr/abc/xyz", "tr{abc}{xyz", "tr[abc]xyz]", "tr/a/b/z", "tr a/b/", "tr\\a\\b\\"];

    for input in malformed {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "parser panicked for malformed input: {input}");
    }
}

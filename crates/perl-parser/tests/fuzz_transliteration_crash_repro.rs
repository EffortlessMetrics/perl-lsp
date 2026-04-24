use perl_parser::quote_parser::extract_transliteration_parts;

#[test]
fn transliteration_non_paired_regressions() {
    let cases = [
        ("tr/abc/xyz/", ("abc", "xyz", "")),
        ("y/abc/xyz/", ("abc", "xyz", "")),
        ("tr/a/b/d", ("a", "b", "d")),
        ("y/x/y/g", ("x", "y", "")), // 'g' is not a valid transliteration modifier
        ("tr /abc/xyz/cdr", ("abc", "xyz", "cdr")),
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed for input {input:?}");
    }
}

#[test]
fn transliteration_paired_delimiter_regressions() {
    let cases = [
        ("tr{abc}{xyz}d", ("abc", "xyz", "d")),
        ("y[abc][xyz]s", ("abc", "xyz", "s")),
        ("tr(abc)(xyz)r", ("abc", "xyz", "r")),
        ("tr<abc>{xyz}cd", ("abc", "xyz", "cd")),
        ("tr [a[b]c] {x{y}z} r", ("a[b]c", "x{y}z", "")),
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed for input {input:?}");
    }
}

#[test]
fn transliteration_malformed_non_panicking_regressions() {
    let malformed_inputs = [
        "tr",                // missing delimiter
        "tr ",               // missing delimiter after whitespace
        "trabc",             // invalid delimiter
        "tr/abc",            // missing replacement section
        "tr/abc/xyz",        // missing closing delimiter
        "tr{abc",            // missing closing delimiter for search section
        "tr{abc}{xyz",       // missing closing delimiter for replacement section
        "tr{abc} xyz",       // paired search with no replacement delimiter
        "y/a/b/q",           // invalid modifier should be dropped
        "y/a/b/rq",          // valid + invalid modifier, keep valid prefix
        "tr#a#b#cd!",        // stop modifiers at first non-alnum
    ];

    for input in malformed_inputs {
        let result = std::panic::catch_unwind(|| extract_transliteration_parts(input));
        assert!(result.is_ok(), "should never panic for malformed input {input:?}");
    }

    assert_eq!(
        extract_transliteration_parts("tr/abc"),
        ("abc".to_string(), String::new(), String::new())
    );
    assert_eq!(
        extract_transliteration_parts("tr/abc/xyz"),
        ("abc".to_string(), String::new(), String::new())
    );
    assert_eq!(
        extract_transliteration_parts("tr{abc}{xyz"),
        ("abc".to_string(), String::new(), String::new())
    );
    assert_eq!(
        extract_transliteration_parts("tr{abc} xyz"),
        ("abc".to_string(), String::new(), String::new())
    );
    assert_eq!(
        extract_transliteration_parts("trabc"),
        (String::new(), String::new(), String::new())
    );
    assert_eq!(
        extract_transliteration_parts("y/a/b/rq"),
        ("a".to_string(), "b".to_string(), "r".to_string())
    );
    assert_eq!(
        extract_transliteration_parts("tr#a#b#cd!"),
        ("a".to_string(), "b".to_string(), "cd".to_string())
    );
}

#[test]
fn transliteration_delimiter_edge_cases() {
    let cases = [
        ("tr||", ("", "", "")),
        ("tr///", ("", "", "")),
        ("tr#\\##\\##", ("\\#", "\\#", "")),
        ("tr@@@", ("", "", "")),
        ("y/🦀/🐪/r", ("🦀", "🐪", "r")),
    ];

    for (input, expected) in cases {
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        let actual = (search.as_str(), replace.as_str(), modifiers.as_str());
        assert_eq!(actual, expected, "failed for input {input:?}");
    }
}

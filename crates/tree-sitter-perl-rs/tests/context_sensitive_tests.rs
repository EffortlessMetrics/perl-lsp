//! Tests for context-sensitive operator parsing

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::context_sensitive::{ContextSensitiveLexer, ContextToken};

    #[test]
    fn test_substitution_operators() {
        let test_cases = vec![
            ("s/foo/bar/", "foo", "bar", ""),
            ("s/foo/bar/g", "foo", "bar", "g"),
            ("s/foo/bar/gi", "foo", "bar", "gi"),
            ("s/foo/bar/gims", "foo", "bar", "gims"),
            ("s|foo|bar|", "foo", "bar", ""),
            ("s{foo}{bar}", "foo", "bar", ""),
            ("s[foo][bar]", "foo", "bar", ""),
            ("s(foo)(bar)", "foo", "bar", ""),
            ("s!foo!bar!", "foo", "bar", ""),
            ("s#foo#bar#", "foo", "bar", ""),
            ("s/\\/usr\\/bin/\\/usr\\/local\\/bin/", "/usr/bin", "/usr/local/bin", ""),
            ("s/\\s+/ /g", "\\s+", " ", "g"),
            ("s/\\w+/WORD/g", "\\w+", "WORD", "g"),
        ];

        for (input, expected_pattern, expected_replacement, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            match lexer.try_parse_operator() {
                Some(ContextToken::Substitution { pattern, replacement, flags }) => {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", input);
                    assert_eq!(replacement, expected_replacement, "Replacement mismatch for {}", input);
                    assert_eq!(flags, expected_flags, "Flags mismatch for {}", input);
                }
                _ => panic!("Failed to parse substitution: {}", input),
            }
        }
    }

    #[test]
    fn test_transliteration_operators() {
        let test_cases = vec![
            ("tr/abc/xyz/", "abc", "xyz", ""),
            ("tr/a-z/A-Z/", "a-z", "A-Z", ""),
            ("tr/0-9/a-j/", "0-9", "a-j", ""),
            ("tr/ /_/", " ", "_", ""),
            ("tr/\\n\\t/ /", "\\n\\t", " ", ""),
            ("y/abc/xyz/", "abc", "xyz", ""),
            ("y/a-z/A-Z/", "a-z", "A-Z", ""),
            ("tr|abc|xyz|", "abc", "xyz", ""),
            ("tr{abc}{xyz}", "abc", "xyz", ""),
            ("tr[abc][xyz]", "abc", "xyz", ""),
            ("tr(abc)(xyz)", "abc", "xyz", ""),
            ("tr!abc!xyz!", "abc", "xyz", ""),
            ("tr#abc#xyz#", "abc", "xyz", ""),
        ];

        for (input, expected_search, expected_replace, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            match lexer.try_parse_operator() {
                Some(ContextToken::Transliteration { search, replace, flags }) => {
                    assert_eq!(search, expected_search, "Search mismatch for {}", input);
                    assert_eq!(replace, expected_replace, "Replace mismatch for {}", input);
                    assert_eq!(flags, expected_flags, "Flags mismatch for {}", input);
                }
                _ => panic!("Failed to parse transliteration: {}", input),
            }
        }
    }

    #[test]
    fn test_match_operators() {
        let test_cases = vec![
            ("m/pattern/", "pattern", ""),
            ("m/pattern/i", "pattern", "i"),
            ("m/pattern/gims", "pattern", "gims"),
            ("m/\\w+/", "\\w+", ""),
            ("m/\\d{2,4}/", "\\d{2,4}", ""),
            ("m/^start/", "^start", ""),
            ("m/end$/", "end$", ""),
            ("m|pattern|", "pattern", ""),
            ("m{pattern}", "pattern", ""),
            ("m[pattern]", "pattern", ""),
            ("m(pattern)", "pattern", ""),
            ("m!pattern!", "pattern", ""),
            ("m#pattern#", "pattern", ""),
            ("m/foo\\/bar/", "foo\\/bar", ""),
        ];

        for (input, expected_pattern, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            match lexer.try_parse_operator() {
                Some(ContextToken::Match { pattern, flags }) => {
                    assert_eq!(pattern, expected_pattern, "Pattern mismatch for {}", input);
                    assert_eq!(flags, expected_flags, "Flags mismatch for {}", input);
                }
                _ => panic!("Failed to parse match: {}", input),
            }
        }
    }

    #[test]
    fn test_escaped_delimiters() {
        let test_cases = vec![
            // Escaped delimiters in patterns
            ("s/\\/home\\/user/\\/Users\\/name/", "\\/home\\/user", "\\/Users\\/name", ""),
            ("m/\\/\\/comment/", "\\/\\/comment", ""),
            ("s/\\//\\\\//g", "\\/", "\\\\", "g"),
            
            // Other escaped characters
            ("s/\\n/\\r\\n/g", "\\n", "\\r\\n", "g"),
            ("s/\\t/ {4}/g", "\\t", " {4}", "g"),
            ("m/\\$\\w+/", "\\$\\w+", ""),
        ];

        for (input, expected_pattern, expected_second, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            match lexer.try_parse_operator() {
                Some(ContextToken::Substitution { pattern, replacement, flags }) => {
                    assert_eq!(pattern, expected_pattern);
                    assert_eq!(replacement, expected_second);
                    assert_eq!(flags, expected_flags);
                }
                Some(ContextToken::Match { pattern, flags }) => {
                    assert_eq!(pattern, expected_pattern);
                    assert_eq!(flags, expected_flags);
                }
                _ => panic!("Failed to parse: {}", input),
            }
        }
    }

    #[test]
    fn test_not_operators() {
        // These should NOT be parsed as operators
        let test_cases = vec![
            "sub", // Just the word sub
            "my", // Just the word my
            "tr", // Just tr without delimiters
            "s", // Just s
            "m", // Just m
            "stringify", // Word starting with s
            "match", // Word starting with m
            "translate", // Word starting with tr
        ];

        for input in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            assert!(lexer.try_parse_operator().is_none(), 
                "Should not parse '{}' as operator", input);
        }
    }

    #[test]
    fn test_complex_patterns() {
        let test_cases = vec![
            // Complex regex patterns
            ("s/(?<word>\\w+)/$+{word}/g", "(?<word>\\w+)", "$+{word}", "g"),
            ("m/(?:https?|ftp):\\/\\//i", "(?:https?|ftp):\\/\\/", "i"),
            ("s/\\b(\\w+)\\s+\\1\\b/$1/g", "\\b(\\w+)\\s+\\1\\b", "$1", "g"),
            
            // Nested delimiters
            ("s{\\{}{{}", "\\{", "{", ""),
            ("s(\\()(())", "\\(", "(", ""),
            
            // Multiple flags
            ("s/foo/bar/gimsxo", "foo", "bar", "gimsxo"),
            ("m/pattern/msixpogcual", "pattern", "msixpogcual"),
        ];

        for (input, expected_pattern, expected_second, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input.to_string());
            match lexer.try_parse_operator() {
                Some(ContextToken::Substitution { pattern, replacement, flags }) => {
                    assert_eq!(pattern, expected_pattern);
                    assert_eq!(replacement, expected_second);
                    assert_eq!(flags, expected_flags);
                }
                Some(ContextToken::Match { pattern, flags }) => {
                    assert_eq!(pattern, expected_pattern);
                    assert_eq!(flags, expected_flags);
                }
                _ => panic!("Failed to parse: {}", input),
            }
        }
    }
}
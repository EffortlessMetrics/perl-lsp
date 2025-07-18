//! Tests for context-sensitive operator parsing

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::context_sensitive::{ContextSensitiveLexer, ContextToken};

    #[test]
    fn test_substitution_operators() {
        let test_cases = vec![
            ("s/foo/bar/", "foo", "bar", ""),
            ("s/foo/bar/g", "foo", "bar", "g"),
            ("s/foo/bar/igs", "foo", "bar", "igs"),
            ("s{old}{new}g", "old", "new", "g"),
            ("s!pattern!replacement!", "pattern", "replacement", ""),
            ("s#\\d+#NUMBER#g", "\\d+", "NUMBER", "g"),
            // With escaped delimiters
            ("s/\\/path/new\\/path/", "\\/path", "new\\/path", ""),
            ("s{\\{braces\\}}{replaced}", "\\{braces\\}", "replaced", ""),
        ];

        for (input, expected_pattern, expected_replacement, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input);
            let tokens = lexer.tokenize();
            
            // Find s/// operator tokens
            let mut found_pattern = false;
            let mut found_replacement = false;
            let mut found_flags = false;
            
            for token in &tokens {
                match &token.token_type {
                    ContextToken::Pattern(p) if p == expected_pattern => found_pattern = true,
                    ContextToken::Replacement(r) if r == expected_replacement => found_replacement = true,
                    ContextToken::Flags(f) if f == expected_flags => found_flags = true,
                    _ => {}
                }
            }
            
            assert!(found_pattern, "Pattern '{}' not found in {}", expected_pattern, input);
            assert!(found_replacement, "Replacement '{}' not found in {}", expected_replacement, input);
            if !expected_flags.is_empty() {
                assert!(found_flags, "Flags '{}' not found in {}", expected_flags, input);
            }
        }
    }

    #[test]
    fn test_transliteration_operators() {
        let test_cases = vec![
            ("tr/abc/xyz/", "abc", "xyz", ""),
            ("tr/a-z/A-Z/", "a-z", "A-Z", ""),
            ("tr/0-9/a-j/", "0-9", "a-j", ""),
            ("y/abc/xyz/", "abc", "xyz", ""),
            ("tr{old}{new}d", "old", "new", "d"),
            ("tr!\\n!\\t!s", "\\n", "\\t", "s"),
        ];

        for (input, expected_search, expected_replace, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input);
            let tokens = lexer.tokenize();
            
            let mut found_search = false;
            let mut found_replace = false;
            let mut found_flags = false;
            
            for token in &tokens {
                match &token.token_type {
                    ContextToken::SearchList(s) if s == expected_search => found_search = true,
                    ContextToken::ReplaceList(r) if r == expected_replace => found_replace = true,
                    ContextToken::Flags(f) if f == expected_flags => found_flags = true,
                    _ => {}
                }
            }
            
            assert!(found_search, "Search list '{}' not found in {}", expected_search, input);
            assert!(found_replace, "Replace list '{}' not found in {}", expected_replace, input);
            if !expected_flags.is_empty() {
                assert!(found_flags, "Flags '{}' not found in {}", expected_flags, input);
            }
        }
    }

    #[test]
    fn test_match_operators() {
        let test_cases = vec![
            ("m/pattern/", "pattern", ""),
            ("m/\\w+/", "\\w+", ""),
            ("m{^start}", "^start", ""),
            ("m!end$!i", "end$", "i"),
            ("m#\\d{2,4}#xms", "\\d{2,4}", "xms"),
            ("/simple/", "simple", ""),
            ("/with flags/gi", "with flags", "gi"),
        ];

        for (input, expected_pattern, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input);
            let tokens = lexer.tokenize();
            
            let mut found_pattern = false;
            let mut found_flags = false;
            
            for token in &tokens {
                match &token.token_type {
                    ContextToken::Pattern(p) if p == expected_pattern => found_pattern = true,
                    ContextToken::Flags(f) if f == expected_flags => found_flags = true,
                    _ => {}
                }
            }
            
            assert!(found_pattern, "Pattern '{}' not found in {}", expected_pattern, input);
            if !expected_flags.is_empty() {
                assert!(found_flags, "Flags '{}' not found in {}", expected_flags, input);
            }
        }
    }

    #[test]
    fn test_qr_operators() {
        let test_cases = vec![
            ("qr/pattern/", "pattern", ""),
            ("qr/\\w+\\s*/i", "\\w+\\s*", "i"),
            ("qr{(?<name>\\w+)}", "(?<name>\\w+)", ""),
            ("qr!\\d+!xms", "\\d+", "xms"),
        ];

        for (input, expected_pattern, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input);
            let tokens = lexer.tokenize();
            
            let mut found_pattern = false;
            let mut found_flags = false;
            
            for token in &tokens {
                match &token.token_type {
                    ContextToken::Pattern(p) if p == expected_pattern => found_pattern = true,
                    ContextToken::Flags(f) if f == expected_flags => found_flags = true,
                    _ => {}
                }
            }
            
            assert!(found_pattern, "Pattern '{}' not found in {}", expected_pattern, input);
            if !expected_flags.is_empty() {
                assert!(found_flags, "Flags '{}' not found in {}", expected_flags, input);
            }
        }
    }

    #[test]
    fn test_complex_delimiters() {
        let test_cases = vec![
            // Balanced delimiters
            ("s{foo}{bar}g", "foo", "bar", "g"),
            ("s[old][new]", "old", "new", ""),
            ("s(pattern)(replacement)i", "pattern", "replacement", "i"),
            ("s<before><after>", "before", "after", ""),
            
            // Mixed delimiters
            ("tr[a-z][A-Z]", "a-z", "A-Z", ""),
            ("m{\\w+}", "\\w+", ""),
            
            // Nested balanced delimiters
            ("s{a{b}c}{x{y}z}", "a{b}c", "x{y}z", ""),
            ("s[a[b]c][x[y]z]", "a[b]c", "x[y]z", ""),
        ];

        for (input, expected_first, expected_second, expected_flags) in test_cases {
            let mut lexer = ContextSensitiveLexer::new(input);
            let tokens = lexer.tokenize();
            
            // Check that we found the expected parts
            let token_strings: Vec<String> = tokens.iter().map(|t| format!("{:?}", t.token_type)).collect();
            let all_tokens = token_strings.join(", ");
            
            assert!(all_tokens.contains(expected_first), 
                "Expected '{}' in tokens for {}: {}", expected_first, input, all_tokens);
            assert!(all_tokens.contains(expected_second), 
                "Expected '{}' in tokens for {}: {}", expected_second, input, all_tokens);
            if !expected_flags.is_empty() {
                assert!(all_tokens.contains(expected_flags), 
                    "Expected flags '{}' in tokens for {}: {}", expected_flags, input, all_tokens);
            }
        }
    }

    #[test]
    fn test_edge_cases() {
        // Empty patterns
        assert!(parse_operator("s///").is_ok());
        assert!(parse_operator("tr///").is_ok());
        assert!(parse_operator("m//").is_ok());
        
        // Special characters
        assert!(parse_operator("s/\\//\\\\//").is_ok()); // s/\//\\/
        assert!(parse_operator("s/\\s+/ /g").is_ok());
        assert!(parse_operator("tr/\\n\\t/ /").is_ok());
    }
    
    fn parse_operator(input: &str) -> Result<Vec<(ContextToken, String)>, String> {
        let mut lexer = ContextSensitiveLexer::new(input);
        let tokens = lexer.tokenize();
        if tokens.is_empty() {
            Err("No tokens found".to_string())
        } else {
            Ok(tokens.into_iter().map(|t| (t.token_type, t.value)).collect())
        }
    }
}
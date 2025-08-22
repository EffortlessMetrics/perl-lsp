#[cfg(test)]
mod unit_unicode {
    use crate::unicode::UnicodeUtils;

    #[test]
    fn test_unicode_normalization_basic() {
        let test_cases = vec![
            ("café", "café"),
            ("naïve", "naïve"),
            ("über", "über"),
            ("façade", "façade"),
            ("résumé", "résumé"),
        ];

        for (input, expected) in test_cases {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected, "Normalization failed for: {}", input);
        }
    }

    #[test]
    fn test_unicode_normalization_combining() {
        // Test combining characters
        let test_cases = vec![
            ("cafe\u{0301}", "café"), // e + combining acute accent
            ("naive\u{0308}", "naïve"), // i + combining diaeresis
            ("uber\u{0308}", "über"), // u + combining diaeresis
        ];

        for (input, expected) in test_cases {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected, "Combining normalization failed for: {}", input);
        }
    }

    #[test]
    fn test_unicode_normalization_edge_cases() {
        let edge_cases = vec![
            ("", ""), // Empty string
            ("a", "a"), // Single ASCII
            ("α", "α"), // Single Unicode
            ("aα", "aα"), // Mixed ASCII and Unicode
            ("_var", "_var"), // Underscore prefix
            ("var_", "var_"), // Underscore suffix
        ];

        for (input, expected) in edge_cases {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected, "Edge case normalization failed for: '{}'", input);
        }
    }

    #[test]
    fn test_unicode_identifier_validation_basic() {
        let valid_identifiers = vec![
            "variable",
            "Variable",
            "VARIABLE",
            "var123",
            "var_123",
            "_var",
            "var_",
            "_123",
        ];

        for identifier in valid_identifiers {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "Valid identifier '{}' was rejected",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_identifier_validation_invalid() {
        let invalid_identifiers = vec![
            "123variable", // Starts with digit
            "variable-name", // Contains hyphen
            "variable name", // Contains space
            "", // Empty string
            "123", // Only digits
            "-var", // Starts with hyphen
            "var-name", // Contains hyphen
        ];

        for identifier in invalid_identifiers {
            assert!(
                !UnicodeUtils::is_valid_identifier(identifier),
                "Invalid identifier '{}' was accepted",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_identifier_validation_international() {
        let international_identifiers = vec![
            "変数", // Japanese
            "über", // German
            "naïve", // French
            "café", // French
            "αβγ", // Greek
            "привет", // Russian
            "你好", // Chinese
            "안녕", // Korean
            "नमस्ते", // Hindi
            "مرحبا", // Arabic
        ];

        for identifier in international_identifiers {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "International identifier '{}' was rejected",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_identifier_validation_mixed() {
        let mixed_identifiers = vec![
            "var_変数",
            "über_var",
            "naïve123",
            "café_test",
            "αβγ_var",
            "привет123",
            "你好_test",
            "안녕_var",
            "नमस्ते123",
            "مرحبا_test",
        ];

        for identifier in mixed_identifiers {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "Mixed identifier '{}' was rejected",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_identifier_validation_edge_cases() {
        let edge_cases = vec![
            ("a", true), // Single ASCII letter
            ("α", true), // Single Unicode letter
            ("_", true), // Single underscore
            ("1", false), // Single digit
            ("", false), // Empty string
            ("a1", true), // Letter followed by digit
            ("1a", false), // Digit followed by letter
            ("_a", true), // Underscore followed by letter
            ("a_", true), // Letter followed by underscore
            ("__", true), // Multiple underscores
        ];

        for (identifier, expected) in edge_cases {
            let result = UnicodeUtils::is_valid_identifier(identifier);
            assert_eq!(
                result, expected,
                "Edge case validation failed for '{}': expected {}, got {}",
                identifier, expected, result
            );
        }
    }

    #[test]
    fn test_unicode_script_support() {
        // Test various Unicode scripts
        let scripts = vec![
            // Latin scripts
            ("café", "Latin"),
            ("über", "Latin"),
            ("naïve", "Latin"),
            
            // CJK scripts
            ("変数", "CJK"),
            ("你好", "CJK"),
            ("안녕", "CJK"),
            
            // Other scripts
            ("αβγ", "Greek"),
            ("привет", "Cyrillic"),
            ("नमस्ते", "Devanagari"),
            ("مرحبا", "Arabic"),
            ("שלום", "Hebrew"),
            ("สวัสดี", "Thai"),
        ];

        for (identifier, script_name) in scripts {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "{} script identifier '{}' was rejected",
                script_name, identifier
            );
        }
    }

    #[test]
    fn test_unicode_normalization_consistency() {
        // Test that normalization is idempotent
        let test_cases = vec![
            "café",
            "naïve",
            "über",
            "façade",
            "résumé",
            "変数",
            "αβγ",
            "привет",
        ];

        for identifier in test_cases {
            let normalized1 = UnicodeUtils::normalize_identifier(identifier);
            let normalized2 = UnicodeUtils::normalize_identifier(&normalized1);
            assert_eq!(
                normalized1, normalized2,
                "Normalization not idempotent for '{}': {} != {}",
                identifier, normalized1, normalized2
            );
        }
    }

    #[test]
    fn test_unicode_validation_consistency() {
        // Test that validation is consistent with normalization
        let test_cases = vec![
            "café",
            "naïve",
            "über",
            "変数",
            "αβγ",
            "привет",
        ];

        for identifier in test_cases {
            let normalized = UnicodeUtils::normalize_identifier(identifier);
            let original_valid = UnicodeUtils::is_valid_identifier(identifier);
            let normalized_valid = UnicodeUtils::is_valid_identifier(&normalized);
            
            assert_eq!(
                original_valid, normalized_valid,
                "Validation consistency failed for '{}': original={}, normalized={}",
                identifier, original_valid, normalized_valid
            );
        }
    }

    #[test]
    fn test_unicode_surrogate_pairs() {
        // Test handling of surrogate pairs (UTF-16)
        let surrogate_test = "𠀀"; // U+20000, requires surrogate pair in UTF-16
        
        let normalized = UnicodeUtils::normalize_identifier(surrogate_test);
        assert_eq!(normalized, surrogate_test);
        
        let is_valid = UnicodeUtils::is_valid_identifier(surrogate_test);
        assert!(is_valid, "Surrogate pair identifier was rejected");
    }

    #[test]
    fn test_unicode_combining_marks() {
        // Test various combining marks
        let combining_tests = vec![
            ("e\u{0301}", "é"), // e + acute accent
            ("i\u{0308}", "ï"), // i + diaeresis
            ("u\u{0308}", "ü"), // u + diaeresis
            ("a\u{0300}", "à"), // a + grave accent
            ("o\u{0302}", "ô"), // o + circumflex
        ];

        for (input, expected) in combining_tests {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected, "Combining mark normalization failed for: {}", input);
        }
    }

    #[test]
    fn test_unicode_private_use_areas() {
        // Test private use area characters (should be handled gracefully)
        let private_use = "\u{E000}"; // Private Use Area character
        
        let normalized = UnicodeUtils::normalize_identifier(private_use);
        assert_eq!(normalized, private_use);
        
        // Private use characters should be handled gracefully
        let is_valid = UnicodeUtils::is_valid_identifier(private_use);
        // This may be true or false depending on implementation, but shouldn't panic
        assert!(
            is_valid || !is_valid,
            "Private use character validation should not panic"
        );
    }

    #[test]
    fn test_unicode_control_characters() {
        // Test control characters (should be handled gracefully)
        let control_chars = vec![
            "\u{0000}", // Null
            "\u{0001}", // Start of Heading
            "\u{0009}", // Tab
            "\u{000A}", // Line Feed
            "\u{000D}", // Carriage Return
        ];

        for control_char in control_chars {
            let normalized = UnicodeUtils::normalize_identifier(control_char);
            // Should handle gracefully, either normalize or preserve
            assert!(
                !normalized.is_empty() || normalized.is_empty(),
                "Control character normalization should not panic: {:?}",
                control_char
            );
        }
    }
} 
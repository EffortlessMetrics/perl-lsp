#[cfg(test)]
mod property_scanner {
    use proptest::prelude::*;
    use crate::scanner::{RustScanner, ScannerConfig, TokenType};
    use crate::unicode::UnicodeUtils;

    proptest! {
        #[test]
        fn test_scanner_does_not_panic_on_arbitrary_bytes(input in any::<Vec<u8>>()) {
            // Scanner should never panic on arbitrary byte sequences
            let mut scanner = RustScanner::new();
            let _result = scanner.scan(&input);
        }

        #[test]
        fn test_scanner_handles_arbitrary_strings(input in "[\\x00-\\xff]*") {
            // Scanner should handle arbitrary strings gracefully
            let mut scanner = RustScanner::new();
            let _result = scanner.scan(input.as_bytes());
        }

        #[test]
        fn test_scanner_position_monotonic(input in "[a-zA-Z0-9_\\s\\{\\}\\(\\)\\[\\]\\\"\\'\\;\\,\\.\\+\\-\\*\\/\\=\\<\\>\\!\\&\\|\\^\\~\\%\\#\\@\\$\\`\\{\\}]+") {
            // Scanner position should be monotonic (never decrease)
            let mut scanner = RustScanner::new();
            let initial_position = scanner.position();
            let _result = scanner.scan(input.as_bytes());
            let final_position = scanner.position();
            assert!(final_position >= initial_position);
        }

        #[test]
        fn test_scanner_serialization_roundtrip(input in "[a-zA-Z0-9_\\s]+") {
            // Scanner state should survive serialization/deserialization
            let mut scanner1 = RustScanner::new();
            let _ = scanner1.scan(input.as_bytes());
            
            let mut buffer = Vec::new();
            let serialize_result = scanner1.serialize(&mut buffer);
            prop_assume!(serialize_result.is_ok());
            
            let mut scanner2 = RustScanner::new();
            let deserialize_result = scanner2.deserialize(&buffer);
            prop_assume!(deserialize_result.is_ok());
            
            // Both scanners should be in equivalent states
            assert_eq!(scanner1.position(), scanner2.position());
        }

        #[test]
        fn test_unicode_normalization_idempotent(input in "[a-zA-Z0-9_\\u{00C0}-\\u{017F}\\u{0100}-\\u{024F}\\u{1E00}-\\u{1EFF}\\u{2C60}-\\u{2C7F}\\u{A720}-\\u{A7FF}]+") {
            // Unicode normalization should be idempotent
            let normalized1 = UnicodeUtils::normalize_identifier(&input);
            let normalized2 = UnicodeUtils::normalize_identifier(&normalized1);
            assert_eq!(normalized1, normalized2);
        }

        #[test]
        fn test_unicode_validation_consistency(input in "[a-zA-Z_][a-zA-Z0-9_]*") {
            // Valid identifiers should remain valid after normalization
            let is_valid = UnicodeUtils::is_valid_identifier(&input);
            prop_assume!(is_valid);
            
            let normalized = UnicodeUtils::normalize_identifier(&input);
            let normalized_valid = UnicodeUtils::is_valid_identifier(&normalized);
            assert!(normalized_valid);
        }

        #[test]
        fn test_scanner_config_validation(
            strict_mode in any::<bool>(),
            unicode_normalization in any::<bool>(),
            max_token_length in 1..=10000u32,
            debug in any::<bool>()
        ) {
            // Scanner config should be valid for all boolean combinations
            let config = ScannerConfig {
                strict_mode,
                unicode_normalization,
                max_token_length,
                debug,
            };
            
            let scanner = RustScanner::with_config(config);
            assert_eq!(scanner.config().strict_mode, strict_mode);
            assert_eq!(scanner.config().unicode_normalization, unicode_normalization);
            assert_eq!(scanner.config().max_token_length, max_token_length);
            assert_eq!(scanner.config().debug, debug);
        }

        #[test]
        fn test_scanner_token_type_consistency(input in "[a-zA-Z0-9_\\s\\{\\}\\(\\)\\[\\]\\\"\\'\\;\\,\\.\\+\\-\\*\\/\\=\\<\\>\\!\\&\\|\\^\\~\\%\\#\\@\\$\\`\\{\\}]+") {
            // Scanner should return consistent token types for same input
            let mut scanner1 = RustScanner::new();
            let mut scanner2 = RustScanner::new();
            
            let result1 = scanner1.scan(input.as_bytes());
            let result2 = scanner2.scan(input.as_bytes());
            
            prop_assume!(result1.is_ok() && result2.is_ok());
            
            if let (Ok(Some(token1)), Ok(Some(token2))) = (result1, result2) {
                assert_eq!(token1, token2);
            }
        }

        #[test]
        fn test_scanner_empty_input_handling(input in "") {
            // Scanner should handle empty input gracefully
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
        }

        #[test]
        fn test_scanner_whitespace_handling(input in "[\\s\\t\\n\\r]+") {
            // Scanner should handle whitespace consistently
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Whitespace as u16);
            }
        }

        #[test]
        fn test_scanner_identifier_handling(input in "[a-zA-Z_][a-zA-Z0-9_]*") {
            // Scanner should handle valid identifiers
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Identifier as u16);
            }
        }

        #[test]
        fn test_scanner_number_handling(input in "[0-9]+(\\.[0-9]+)?([eE][+-]?[0-9]+)?") {
            // Scanner should handle numeric literals
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::NumberLiteral as u16);
            }
        }

        #[test]
        fn test_scanner_string_handling(input in "'[^']*'|\"[^\"]*\"") {
            // Scanner should handle string literals
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::StringLiteral as u16);
            }
        }

        #[test]
        fn test_scanner_operator_handling(input in "[+\\-*/=<>!&|^~%#@$`]+") {
            // Scanner should handle operators
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Operator as u16);
            }
        }

        #[test]
        fn test_scanner_comment_handling(input in "#[^\\n]*") {
            // Scanner should handle comments
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Comment as u16);
            }
        }

        #[test]
        fn test_scanner_keyword_handling(input in "(my|our|local|sub|if|unless|while|until|for|foreach|return|print|say|die|warn|defined|undef)") {
            // Scanner should handle keywords
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Keyword as u16);
            }
        }

        #[test]
        fn test_scanner_multiple_tokens(input in "[a-zA-Z_][a-zA-Z0-9_]*\\s+[0-9]+") {
            // Scanner should handle multiple tokens in sequence
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
        }

        #[test]
        fn test_scanner_unicode_identifier_handling(input in "[\\u{00C0}-\\u{017F}\\u{0100}-\\u{024F}\\u{1E00}-\\u{1EFF}\\u{2C60}-\\u{2C7F}\\u{A720}-\\u{A7FF}]+") {
            // Scanner should handle Unicode identifiers
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Identifier as u16);
            }
        }

        #[test]
        fn test_scanner_mixed_ascii_unicode_handling(input in "[a-zA-Z_][a-zA-Z0-9_\\u{00C0}-\\u{017F}]*") {
            // Scanner should handle mixed ASCII/Unicode identifiers
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Identifier as u16);
            }
        }

        #[test]
        fn test_scanner_large_input_handling(input in "[a-zA-Z0-9_\\s]{100,1000}") {
            // Scanner should handle large inputs
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input.as_bytes());
            assert!(result.is_ok());
        }

        #[test]
        fn test_scanner_repeated_scanning(input in "[a-zA-Z0-9_\\s]+") {
            // Scanner should work correctly with repeated scanning
            let mut scanner = RustScanner::new();
            
            for _ in 0..10 {
                let result = scanner.scan(input.as_bytes());
                assert!(result.is_ok());
            }
        }

        #[test]
        fn test_scanner_state_preservation(input in "[a-zA-Z0-9_\\s]+") {
            // Scanner should preserve state correctly
            let mut scanner = RustScanner::new();
            let initial_position = scanner.position();
            
            let _ = scanner.scan(input.as_bytes());
            let final_position = scanner.position();
            
            // Position should advance by at least the input length
            assert!(final_position >= initial_position + input.len());
        }
    }
} 
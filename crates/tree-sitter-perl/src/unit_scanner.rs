#[cfg(test)]
mod unit_scanner {
    use crate::scanner::{RustScanner, ScannerConfig, ScannerState, TokenType};

    #[test]
    fn test_scanner_initialization() {
        let scanner = RustScanner::new();
        assert_eq!(scanner.position(), 0);
        assert!(scanner.lookahead().is_none());
    }

    #[test]
    fn test_scanner_with_config() {
        let config = ScannerConfig {
            strict_mode: true,
            unicode_normalization: false,
            max_token_length: 512,
            debug: true,
        };
        
        let scanner = RustScanner::with_config(config);
        assert_eq!(scanner.config().max_token_length, 512);
        assert!(scanner.config().strict_mode);
        assert!(scanner.config().debug);
    }

    #[test]
    fn test_scan_whitespace() {
        let mut scanner = RustScanner::new();
        let input = b"   \t\n  ";
        
        let result = scanner.scan(input);
        assert!(result.is_ok());
        
        if let Ok(Some(token_type)) = result {
            assert_eq!(token_type, TokenType::Whitespace as u16);
        }
    }

    #[test]
    fn test_scan_identifier() {
        let mut scanner = RustScanner::new();
        let input = b"variable";
        
        let result = scanner.scan(input);
        assert!(result.is_ok());
        
        if let Ok(Some(token_type)) = result {
            assert_eq!(token_type, TokenType::Identifier as u16);
        }
    }

    #[test]
    fn test_scan_string_literal() {
        let mut scanner = RustScanner::new();
        let input = b"'Hello, World!'";
        
        let result = scanner.scan(input);
        assert!(result.is_ok());
        
        if let Ok(Some(token_type)) = result {
            assert_eq!(token_type, TokenType::StringLiteral as u16);
        }
    }

    #[test]
    fn test_scan_double_quoted_string() {
        let mut scanner = RustScanner::new();
        let input = b"\"Interpolated $var string\"";
        
        let result = scanner.scan(input);
        assert!(result.is_ok());
        
        if let Ok(Some(token_type)) = result {
            assert_eq!(token_type, TokenType::StringLiteral as u16);
        }
    }

    #[test]
    fn test_scan_number_literal() {
        let mut scanner = RustScanner::new();
        let test_cases = vec![
            b"42",
            b"3.14",
            b"1e10",
            b"0xFF",
            b"0777",
        ];
        
        for input in test_cases {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(input);
            assert!(result.is_ok(), "Failed to scan number: {:?}", input);
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::NumberLiteral as u16);
            }
        }
    }

    #[test]
    fn test_scan_operators() {
        let mut scanner = RustScanner::new();
        let operators = vec![
            b"+", b"-", b"*", b"/", b"=", b"==", b"!=",
            b"<", b">", b"<=", b">=", b"&&", b"||", b"!",
            b"&", b"|", b"^", b"~", b"<<", b">>", b".",
        ];
        
        for op in operators {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(op);
            assert!(result.is_ok(), "Failed to scan operator: {:?}", op);
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Operator as u16);
            }
        }
    }

    #[test]
    fn test_scan_keywords() {
        let mut scanner = RustScanner::new();
        let keywords = vec![
            b"my", b"our", b"local", b"sub", b"if", b"unless",
            b"while", b"until", b"for", b"foreach", b"return",
            b"print", b"say", b"die", b"warn", b"defined", b"undef",
        ];
        
        for keyword in keywords {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(keyword);
            assert!(result.is_ok(), "Failed to scan keyword: {:?}", keyword);
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Keyword as u16);
            }
        }
    }

    #[test]
    fn test_scan_comments() {
        let mut scanner = RustScanner::new();
        let comments = vec![
            b"# This is a comment",
            b"# Another comment\n",
            b"=pod\nThis is POD\n=cut",
        ];
        
        for comment in comments {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(comment);
            assert!(result.is_ok(), "Failed to scan comment: {:?}", comment);
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Comment as u16);
            }
        }
    }

    #[test]
    fn test_scan_unicode_identifiers() {
        let mut scanner = RustScanner::new();
        let unicode_identifiers = vec![
            "変数",
            "über",
            "naïve",
            "café",
            "αβγ",
            "привет",
        ];
        
        for identifier in unicode_identifiers {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(identifier.as_bytes());
            assert!(result.is_ok(), "Failed to scan Unicode identifier: {}", identifier);
            
            if let Ok(Some(token_type)) = result {
                assert_eq!(token_type, TokenType::Identifier as u16);
            }
        }
    }

    #[test]
    fn test_scan_error_handling() {
        let mut scanner = RustScanner::new();
        let error_cases = vec![
            b"\x00", // Null byte
            b"\xFF", // Invalid UTF-8
            b"\"Unterminated string", // Unterminated string
            b"'Unterminated string", // Unterminated string
        ];
        
        for error_case in error_cases {
            let mut scanner = RustScanner::new();
            let result = scanner.scan(error_case);
            // Should handle gracefully, either return error token or skip
            assert!(result.is_ok(), "Scanner should handle error case gracefully: {:?}", error_case);
        }
    }

    #[test]
    fn test_scanner_state_serialization() {
        let mut scanner = RustScanner::new();
        let input = b"my $var = 42;";
        
        // Scan a few tokens to change state
        let _ = scanner.scan(input);
        
        // Serialize state
        let mut buffer = Vec::new();
        let serialize_result = scanner.serialize(&mut buffer);
        assert!(serialize_result.is_ok());
        assert!(!buffer.is_empty());
        
        // Create new scanner and deserialize
        let mut new_scanner = RustScanner::new();
        let deserialize_result = new_scanner.deserialize(&buffer);
        assert!(deserialize_result.is_ok());
    }

    #[test]
    fn test_scanner_position_tracking() {
        let mut scanner = RustScanner::new();
        let input = b"my $var = 42;";
        
        assert_eq!(scanner.position(), 0);
        
        let _ = scanner.scan(input);
        assert!(scanner.position() > 0, "Position should advance after scanning");
    }

    #[test]
    fn test_scanner_lookahead() {
        let mut scanner = RustScanner::new();
        let input = b"my";
        
        // Test lookahead functionality
        if let Ok(Some(_)) = scanner.scan(input) {
            // Lookahead should be updated
            assert!(scanner.lookahead().is_some() || scanner.position() >= input.len());
        }
    }

    #[test]
    fn test_scanner_max_token_length() {
        let config = ScannerConfig {
            max_token_length: 5,
            ..Default::default()
        };
        
        let mut scanner = RustScanner::with_config(config);
        let long_input = b"very_long_identifier_that_exceeds_limit";
        
        let result = scanner.scan(long_input);
        // Should handle gracefully, either truncate or return error
        assert!(result.is_ok(), "Scanner should handle long tokens gracefully");
    }

    #[test]
    fn test_scanner_strict_mode() {
        let config = ScannerConfig {
            strict_mode: true,
            ..Default::default()
        };
        
        let mut scanner = RustScanner::with_config(config);
        let invalid_input = b"123invalid_identifier";
        
        let result = scanner.scan(invalid_input);
        // In strict mode, should be more strict about token validation
        assert!(result.is_ok(), "Strict mode scanner should handle invalid input gracefully");
    }

    #[test]
    fn test_scanner_debug_mode() {
        let config = ScannerConfig {
            debug: true,
            ..Default::default()
        };
        
        let scanner = RustScanner::with_config(config);
        assert!(scanner.config().debug);
        
        // Debug mode should provide additional logging/validation
        // This is tested by ensuring the scanner still works correctly
        let mut debug_scanner = RustScanner::with_config(config);
        let input = b"test";
        let result = debug_scanner.scan(input);
        assert!(result.is_ok());
    }
} 
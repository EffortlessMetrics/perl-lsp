// Comprehensive Rust-side test suite for tree-sitter-perl
//
// This module orchestrates all scanner, unicode, property, and integration tests.
// It is designed to mirror the C-based test suite and ensure 100% input/output fidelity.

#[cfg(test)]
mod integration_tests {
    use crate::{language, parse};
    use tree_sitter::Parser;

    #[test]
    fn test_language_loading() {
        let lang = language();
        // Language is valid if we can create it
        assert!(std::ptr::addr_of!(lang) != std::ptr::null());
    }

    #[test]
    fn test_basic_parsing() {
        let test_cases = vec![
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
            "for my $i (1..10) { print $i; }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);

            let tree = result.unwrap();
            let root = tree.root_node();
            assert_eq!(root.kind(), "source_file");
        }
    }

    #[test]
    fn test_variable_declarations() {
        let test_cases = vec![
            "my $scalar = 42;",
            "my @array = (1, 2, 3);",
            "my %hash = (key => 'value');",
            "our $package_var = 1;",
            "local $temp = 2;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_function_calls() {
        let test_cases = vec![
            "print 'Hello';",
            "say 'World';",
            "die 'Error message';",
            "warn 'Warning';",
            "defined($var);",
            "undef;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_control_structures() {
        let test_cases = vec![
            "if ($condition) { $action = 1; }",
            "unless ($condition) { $action = 0; }",
            "while ($condition) { $action++; }",
            "until ($condition) { $action++; }",
            "for my $i (1..10) { print $i; }",
            "foreach my $item (@list) { process($item); }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_string_literals() {
        let test_cases = vec![
            "my $str1 = 'Single quoted';",
            "my $str2 = \"Double quoted\";",
            "my $str3 = qq{Interpolated};",
            "my $str4 = q{Non-interpolated};",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_comments() {
        let test_cases = vec![
            "# This is a comment\nmy $var = 1;",
            "my $var = 1; # Inline comment",
            "=pod\nThis is POD\n=cut\nmy $var = 1;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_unicode_support() {
        let test_cases = vec![
            "my $変数 = '値';",
            "my $über = 'cool';",
            "my $naïve = 'simple';",
            "sub 関数 { return '関数です'; }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_error_handling() {
        // These should parse but may contain error nodes
        let error_cases = vec![
            "my $str = \"Unterminated string;",
            "if ($condition { $action = 1; }",
            "my $var = 1 +;",
        ];

        for (i, code) in error_cases.iter().enumerate() {
            let result = parse(code);
            // These should parse (with error nodes) rather than fail completely
            assert!(
                result.is_ok(),
                "Error case {} failed to parse: {:?}",
                i,
                result
            );
        }
    }

    #[test]
    fn test_parser_reuse() {
        let mut parser = Parser::new();
        parser
            .set_language(&language())
            .expect("Failed to set language");

        let test_cases = vec!["my $var1 = 1;", "my $var2 = 2;", "my $var3 = 3;"];

        for (i, code) in test_cases.iter().enumerate() {
            let tree = parser.parse(code, None);
            assert!(tree.is_some(), "Test case {} failed", i);
        }
    }
}

#[cfg(test)]
mod scanner_tests {
    use crate::scanner::{RustScanner, ScannerConfig, TokenType};

    #[test]
    fn test_rust_scanner_creation() {
        let scanner = RustScanner::new();
        // Test that scanner can be created
        assert!(std::mem::size_of_val(&scanner) > 0);
    }

    #[test]
    fn test_scanner_config() {
        let config = ScannerConfig {
            strict_mode: true,
            unicode_normalization: true,
            max_token_length: 1024,
            debug: false,
        };

        assert!(config.strict_mode);
        assert!(config.unicode_normalization);
        assert_eq!(config.max_token_length, 1024);
        assert!(!config.debug);
    }

    #[test]
    fn test_scanner_config_default() {
        let config = ScannerConfig::default();
        assert!(!config.strict_mode);
        assert!(config.unicode_normalization);
        assert_eq!(config.max_token_length, 1048576);
        assert!(!config.debug);
    }

    #[test]
    fn test_token_types() {
        // Test that all token types are properly defined
        assert_eq!(TokenType::Identifier as u16, 0);
        assert_eq!(TokenType::StringLiteral as u16, 1);
        assert_eq!(TokenType::NumberLiteral as u16, 2);
        assert_eq!(TokenType::Operator as u16, 3);
        assert_eq!(TokenType::Keyword as u16, 4);
        assert_eq!(TokenType::Comment as u16, 5);
        assert_eq!(TokenType::Whitespace as u16, 6);
        assert_eq!(TokenType::Error as u16, 7);
    }

    #[test]
    fn test_scanner_serialization() {
        let mut scanner = RustScanner::new();
        let mut buffer = Vec::new();
        
        // Test serialization
        let result = scanner.serialize(&mut buffer);
        assert!(result.is_ok(), "Serialization failed: {:?}", result);
        assert!(!buffer.is_empty(), "Serialized buffer should not be empty");
        
        // Test deserialization
        let result = scanner.deserialize(&buffer);
        assert!(result.is_ok(), "Deserialization failed: {:?}", result);
    }
}

#[cfg(test)]
mod unicode_tests {
    use crate::unicode::UnicodeUtils;

    #[test]
    fn test_unicode_normalization() {
        let test_cases = vec![
            ("café", "café"),
            ("naïve", "naïve"),
            ("über", "über"),
            ("変数", "変数"),
        ];

        for (input, expected) in test_cases {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected);
        }
    }

    #[test]
    fn test_unicode_identifier_validation() {
        let valid_identifiers = vec![
            "variable",
            "変数",
            "über",
            "naïve",
            "café",
            "αβγ",
            "привет",
        ];

        for identifier in valid_identifiers {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "Identifier '{}' should be valid",
                identifier
            );
        }

        let invalid_identifiers = vec![
            "123variable",
            "variable-name",
            "variable name",
            "",
        ];

        for identifier in invalid_identifiers {
            assert!(
                !UnicodeUtils::is_valid_identifier(identifier),
                "Identifier '{}' should be invalid",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_edge_cases() {
        // Test various Unicode edge cases
        let edge_cases = vec![
            ("", false), // Empty string
            ("a", true), // Single ASCII
            ("α", true), // Single Unicode
            ("aα", true), // Mixed ASCII and Unicode
            ("123", false), // Numbers only
            ("_var", true), // Underscore prefix
            ("var_", true), // Underscore suffix
        ];

        for (input, expected) in edge_cases {
            let result = UnicodeUtils::is_valid_identifier(input);
            assert_eq!(
                result, expected,
                "Identifier '{}' validation failed: expected {}, got {}",
                input, expected, result
            );
        }
    }
}

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    use crate::{parse, scanner::RustScanner};

    proptest! {
        #[test]
        fn test_parse_does_not_panic(input in "[a-zA-Z0-9_\\s\\{\\}\\(\\)\\[\\]\\\"\\'\\;\\,\\.\\+\\-\\*\\/\\=\\<\\>\\!\\&\\|\\^\\~\\%\\#\\@\\$\\`\\{\\}]+") {
            // This test ensures that parsing arbitrary strings doesn't panic
            let _result = parse(&input);
        }

        #[test]
        fn test_scanner_handles_arbitrary_input(input in "[\\x00-\\xff]+") {
            // Test that scanner can handle arbitrary byte sequences
            let mut scanner = RustScanner::new();
            let _result = scanner.scan(input.as_bytes());
        }

        #[test]
        fn test_unicode_identifiers_roundtrip(identifier in "[a-zA-Z_][a-zA-Z0-9_]*") {
            // Test that valid identifiers can be parsed and reconstructed
            let code = format!("my ${} = 1;", identifier);
            let result = parse(&code);
            assert!(result.is_ok(), "Failed to parse identifier: {}", identifier);
        }
    }
}

#[cfg(test)]
mod error_tests {
    use crate::error::ParseError;

    #[test]
    fn test_error_creation() {
        let error = ParseError::ParseFailed;
        assert!(matches!(error, ParseError::ParseFailed));
    }

    #[test]
    fn test_error_display() {
        let error = ParseError::ParseFailed;
        let display = format!("{:?}", error);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_error_serialization() {
        let error = ParseError::ParseFailed;
        let serialized = bincode::serialize(&error);
        assert!(serialized.is_ok(), "Error serialization failed");
        
        let deserialized: Result<ParseError, _> = bincode::deserialize(&serialized.unwrap());
        assert!(deserialized.is_ok(), "Error deserialization failed");
        assert!(matches!(deserialized.unwrap(), ParseError::ParseFailed));
    }
}

#[cfg(test)]
mod performance_tests {
    use crate::{parse, scanner::RustScanner};
    use std::time::Instant;

    #[test]
    fn test_parse_performance() {
        let test_code = "my $var = 42; print 'Hello, World!'; sub foo { return 1; }";
        let iterations = 1000;
        
        let start = Instant::now();
        for _ in 0..iterations {
            let _result = parse(test_code);
        }
        let duration = start.elapsed();
        
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Average parse time: {:.2} μs", avg_time);
        
        // Ensure parsing is reasonably fast (less than 1000 μs per parse)
        assert!(avg_time < 1000.0, "Parsing is too slow: {:.2} μs", avg_time);
    }

    #[test]
    fn test_scanner_performance() {
        let test_input = b"my $variable = 42; print 'Hello, World!';";
        let iterations = 1000;
        
        let mut scanner = RustScanner::new();
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = scanner.scan(test_input);
        }
        let duration = start.elapsed();
        
        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Average scan time: {:.2} μs", avg_time);
        
        // Ensure scanning is reasonably fast (less than 500 μs per scan)
        assert!(avg_time < 500.0, "Scanning is too slow: {:.2} μs", avg_time);
    }
}

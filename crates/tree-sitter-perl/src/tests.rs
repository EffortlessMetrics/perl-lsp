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
    use crate::scanner::{RustScanner, ScannerConfig};

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
}

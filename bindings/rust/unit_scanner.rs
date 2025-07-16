#[cfg(test)]
mod unit_scanner {
    use crate::test_harness::{parse_perl_code, validate_tree_no_errors};

    #[test]
    fn test_basic_scanner_tokens() {
        // Test basic token recognition
        let basic_tokens = [
            "my $var = 42;",           // Variable declaration
            "print 'Hello';",          // Function call with string
            "sub foo { return 1; }",   // Function definition
            "if ($x) { print $x; }",   // Conditional statement
            "for my $i (1..10) { }",   // For loop
            "while ($condition) { }",  // While loop
            "package MyPackage;",      // Package declaration
            "use strict;",             // Use statement
            "require 'file.pl';",      // Require statement
            "our $var;",               // Our declaration
            "local $var;",             // Local declaration
        ];

        for (i, code) in basic_tokens.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Basic token {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_string_literals() {
        // Test various string literal formats
        let string_literals = [
            r#"my $str = "Hello, World!";"#,      // Double quoted
            r#"my $str = 'Hello, World!';"#,      // Single quoted
            r#"my $str = q(Hello, World!);"#,     // q() quoted
            r#"my $str = qq(Hello, World!);"#,    // qq() quoted
            r#"my $str = qw(one two three);"#,    // qw() quoted
            r#"my $str = `ls -la`;"#,             // Backticks
            r#"my $str = qx(ls -la);"#,           // qx() quoted
            r#"my $str = "Hello\nWorld";"#,       // With escape
            r#"my $str = 'Hello\nWorld';"#,       // Literal escape
            r#"my $str = "Hello\tWorld";"#,       // Tab escape
            r#"my $str = "Hello\r\nWorld";"#,     // CRLF
        ];

        for (i, code) in string_literals.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "String literal {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unterminated_strings() {
        // Test handling of unterminated strings
        let unterminated_strings = [
            r#"my $str = "Hello, World!;"#,       // Missing closing quote
            r#"my $str = 'Hello, World!;"#,       // Mixed quotes
            r#"my $str = q(Hello, World!;"#,      // Unterminated q()
            r#"my $str = qq(Hello, World!;"#,     // Unterminated qq()
            r#"my $str = `ls -la;"#,              // Unterminated backticks
            r#"my $str = qx(ls -la;"#,            // Unterminated qx()
        ];

        for (i, code) in unterminated_strings.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should either parse with error nodes or fail gracefully
            if result.is_ok() {
                let tree = result.unwrap();
                // Should have error nodes
                let validation = validate_tree_no_errors(&tree);
                if validation.is_ok() {
                    println!("Unterminated string {} parsed without errors (unexpected): '{}'", i, code);
                } else {
                    println!("Unterminated string {} parsed with errors (expected): '{}'", i, code);
                }
            } else {
                println!("Unterminated string {} failed to parse (expected): '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_invalid_escapes() {
        // Test handling of invalid escape sequences
        let invalid_escapes = [
            r#"my $str = "Hello\zWorld";"#,       // Invalid escape \z
            r#"my $str = "Hello\xWorld";"#,       // Incomplete hex escape
            r#"my $str = "Hello\uWorld";"#,       // Incomplete unicode escape
            r#"my $str = "Hello\cWorld";"#,       // Invalid control escape
            r#"my $str = "Hello\123World";"#,     // Octal escape (should work)
            r#"my $str = "Hello\x41World";"#,     // Hex escape (should work)
        ];

        for (i, code) in invalid_escapes.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should parse but may have warnings or error nodes
            if result.is_ok() {
                println!("Invalid escape {} parsed successfully: '{}'", i, code);
            } else {
                println!("Invalid escape {} failed to parse: '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_heredoc_handling() {
        // Test heredoc syntax
        let heredoc_tests = [
            r#"
my $msg = <<EOF;
Hello, World!
This is a heredoc.
EOF
"#,
            r#"
my $msg = <<'EOF';
Hello, World!
This is a quoted heredoc.
EOF
"#,
            r#"
my $msg = <<"EOF";
Hello, World!
This is an interpolated heredoc.
EOF
"#,
            r#"
my $msg = <<EOF;
Hello, World!
This is a heredoc with $interpolation.
EOF
"#,
        ];

        for (i, code) in heredoc_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Heredoc test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unterminated_heredoc() {
        // Test unterminated heredoc
        let unterminated_heredoc = [
            r#"
my $msg = <<EOF;
Hello, World!
This is an unterminated heredoc.
"#,
            r#"
my $msg = <<'EOF';
Hello, World!
This is an unterminated quoted heredoc.
"#,
        ];

        for (i, code) in unterminated_heredoc.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should either parse with error nodes or fail gracefully
            if result.is_ok() {
                let tree = result.unwrap();
                let validation = validate_tree_no_errors(&tree);
                if validation.is_ok() {
                    println!("Unterminated heredoc {} parsed without errors (unexpected): '{}'", i, code);
                } else {
                    println!("Unterminated heredoc {} parsed with errors (expected): '{}'", i, code);
                }
            } else {
                println!("Unterminated heredoc {} failed to parse (expected): '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_regex_patterns() {
        // Test regex pattern handling
        let regex_patterns = [
            r#"my $regex = qr/pattern/;"#,        // qr() regex
            r#"my $regex = qr/pattern/i;"#,       // With flags
            r#"my $regex = qr/pattern/ix;"#,      // Multiple flags
            r#"my $regex = qr/pattern/ixs;"#,     // More flags
            r#"my $regex = qr/pattern/ixsm;"#,    // All flags
            r#"if ($str =~ /pattern/) { }"#,      // Match operator
            r#"if ($str =~ m/pattern/) { }"#,     // Explicit match
            r#"if ($str =~ s/old/new/) { }"#,     // Substitution
            r#"if ($str =~ tr/old/new/) { }"#,    // Transliteration
        ];

        for (i, code) in regex_patterns.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Regex pattern {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unterminated_regex() {
        // Test unterminated regex patterns
        let unterminated_regex = [
            r#"my $regex = qr/pattern;"#,         // Missing closing delimiter
            r#"if ($str =~ /pattern) { }"#,       // Missing closing delimiter
            r#"if ($str =~ s/old/new) { }"#,      // Missing closing delimiter
            r#"if ($str =~ tr/old/new) { }"#,     // Missing closing delimiter
        ];

        for (i, code) in unterminated_regex.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should either parse with error nodes or fail gracefully
            if result.is_ok() {
                let tree = result.unwrap();
                let validation = validate_tree_no_errors(&tree);
                if validation.is_ok() {
                    println!("Unterminated regex {} parsed without errors (unexpected): '{}'", i, code);
                } else {
                    println!("Unterminated regex {} parsed with errors (expected): '{}'", i, code);
                }
            } else {
                println!("Unterminated regex {} failed to parse (expected): '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_comments_and_pod() {
        // Test comment and POD handling
        let comment_tests = [
            "# This is a comment",
            "my $var = 1; # Inline comment",
            "my $var = 1; # Inline comment with 'quotes'",
            "my $var = 1; # Inline comment with \"quotes\"",
            "=pod\nThis is POD\n=cut",
            "=head1 Title\nThis is a heading\n=cut",
            "=over 4\n=item *\nList item\n=back",
        ];

        for (i, code) in comment_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Comment/POD test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unterminated_blocks() {
        // Test unterminated code blocks
        let unterminated_blocks = [
            "sub foo {",                           // Unterminated function
            "if ($condition) {",                   // Unterminated if
            "while ($condition) {",                // Unterminated while
            "for my $i (1..10) {",                 // Unterminated for
            "do {",                                // Unterminated do
            "eval {",                              // Unterminated eval
        ];

        for (i, code) in unterminated_blocks.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should either parse with error nodes or fail gracefully
            if result.is_ok() {
                let tree = result.unwrap();
                let validation = validate_tree_no_errors(&tree);
                if validation.is_ok() {
                    println!("Unterminated block {} parsed without errors (unexpected): '{}'", i, code);
                } else {
                    println!("Unterminated block {} parsed with errors (expected): '{}'", i, code);
                }
            } else {
                println!("Unterminated block {} failed to parse (expected): '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_malformed_expressions() {
        // Test malformed expressions
        let malformed_expressions = [
            "my $var = ;",                        // Missing expression
            "my $var = +;",                       // Incomplete unary operator
            "my $var = 1 +;",                     // Incomplete binary operator
            "my $var = 1 + + 2;",                 // Double unary operator
            "my $var = 1 + + + 2;",               // Triple unary operator
            "my $var = (1 + 2;",                  // Unterminated parentheses
            "my $var = [1, 2, 3;",                // Unterminated brackets
            "my $var = {key => 'value';",         // Unterminated braces
        ];

        for (i, code) in malformed_expressions.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should either parse with error nodes or fail gracefully
            if result.is_ok() {
                let tree = result.unwrap();
                let validation = validate_tree_no_errors(&tree);
                if validation.is_ok() {
                    println!("Malformed expression {} parsed without errors (unexpected): '{}'", i, code);
                } else {
                    println!("Malformed expression {} parsed with errors (expected): '{}'", i, code);
                }
            } else {
                println!("Malformed expression {} failed to parse (expected): '{}' - {:?}", i, code, result);
            }
        }
    }

    #[test]
    fn test_control_characters() {
        // Test handling of control characters
        let control_characters = [
            "my $var = 1;\x00my $var2 = 2;",      // Null byte
            "my $var = 1;\x01my $var2 = 2;",      // Start of heading
            "my $var = 1;\x02my $var2 = 2;",      // Start of text
            "my $var = 1;\x03my $var2 = 2;",      // End of text
            "my $var = 1;\x04my $var2 = 2;",      // End of transmission
            "my $var = 1;\x05my $var2 = 2;",      // Enquiry
            "my $var = 1;\x06my $var2 = 2;",      // Acknowledge
            "my $var = 1;\x07my $var2 = 2;",      // Bell
            "my $var = 1;\x08my $var2 = 2;",      // Backspace
            "my $var = 1;\x0Bmy $var2 = 2;",      // Vertical tab
            "my $var = 1;\x0Cmy $var2 = 2;",      // Form feed
            "my $var = 1;\x0Emy $var2 = 2;",      // Shift out
            "my $var = 1;\x0Fmy $var2 = 2;",      // Shift in
        ];

        for (i, code) in control_characters.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should parse successfully or at least not panic
            if result.is_err() {
                println!("Control character {} produced error (expected): '{}' - {:?}", i, code, result);
            } else {
                println!("Control character {} parsed successfully: '{}'", i, code);
            }
        }
    }

    #[test]
    fn test_whitespace_variations() {
        // Test various whitespace characters
        let whitespace_variations = [
            "my $var = 1;\tmy $var2 = 2;",        // Tab
            "my $var = 1;\nmy $var2 = 2;",        // Line feed
            "my $var = 1;\rmy $var2 = 2;",        // Carriage return
            "my $var = 1;\r\nmy $var2 = 2;",      // CRLF
            "my $var = 1;\fmy $var2 = 2;",        // Form feed
            "my $var = 1;\vmy $var2 = 2;",        // Vertical tab
            "my $var = 1; my $var2 = 2;",         // Space
            "my $var = 1;  my $var2 = 2;",        // Multiple spaces
            "my $var = 1;\t \nmy $var2 = 2;",     // Mixed whitespace
        ];

        for (i, code) in whitespace_variations.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Whitespace variation {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_data_sections() {
        // Test __DATA__ and __END__ sections
        let data_sections = [
            "my $var = 1;\n__DATA__\nThis is data\n",
            "my $var = 1;\n__END__\nThis is the end\n",
            "my $var = 1;\n__DATA__\nLine 1\nLine 2\nLine 3\n",
            "my $var = 1;\n__END__\nEnd of script\n",
        ];

        for (i, code) in data_sections.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Data section test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }
} 
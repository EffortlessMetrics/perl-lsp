#[cfg(test)]
mod unit_unicode {
    use crate::test_harness::parse_perl_code;

    #[test]
    fn test_basic_unicode_identifiers() {
        // Test basic Unicode identifier support
        let unicode_identifiers = [
            "my $α = 1;",              // Greek letter
            "my $β = 2;",              // Greek letter
            "my $変数 = 42;",           // Japanese variable name
            "my $über = 'cool';",      // German umlaut
            "my $café = 'coffee';",    // French accent
            "my $naïve = true;",       // French with diaeresis
            "my $résumé = 'cv';",      // French with multiple accents
            "my $piñata = 'party';",   // Spanish with tilde
            "my $mañana = 'tomorrow';", // Spanish with tilde
            "my $schön = 'beautiful';", // German with umlaut
            "my $cœur = 'heart';",     // French with ligature
            "my $naïve = 'naive';",    // French with diaeresis
        ];

        for (i, code) in unicode_identifiers.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode identifier {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_combining_marks() {
        // Test Unicode combining marks and normalization
        let combining_mark_tests = [
            // Base character + combining mark
            "my $e\u{0301} = 'e with acute';",  // e + combining acute
            "my $a\u{0308} = 'a with umlaut';", // a + combining umlaut
            "my $c\u{0327} = 'c with cedilla';", // c + combining cedilla
            "my $n\u{0303} = 'n with tilde';",   // n + combining tilde
            
            // Multiple combining marks
            "my $e\u{0301}\u{0308} = 'e with acute and umlaut';",
            
            // Precomposed vs decomposed
            "my $é = 'precomposed e acute';",    // Precomposed
            "my $e\u{0301} = 'decomposed e acute';", // Decomposed
        ];

        for (i, code) in combining_mark_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Combining mark test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_surrogate_pairs() {
        // Test Unicode surrogate pairs (characters outside BMP)
        let surrogate_pair_tests = [
            // Emoji and other supplementary characters
            "my $emoji = '😀';",           // Grinning face emoji
            "my $heart = '❤️';",           // Heart emoji
            "my $flag = '🇺🇸';",           // Flag emoji (composite)
            "my $math = '𝔄';",             // Mathematical fraktur A
            "my $musical = '𝄞';",         // Musical symbol
            "my $cuneiform = '𒀀';",       // Cuneiform sign
        ];

        for (i, code) in surrogate_pair_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Surrogate pair test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_whitespace() {
        // Test various Unicode whitespace characters
        let unicode_whitespace_tests = [
            "my $var = 1;\u{00A0}my $var2 = 2;",  // Non-breaking space
            "my $var = 1;\u{2000}my $var2 = 2;",  // En quad
            "my $var = 1;\u{2001}my $var2 = 2;",  // Em quad
            "my $var = 1;\u{2002}my $var2 = 2;",  // En space
            "my $var = 1;\u{2003}my $var2 = 2;",  // Em space
            "my $var = 1;\u{2004}my $var2 = 2;",  // Three-per-em space
            "my $var = 1;\u{2005}my $var2 = 2;",  // Four-per-em space
            "my $var = 1;\u{2006}my $var2 = 2;",  // Six-per-em space
            "my $var = 1;\u{2007}my $var2 = 2;",  // Figure space
            "my $var = 1;\u{2008}my $var2 = 2;",  // Punctuation space
            "my $var = 1;\u{2009}my $var2 = 2;",  // Thin space
            "my $var = 1;\u{200A}my $var2 = 2;",  // Hair space
            "my $var = 1;\u{202F}my $var2 = 2;",  // Narrow no-break space
            "my $var = 1;\u{205F}my $var2 = 2;",  // Medium mathematical space
            "my $var = 1;\u{3000}my $var2 = 2;",  // Ideographic space
        ];

        for (i, code) in unicode_whitespace_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode whitespace test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_string_literals() {
        // Test Unicode in string literals
        let unicode_string_tests = [
            r#"my $msg = "Hello, 世界!";"#,        // Japanese in double quotes
            r#"my $msg = 'Hello, 世界!';"#,        // Japanese in single quotes
            r#"my $msg = "Hello, 🌍!";"#,          // Emoji in double quotes
            r#"my $msg = 'Hello, 🌍!';"#,          // Emoji in single quotes
            r#"my $msg = "Café au lait";"#,        // French with accent
            r#"my $msg = 'Café au lait';"#,        // French with accent
            r#"my $msg = "Résumé";"#,              // French with multiple accents
            r#"my $msg = 'Résumé';"#,              // French with multiple accents
            r#"my $msg = "Piñata party";"#,        // Spanish with tilde
            r#"my $msg = 'Piñata party';"#,        // Spanish with tilde
        ];

        for (i, code) in unicode_string_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode string test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_comments() {
        // Test Unicode in comments
        let unicode_comment_tests = [
            "# This is a comment with 日本語",
            "# This is a comment with 🌍 emoji",
            "# This is a comment with café",
            "# This is a comment with résumé",
            "# This is a comment with piñata",
            "my $var = 1; # Inline comment with 日本語",
            "my $var = 1; # Inline comment with 🌍",
            "my $var = 1; # Inline comment with café",
        ];

        for (i, code) in unicode_comment_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode comment test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_function_names() {
        // Test Unicode in function names
        let unicode_function_tests = [
            "sub 関数 { return 1; }",
            "sub 計算 { my $x = shift; return $x * 2; }",
            "sub café { return 'coffee'; }",
            "sub résumé { return 'cv'; }",
            "sub piñata { return 'party'; }",
            "sub こんにちは { return 'hello'; }",
            "sub 🌍 { return 'world'; }",
        ];

        for (i, code) in unicode_function_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode function test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_package_names() {
        // Test Unicode in package names
        let unicode_package_tests = [
            "package 日本語;",
            "package Café;",
            "package Résumé;",
            "package Piñata;",
            "package こんにちは;",
            "package 🌍;",
        ];

        for (i, code) in unicode_package_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode package test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_edge_cases() {
        // Test edge cases and potential issues
        let unicode_edge_cases = [
            // Zero-width characters
            "my $var\u{200B} = 1;",  // Zero-width space
            "my $var\u{FEFF} = 1;",  // Zero-width no-break space (BOM)
            
            // Control characters (should be handled gracefully)
            "my $var = 1;\u{0009}my $var2 = 2;",  // Tab
            "my $var = 1;\u{000A}my $var2 = 2;",  // Line feed
            "my $var = 1;\u{000D}my $var2 = 2;",  // Carriage return
            
            // Right-to-left text
            "my $var = 'שלום';",     // Hebrew
            "my $var = 'مرحبا';",     // Arabic
            
            // Mixed scripts
            "my $var = 'Hello世界';",  // Latin + CJK
            "my $var = 'Hello🌍';",    // Latin + emoji
        ];

        for (i, code) in unicode_edge_cases.iter().enumerate() {
            let result = parse_perl_code(code);
            // These should parse successfully or at least not panic
            if result.is_err() {
                println!("Unicode edge case {} produced error (expected): '{}' - {:?}", i, code, result);
            } else {
                println!("Unicode edge case {} parsed successfully: '{}'", i, code);
            }
        }
    }

    #[test]
    fn test_unicode_normalization() {
        // Test Unicode normalization forms
        let normalization_tests = [
            // NFC vs NFD
            "my $var = 'é';",        // NFC (precomposed)
            "my $var = 'e\u{0301}';", // NFD (decomposed)
            
            // NFKC vs NFKD
            "my $var = 'ﬁ';",        // NFKC (precomposed ligature)
            "my $var = 'fi';",       // NFKD (decomposed ligature)
        ];

        for (i, code) in normalization_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Normalization test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }

    #[test]
    fn test_unicode_heredoc() {
        // Test Unicode in heredoc
        let unicode_heredoc_tests = [
            r#"
my $msg = <<'EOF';
Hello, 世界!
こんにちは
Café au lait
🌍
EOF
"#,
            r#"
my $msg = <<"EOF";
Hello, 世界!
こんにちは
Café au lait
🌍
EOF
"#,
        ];

        for (i, code) in unicode_heredoc_tests.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode heredoc test {} failed: '{}' - {:?}",
                i, code, result
            );
        }
    }
} 
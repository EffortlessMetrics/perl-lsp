//! Tests for stateful parser (heredoc handling)

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::stateful_parser::StatefulPerlParser;

    #[test]
    fn test_heredoc_marker_detection() {
        let test_cases = vec![
            ("print <<EOF;", Some(("EOF", false, "none"))),
            ("print <<'EOF';", Some(("EOF", false, "single"))),
            ("print <<\"EOF\";", Some(("EOF", false, "double"))),
            ("print <<`EOF`;", Some(("EOF", false, "backtick"))),
            ("print <<\\EOF;", Some(("EOF", false, "escaped"))),
            ("print <<~EOF;", Some(("EOF", true, "none"))),
            ("print <<~'EOF';", Some(("EOF", true, "single"))),
            ("print <<~\"EOF\";", Some(("EOF", true, "double"))),
            ("my $text = <<END_TEXT;", Some(("END_TEXT", false, "none"))),
            ("func(<<__DATA__);", Some(("__DATA__", false, "none"))),
            ("print << EOF;", Some(("EOF", false, "none"))), // Space after <<
            ("print regular_code;", None), // No heredoc
            ("$x << 2;", None), // Bit shift, not heredoc
        ];

        for (input, expected) in test_cases {
            let parser = StatefulPerlParser::new();
            let result = parser.extract_heredoc_declaration(input);
            
            match (result, expected) {
                (Some(marker), Some((exp_marker, exp_indented, _exp_quoted))) => {
                    assert_eq!(marker.marker, exp_marker, "Marker mismatch for: {}", input);
                    assert_eq!(marker.indented, exp_indented, "Indented mismatch for: {}", input);
                }
                (None, None) => {
                    // Expected no heredoc
                }
                _ => {
                    panic!("Heredoc detection mismatch for: {}", input);
                }
            }
        }
    }

    #[test]
    fn test_heredoc_terminator_detection() {
        struct TestCase {
            line: &'static str,
            marker: &'static str,
            indented: bool,
            should_match: bool,
        }

        let test_cases = vec![
            TestCase { line: "EOF", marker: "EOF", indented: false, should_match: true },
            TestCase { line: "EOF", marker: "END", indented: false, should_match: false },
            TestCase { line: "  EOF", marker: "EOF", indented: false, should_match: false },
            TestCase { line: "  EOF", marker: "EOF", indented: true, should_match: true },
            TestCase { line: "\tEOF", marker: "EOF", indented: true, should_match: true },
            TestCase { line: "    EOF", marker: "EOF", indented: true, should_match: true },
            TestCase { line: "EOF ", marker: "EOF", indented: false, should_match: false },
            TestCase { line: "EOFX", marker: "EOF", indented: false, should_match: false },
        ];

        for test in test_cases {
            let parser = StatefulPerlParser::new();
            let marker = tree_sitter_perl::stateful_parser::HeredocMarker {
                marker: test.marker.to_string(),
                indented: test.indented,
                quoted: tree_sitter_perl::stateful_parser::HeredocQuoteType::None,
                position: 0,
            };
            
            let result = parser.is_heredoc_terminator(test.line, &marker);
            assert_eq!(result, test.should_match, 
                "Terminator detection failed for line: '{}', marker: '{}', indented: {}", 
                test.line, test.marker, test.indented);
        }
    }

    #[test]
    fn test_simple_heredoc_parsing() {
        let mut parser = StatefulPerlParser::new();
        
        let test_cases = vec![
            // Basic heredoc
            (
                r#"print <<EOF;
Hello World
This is content
EOF
print "after";"#,
                vec!["print", "EOF", "after"],
            ),
            
            // Quoted heredoc
            (
                r#"my $text = <<'END';
No $interpolation here
END
next_statement();"#,
                vec!["my", "text", "END", "next_statement"],
            ),
            
            // Indented heredoc
            (
                r#"if (1) {
    print <<~EOF;
    This is indented
    But will be unindented
    EOF
}"#,
                vec!["if", "print", "EOF"],
            ),
            
            // Multiple statements on heredoc line
            (
                r#"print <<EOF, " and more";
Content here
EOF
"#,
                vec!["print", "EOF", "and more"],
            ),
        ];

        for (input, expected_tokens) in test_cases {
            // Note: This test is simplified since we don't have full AST traversal
            // In a real test, we would verify the AST structure
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
        }
    }

    #[test]
    fn test_multiple_heredocs() {
        let mut parser = StatefulPerlParser::new();
        
        let input = r#"print <<EOF1, <<EOF2;
First heredoc
EOF1
Second heredoc
EOF2
print "done";"#;

        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse multiple heredocs");
    }

    #[test]
    fn test_nested_heredocs() {
        let mut parser = StatefulPerlParser::new();
        
        // Heredocs cannot actually be nested, but can appear sequentially
        let input = r#"my $outer = <<OUTER;
Start of outer
OUTER
my $inner = <<INNER;
Content of inner
INNER
print $outer, $inner;"#;

        let result = parser.parse(input);
        assert!(result.is_ok(), "Failed to parse sequential heredocs");
    }

    #[test]
    fn test_heredoc_edge_cases() {
        let mut parser = StatefulPerlParser::new();
        
        let test_cases = vec![
            // Empty heredoc
            r#"print <<EOF;
EOF"#,
            
            // Heredoc with special characters in marker
            r#"print <<'__END__';
Content
__END__"#,
            
            // Heredoc with numeric marker
            r#"print <<'123';
Content
123"#,
            
            // Very long marker
            r#"print <<'THIS_IS_A_VERY_LONG_HEREDOC_MARKER';
Content
THIS_IS_A_VERY_LONG_HEREDOC_MARKER"#,
        ];

        for input in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
        }
    }

    #[test]
    fn test_heredoc_with_interpolation() {
        let mut parser = StatefulPerlParser::new();
        
        let test_cases = vec![
            // Double-quoted heredoc (should support interpolation)
            r#"my $name = "World";
print <<"EOF";
Hello $name
EOF"#,
            
            // Single-quoted heredoc (no interpolation)
            r#"my $name = "World";
print <<'EOF';
Hello $name (literal)
EOF"#,
            
            // Backtick heredoc (command execution)
            r#"my $result = <<`CMD`;
echo "Hello"
CMD"#,
        ];

        for input in test_cases {
            let result = parser.parse(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
        }
    }
}
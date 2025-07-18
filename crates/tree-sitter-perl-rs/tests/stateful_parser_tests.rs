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
            ("print <<~EOF;", Some(("EOF", true, "none"))),
            ("print <<~'EOF';", Some(("EOF", true, "single"))),
            ("print <<~\"EOF\";", Some(("EOF", true, "double"))),
            ("my $text = <<END;", Some(("END", false, "none"))),
            ("regular print statement;", None),
        ];

        let parser = StatefulPerlParser::new();
        
        for (input, expected) in test_cases {
            let result = parser.extract_heredoc_declaration(input);
            match (result, expected) {
                (Some(marker), Some((exp_marker, exp_indented, exp_quoted))) => {
                    assert_eq!(marker.marker, exp_marker, "Marker mismatch for {}", input);
                    assert_eq!(marker.indented, exp_indented, "Indented flag mismatch for {}", input);
                    let quoted_str = match marker.quoted {
                        tree_sitter_perl::stateful_parser::HeredocQuoteType::None => "none",
                        tree_sitter_perl::stateful_parser::HeredocQuoteType::Single => "single",
                        tree_sitter_perl::stateful_parser::HeredocQuoteType::Double => "double",
                    };
                    assert_eq!(quoted_str, exp_quoted, "Quote type mismatch for {}", input);
                }
                (None, None) => {} // Both none, test passes
                _ => panic!("Mismatch for {}: got {:?}", input, result),
            }
        }
    }

    #[test]
    fn test_heredoc_terminator_recognition() {
        use tree_sitter_perl::stateful_parser::{HeredocMarker, HeredocQuoteType};
        
        let parser = StatefulPerlParser::new();
        
        // Non-indented heredoc
        let marker = HeredocMarker {
            marker: "EOF".to_string(),
            indented: false,
            quoted: HeredocQuoteType::None,
            position: 0,
        };
        
        assert!(parser.is_heredoc_terminator("EOF", &marker));
        assert!(!parser.is_heredoc_terminator("  EOF", &marker));
        assert!(!parser.is_heredoc_terminator("EOF  ", &marker));
        assert!(!parser.is_heredoc_terminator("EOFX", &marker));
        
        // Indented heredoc
        let indented_marker = HeredocMarker {
            marker: "END".to_string(),
            indented: true,
            quoted: HeredocQuoteType::None,
            position: 0,
        };
        
        assert!(parser.is_heredoc_terminator("END", &indented_marker));
        assert!(parser.is_heredoc_terminator("  END", &indented_marker));
        assert!(parser.is_heredoc_terminator("\t\tEND", &indented_marker));
        assert!(!parser.is_heredoc_terminator("END  ", &indented_marker));
        assert!(!parser.is_heredoc_terminator("  END  ", &indented_marker));
    }

    #[test]
    fn test_parse_with_heredoc() {
        let test_cases = vec![
            // Simple heredoc
            vec![
                "print <<EOF;",
                "Hello, world!",
                "This is a heredoc.",
                "EOF",
                "print \"done\";",
            ],
            
            // Indented heredoc
            vec![
                "my $text = <<~END;",
                "    This is indented",
                "    content in heredoc",
                "  END",
                "print $text;",
            ],
            
            // Multiple statements with heredoc
            vec![
                "my $a = 1;",
                "print <<MSG;",
                "Message content",
                "MSG",
                "my $b = 2;",
            ],
        ];

        for lines in test_cases {
            let mut parser = StatefulPerlParser::new();
            let result = parser.parse_lines(lines.clone());
            
            // Check that we have AST nodes
            assert!(!result.ast.is_empty(), "No AST nodes for input: {:?}", lines);
            
            // Check that heredoc content was captured
            let has_heredoc = lines.iter().any(|line| line.contains("<<"));
            if has_heredoc {
                assert!(!result.heredocs.is_empty(), "No heredoc captured for input: {:?}", lines);
            }
        }
    }

    #[test]
    fn test_nested_heredocs() {
        let lines = vec![
            "print <<OUTER;",
            "This is outer heredoc",
            "with <<INNER;",
            "nested content",
            "INNER",
            "back to outer",
            "OUTER",
            "done",
        ];
        
        let mut parser = StatefulPerlParser::new();
        let result = parser.parse_lines(lines);
        
        // Should have captured at least one heredoc
        assert!(!result.heredocs.is_empty());
        
        // The outer heredoc should include the inner heredoc declaration
        let outer_content = &result.heredocs[0].content;
        assert!(outer_content.contains("<<INNER"));
    }

    #[test]
    fn test_quoted_heredoc_markers() {
        use tree_sitter_perl::stateful_parser::HeredocQuoteType;
        
        let test_cases = vec![
            // Single quoted - no interpolation
            (vec![
                "print <<'EOF';",
                "$var is literal",
                "EOF",
            ], HeredocQuoteType::Single),
            
            // Double quoted - with interpolation
            (vec![
                "print <<\"EOF\";",
                "$var is interpolated",
                "EOF",
            ], HeredocQuoteType::Double),
            
            // Unquoted - treated as double quoted
            (vec![
                "print <<EOF;",
                "$var is also interpolated",
                "EOF",
            ], HeredocQuoteType::None),
        ];
        
        for (lines, expected_quote_type) in test_cases {
            let mut parser = StatefulPerlParser::new();
            let result = parser.parse_lines(lines.clone());
            
            assert!(!result.heredocs.is_empty(), "No heredoc found for: {:?}", lines);
            assert_eq!(result.heredocs[0].quote_type, expected_quote_type);
        }
    }

    #[test]
    fn test_heredoc_with_expressions() {
        let lines = vec![
            "my $result = $x + <<EOF;",
            "10",
            "EOF",
            "print $result;",
        ];
        
        let mut parser = StatefulPerlParser::new();
        let result = parser.parse_lines(lines);
        
        assert!(!result.heredocs.is_empty());
        assert_eq!(result.heredocs[0].content.trim(), "10");
    }

    #[test]
    fn test_multiple_heredocs_on_line() {
        let lines = vec![
            "print <<EOF1, <<EOF2;",
            "First heredoc",
            "EOF1",
            "Second heredoc",
            "EOF2",
            "done",
        ];
        
        let mut parser = StatefulPerlParser::new();
        let result = parser.parse_lines(lines);
        
        // Should capture both heredocs
        assert_eq!(result.heredocs.len(), 2);
        assert_eq!(result.heredocs[0].marker, "EOF1");
        assert_eq!(result.heredocs[1].marker, "EOF2");
    }
}
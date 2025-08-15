//! Integration tests for incremental parsing functionality

#[cfg(feature = "incremental")]
mod incremental_tests {
    use perl_parser::incremental_integration::{
        DocumentParser, IncrementalConfig, byte_to_lsp_pos, lsp_pos_to_byte,
    };
    use ropey::Rope;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    #[serial_test::serial]
    fn test_incremental_parsing_small_edit() {
        // Enable incremental parsing
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();
        assert!(config.enabled);

        // Create initial document
        let initial_code = r#"
my $x = 42;
my $y = 100;
print $x + $y;
"#;

        let mut doc = DocumentParser::new(initial_code.to_string(), &config).unwrap();

        // Verify initial AST
        let ast1 = doc.ast().unwrap();
        assert!(format!("{:?}", ast1).contains("Variable"));

        // Apply incremental edit (change 42 to 99)
        let changes = vec![json!({
            "range": {
                "start": {"line": 1, "character": 8},
                "end": {"line": 1, "character": 10}
            },
            "text": "99"
        })];

        let start = Instant::now();
        doc.apply_changes(&changes, &config).unwrap();
        let parse_time = start.elapsed();

        // Verify parse time is reasonable
        assert!(parse_time.as_millis() < 100, "Parse time too slow: {:?}", parse_time);

        // Verify updated AST
        let ast2 = doc.ast().unwrap();
        assert!(doc.content().contains("99"));
        assert!(format!("{:?}", ast2).contains("Variable"));

        // Check metrics if available
        if let Some(metrics) = doc.metrics() {
            println!("Incremental parse metrics: {}", metrics);
        }

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_parsing_multiple_edits() {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = r#"
sub hello {
    print "Hello";
}

sub world {
    print "World";
}
"#;

        let mut doc = DocumentParser::new(initial_code.to_string(), &config).unwrap();

        // Apply multiple edits
        let changes = vec![
            json!({
                "range": {
                    "start": {"line": 2, "character": 11},
                    "end": {"line": 2, "character": 16}
                },
                "text": "Hi"
            }),
            json!({
                "range": {
                    "start": {"line": 6, "character": 11},
                    "end": {"line": 6, "character": 16}
                },
                "text": "Universe"
            }),
        ];

        doc.apply_changes(&changes, &config).unwrap();

        // Verify both changes applied
        assert!(doc.content().contains("Hi"));
        assert!(doc.content().contains("Universe"));
        assert!(!doc.content().contains("Hello"));
        assert!(!doc.content().contains("World"));

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
    }

    #[test]
    fn test_crlf_position_conversion() {
        let text_lf = "Hello\nWorld\n";
        let text_crlf = "Hello\r\nWorld\r\n";

        let rope_lf = Rope::from_str(text_lf);
        let rope_crlf = Rope::from_str(text_crlf);

        // Test LF handling
        assert_eq!(lsp_pos_to_byte(&rope_lf, 0, 0), 0); // Start of first line
        assert_eq!(lsp_pos_to_byte(&rope_lf, 1, 0), 6); // Start of second line
        assert_eq!(lsp_pos_to_byte(&rope_lf, 1, 5), 11); // End of "World"

        // Test CRLF handling
        assert_eq!(lsp_pos_to_byte(&rope_crlf, 0, 0), 0); // Start of first line
        assert_eq!(lsp_pos_to_byte(&rope_crlf, 1, 0), 7); // Start of second line (after \r\n)
        assert_eq!(lsp_pos_to_byte(&rope_crlf, 1, 5), 12); // End of "World"

        // Test reverse conversion
        assert_eq!(byte_to_lsp_pos(&rope_lf, 0), (0, 0));
        assert_eq!(byte_to_lsp_pos(&rope_lf, 6), (1, 0));
        assert_eq!(byte_to_lsp_pos(&rope_crlf, 0), (0, 0));
        assert_eq!(byte_to_lsp_pos(&rope_crlf, 7), (1, 0));
    }

    #[test]
    fn test_utf16_emoji_handling() {
        let text = "Hello 😀 World";
        let rope = Rope::from_str(text);

        // The emoji takes 2 UTF-16 code units
        // "Hello " = 6 UTF-16 units
        // "😀" = 2 UTF-16 units
        // " World" starts at UTF-16 position 9

        // Position after "Hello " (before emoji)
        assert_eq!(lsp_pos_to_byte(&rope, 0, 6), 6);

        // Position after emoji (before " World")
        assert_eq!(lsp_pos_to_byte(&rope, 0, 8), 10); // 6 bytes for "Hello " + 4 bytes for emoji

        // Reverse conversion
        let (line, col) = byte_to_lsp_pos(&rope, 10);
        assert_eq!(line, 0);
        assert_eq!(col, 8); // 6 for "Hello " + 2 for emoji in UTF-16
    }

    #[test]
    #[serial_test::serial]
    fn test_full_document_replacement() {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $x = 1;";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config).unwrap();

        // Full document replacement (no range)
        let changes = vec![json!({
            "text": "my $y = 2;\nmy $z = 3;"
        })];

        doc.apply_changes(&changes, &config).unwrap();

        assert_eq!(doc.content(), "my $y = 2;\nmy $z = 3;");
        assert!(doc.ast().is_some());

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_disabled_fallback() {
        // Ensure incremental is disabled
        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        let config = IncrementalConfig::default();
        assert!(!config.enabled);

        // Should still work with full parsing
        let code = "my $x = 42;";
        let mut doc = DocumentParser::new(code.to_string(), &config).unwrap();

        let changes = vec![json!({"text": "my $y = 99;"})];
        doc.apply_changes(&changes, &config).unwrap();

        assert_eq!(doc.content(), "my $y = 99;");
        assert!(doc.ast().is_some());
    }
}

// Test without incremental feature
#[cfg(not(feature = "incremental"))]
#[test]
fn test_incremental_feature_disabled() {
    // Just verify the crate compiles without the feature
    use perl_parser::Parser;

    let code = "my $x = 42;";
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    assert!(ast.to_sexp().contains("scalar_variable"));
}

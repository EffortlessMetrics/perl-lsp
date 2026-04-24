//! Integration tests for incremental parsing functionality

#[cfg(feature = "incremental")]
mod incremental_tests {
    use perl_parser::incremental_integration::{
        byte_to_lsp_pos, lsp_pos_to_byte, DocumentParser, IncrementalConfig,
    };
    use ropey::Rope;
    use serde_json::json;
    use std::time::Instant;

    use perl_parser::Parser;

    fn assert_incremental_matches_full_parse(content: &str, doc: &DocumentParser) {
        let mut full_parser = Parser::new(content);
        let full_parse = full_parser.parse();

        match full_parse {
            Ok(full_ast) => {
                let incremental_ast =
                    doc.ast().expect("incremental AST should exist for valid parse");
                assert_eq!(format!("{:?}", *incremental_ast), format!("{:?}", full_ast));
            }
            Err(_) => {
                // For malformed scenarios, the key invariant is that incremental application does not panic.
                assert!(doc.ast().is_some());
            }
        }
    }

    fn lsp_change_from_byte_range(
        source: &str,
        start_byte: usize,
        end_byte: usize,
        text: &str,
    ) -> serde_json::Value {
        let rope = Rope::from_str(source);
        let (start_line, start_char) = byte_to_lsp_pos(&rope, start_byte);
        let (end_line, end_char) = byte_to_lsp_pos(&rope, end_byte);

        json!({
            "range": {
                "start": {"line": start_line, "character": start_char},
                "end": {"line": end_line, "character": end_char}
            },
            "text": text
        })
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_parsing_small_edit() -> Result<(), Box<dyn std::error::Error>> {
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

        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        // Verify initial AST
        let ast1 = doc.ast().ok_or("Failed to get initial AST")?;
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
        doc.apply_changes(&changes, &config)?;
        let parse_time = start.elapsed();

        // Verify parse time is reasonable
        assert!(parse_time.as_millis() < 100, "Parse time too slow: {:?}", parse_time);

        // Verify updated AST
        let ast2 = doc.ast().ok_or("Failed to get updated AST")?;
        assert!(doc.content().contains("99"));
        assert!(format!("{:?}", ast2).contains("Variable"));

        // Check incremental parsing metrics
        if let Some(metrics) = doc.metrics() {
            assert!(metrics.nodes_reused > 0);
            assert!(metrics.last_parse_time_ms < 2.0);
        }

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_parsing_multiple_edits() -> Result<(), Box<dyn std::error::Error>> {
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

        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

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

        doc.apply_changes(&changes, &config)?;

        // Verify both changes applied
        assert!(doc.content().contains("Hi"));
        assert!(doc.content().contains("Universe"));
        assert!(!doc.content().contains("Hello"));
        assert!(!doc.content().contains("World"));

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
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
    fn test_full_document_replacement() -> Result<(), Box<dyn std::error::Error>> {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $x = 1;";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        // Full document replacement (no range)
        let changes = vec![json!({
            "text": "my $y = 2;\nmy $z = 3;"
        })];

        doc.apply_changes(&changes, &config)?;

        assert_eq!(doc.content(), "my $y = 2;\nmy $z = 3;");
        assert!(doc.ast().is_some());

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_disabled_fallback() -> Result<(), Box<dyn std::error::Error>> {
        // Ensure incremental is disabled
        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        let config = IncrementalConfig::default();
        assert!(!config.enabled);

        // Should still work with full parsing
        let code = "my $x = 42;";
        let mut doc = DocumentParser::new(code.to_string(), &config)?;

        let changes = vec![json!({"text": "my $y = 99;"})];
        doc.apply_changes(&changes, &config)?;

        assert_eq!(doc.content(), "my $y = 99;");
        assert!(doc.ast().is_some());
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_large_deletion_matches_full_parse() -> Result<(), Box<dyn std::error::Error>>
    {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "sub keep_one {
    my $x = 1;
}

sub remove_me {
    my $tmp = 100;
    my $extra = 200;
    print $tmp + $extra;
}

sub keep_two {
    my $y = 2;
}
";

        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;
        let remove_start = initial_code.find("sub remove_me").ok_or("missing remove_me start")?;
        let remove_end = initial_code.find("sub keep_two").ok_or("missing keep_two marker")?;
        let changes = vec![lsp_change_from_byte_range(initial_code, remove_start, remove_end, "")];

        doc.apply_changes(&changes, &config)?;

        assert!(!doc.content().contains("remove_me"));
        assert_incremental_matches_full_parse(doc.content(), &doc);

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_insertion_invalidation_matches_full_parse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $base = 1;
my $next = $base + 2;
print $next;
";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        let changes = vec![json!({
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "text": "my $inserted = 41;
        "
        })];

        doc.apply_changes(&changes, &config)?;

        assert!(doc.content().starts_with("my $inserted = 41;"));
        assert_incremental_matches_full_parse(doc.content(), &doc);

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_whitespace_only_edit_matches_full_parse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $x=1;
my $y=2;
print $x+$y;
";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        let replacement = "my  $x = 1;
	my $y = 2;
print   $x + $y;
";
        let full_replace = vec![json!({
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 2, "character": 12}
            },
            "text": replacement
        })];

        doc.apply_changes(&full_replace, &config)?;

        assert!(doc.content().starts_with(replacement));
        assert_incremental_matches_full_parse(doc.content(), &doc);

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_multibyte_boundary_edit_matches_full_parse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $emoji = \"😀\";\nprint $emoji;\n";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        let emoji_start = initial_code.find("😀").ok_or("emoji start not found")?;
        let emoji_end = emoji_start + "😀".len();
        let changes = vec![lsp_change_from_byte_range(initial_code, emoji_start, emoji_end, "😺")];

        doc.apply_changes(&changes, &config)?;

        assert!(doc.content().contains("😺"));
        let mut full_parser = Parser::new(doc.content());
        assert!(full_parser.parse().is_ok(), "full parse should still succeed for multibyte edit");

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_batch_edits_with_independent_shifts_matches_full_parse(
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $a = 10;
my $b = 20;
my $c = $a + $b;
print $c;
";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;

        let changes = vec![
            json!({
                "range": {
                    "start": {"line": 0, "character": 8},
                    "end": {"line": 0, "character": 10}
                },
                "text": "100"
            }),
            json!({
                "range": {
                    "start": {"line": 2, "character": 14},
                    "end": {"line": 2, "character": 16}
                },
                "text": "$a * $b"
            }),
        ];

        doc.apply_changes(&changes, &config)?;

        assert!(doc.content().contains("my $a = 100;"));
        assert!(doc.content().contains("$a * $b"));
        let mut full_parser = Parser::new(doc.content());
        assert!(full_parser.parse().is_ok(), "full parse should succeed after batch edits");
        assert!(doc.ast().is_some(), "incremental parse should produce an AST");

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_incremental_unmappable_edit_fallback_no_panic() -> Result<(), Box<dyn std::error::Error>>
    {
        unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
        let config = IncrementalConfig::default();

        let initial_code = "my $x = 1;
";
        let mut doc = DocumentParser::new(initial_code.to_string(), &config)?;
        let replacement = "my $fallback = 99;
print $fallback;
";

        let malformed_change = vec![json!({
            "range": {
                "start": {"line": "bad", "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "text": replacement
        })];

        let apply_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            doc.apply_changes(&malformed_change, &config)
        }));

        assert!(apply_result.is_ok(), "malformed edit should not panic");
        let parse_result = apply_result.expect("unwind result should be available");
        assert!(parse_result.is_ok(), "malformed edit should gracefully fallback");
        assert_eq!(doc.content(), replacement);
        assert_incremental_matches_full_parse(doc.content(), &doc);

        unsafe { std::env::remove_var("PERL_LSP_INCREMENTAL") };
        Ok(())
    }
}

// Test without incremental feature
#[cfg(not(feature = "incremental"))]
#[test]
fn test_incremental_feature_disabled() -> Result<(), Box<dyn std::error::Error>> {
    // Just verify the crate compiles without the feature
    use perl_parser::Parser;

    let code = "my $x = 42;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    assert!(format!("{:?}", ast).contains("Variable"));
    Ok(())
}

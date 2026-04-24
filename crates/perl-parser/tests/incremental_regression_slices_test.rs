//! Focused regression slices for incremental integration behavior.

#[cfg(feature = "incremental")]
mod incremental_regression_slices_tests {
    use perl_parser::incremental_integration::{byte_to_lsp_pos, DocumentParser, IncrementalConfig};
    use perl_parser::Parser;
    use ropey::Rope;
    use serde_json::{json, Value};

    fn incremental_config() -> IncrementalConfig {
        IncrementalConfig {
            enabled: true,
            ..IncrementalConfig::default()
        }
    }

    fn change_for_substring(source: &str, needle: &str, replacement: &str) -> Result<Value, String> {
        let start_byte = source.find(needle).ok_or_else(|| format!("missing needle: {needle}"))?;
        let end_byte = start_byte + needle.len();
        let rope = Rope::from_str(source);
        let (start_line, start_char) = byte_to_lsp_pos(&rope, start_byte);
        let (end_line, end_char) = byte_to_lsp_pos(&rope, end_byte);

        Ok(json!({
            "range": {
                "start": {"line": start_line, "character": start_char},
                "end": {"line": end_line, "character": end_char}
            },
            "text": replacement
        }))
    }

    fn assert_incremental_parse_result(
        initial_source: &str,
        changes: Vec<Value>,
        expected_source: &str,
        expect_ast_equivalence: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = incremental_config();
        let mut doc = DocumentParser::new(initial_source.to_string(), &config)?;
        doc.apply_changes(&changes, &config)?;

        assert_eq!(doc.content(), expected_source, "incremental content mismatch");

        let incremental_ast = doc.ast().ok_or("missing incremental AST")?;
        let mut full_parser = Parser::new(expected_source);
        let full_ast = full_parser.parse()?;

        let incremental_debug = format!("{incremental_ast:?}");
        let full_debug = format!("{full_ast:?}");

        if expect_ast_equivalence {
            assert_eq!(
                incremental_debug, full_debug,
                "incremental AST diverged from fresh full parse"
            );
        } else {
            assert_ne!(
                incremental_debug, full_debug,
                "expected known divergence between incremental and fresh full parse"
            );
        }

        Ok(())
    }

    #[test]
    fn large_deletion_matches_fresh_full_parse() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "sub keep {\n    my $a = 1;\n}\n\nsub drop {\n    my $x = 10;\n    my $y = 20;\n}\n";
        let deleted = "sub drop {\n    my $x = 10;\n    my $y = 20;\n}\n";
        let changes = vec![change_for_substring(initial, deleted, "")?];
        let expected = "sub keep {\n    my $a = 1;\n}\n\n";

        assert_incremental_parse_result(initial, changes, expected, true)
    }

    #[test]
    fn insertion_invalidation_matches_fresh_full_parse() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "sub work {\n    my $x = 1;\n    return $x;\n}\n";
        let insertion_anchor = "    my $x = 1;\n";
        let changes = vec![change_for_substring(
            initial,
            insertion_anchor,
            "    my $pre = 0;\n    my $x = 1;\n",
        )?];
        let expected = "sub work {\n    my $pre = 0;\n    my $x = 1;\n    return $x;\n}\n";

        assert_incremental_parse_result(initial, changes, expected, true)
    }

    #[test]
    fn whitespace_only_edit_matches_fresh_full_parse() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "my $x = 1;\nmy $y = 2;\n";
        let changes = vec![change_for_substring(initial, "\n", "\n\n")?];
        let expected = "my $x = 1;\n\nmy $y = 2;\n";

        assert_incremental_parse_result(initial, changes, expected, true)
    }

    #[test]
    fn multibyte_boundary_edit_tracks_known_divergence() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "my $emoji = \"😀\";\nmy $x = 1;\n";
        let changes = vec![change_for_substring(initial, "😀", "😺")?];
        let expected = "my $emoji = \"😺\";\nmy $x = 1;\n";

        assert_incremental_parse_result(initial, changes, expected, false)
    }

    #[test]
    fn batch_edits_with_independent_shifts_track_known_divergence(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let initial = "my $first = 111;\nmy $middle = 0;\nmy $second = 222;\n";
        let changes = vec![
            change_for_substring(initial, "111", "1")?,
            change_for_substring(initial, "222", "99999")?,
        ];
        let expected = "my $first = 1;\nmy $middle = 0;\nmy $second = 99999;\n";

        assert_incremental_parse_result(initial, changes, expected, false)
    }

    #[test]
    fn unmappable_edit_falls_back_to_full_replacement() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "my $x = 1;\nmy $y = 2;\n";
        let expected = "my $replacement = 42;\n";
        let changes = vec![json!({
            "range": {
                "start": {"line": "invalid", "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "text": expected
        })];

        assert_incremental_parse_result(initial, changes, expected, true)
    }

    #[test]
    fn malformed_inverted_range_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
        let initial = "my $x = 1;\nmy $y = 2;\n";
        let changes = vec![json!({
            "range": {
                "start": {"line": 1, "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "text": ""
        })];

        assert_incremental_parse_result(initial, changes, initial, true)
    }
}

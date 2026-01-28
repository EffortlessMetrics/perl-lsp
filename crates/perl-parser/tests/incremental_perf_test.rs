//! Performance and correctness tests for incremental parsing

#[cfg(test)]
mod tests {
    use perl_parser::position::{PositionMapper, WirePosition as Position, apply_edit_utf8};
    use std::time::Instant;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    #[serial_test::serial]
    fn test_big_file_edit_performance() -> TestResult {
        // Generate a 10k-line file
        let line = "my $x = 42; # Some comment here\n";
        let big_file = line.repeat(10_000);

        // Create mapper
        let mut mapper = PositionMapper::new(&big_file);

        // Edit near the middle (line 5000)
        let middle_line = 5000;
        let edit_pos = Position {
            line: middle_line,
            character: 8, // After "my $x = "
        };

        // Time the edit operation
        let start = Instant::now();

        // Calculate byte offset
        let byte_offset = mapper.lsp_pos_to_byte(edit_pos).ok_or("invalid edit position")?;

        // Apply edit
        let mut text = big_file.clone();
        apply_edit_utf8(&mut text, byte_offset, byte_offset + 2, "999");

        // Rebuild mapper
        mapper = PositionMapper::new(&text);

        // Verify position mapping still works
        let check_pos = Position {
            line: middle_line,
            character: 10, // After the edited value
        };
        let _byte = mapper.lsp_pos_to_byte(check_pos);

        let elapsed = start.elapsed();

        // Assert performance: should be < 50ms (allow 150ms for CI)
        assert!(elapsed.as_millis() < 150, "Edit took {}ms, expected < 150ms", elapsed.as_millis());

        // Verify the edit was applied correctly
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[middle_line as usize].contains("999"));
        Ok(())
    }

    #[test]
    fn test_emoji_edit_utf16_handling() -> TestResult {
        // Text with emoji (🦀 is 2 UTF-16 code units, 4 UTF-8 bytes)
        let text = "let 🦀 = rust;";
        let mapper = PositionMapper::new(text);

        // Position after emoji (char 6 in UTF-16: "let " = 4, "🦀" = 2)
        let pos_after_emoji = Position {
            line: 0,
            character: 6, // UTF-16 position
        };

        // Get byte offset
        let byte_offset =
            mapper.lsp_pos_to_byte(pos_after_emoji).ok_or("invalid emoji position")?;

        // The emoji starts at byte 4 ("let ") and is 4 bytes long
        assert_eq!(byte_offset, 8, "Should be at byte 8 after 4-byte emoji");

        // Edit: replace emoji with text
        let mut mutable_text = text.to_string();
        apply_edit_utf8(&mut mutable_text, 4, 8, "crab");
        assert_eq!(mutable_text, "let crab = rust;");

        // Test round-trip
        let new_mapper = PositionMapper::new(&mutable_text);
        let pos = Position { line: 0, character: 8 }; // After "let crab"
        let byte = new_mapper.lsp_pos_to_byte(pos).ok_or("invalid round-trip position")?;
        let back_pos = new_mapper.byte_to_lsp_pos(byte);
        assert_eq!(back_pos.character, 8);
        Ok(())
    }

    #[test]
    fn test_crlf_fixture_windows_compatibility() -> TestResult {
        // Windows-style CRLF text
        let crlf_text = "line one\r\nline two\r\nline three";
        let mapper = PositionMapper::new(crlf_text);

        // Test positions at line boundaries

        // Start of line 2 (after \r\n)
        let line2_start = Position { line: 1, character: 0 };
        let byte = mapper.lsp_pos_to_byte(line2_start).ok_or("invalid line2_start position")?;
        assert_eq!(byte, 10); // "line one\r\n" = 10 bytes

        // Middle of line 2
        let line2_mid = Position { line: 1, character: 5 };
        let byte = mapper.lsp_pos_to_byte(line2_mid).ok_or("invalid line2_mid position")?;
        assert_eq!(byte, 15); // 10 + 5

        // Edit across CRLF boundary
        let mut text = crlf_text.to_string();
        apply_edit_utf8(&mut text, 8, 10, ""); // Remove \r\n
        assert_eq!(text, "line oneline two\r\nline three");

        // Test mixed endings
        let mixed = "unix\nwindows\r\nmac\rend";
        let mixed_mapper = PositionMapper::new(mixed);

        // Each line start
        assert_eq!(mixed_mapper.byte_to_lsp_pos(0).line, 0); // unix
        assert_eq!(mixed_mapper.byte_to_lsp_pos(5).line, 1); // windows
        assert_eq!(mixed_mapper.byte_to_lsp_pos(14).line, 2); // mac
        assert_eq!(mixed_mapper.byte_to_lsp_pos(18).line, 3); // end
        Ok(())
    }

    #[test]
    fn test_multibyte_char_edit() -> TestResult {
        // Text with various multibyte characters
        let text = "café ☕ 世界";
        let mapper = PositionMapper::new(text);

        // Test position in middle of multibyte sequence
        let pos = Position { line: 0, character: 5 }; // After "café "
        let byte_offset = mapper.lsp_pos_to_byte(pos).ok_or("invalid multibyte position")?;

        // "café " = 'c'(1) + 'a'(1) + 'f'(1) + 'é'(2) + ' '(1) = 6 bytes
        assert_eq!(byte_offset, 6);

        // Edit multibyte character
        let mut mutable = text.to_string();
        apply_edit_utf8(&mut mutable, 3, 5, "e"); // Replace 'é' with 'e'
        assert_eq!(mutable, "cafe ☕ 世界");
        Ok(())
    }

    #[test]
    fn test_incremental_multiple_edits() {
        let mut text = "line1\nline2\nline3".to_string();

        // Apply multiple edits in sequence
        let edits = vec![
            (0, 5, "LINE1"),   // Replace line1 with LINE1
            (6, 11, "LINE2"),  // Replace line2 with LINE2
            (12, 17, "LINE3"), // Replace line3 with LINE3
        ];

        for (start, end, replacement) in edits {
            apply_edit_utf8(&mut text, start, end, replacement);
        }

        assert_eq!(text, "LINE1\nLINE2\nLINE3");
    }
}

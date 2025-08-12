#[cfg(test)]
mod utf16_round_trip_tests {
    use perl_parser::lsp_server::LspServer;

    fn test_round_trip(server: &LspServer, text: &str, line: u32, character: u32) -> bool {
        // Convert position to offset
        let offset = server.position_to_offset(text, line, character);

        // Convert offset back to position
        let (rt_line, rt_char) = server.offset_to_position(text, offset);

        // Check if we got back the same position
        rt_line == line && rt_char == character
    }

    #[test]
    fn test_crlf_round_trip() {
        let server = LspServer::new();
        let text = "my $x = 1;\r\n$x++;\r\nprint $x;";

        // Test position at start of line
        assert!(test_round_trip(&server, text, 0, 0), "Start of first line");
        assert!(test_round_trip(&server, text, 1, 0), "Start of second line");
        assert!(test_round_trip(&server, text, 2, 0), "Start of third line");

        // Test position at \r (should map to same as start of \n)
        let offset_at_cr = server.position_to_offset(text, 0, 10); // At \r
        let offset_at_lf = server.position_to_offset(text, 1, 0); // At start of next line
        assert_eq!(
            offset_at_cr,
            offset_at_lf - 1,
            "\\r maps to position before \\n"
        );

        // Test round-trip at various positions
        assert!(test_round_trip(&server, text, 0, 5), "Middle of first line");
        assert!(
            test_round_trip(&server, text, 1, 3),
            "Middle of second line"
        );
        assert!(test_round_trip(&server, text, 2, 8), "End of third line");

        // CRLF clamp assertion: position at huge column should map to EOL
        let clamped_offset = server.position_to_offset(text, 0, 1000);
        let (rt_line, rt_char) = server.offset_to_position(text, clamped_offset);
        assert_eq!(rt_line, 0, "Should stay on first line");
        assert_eq!(rt_char, 10, "Should clamp to before \\r");
    }

    #[test]
    fn test_emoji_round_trip() {
        let server = LspServer::new();
        let text = "my $x = \"🐍\";\n";

        // Test positions before, at, and after the emoji
        assert!(test_round_trip(&server, text, 0, 0), "Start of line");
        assert!(
            test_round_trip(&server, text, 0, 8),
            "Before emoji (after opening quote)"
        );
        assert!(
            test_round_trip(&server, text, 0, 10),
            "After emoji (before closing quote)"
        );
        assert!(test_round_trip(&server, text, 0, 11), "After closing quote");
        assert!(test_round_trip(&server, text, 0, 12), "At semicolon");

        // Test that emoji counts as 2 UTF-16 units
        let offset_before = server.position_to_offset(text, 0, 8);
        let offset_after = server.position_to_offset(text, 0, 10);
        assert_eq!(offset_after - offset_before, 4, "Emoji is 4 bytes");

        // Verify the character difference is 2 (surrogate pair)
        let (_, char_before) = server.offset_to_position(text, offset_before);
        let (_, char_after) = server.offset_to_position(text, offset_after);
        assert_eq!(
            char_after - char_before,
            2,
            "Emoji counts as 2 UTF-16 units"
        );

        // Surrogate pair fence assertion: positions at boundaries map correctly
        let text_just_emoji = "🐍";
        let before_offset = server.position_to_offset(text_just_emoji, 0, 0);
        assert_eq!(before_offset, 0, "Position before emoji maps to byte 0");

        let after_offset = server.position_to_offset(text_just_emoji, 0, 2);
        assert_eq!(after_offset, 4, "Position after emoji maps to byte 4");

        // Round-trip both positions
        let (line1, char1) = server.offset_to_position(text_just_emoji, before_offset);
        assert_eq!((line1, char1), (0, 0), "Before emoji round-trips correctly");

        let (line2, char2) = server.offset_to_position(text_just_emoji, after_offset);
        assert_eq!((line2, char2), (0, 2), "After emoji round-trips correctly");
    }

    #[test]
    fn test_pi_symbol_round_trip() {
        let server = LspServer::new();
        let text = "my $π = 3.14159;\n$π++;\n";

        // Test positions around π (counts as 1 UTF-16 unit)
        assert!(test_round_trip(&server, text, 0, 0), "Start of first line");
        assert!(test_round_trip(&server, text, 0, 3), "Before π");
        assert!(test_round_trip(&server, text, 0, 4), "After π");
        assert!(test_round_trip(&server, text, 0, 16), "End of first line");
        assert!(test_round_trip(&server, text, 1, 0), "Start of second line");
        assert!(
            test_round_trip(&server, text, 1, 1),
            "After $ on second line"
        );
        assert!(
            test_round_trip(&server, text, 1, 2),
            "After π on second line"
        );

        // Verify π counts as 1 UTF-16 unit but is 2 bytes
        let offset_before = server.position_to_offset(text, 0, 3);
        let offset_after = server.position_to_offset(text, 0, 4);
        assert_eq!(offset_after - offset_before, 2, "π is 2 bytes (UTF-8)");
    }

    #[test]
    fn test_mixed_unicode_round_trip() {
        let server = LspServer::new();
        let text = "my $café = \"☕\";\r\nmy $Σ = 100;\r\n";

        // Test various positions
        assert!(test_round_trip(&server, text, 0, 0), "Start");
        assert!(test_round_trip(&server, text, 0, 3), "Before $");
        assert!(test_round_trip(&server, text, 0, 8), "After café");
        assert!(test_round_trip(&server, text, 0, 12), "After coffee emoji");
        assert!(test_round_trip(&server, text, 1, 4), "After Σ");

        // Test that all positions round-trip correctly
        for line in 0..2 {
            for char in 0..20 {
                let offset = server.position_to_offset(text, line, char);
                let (rt_line, rt_char) = server.offset_to_position(text, offset);
                if char <= (if line == 0 { 15 } else { 12 }) {
                    assert_eq!(
                        (rt_line, rt_char),
                        (line, char),
                        "Round-trip at ({}, {})",
                        line,
                        char
                    );
                }
            }
        }
    }

    #[test]
    fn test_edge_positions() {
        let server = LspServer::new();

        // Empty string
        assert!(test_round_trip(&server, "", 0, 0), "Empty string");

        // Single character
        assert!(test_round_trip(&server, "x", 0, 0), "Before char");
        assert!(test_round_trip(&server, "x", 0, 1), "After char");

        // Position past end of line (should clamp)
        let text = "short";
        let offset = server.position_to_offset(text, 0, 100);
        assert_eq!(offset, 5, "Clamped to end of text");

        // Position on non-existent line (should clamp)
        let offset = server.position_to_offset(text, 10, 0);
        assert_eq!(offset, 5, "Clamped to end of text");
    }

    #[test]
    fn test_complex_emoji_sequences() {
        let server = LspServer::new();

        // Test with emoji combinations and zero-width joiners
        let text = "👨‍👩‍👧‍👦 family\n"; // Family emoji (multiple codepoints)

        // The family emoji is complex - just verify we can round-trip positions
        assert!(test_round_trip(&server, text, 0, 0), "Start");

        // Position after the complex emoji (it's many UTF-16 units)
        let offset_start = server.position_to_offset(text, 0, 0);
        let offset_after_emoji = text.find(" family").unwrap();
        let (line, char) = server.offset_to_position(text, offset_after_emoji);
        assert_eq!(line, 0, "Still on first line");
        assert!(char > 0, "Character position advanced past emoji");

        // Test round-trip at "family" text
        let family_start = text.find("family").unwrap();
        let (line, char) = server.offset_to_position(text, family_start);
        assert!(
            test_round_trip(&server, text, line, char),
            "At 'family' text"
        );
    }
}

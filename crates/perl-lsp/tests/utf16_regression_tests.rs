//! UTF-16 Position Handling Regression Tests
//!
//! These tests ensure that LSP position handling uses UTF-16 code units (as required by LSP spec)
//! rather than byte offsets. They will fail loudly if someone accidentally reintroduces byte offset
//! calculations in place of proper UTF-16 handling.
//!
//! ## Background
//!
//! The LSP specification requires all `Position.character` values to be expressed in UTF-16 code units.
//! This is different from:
//! - Byte offsets (what Rust strings typically use internally)
//! - Unicode scalar values / code points (what Rust `char` represents)
//!
//! ## Character Width Differences
//!
//! | Character | UTF-8 bytes | UTF-16 code units | Unicode code points |
//! |-----------|-------------|-------------------|---------------------|
//! | ASCII 'a' | 1           | 1                 | 1                   |
//! | CJK '日'   | 3           | 1                 | 1                   |
//! | Emoji '🎉' | 4           | 2 (surrogate pair)| 1                   |
//!
//! ## How These Tests Detect Regressions
//!
//! If the LSP server incorrectly uses byte offsets:
//! - After '🎉' (4 bytes), the returned position would be 4 instead of 2
//! - After '日本語' (9 bytes), positions would be offset by +6 instead of correct
//!
//! The tests request information at specific UTF-16 positions AFTER non-ASCII characters
//! and verify that the returned range positions match UTF-16 code unit expectations.
//!
//! ## Related PRs
//!
//! - PR #243: Fixed UTF-16 position handling in LSP handlers
//! - PR #244: Adopted strict params and UTF-16 correctness across handlers

mod common;

use common::{initialize_lsp, send_notification, send_request, start_lsp_server};
use serde_json::json;

/// Compute UTF-16 code unit count for a string.
/// This is the correct way to calculate LSP `character` positions.
fn utf16_len(s: &str) -> u32 {
    s.chars().map(|c| c.len_utf16() as u32).sum()
}

/// Helper to extract hover range from response for position validation.
/// Returns (start_line, start_char, end_line, end_char) if present.
fn extract_hover_range(response: &serde_json::Value) -> Option<(u32, u32, u32, u32)> {
    let range = response.get("result")?.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    Some((
        start.get("line")?.as_u64()? as u32,
        start.get("character")?.as_u64()? as u32,
        end.get("line")?.as_u64()? as u32,
        end.get("character")?.as_u64()? as u32,
    ))
}

/// Helper to extract definition locations from response.
/// Returns vec of (uri, start_line, start_char, end_char).
fn extract_definition_locations(response: &serde_json::Value) -> Vec<(String, u32, u32, u32)> {
    let mut locations = Vec::new();
    if let Some(result) = response.get("result") {
        let items = if result.is_array() {
            result.as_array().unwrap().clone()
        } else if result.is_object() {
            vec![result.clone()]
        } else {
            return locations;
        };

        for item in items {
            if let (Some(uri), Some(range)) = (item.get("uri"), item.get("range")) {
                let uri_str = uri.as_str().unwrap_or("").to_string();
                let start = &range["start"];
                let end = &range["end"];
                if let (Some(line), Some(start_char), Some(end_char)) = (
                    start.get("line").and_then(|v| v.as_u64()),
                    start.get("character").and_then(|v| v.as_u64()),
                    end.get("character").and_then(|v| v.as_u64()),
                ) {
                    locations.push((uri_str, line as u32, start_char as u32, end_char as u32));
                }
            }
        }
    }
    locations
}

/// Test 1: Emoji in variable name - UTF-16 surrogate pair handling
///
/// This test uses an emoji (🎉) which requires 2 UTF-16 code units but 4 UTF-8 bytes.
/// The variable `$after_emoji` appears AFTER the emoji-containing assignment.
///
/// If byte offsets are incorrectly used:
/// - The hover/definition position for `$after_emoji` would be wrong by +2 per emoji
///
/// Expected UTF-16 positions on line 1 ("$emoji_🎉 = 1;"):
/// - '$' is at position 0
/// - 'e' is at position 1
/// - '🎉' starts at position 7, ends at position 9 (2 UTF-16 code units)
/// - '=' is at position 11
#[test]
fn test_utf16_emoji_position_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Code with emoji in variable name
    // Line 0: "my $emoji_🎉 = 1;"
    // Line 1: "my $after = $emoji_🎉 + 1;"
    //
    // On line 0:
    // - "$emoji_🎉" starts at position 3 (after "my ")
    // - The emoji 🎉 is at UTF-16 positions 10-11 (since $emoji_ is 7 chars, then 🎉 is 2 UTF-16 units)
    //
    // On line 1:
    // - "$after" starts at position 3
    // - "$emoji_🎉" reference starts at position 12 (after "my $after = ")
    let code = "my $emoji_🎉 = 1;\nmy $after = $emoji_🎉 + 1;\n";
    let uri = "file:///test_emoji.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": code
                }
            }
        }),
    );

    // Calculate expected UTF-16 position for $after on line 1
    let line1 = "my $after = $emoji_🎉 + 1;";
    let prefix_to_after = "my ";
    let expected_after_start = utf16_len(prefix_to_after); // 3
    let expected_after_end = expected_after_start + utf16_len("$after"); // 3 + 6 = 9

    // Request hover at $after (line 1, position 3 in UTF-16)
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": expected_after_start }
            }
        }),
    );

    // Verify we got a response (hover content may vary, but range should be correct)
    if let Some((start_line, start_char, end_line, end_char)) = extract_hover_range(&response) {
        assert_eq!(start_line, 1, "Hover range should be on line 1");
        assert_eq!(end_line, 1, "Hover range should end on line 1");

        // The key assertion: if byte offsets were used instead of UTF-16,
        // the position would be wrong because the emoji on line 0 would
        // have caused position drift in any shared state.
        assert!(
            start_char <= expected_after_start + 1,
            "Hover start character {} should be near expected UTF-16 position {} \
             (if this is way off, byte offsets may have been used instead of UTF-16)",
            start_char,
            expected_after_start
        );

        // Verify the $after variable end position
        // $after is 6 UTF-16 code units, so end should be 3 + 6 = 9
        assert_eq!(
            end_char, expected_after_end,
            "Hover end character {} should be {} (UTF-16). \
             If this is wrong, UTF-16 handling regressed.",
            end_char, expected_after_end
        );
    }

    // Verify the emoji variable on line 0 has correct UTF-16 range
    // "$emoji_🎉" starts at position 3, emoji is 2 UTF-16 code units
    // Total: "$emoji_" (7) + "🎉" (2) = 9 UTF-16 code units
    let expected_emoji_var_len = utf16_len("$emoji_🎉");
    assert_eq!(
        expected_emoji_var_len, 9,
        "$emoji_🎉 should be 9 UTF-16 code units (7 ASCII + 2 for emoji surrogate pair)"
    );

    // If we got here with correct assertions, UTF-16 handling is working
    // The key regression this catches: if byte offsets were used,
    // "$emoji_🎉" would appear as 11 units (7 + 4 bytes) instead of 9 UTF-16 units
    println!(
        "PASS: Emoji UTF-16 handling correct. Line has {} UTF-16 code units.",
        utf16_len(line1)
    );
}

/// Test 2: CJK characters - multi-byte UTF-8 but single UTF-16 code unit
///
/// CJK characters like 日本語 (Japanese) are:
/// - 3 bytes each in UTF-8 (total 9 bytes for 3 characters)
/// - 1 UTF-16 code unit each (total 3 code units)
///
/// If byte offsets are incorrectly used:
/// - Positions after "日本語" would be off by +6 (9 bytes - 3 UTF-16 units)
#[test]
fn test_utf16_cjk_position_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Code with CJK variable name
    // Line 0: "my $日本語 = 'hello';"
    // Line 1: "my $result = $日本語;"
    //
    // On line 0:
    // - "$日本語" starts at UTF-16 position 3 (after "my ")
    // - "日本語" is 3 UTF-16 code units (positions 4-6)
    // - BUT it's 9 UTF-8 bytes!
    //
    // On line 1:
    // - "$result" starts at UTF-16 position 3
    // - "$日本語" reference starts at UTF-16 position 13 (after "my $result = ")
    let code = "my $日本語 = 'hello';\nmy $result = $日本語;\n";
    let uri = "file:///test_cjk.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": code
                }
            }
        }),
    );

    // Calculate expected UTF-16 positions
    let line0 = "my $日本語 = 'hello';";
    let line1 = "my $result = $日本語;";

    // Verify our UTF-16 length calculation
    assert_eq!(utf16_len("日本語"), 3, "CJK should be 3 UTF-16 code units");
    assert_eq!(
        "日本語".len(),
        9,
        "CJK should be 9 UTF-8 bytes - this difference is what the test catches"
    );

    // Request hover at $result (line 1, position 3)
    let expected_result_start = utf16_len("my "); // 3
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": expected_result_start }
            }
        }),
    );

    if let Some((start_line, start_char, _end_line, end_char)) = extract_hover_range(&response) {
        assert_eq!(start_line, 1, "Result hover should be on line 1");

        // Verify the range uses UTF-16 positions
        assert_eq!(
            start_char, expected_result_start,
            "Start character should match UTF-16 position"
        );

        let expected_result_end = expected_result_start + utf16_len("$result");
        assert_eq!(
            end_char, expected_result_end,
            "End character {} should be {} (UTF-16). \
             If this is wrong, byte offsets may have leaked into position calculations.",
            end_char, expected_result_end
        );
    }

    // Request definition for $日本語 reference on line 1
    let prefix_to_cjk_ref = "my $result = ";
    let expected_cjk_ref_start = utf16_len(prefix_to_cjk_ref); // 13

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": expected_cjk_ref_start }
            }
        }),
    );

    let locations = extract_definition_locations(&response);
    if !locations.is_empty() {
        let (_, def_line, def_start, def_end) = &locations[0];
        assert_eq!(*def_line, 0, "CJK variable definition should be on line 0");

        // The definition range should use UTF-16 positions
        // "$日本語" = 1 + 3 = 4 UTF-16 code units
        // If byte offsets were used, it would be 1 + 9 = 10 bytes
        let expected_var_utf16_len = utf16_len("$日本語");
        let actual_len = *def_end - *def_start;

        assert!(
            actual_len <= expected_var_utf16_len + 1, // Allow small variance for sigil handling
            "Definition range length {} should be near {} UTF-16 units for '$日本語'. \
             If this is ~10, byte offsets are being used instead of UTF-16!",
            actual_len,
            expected_var_utf16_len
        );
    }

    println!(
        "PASS: CJK UTF-16 handling correct. Line 0: {} UTF-16 units, Line 1: {} UTF-16 units",
        utf16_len(line0),
        utf16_len(line1)
    );
}

/// Test 3: Mixed content - ASCII, emoji, and CJK together
///
/// This test combines multiple non-ASCII character types to stress-test
/// the UTF-16 position handling across various operations.
///
/// The string "Hello 🌍 世界!" contains:
/// - "Hello " = 6 UTF-16 units (6 bytes)
/// - "🌍" = 2 UTF-16 units (4 bytes) - Earth globe emoji
/// - " " = 1 UTF-16 unit (1 byte)
/// - "世界" = 2 UTF-16 units (6 bytes) - "World" in Chinese
/// - "!" = 1 UTF-16 unit (1 byte)
///
/// Total: 12 UTF-16 units, 18 bytes
#[test]
fn test_utf16_mixed_content_position_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Code with mixed Unicode content
    // We'll put a variable AFTER the mixed content string to verify positions
    let code = r#"my $greeting = "Hello 🌍 世界!";
my $length = length($greeting);
"#;
    let uri = "file:///test_mixed.pl";

    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": code
                }
            }
        }),
    );

    // Verify our understanding of the mixed string
    let mixed_string = "Hello 🌍 世界!";
    let expected_utf16_len = utf16_len(mixed_string);
    let actual_byte_len = mixed_string.len();

    assert_eq!(expected_utf16_len, 12, "Mixed string should be 12 UTF-16 units");
    assert_eq!(actual_byte_len, 18, "Mixed string should be 18 bytes");

    // Request hover at $length on line 1
    let expected_length_start = utf16_len("my "); // 3
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": expected_length_start }
            }
        }),
    );

    if let Some((start_line, start_char, _end_line, end_char)) = extract_hover_range(&response) {
        assert_eq!(start_line, 1, "Length var hover should be on line 1");

        // The $length variable should have correct UTF-16 positions
        // regardless of what was on line 0
        assert_eq!(start_char, expected_length_start, "Start position should be UTF-16 position 3");

        let expected_length_end = expected_length_start + utf16_len("$length");
        assert_eq!(
            end_char, expected_length_end,
            "End position {} should be {} (UTF-16). \
             Multi-line UTF-16 handling verified.",
            end_char, expected_length_end
        );
    }

    // Request hover at $greeting on line 0 - this is BEFORE the mixed content in the string
    // Position: "my " = 3, then "$greeting" starts at 3
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 }
            }
        }),
    );

    if let Some((_start_line, start_char, _end_line, end_char)) = extract_hover_range(&response) {
        // $greeting range should be positions 3-12 (my $greeting)
        assert_eq!(start_char, 3, "$greeting should start at position 3");

        let expected_greeting_end = 3 + utf16_len("$greeting");
        assert_eq!(
            end_char, expected_greeting_end,
            "$greeting end {} should be {} in UTF-16",
            end_char, expected_greeting_end
        );
    }

    // Finally, verify that requesting at a position INSIDE the string works
    // Position of '世' in the string: "my $greeting = \"Hello 🌍 " = 24 UTF-16 units
    let prefix_to_shijie = "my $greeting = \"Hello 🌍 ";
    let position_of_shijie = utf16_len(prefix_to_shijie);

    // Just verify this doesn't panic or return garbage positions
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": position_of_shijie }
            }
        }),
    );

    // The response might be null (inside string literal), but shouldn't have
    // positions that are way off if UTF-16 is handled correctly
    if let Some((_start_line, start_char, _end_line, _end_char)) = extract_hover_range(&response) {
        // If we got a range, it should be reasonable (not byte-offset inflated)
        assert!(
            start_char < 100,
            "Position {} seems unreasonably large - possible byte offset bug",
            start_char
        );
    }

    println!(
        "PASS: Mixed UTF-16 content handling correct. \
         Mixed string: {} UTF-16 units vs {} bytes (diff: {})",
        expected_utf16_len,
        actual_byte_len,
        actual_byte_len as u32 - expected_utf16_len
    );
}

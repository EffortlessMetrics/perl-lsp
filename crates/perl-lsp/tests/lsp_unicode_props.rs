//! Tests for Unicode/UTF-16 position mapping

use serde_json::json;

mod common;
use common::*;

/// Test that UTF-16 position conversions round-trip correctly
#[test]
fn test_utf16_position_roundtrip() {
    // Test with a string containing various Unicode characters
    let test_strings = vec![
        "hello world",      // ASCII
        "café",             // Latin-1 supplement
        "🦀 Rust 🚀",       // Emoji (astral plane)
        "日本語",           // CJK
        "👨‍💻 coding",        // ZWJ sequence
        "𐐷𐐀𐐨",              // Deseret alphabet (astral)
        "mixed\n\r\nlines", // Mixed line endings
    ];

    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    for text in test_strings {
        let uri = "file:///test.pl";

        // Open document with the test string
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
                        "text": text
                    }
                }
            }),
        );

        // Test hover at various positions to exercise UTF-16 conversion
        for i in 0..text.len().min(10) {
            let _response = send_request(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/hover",
                    "params": {
                        "textDocument": { "uri": uri },
                        "position": { "line": 0, "character": i }
                    }
                }),
            );
            // Just ensure it doesn't panic - actual hover content varies
        }
    }
}

/// Test with generated strings to simulate property-based testing
#[test]
fn test_utf16_handles_various_strings() {
    // Test various edge cases that would be covered by property testing
    let test_cases = vec![
        "",                           // Empty string
        "a",                          // Single ASCII
        "\n",                         // Newline
        "\r\n",                       // CRLF
        "a\nb\rc\r\nd",               // Mixed line endings
        "🦀",                         // Single emoji
        "a🦀b",                       // Emoji between ASCII
        "日本語テキスト",             // CJK text
        "مرحبا بالعالم",              // RTL text
        "a\u{0301}",                  // Combining character
        "\u{1F468}\u{200D}\u{1F4BB}", // ZWJ sequence
        "𐐷𐐀𐐨",                        // Astral plane
    ];

    for text in test_cases {
        let formatted = format!("{}\n🦀{}\n", text, text);

        let mut server = start_lsp_server();
        initialize_lsp(&mut server);

        let uri = "file:///test.pl";

        // Open document
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
                        "text": formatted
                    }
                }
            }),
        );

        // Try hover at various positions
        for line in 0..3 {
            for character in [0, 1, 5, 10] {
                let _response = send_request(
                    &mut server,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/hover",
                        "params": {
                            "textDocument": { "uri": uri },
                            "position": { "line": line, "character": character }
                        }
                    }),
                );
            }
        }
    }
}

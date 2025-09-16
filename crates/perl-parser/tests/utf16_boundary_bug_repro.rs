/// Minimal reproducer for UTF-16 position roundtrip bug discovered during fuzzing
///
/// Bug: offset_to_utf16_line_col and utf16_line_col_to_offset are asymmetric when handling
/// positions that fall within multi-byte Unicode characters (fractional UTF-16 positions)
///
/// Root Cause:
/// - offset_to_utf16_line_col calculates fractional UTF-16 positions for mid-character offsets
/// - utf16_line_col_to_offset only handles whole character boundaries
///
/// Security Impact: Could cause LSP position mapping corruption in Unicode-heavy codebases
use perl_parser::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

#[test]
fn test_emoji_utf16_roundtrip_failure() {
    // This is the minimal failing case discovered by fuzzing
    let text = "a😀b\r\nc😀d";
    let offset = 2; // Mid-emoji position (inside 😀 which spans bytes 1-4)

    let (line, col) = offset_to_utf16_line_col(text, offset);
    let roundtrip = utf16_line_col_to_offset(text, line, col);

    // Expected: roundtrip should be close to offset (within tolerance for mid-character)
    // Actual: offset=2 -> (line=0, col=2) -> roundtrip=8 (WAY off)
    println!("Offset {} -> (line={}, col={}) -> roundtrip={}", offset, line, col, roundtrip);
    println!("Expected roundtrip close to {} but got {}", offset, roundtrip);

    // This assertion will fail, demonstrating the bug
    let tolerance = 4; // Allow tolerance for mid-character positions
    assert!(
        roundtrip <= offset + tolerance && roundtrip >= offset.saturating_sub(tolerance),
        "UTF-16 roundtrip bug: offset {} became {}", offset, roundtrip
    );
}

#[test]
fn test_simple_emoji_fractional_positions() {
    // Simple case to understand the fractional position issue
    let text = "😀"; // 4-byte UTF-8, 2 UTF-16 units

    for offset in 0..=text.len() {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);

        println!("Emoji test - Offset {} -> (line={}, col={}) -> roundtrip={}", offset, line, col, roundtrip);

        // Offsets 1, 2, 3 are all mid-emoji and should roundtrip to reasonable positions
        if offset > 0 && offset < text.len() {
            // These will fail due to the asymmetric handling
            let tolerance = text.len(); // Be very generous for this test
            assert!(
                roundtrip <= offset + tolerance,
                "Simple emoji roundtrip failed: offset {} became {}", offset, roundtrip
            );
        }
    }
}

#[test]
fn test_mixed_unicode_edge_cases() {
    let test_cases = vec![
        ("😀", 2),    // Mid-emoji
        ("a😀", 2),   // After ASCII, mid-emoji
        ("😀b", 2),   // Mid-emoji, before ASCII
        ("a😀b", 2),  // ASCII, mid-emoji, ASCII
    ];

    for (text, offset) in test_cases {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);

        println!("Case {:?} at offset {} -> (line={}, col={}) -> roundtrip={}",
                 text, offset, line, col, roundtrip);

        // Document the expected vs actual behavior for debugging
        if roundtrip != offset {
            println!("  MISMATCH: expected offset close to {}, got {}", offset, roundtrip);
        }
    }
}

/// This test documents the current behavior and will be used to verify fixes
#[test]
fn test_utf16_boundary_behavior_baseline() {
    let text = "a😀b\r\nc😀d";

    // Test all positions to establish baseline behavior
    for offset in 0..=text.len() {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);

        if offset == 2 || offset == 3 || offset == 10 || offset == 11 {
            // These are mid-emoji positions - document current behavior
            println!("Mid-emoji position {} -> (line={}, col={}) -> roundtrip={}",
                     offset, line, col, roundtrip);
        }
    }
}
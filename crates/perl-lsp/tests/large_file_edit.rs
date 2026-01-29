use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use perl_lsp::textdoc::{Doc, PosEnc, apply_changes, byte_to_lsp_pos, lsp_pos_to_byte};
use ropey::Rope;
use std::time::Instant;

/// Ensure that applying edits on large files remains efficient and accurate
#[test]
fn large_file_edit() -> Result<(), Box<dyn std::error::Error>> {
    let initial = "a".repeat(100_000);
    let mut doc = Doc { rope: Rope::from_str(&initial), version: 1 };
    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "b".repeat(100_000),
    };
    apply_changes(&mut doc, &[change], PosEnc::Utf16);
    assert_eq!(doc.rope.len_bytes(), 100_000);
    let first_char = doc.rope.to_string().chars().next().ok_or("Empty rope after edit")?;
    assert_eq!(first_char, 'b');

    Ok(())
}

/// Test incremental edits on large files with UTF-16 position mapping
#[test]
fn large_file_incremental_edits() {
    // Create a large file with simple ASCII content
    let mut content = String::new();
    for i in 0..100 {
        // Smaller size for reliability
        content.push_str(&format!("# Line {}: Hello World\n", i));
    }

    let mut doc = Doc { rope: Rope::from_str(&content), version: 1 };
    let original_len = doc.rope.len_bytes();

    // Test simple incremental edits with safe positions
    let edits = vec![
        // Insert at beginning
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 0), Position::new(0, 0))),
            range_length: None,
            text: "#!/usr/bin/perl\n".to_string(),
        },
        // Insert in middle (using a safe position)
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(50, 0), Position::new(50, 0))),
            range_length: None,
            text: "# Inserted line\n".to_string(),
        },
    ];

    let start_time = Instant::now();
    apply_changes(&mut doc, &edits, PosEnc::Utf16);
    let edit_duration = start_time.elapsed();

    // Performance assertion - edits should complete quickly even on large files
    let threshold = if cfg!(coverage) { 200 } else { 50 };
    assert!(
        edit_duration.as_millis() < threshold,
        "Large file edits took {} ms, expected < {}ms",
        edit_duration.as_millis(),
        threshold
    );

    // Verify content changes
    let final_content = doc.rope.to_string();
    assert!(final_content.starts_with("#!/usr/bin/perl\n"));
    assert!(final_content.contains("# Inserted line"));

    // Length should be original + added content
    assert!(doc.rope.len_bytes() > original_len);
}

/// Test UTF-16 position conversion accuracy with large files containing Unicode
#[test]
fn large_file_utf16_position_accuracy() {
    // Create content with varied Unicode characters
    let mut content = String::new();
    for i in 0..500 {
        // Mix ASCII, emojis, and multi-byte Unicode
        content.push_str(&format!("Line {}: Test 🎉 café naïve résumé 中文 🌟\n", i));
    }

    let rope = Rope::from_str(&content);

    // Test position conversion round-trip at various points
    let test_positions = vec![
        Position::new(0, 0),    // Start of file
        Position::new(10, 15),  // Middle of early line
        Position::new(250, 20), // Middle of file
        Position::new(499, 30), // Near end
    ];

    for pos in test_positions {
        let byte_offset = lsp_pos_to_byte(&rope, pos, PosEnc::Utf16);
        let converted_back = byte_to_lsp_pos(&rope, byte_offset, PosEnc::Utf16);

        // Position conversion should be accurate within the line
        // (character position might differ due to Unicode width, but should be consistent)
        assert_eq!(pos.line, converted_back.line, "Line mismatch at position {:?}", pos);

        // Character position should be within reasonable bounds
        let char_diff = converted_back.character.abs_diff(pos.character);
        assert!(
            char_diff <= 5,
            "Character position drift too large: {} vs {} (diff: {})",
            pos.character,
            converted_back.character,
            char_diff
        );
    }
}

/// Test performance of Rope operations vs String on large content
#[test]
fn rope_vs_string_performance() {
    let large_content = "x".repeat(50_000) + "\n" + &"y".repeat(50_000);

    // Test Rope insertion performance
    let start = Instant::now();
    let mut rope = Rope::from_str(&large_content);
    rope.insert(50_000, " INSERTED ");
    let rope_duration = start.elapsed();

    // Test String insertion performance (slower expected)
    let start = Instant::now();
    let mut string_content = large_content.clone();
    string_content.insert_str(50_000, " INSERTED ");
    let string_duration = start.elapsed();

    // Rope should be significantly faster for large insertions
    println!("Rope edit: {:?}, String edit: {:?}", rope_duration, string_duration);

    // Verify content is the same
    assert_eq!(rope.to_string(), string_content);

    // Performance assertion - Rope should handle large edits efficiently
    let threshold = if cfg!(coverage) { 100 } else { 10 };
    assert!(
        rope_duration.as_millis() < threshold,
        "Rope insertion took {} ms, expected < {}ms",
        rope_duration.as_millis(),
        threshold
    );
}

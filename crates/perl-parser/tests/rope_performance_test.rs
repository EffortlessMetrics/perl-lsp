use perl_parser::textdoc::{Doc, PosEnc, apply_changes, lsp_pos_to_byte, byte_to_lsp_pos};
use ropey::Rope;
use lsp_types::{TextDocumentContentChangeEvent, Position, Range};
use std::time::Instant;

/// Integration test for Rope performance characteristics
#[test]
fn rope_performance_characteristics() {
    // Test 1: Large document insertion performance
    println!("Testing Rope insertion performance...");
    let large_content = "x".repeat(100_000);
    
    let start = Instant::now();
    let mut rope = Rope::from_str(&large_content);
    rope.insert(50_000, " INSERTED TEXT ");
    let rope_duration = start.elapsed();
    
    println!("Rope insertion in large doc: {:?}", rope_duration);
    
    // Rope should handle large insertions efficiently (under 5ms)
    assert!(rope_duration.as_millis() < 5, 
            "Rope insertion took {} ms, expected < 5ms", rope_duration.as_millis());
    
    // Test 2: Position conversion performance
    println!("Testing position conversion performance...");
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("Line {}: Some content with Unicode 🚀 characters\n", i));
    }
    
    let rope = Rope::from_str(&content);
    
    let start = Instant::now();
    for i in 0..100 {
        let pos = Position::new(i, 10);
        let byte_offset = lsp_pos_to_byte(&rope, pos, PosEnc::Utf16);
        let _back_to_pos = byte_to_lsp_pos(&rope, byte_offset, PosEnc::Utf16);
    }
    let conversion_duration = start.elapsed();
    
    println!("100 position conversions: {:?}", conversion_duration);
    
    // Position conversions should be fast (under 10ms for 100 conversions)
    assert!(conversion_duration.as_millis() < 10,
            "Position conversions took {} ms, expected < 10ms", conversion_duration.as_millis());
    
    // Test 3: Incremental edit performance (realistic LSP scenario)
    println!("Testing incremental edit performance...");
    let mut doc = Doc { rope: Rope::from_str(&content), version: 1 };
    
    let edits = vec![
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(100, 0), Position::new(100, 0))),
            range_length: None,
            text: "# Inserted line 1\n".to_string(),
        },
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(500, 5), Position::new(500, 10))),
            range_length: None,
            text: "CHANGED".to_string(),
        },
        TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(800, 0), Position::new(800, 0))),
            range_length: None,
            text: "# Inserted line 2\n".to_string(),
        },
    ];
    
    let start = Instant::now();
    apply_changes(&mut doc, &edits, PosEnc::Utf16);
    let edit_duration = start.elapsed();
    
    println!("Multiple incremental edits: {:?}", edit_duration);
    
    // Incremental edits should be very fast (under 2ms)
    assert!(edit_duration.as_millis() < 2,
            "Incremental edits took {} ms, expected < 2ms", edit_duration.as_millis());
    
    println!("✅ All Rope performance characteristics meet requirements");
}

/// Test comparing Rope vs String performance for demonstration
#[test] 
fn rope_vs_string_comparison() {
    let large_content = "x".repeat(50_000);
    let insertion_text = " INSERTION ";
    
    // Rope performance
    let start = Instant::now();
    let mut rope = Rope::from_str(&large_content);
    rope.insert(25_000, insertion_text);
    let rope_time = start.elapsed();
    let rope_result = rope.to_string();
    
    // String performance (expected to be slower)
    let start = Instant::now();
    let mut string_content = large_content.clone();
    string_content.insert_str(25_000, insertion_text);
    let string_time = start.elapsed();
    
    println!("Performance comparison:");
    println!("  Rope insertion:   {:?}", rope_time);
    println!("  String insertion: {:?}", string_time);
    
    // Verify content is equivalent
    assert_eq!(rope_result, string_content);
    
    // Performance metrics (informational)
    if string_time > rope_time {
        let ratio = string_time.as_nanos() as f64 / rope_time.as_nanos() as f64;
        println!("  Rope is {:.1}x faster than String", ratio);
    } else {
        println!("  Performance difference within measurement variance");
    }
    
    println!("✅ Rope vs String comparison completed");
}
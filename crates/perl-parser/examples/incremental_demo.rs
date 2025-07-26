//! Demonstration of incremental parsing infrastructure
//!
//! This example shows how the position tracking, edit tracking, and 
//! incremental parsing features work together.

use perl_parser::{
    edit::{Edit, EditSet},
    incremental::IncrementalParser,
    position::Position,
    token_wrapper::PositionTracker,
};

fn main() {
    println!("=== Incremental Parsing Infrastructure Demo ===\n");
    
    // Demo 1: Position tracking
    demo_position_tracking();
    
    // Demo 2: Edit tracking
    demo_edit_tracking();
    
    // Demo 3: Incremental parsing
    demo_incremental_parsing();
}

fn demo_position_tracking() {
    println!("1. Position Tracking Demo");
    println!("------------------------");
    
    let source = "my $x = 42;\nprint $x;\n";
    let tracker = PositionTracker::new(source);
    
    // Show position conversions
    let positions = vec![0, 3, 7, 11, 12, 18, 21];
    
    println!("Source:\n{}", source);
    println!("\nByte positions to line:column:");
    
    for &byte in &positions {
        let pos = tracker.byte_to_position(byte);
        let ch = source.chars().nth(byte).unwrap_or('\0');
        let ch_display = if ch == '\n' { "\\n".to_string() } else { ch.to_string() };
        println!("  Byte {} ('{}') -> {}:{}", byte, ch_display, pos.line, pos.column);
    }
    
    println!("\n");
}

fn demo_edit_tracking() {
    println!("2. Edit Tracking Demo");
    println!("--------------------");
    
    // Simulate changing "42" to "4242"
    let edit = Edit::new(
        8,  // start byte (at '4')
        10, // old end byte (after '2')
        12, // new end byte (after '4242')
        Position::new(8, 1, 9),   // start position
        Position::new(10, 1, 11), // old end position
        Position::new(12, 1, 13), // new end position
    );
    
    println!("Edit: Replace \"42\" with \"4242\" at position 8");
    println!("  Byte shift: {}", edit.byte_shift());
    println!("  Line shift: {}", edit.line_shift());
    
    // Test position adjustment
    let test_positions = vec![
        (Position::new(5, 1, 6), "Before edit"),
        (Position::new(15, 2, 3), "After edit"),
        (Position::new(9, 1, 10), "Inside edit"),
    ];
    
    println!("\nPosition adjustments:");
    for (pos, desc) in test_positions {
        match edit.apply_to_position(pos) {
            Some(new_pos) => {
                println!("  {} - {}:{} -> {}:{}", 
                         desc, pos.line, pos.column, 
                         new_pos.line, new_pos.column);
            }
            None => {
                println!("  {} - {}:{} -> INVALIDATED", 
                         desc, pos.line, pos.column);
            }
        }
    }
    
    // Test edit set
    let mut edits = EditSet::new();
    edits.add(edit);
    
    // Add another edit later in the file
    edits.add(Edit::new(
        20, 22, 25,
        Position::new(20, 2, 8),
        Position::new(22, 2, 10),
        Position::new(25, 2, 13),
    ));
    
    println!("\nCumulative shift at byte 30: {}", edits.byte_shift_at(30));
    println!("\n");
}

fn demo_incremental_parsing() {
    println!("3. Incremental Parsing Demo");
    println!("--------------------------");
    
    let mut parser = IncrementalParser::new();
    
    // Initial parse
    let source1 = "my $x = 42;\nprint $x;";
    println!("Initial source:\n{}", source1);
    
    match parser.parse(source1) {
        Ok(tree) => {
            println!("\nParsed successfully!");
            println!("Root node: {:?}", tree.root.kind);
            println!("Nodes in range [0, 10]: {}", 
                     tree.find_nodes_in_range(0, 10).len());
        }
        Err(e) => {
            println!("\nParse error: {}", e);
        }
    }
    
    // Simulate an edit
    println!("\nApplying edit: change '42' to '4242'");
    parser.edit(Edit::new(
        8, 10, 12,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));
    
    let source2 = "my $x = 4242;\nprint $x;";
    match parser.parse(source2) {
        Ok(_tree) => {
            println!("Re-parsed successfully after edit!");
            
            let stats = parser.stats();
            println!("\nIncremental parser stats:");
            println!("  Has cached tree: {}", stats.has_tree);
            println!("  Pending edits: {}", stats.pending_edits);
            println!("  Reused nodes: {}", stats.reused_nodes);
            println!("  Reparsed nodes: {}", stats.reparsed_nodes);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
    
    println!("\n");
}


// Quick test of incremental parsing functionality
use perl_parser::{edit::Edit, incremental_v2::IncrementalParserV2, position::Position};

fn main() {
    println!("Testing incremental parser V2...");

    let mut parser = IncrementalParserV2::new();

    // Initial parse
    let source1 = "my $x = 42;";
    println!("Parsing: {}", source1);

    match parser.parse(source1) {
        Ok(tree) => {
            println!("Initial parse successful!");
            println!("Reparsed nodes: {}", parser.reparsed_nodes);
            println!("Reused nodes: {}", parser.reused_nodes);

            // Make an edit
            parser.edit(Edit::new(
                8,                        // start_byte: position of "42"
                10,                       // old_end_byte: end of "42"
                12,                       // new_end_byte: end of "4242"
                Position::new(8, 1, 9),   // start position
                Position::new(10, 1, 11), // old end position
                Position::new(12, 1, 13), // new end position
            ));

            let source2 = "my $x = 4242;";
            println!("Parsing after edit: {}", source2);

            match parser.parse(source2) {
                Ok(_tree2) => {
                    println!("Incremental parse successful!");
                    println!("Reparsed nodes: {}", parser.reparsed_nodes);
                    println!("Reused nodes: {}", parser.reused_nodes);

                    if parser.reused_nodes > 0 {
                        println!("✅ Node reuse is working!");
                    } else {
                        println!("⚠️  No nodes were reused (fell back to full parse)");
                    }
                }
                Err(e) => println!("Incremental parse error: {:?}", e),
            }
        }
        Err(e) => println!("Initial parse error: {:?}", e),
    }
}

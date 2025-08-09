//! Demonstration of tree reuse in incremental parsing
//!
//! This example shows how unchanged parts of the parse tree are reused
//! when making small edits to the source code.

#[cfg(feature = "incremental")]
use perl_parser::{
    edit::Edit,
    incremental::IncrementalParser,
    position::Position,
};

fn main() {
    #[cfg(not(feature = "incremental"))]
    {
        println!("This demo requires the 'incremental' feature to be enabled.");
        println!("Run with: cargo run --example incremental_tree_reuse --features incremental");
        return;
    }
    
    #[cfg(feature = "incremental")]
    {
    println!("=== Tree Reuse Demonstration ===\n");
    
    // Demo 1: Edit that doesn't affect structure
    demo_simple_edit();
    
    // Demo 2: Edit affecting only one statement
    demo_statement_edit();
    
    // Demo 3: Multiple edits
    demo_multiple_edits();
    } // End of #[cfg(feature = "incremental")] block
}

#[cfg(feature = "incremental")]
fn demo_simple_edit() {
    println!("1. Simple Value Edit (Maximum Reuse)");
    println!("-----------------------------------");
    
    let mut parser = IncrementalParser::new();
    
    // Initial parse with multiple statements
    let source1 = r#"
my $x = 42;
my $y = 100;
my $z = $x + $y;
print "Result: $z\n";
"#.trim();
    
    println!("Initial source:\n{}\n", source1);
    
    match parser.parse(source1) {
        Ok(_) => {
            let stats = parser.stats();
            println!("Initial parse:");
            println!("  Total nodes: {}", stats.reparsed_nodes);
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Change just the value 42 to 4242
    println!("\nChanging '42' to '4242'...");
    parser.edit(Edit::new(
        8, 10, 12,  // "42" -> "4242"
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));
    
    let source2 = r#"
my $x = 4242;
my $y = 100;
my $z = $x + $y;
print "Result: $z\n";
"#.trim();
    
    match parser.parse(source2) {
        Ok(_) => {
            let stats = parser.stats();
            println!("\nIncremental parse:");
            println!("  Reused nodes: {}", stats.reused_nodes);
            println!("  Reparsed nodes: {}", stats.reparsed_nodes);
            
            if stats.reused_nodes > 0 {
                let reuse_percent = (stats.reused_nodes as f64 / 
                    (stats.reused_nodes + stats.reparsed_nodes) as f64) * 100.0;
                println!("  Reuse percentage: {:.1}%", reuse_percent);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n");
}

#[cfg(feature = "incremental")]
fn demo_statement_edit() {
    println!("2. Statement-Level Edit");
    println!("----------------------");
    
    let mut parser = IncrementalParser::new();
    
    let source1 = r#"
sub calculate {
    my $a = shift;
    my $b = shift;
    my $result = $a * $b;
    return $result;
}

my $x = calculate(5, 10);
print "Answer: $x\n";
"#.trim();
    
    println!("Initial source:\n{}\n", source1);
    
    match parser.parse(source1) {
        Ok(_) => {
            let stats = parser.stats();
            println!("Initial parse: {} nodes", stats.reparsed_nodes);
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Change multiplication to addition
    println!("\nChanging '$a * $b' to '$a + $b'...");
    
    // First need to find the position of the * operator
    // For this demo, we'll assume we know the position
    let mul_pos = source1.find(" * ").unwrap();
    parser.edit(Edit::new(
        mul_pos + 1, mul_pos + 2, mul_pos + 2,  // "*" -> "+"
        Position::new(mul_pos + 1, 4, 19),
        Position::new(mul_pos + 2, 4, 20),
        Position::new(mul_pos + 2, 4, 20),
    ));
    
    let source2 = source1.replace(" * ", " + ");
    
    match parser.parse(&source2) {
        Ok(_) => {
            let stats = parser.stats();
            println!("\nIncremental parse:");
            println!("  Reused nodes: {}", stats.reused_nodes);
            println!("  Reparsed nodes: {}", stats.reparsed_nodes);
            
            if stats.reused_nodes > 0 {
                let reuse_percent = (stats.reused_nodes as f64 / 
                    (stats.reused_nodes + stats.reparsed_nodes) as f64) * 100.0;
                println!("  Reuse percentage: {:.1}%", reuse_percent);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n");
}

#[cfg(feature = "incremental")]
fn demo_multiple_edits() {
    println!("3. Multiple Edits in Different Locations");
    println!("---------------------------------------");
    
    let mut parser = IncrementalParser::new();
    
    let source1 = r#"
my $count = 0;
for (my $i = 0; $i < 10; $i++) {
    $count += $i;
}
print "Count: $count\n";

my $name = "Alice";
print "Hello, $name!\n";
"#.trim();
    
    println!("Initial source:\n{}\n", source1);
    
    match parser.parse(source1) {
        Ok(_) => {
            let stats = parser.stats();
            println!("Initial parse: {} nodes", stats.reparsed_nodes);
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Apply two edits: change 10 to 100 and "Alice" to "Bob"
    println!("\nApplying two edits:");
    println!("  - Changing '10' to '100'");
    println!("  - Changing '\"Alice\"' to '\"Bob\"'");
    
    // Find positions (in a real system, these would be tracked)
    let ten_pos = source1.find("< 10").unwrap() + 2;
    parser.edit(Edit::new(
        ten_pos, ten_pos + 2, ten_pos + 3,  // "10" -> "100"
        Position::new(ten_pos, 2, 21),
        Position::new(ten_pos + 2, 2, 23),
        Position::new(ten_pos + 3, 2, 24),
    ));
    
    let alice_pos = source1.find("\"Alice\"").unwrap();
    parser.edit(Edit::new(
        alice_pos, alice_pos + 7, alice_pos + 5,  // "Alice" -> "Bob"
        Position::new(alice_pos, 7, 12),
        Position::new(alice_pos + 7, 7, 19),
        Position::new(alice_pos + 5, 7, 17),
    ));
    
    let source2 = source1.replace("< 10", "< 100").replace("\"Alice\"", "\"Bob\"");
    
    match parser.parse(&source2) {
        Ok(_) => {
            let stats = parser.stats();
            println!("\nIncremental parse with multiple edits:");
            println!("  Reused nodes: {}", stats.reused_nodes);
            println!("  Reparsed nodes: {}", stats.reparsed_nodes);
            
            if stats.reused_nodes > 0 {
                let reuse_percent = (stats.reused_nodes as f64 / 
                    (stats.reused_nodes + stats.reparsed_nodes) as f64) * 100.0;
                println!("  Reuse percentage: {:.1}%", reuse_percent);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
    
    println!("\n");
}
//! Demonstration of working incremental parsing with actual tree reuse
//!
//! This example shows real tree reuse in action for value edits.

#[cfg(feature = "incremental")]
use perl_parser::{edit::Edit, incremental_v2::IncrementalParserV2, position::Position};
#[cfg(feature = "incremental")]
use std::time::Instant;

fn main() {
    #[cfg(not(feature = "incremental"))]
    {
        println!("This demo requires the 'incremental' feature to be enabled.");
        println!("Run with: cargo run --example incremental_working --features incremental");
        return;
    }

    #[cfg(feature = "incremental")]
    {
        println!("=== Working Incremental Parsing Demo ===\n");

        // Demo 1: Simple value change
        demo_simple_value_change();

        // Demo 2: Multiple value changes
        demo_multiple_values();

        // Demo 3: Performance comparison
        demo_performance();
    } // End of #[cfg(feature = "incremental")] block
}

#[cfg(feature = "incremental")]
fn demo_simple_value_change() {
    println!("1. Simple Value Change");
    println!("---------------------");

    let mut parser = IncrementalParserV2::new();

    // Initial parse
    let source1 = "my $x = 42;\nmy $y = $x * 2;\nprint $y;";
    println!("Initial source:\n{}\n", source1);

    let tree1 = parser.parse(source1).unwrap();
    println!("Initial parse:");
    println!("  Total nodes: {}", parser.reparsed_nodes);

    // Change just the value 42 to 4242
    parser.edit(Edit::new(
        8,
        10,
        12, // "42" -> "4242"
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));

    let source2 = "my $x = 4242;\nmy $y = $x * 2;\nprint $y;";
    println!("\nAfter changing '42' to '4242':");

    let tree2 = parser.parse(source2).unwrap();
    println!("  Reused nodes: {}", parser.reused_nodes);
    println!("  Reparsed nodes: {}", parser.reparsed_nodes);

    let reuse_percent =
        (parser.reused_nodes as f64 / (parser.reused_nodes + parser.reparsed_nodes) as f64) * 100.0;
    println!("  Tree reuse: {:.1}%", reuse_percent);

    // Verify the change was applied
    println!("\nVerification: {}", tree2.to_sexp());
    println!();
}

#[cfg(feature = "incremental")]
fn demo_multiple_values() {
    println!("2. Multiple Value Changes");
    println!("------------------------");

    let mut parser = IncrementalParserV2::new();

    // More complex code with multiple values
    let source1 = r#"
my $width = 10;
my $height = 20;
my $depth = 30;
my $volume = $width * $height * $depth;
print "Volume: $volume\n";
"#
    .trim();

    println!("Initial source:\n{}\n", source1);

    parser.parse(source1).unwrap();
    let initial_nodes = parser.reparsed_nodes;
    println!("Initial parse: {} nodes", initial_nodes);

    // Change all three dimensions
    parser.edit(Edit::new(
        12,
        14,
        15, // "10" -> "100"
        Position::new(12, 1, 13),
        Position::new(14, 1, 15),
        Position::new(15, 1, 16),
    ));

    parser.edit(Edit::new(
        30,
        32,
        34, // "20" -> "200" (adjusted)
        Position::new(30, 2, 14),
        Position::new(32, 2, 16),
        Position::new(34, 2, 18),
    ));

    parser.edit(Edit::new(
        49,
        51,
        53, // "30" -> "300" (adjusted)
        Position::new(49, 3, 13),
        Position::new(51, 3, 15),
        Position::new(53, 3, 17),
    ));

    let source2 = r#"
my $width = 100;
my $height = 200;
my $depth = 300;
my $volume = $width * $height * $depth;
print "Volume: $volume\n";
"#
    .trim();

    println!("\nAfter changing dimensions to 100, 200, 300:");

    parser.parse(source2).unwrap();
    println!("  Reused nodes: {}", parser.reused_nodes);
    println!("  Reparsed nodes: {}", parser.reparsed_nodes);

    let reuse_percent =
        (parser.reused_nodes as f64 / (parser.reused_nodes + parser.reparsed_nodes) as f64) * 100.0;
    println!("  Tree reuse: {:.1}%", reuse_percent);
    println!();
}

#[cfg(feature = "incremental")]
fn demo_performance() {
    println!("3. Performance Comparison");
    println!("------------------------");

    // Generate a larger program
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!("my $var{} = {};\n", i, i * 10));
    }
    source.push_str("my $sum = 0;\n");
    for i in 0..100 {
        source.push_str(&format!("$sum += $var{};\n", i));
    }
    source.push_str("print $sum;\n");

    println!(
        "Test program: {} lines, {} bytes",
        source.lines().count(),
        source.len()
    );

    // Full parse timing
    let start = Instant::now();
    let mut full_parser = perl_parser::Parser::new(&source);
    full_parser.parse().unwrap();
    let full_parse_time = start.elapsed();

    // Incremental parse setup
    let mut inc_parser = IncrementalParserV2::new();
    inc_parser.parse(&source).unwrap();

    // Make a small edit (change one value)
    inc_parser.edit(Edit::new(
        15,
        16,
        17, // Change first "0" to "00"
        Position::new(15, 1, 16),
        Position::new(16, 1, 17),
        Position::new(17, 1, 18),
    ));

    let mut edited_source = source.clone();
    edited_source.replace_range(15..16, "00");

    // Incremental parse timing
    let start = Instant::now();
    inc_parser.parse(&edited_source).unwrap();
    let inc_parse_time = start.elapsed();

    println!("\nFull parse time: {:?}", full_parse_time);
    println!("Incremental parse time: {:?}", inc_parse_time);
    println!(
        "Speedup: {:.1}x",
        full_parse_time.as_secs_f64() / inc_parse_time.as_secs_f64()
    );

    println!("\nIncremental parse statistics:");
    println!("  Reused nodes: {}", inc_parser.reused_nodes);
    println!("  Reparsed nodes: {}", inc_parser.reparsed_nodes);

    let reuse_percent = (inc_parser.reused_nodes as f64
        / (inc_parser.reused_nodes + inc_parser.reparsed_nodes) as f64)
        * 100.0;
    println!("  Tree reuse: {:.1}%", reuse_percent);
}

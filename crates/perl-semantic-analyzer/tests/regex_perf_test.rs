//! Performance test for regex in interpolated strings
//! Run with: cargo test -p perl-semantic-analyzer --test regex_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    // Generate test code with MANY interpolated strings
    let mut code = String::from("package TestRegex;\n\nsub test {\n");

    // Generate 5000 lines of interpolated strings
    for i in 0..5000 {
        code.push_str(&format!("    my $str_{} = \"Hello $name_{} and ${{other_{}}}\";\n", i, i, i));
    }
    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());

    // Warm up
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse");

    // Benchmark
    let iterations = 10;
    let mut total_time = std::time::Duration::ZERO;
    let mut ref_count = 0;

    for _ in 0..iterations {
        let start = Instant::now();
        let extractor = SymbolExtractor::new_with_source(&code);
        let table = extractor.extract(&ast);
        let duration = start.elapsed();

        total_time += duration;
        ref_count = table.references.len();
        println!("Time: {:?}", duration);
    }

    let avg_time = total_time / iterations;
    println!("\n=== Regex Benchmark Results ===");
    println!("Average time: {:?}", avg_time);
    println!("Total references: {}", ref_count);
}

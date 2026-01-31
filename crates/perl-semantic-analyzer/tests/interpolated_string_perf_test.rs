//! Performance test for interpolated string symbol extraction
//! Run with: cargo test -p perl-semantic-analyzer --test interpolated_string_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_interpolated_string_extraction() {
    // Generate code with many interpolated strings
    let mut code = String::from("package TestPackage;\n\n");
    code.push_str("sub test_sub {\n");

    // Generate 1000 interpolated strings
    for i in 0..1000 {
        code.push_str(&format!(
            "    my $str_{} = \"Hello $name_{}, how is $thing_{}?\";\n",
            i, i, i
        ));
    }

    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());

    // Warm up
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let extractor = SymbolExtractor::new_with_source(&code);
            let _table = extractor.extract(&ast);
        }
    }

    // Benchmark
    let iterations = 10;
    let mut total_time = std::time::Duration::ZERO;
    let mut symbol_count = 0;
    let mut ref_count = 0;

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let extractor = SymbolExtractor::new_with_source(&code);
            let table = extractor.extract(&ast);
            let duration = start.elapsed();

            total_time += duration;
            symbol_count = table.symbols.len();
            ref_count = table.references.len();
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average extraction time: {:?}", avg_time);
    println!("Total symbols: {}", symbol_count);
    println!("Total references: {}", ref_count);
}

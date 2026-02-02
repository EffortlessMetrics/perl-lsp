//! Performance benchmark for symbol extraction from interpolated strings
//! Run with: cargo test -p perl-semantic-analyzer --test interpolated_string_bench -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    // Generate large test code with many interpolated strings
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 1000 interpolated strings
    for i in 0..1000 {
        code.push_str(&format!(
            r#"    my $var_{} = "Hello $name, count is $count, and value is ${{val_{}}}";
"#,
            i, i
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

    for _ in 0..iterations {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let start = Instant::now();
            let extractor = SymbolExtractor::new_with_source(&code);
            let _table = extractor.extract(&ast);
            let duration = start.elapsed();

            total_time += duration;
            println!("Iteration time: {:?}", duration);
        }
    }

    let avg_time = total_time / iterations;
    println!("\n=== Interpolated String Benchmark Results ===");
    println!("Average extraction time: {:?}", avg_time);
}

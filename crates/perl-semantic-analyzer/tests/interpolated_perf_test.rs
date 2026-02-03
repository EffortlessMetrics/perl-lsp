//! Performance test for interpolated string variable extraction
//! Run with: cargo test -p perl-semantic-analyzer --test interpolated_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    // Generate code with many interpolated strings
    let count = 5000;
    let mut code = String::with_capacity(count * 50);
    code.push_str("package TestPerf;\nsub test {\n");

    for i in 0..count {
        // Mix of simple and complex interpolation
        code.push_str(&format!(r#"    my $v{0} = "Value: $var_{0} and ${{complex_{0}}}";"#, i));
        code.push('\n');
    }
    code.push_str("}\n");

    println!("Generated code size: {} bytes", code.len());
    println!("Total interpolated strings: {}", count);

    // Warm up
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse failed");

    // Benchmark just the extraction phase
    let iterations = 5;
    let mut total_time = std::time::Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        let extractor = SymbolExtractor::new_with_source(&code);
        let table = extractor.extract(&ast);
        let duration = start.elapsed();
        total_time += duration;

        // Sanity check
        assert!(table.references.len() > count);
    }

    let avg_time = total_time / iterations;
    println!("\n=== Interpolated String Benchmark Results ===");
    println!("Average extraction time: {:?}", avg_time);
    println!("Time per string: {:?}", avg_time / count as u32);
}

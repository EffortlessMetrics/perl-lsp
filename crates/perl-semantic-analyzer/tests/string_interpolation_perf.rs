//! Performance test for string interpolation variable extraction
//! Run with: cargo test -p perl-semantic-analyzer --test string_interpolation_perf -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_string_interpolation() {
    // Generate code with many interpolated strings
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 lines with interpolated strings
    for i in 0..5000 {
        code.push_str(&format!(
            r#"    my $var_{} = "Value {}";
    print "Interpolating $var_{} and ${{var_{}}} in a string";
"#,
            i, i, i, i
        ));
    }
    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());

    // Parse once to get AST
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    // Benchmark just the symbol extraction phase
    let iterations = 5;
    let mut total_duration = std::time::Duration::ZERO;
    let mut total_refs = 0;

    println!("Starting benchmark with {} iterations...", iterations);

    for i in 0..iterations {
        let start = Instant::now();
        let extractor = SymbolExtractor::new_with_source(&code);
        let table = extractor.extract(&ast);
        let duration = start.elapsed();

        total_duration += duration;
        total_refs = table.references.len();

        println!("Iteration {}: {:?}", i + 1, duration);
    }

    let avg_duration = total_duration / iterations;
    println!("\n=== Results ===");
    println!("Average extraction time: {:?}", avg_duration);
    println!("Total references found: {}", total_refs);
    println!("Time per reference: {:?}", avg_duration / total_refs as u32);
}

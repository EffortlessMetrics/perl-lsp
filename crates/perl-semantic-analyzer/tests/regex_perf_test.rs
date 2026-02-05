
use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 interpolated strings
    for i in 0..5000 {
        code.push_str(&format!(
            "    my $v{} = \"Hello $name{} and ${{other{}}}\";\n",
            i, i, i
        ));
    }
    code.push_str("}\n");

    println!("\nCode size: {} bytes", code.len());

    // Warm up
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse");

    // Benchmark
    let iterations = 10;
    let mut total_time = std::time::Duration::ZERO;

    for _ in 0..iterations {
        let start = Instant::now();
        let extractor = SymbolExtractor::new_with_source(&code);
        let _table = extractor.extract(&ast);
        let duration = start.elapsed();
        total_time += duration;
        println!("Iteration time: {:?}", duration);
    }

    let avg_time = total_time / iterations;
    println!("\n=== Benchmark Results ===");
    println!("Average extraction time: {:?}", avg_time);
}

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_strings() {
    // Generate Perl code with many interpolated strings
    let mut code = String::from("package Bench;\n\nsub test {\n");

    // Add 1000 interpolated strings with variables
    for i in 0..1000 {
        code.push_str(&format!("    my $v{} = \"Hello $name_{}\";\n", i, i));
        code.push_str(&format!("    my $x{} = \"Value is ${{{}}}\";\n", i, i));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    // Parse once (not part of the benchmark)
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    // Warm up
    let _ = SymbolExtractor::new_with_source(&code).extract(&ast);

    // Benchmark
    let start = Instant::now();
    let _table = SymbolExtractor::new_with_source(&code).extract(&ast);
    let duration = start.elapsed();

    println!("Extraction time for 1000 interpolated strings: {:?}", duration);
}

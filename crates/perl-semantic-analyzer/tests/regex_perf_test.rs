//! Performance test for regex-based symbol extraction
//! Run with: cargo test -p perl-semantic-analyzer --test regex_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 interpolated strings
    // This forces 5000 regex compilations in the unoptimized version
    for i in 0..5000 {
        code.push_str(&format!("    my $x_{} = \"Value is $val_{} and ${{other_{}}}\";\n", i, i, i));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    // Parse once (not measuring parser perf here)
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    // Warmup
    {
        let extractor = SymbolExtractor::new_with_source(&code);
        let _ = extractor.extract(&ast);
    }

    // Measure
    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Extraction time for 5000 interpolated strings: {:?}", duration);
    println!("Total symbols: {}", table.symbols.len());
    println!("Total references: {}", table.references.len());
}

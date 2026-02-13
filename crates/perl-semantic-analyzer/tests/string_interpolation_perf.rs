//! Performance test for string interpolation symbol extraction
//! Run with: cargo test -p perl-semantic-analyzer --test string_interpolation_perf -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore] // Only run when explicitly requested
fn benchmark_string_interpolation_extraction() {
    let num_strings = 5000;
    println!("Generating {} interpolated strings...", num_strings);

    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate many interpolated strings
    // We use unique variable names to avoid deduplication confusion, although SymbolTable stores all references
    for i in 0..num_strings {
        code.push_str(&format!(
            "    my $v{} = \"Hello $name, how are you today? Also check ${{this_var}}.\";\n",
            i
        ));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    // Parse the code once
    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    println!("Starting benchmark...");
    let start = Instant::now();

    // We only measure the extraction part as that's what we are optimizing
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);

    let duration = start.elapsed();

    println!("Extraction time: {:?}", duration);
    println!("Total symbols: {}", table.symbols.len());

    // Count total reference instances
    let total_ref_instances: usize = table.references.values().map(|v| v.len()).sum();
    println!("Total reference instances: {}", total_ref_instances);

    // We expect at least num_strings * 2 references from the strings
    assert!(
        total_ref_instances >= num_strings * 2,
        "Expected at least {} references, found {}",
        num_strings * 2,
        total_ref_instances
    );
}

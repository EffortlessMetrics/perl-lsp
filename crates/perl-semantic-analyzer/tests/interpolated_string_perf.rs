use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
fn benchmark_interpolated_string_extraction() {
    let mut code = String::from("package Test;\nsub test {\n");

    // Generate 5000 interpolated strings
    for i in 0..5000 {
        code.push_str(&format!(
            "    my $v{0} = \"This string has an interpolated variable $v{0} inside it\";\n",
            i
        ));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse failed");

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Extraction time for 5000 interpolated strings: {:?}", duration);

    // Verify we found symbols
    assert!(table.symbols.len() >= 5000);

    // Check total references
    let ref_count: usize = table.references.values().map(|v| v.len()).sum();
    println!("Total references found: {}", ref_count);
    assert!(ref_count >= 5000);
}

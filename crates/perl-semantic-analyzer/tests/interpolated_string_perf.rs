use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_interpolated_string_extraction() {
    let mut code = String::from("package Test;\nsub test {\n");
    // Generate 5000 lines of interpolated strings
    for i in 0..5000 {
        code.push_str(&format!("    my $v{0} = \"start $v{0} end\";\n", i));
    }
    code.push_str("}\n");

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse failed");

    println!("Starting extraction for {} lines...", 5000);
    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Time taken: {:?}", duration);
    println!("Symbols extracted: {}", table.symbols.len());
    println!("References extracted: {}", table.references.len());
}

use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_string_interpolation_extraction() {
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 interpolated strings
    for i in 0..5000 {
        code.push_str(&format!("    my $str_{} = \"Hello $name_{}, how are you ${{{}}}\";\n", i, i, i));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse");

    // Warm up
    {
        let extractor = SymbolExtractor::new_with_source(&code);
        let _ = extractor.extract(&ast);
    }

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Extraction time with 5000 interpolated strings: {:?}", duration);
    println!("Symbols found: {}", table.symbols.len());
    println!("References found: {}", table.references.len());
}

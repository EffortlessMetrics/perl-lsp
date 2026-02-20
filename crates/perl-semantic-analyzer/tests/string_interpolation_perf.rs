use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_string_interpolation() {
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 interpolated strings
    for i in 0..5000 {
        code.push_str(&format!("    my $str_{} = \"Hello $name_{}, how are you?\";\n", i, i));
    }
    code.push_str("}\n");

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("parse");

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let _table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Time to extract symbols from 5000 interpolated strings: {:?}", duration);
}

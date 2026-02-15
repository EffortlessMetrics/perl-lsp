use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_string_interpolation() {
    // Generate a file with many interpolated strings
    let mut code = String::from("package TestPackage;\n\nsub test {\n");

    // Generate 5000 lines of interpolated strings
    for i in 0..5000 {
        code.push_str(&format!("    my $var_{} = \"Hello $name_{}, how is ${{{}}}\";\n", i, i, i));
    }
    code.push_str("}\n");

    println!("Code size: {} bytes", code.len());

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    let start = Instant::now();
    let extractor = SymbolExtractor::new_with_source(&code);
    let table = extractor.extract(&ast);
    let duration = start.elapsed();

    println!("Extraction time: {:?}", duration);

    // Verify we found symbols
    let ref_count = table.references.len();
    println!("References found: {}", ref_count);

    // We expect at least 1 reference per line (the variable being assigned to is a definition,
    // but the interpolated variables are references).
    // In "my $var = "Hello $name"", $var is a definition, $name is a reference.
    // The SymbolExtractor adds references for interpolated variables.
    assert!(ref_count > 0);
}

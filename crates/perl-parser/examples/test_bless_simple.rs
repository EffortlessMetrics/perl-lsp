//! Test simple bless cases
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // These work
        "[]",
        "bless",
        "$ref",
        "'MyClass'",
        
        // Test step by step
        "bless []",
        "bless([])",
        "bless([], 'MyClass')",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
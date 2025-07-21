//! Test package parsing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "package Test;",
        "package Test::Module;",
        "package Foo::Bar::Baz;",
    ];
    
    for code in tests {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
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
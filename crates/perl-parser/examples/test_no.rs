//! Test no statement parsing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "no strict;",
        "no warnings;",
        "no warnings 'uninitialized';",
        "no utf8;",
        "no feature;",
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

//! Test version strings and pragma arguments
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Version strings
        "use v5.36",
        "use v5.36;",
        "use 5.036",
        "use 5.036;",
        "use 5.036_001",
        
        // Pragma with arguments
        "no warnings 'void'",
        "no warnings 'void';",
        "use warnings",
        "use warnings;",
        "use strict 'refs'",
        "use feature 'say'",
        "use feature qw(say state)",
        
        // More complex
        "use v5.36; use warnings;",
        "no warnings qw(void uninitialized)",
    ];

    for test in tests {
        print!("Testing: {:40} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
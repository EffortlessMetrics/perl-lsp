//! Test BEGIN/END blocks
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic blocks
        "BEGIN { }",
        "END { }",
        "CHECK { }",
        "INIT { }",
        "UNITCHECK { }",
        
        // With code
        "BEGIN { print 'starting' }",
        "END { print 'cleanup' }",
        
        // Multiple statements
        "BEGIN { $x = 1; $y = 2; }",
        
        // In context
        "BEGIN { use strict; }",
        "END { close(FILEHANDLE); }",
        
        // Multiple blocks
        "BEGIN { } BEGIN { }",
        "END { } END { }",
        
        // Mixed with regular code
        "my $x = 1; BEGIN { print 'init' } print $x;",
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
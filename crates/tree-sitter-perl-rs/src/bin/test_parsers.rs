//! Simple test to verify both stacker and iterative parsers work

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

fn main() {
    #[cfg(not(feature = "pure-rust"))]
    {
        eprintln!("This test requires the pure-rust feature");
        std::process::exit(1);
    }
    
    #[cfg(feature = "pure-rust")]
    {
        println!("Testing parser implementations...\n");
        
        // Test 1: Simple expression
        test_parse("Simple", "$x = 42");
        
        // Test 2: Nested expression
        test_parse("Nested", "(((1 + 2) * 3) - 4)");
        
        // Test 3: Deep nesting (only in debug to show difference)
        #[cfg(debug_assertions)]
        {
            println!("\nDebug mode deep nesting test:");
            let mut expr = "42".to_string();
            for _ in 0..100 {
                expr = format!("({})", expr);
            }
            test_parse("Deep (100)", &expr);
        }
        
        println!("\n✅ All tests completed!");
    }
}

#[cfg(feature = "pure-rust")]
fn test_parse(name: &str, input: &str) {
    use std::time::Instant;
    
    print!("{:15} - ", name);
    
    let mut parser = PureRustPerlParser::new();
    let start = Instant::now();
    
    match parser.parse(input) {
        Ok(ast) => {
            let duration = start.elapsed();
            println!("Success ({:?})", duration);
            
            // Show a bit of the AST
            let sexp = parser.to_sexp(&ast);
            if sexp.len() < 100 {
                println!("  AST: {}", sexp);
            } else {
                println!("  AST: {}...", &sexp[..100]);
            }
        }
        Err(e) => {
            println!("Failed: {:?}", e);
        }
    }
}
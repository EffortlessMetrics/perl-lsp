// Simple test to verify stacker integration
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
        println!("Testing stacker integration...");
        
        // Test with increasing depths
        for depth in [100, 500, 1000, 1500, 2000] {
            print!("Testing depth {}: ", depth);
            
            let mut expr = "1".to_string();
            for _ in 0..depth {
                expr = format!("({})", expr);
            }
            
            let mut parser = PureRustPerlParser::new();
            match parser.parse(&expr) {
                Ok(_) => println!("✅ Success"),
                Err(e) => {
                    println!("❌ Failed: {:?}", e);
                    break;
                }
            }
        }
    }
}
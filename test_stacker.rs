// Simple test to verify stacker integration
use tree_sitter_perl::pure_rust_parser::parse_perl;

fn main() {
    println!("Testing stacker integration...");
    
    // Test with increasing depths
    for depth in [100, 500, 1000, 1500, 2000] {
        print!("Testing depth {}: ", depth);
        
        let mut expr = "1".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }
        
        match parse_perl(&expr) {
            Ok(_) => println!("✅ Success"),
            Err(e) => {
                println!("❌ Failed: {:?}", e);
                break;
            }
        }
    }
}
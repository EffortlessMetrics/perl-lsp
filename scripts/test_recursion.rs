#!/usr/bin/env rust-script
//! Quick script to test recursion depth
//! ```cargo
//! [dependencies]
//! tree-sitter-perl = { path = "../crates/tree-sitter-perl-rs", features = ["pure-rust"] }
//! ```

use tree_sitter_perl::pure_rust_parser::parse_perl;

fn main() {
    println!("Testing recursion depth for Pure Rust parser...");
    
    // Test different depths
    for depth in [10, 50, 100, 200, 500, 1000, 1500] {
        println!("\nTesting depth: {}", depth);
        
        // Create nested expression
        let mut expr = "1".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }
        
        println!("Expression length: {} bytes", expr.len());
        
        match parse_perl(&expr) {
            Ok(_) => println!("✅ Successfully parsed at depth {}", depth),
            Err(e) => {
                println!("❌ Failed at depth {}: {:?}", depth, e);
                break;
            }
        }
    }
}
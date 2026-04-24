#!/usr/bin/env rust-script
//! Quick script to test recursion depth
//! ```cargo
//! [dependencies]
//! perl-parser-pest = { path = "../crates/perl-parser-pest" }
//! ```

use perl_parser_pest::PureRustPerlParser;

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

        let mut parser = PureRustPerlParser::new();
        match parser.parse(&expr) {
            Ok(_) => println!("✅ Successfully parsed at depth {}", depth),
            Err(e) => {
                println!("❌ Failed at depth {}: {:?}", depth, e);
                break;
            }
        }
    }
}
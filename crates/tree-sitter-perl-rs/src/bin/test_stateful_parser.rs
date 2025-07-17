//! Test binary for the stateful parser

use tree_sitter_perl::stateful_parser::StatefulPerlParser;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <file.pl>", args[0]);
        std::process::exit(1);
    }
    
    let filename = &args[1];
    let content = fs::read_to_string(filename)?;
    
    println!("=== Source ===");
    println!("{}", content);
    println!();
    
    let mut parser = StatefulPerlParser::new();
    
    match parser.parse(&content) {
        Ok(ast) => {
            println!("=== Parse Success ===");
            // TODO: Print S-expression representation
            println!("{:#?}", ast);
        }
        Err(e) => {
            println!("=== Parse Error ===");
            println!("{}", e);
        }
    }
    
    Ok(())
}
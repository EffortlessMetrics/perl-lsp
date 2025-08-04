//! Example: Parse a Perl file and print the AST

use perl_parser::{Parser, ParseOptions};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <perl-file>", args[0]);
        process::exit(1);
    }
    
    let filename = &args[1];
    
    // Read the file
    let source = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    
    // Parse the source
    let options = ParseOptions::default();
    match Parser::parse(&source, options) {
        Ok(ast) => {
            // Print as S-expression (tree-sitter format)
            println!("{}", ast.to_sexp());
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    }
}
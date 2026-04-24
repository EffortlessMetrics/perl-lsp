//! Command-line tool for parsing Perl files with the Pure Rust parser

use std::env;
use std::fs;
use std::io::{self, Read};
use tree_sitter_perl::full_parser::FullPerlParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let (input, filename) = if args.len() < 2 || args[1] == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        (buffer, "<stdin>".to_string())
    } else {
        // Read from file
        let filename = &args[1];
        let content = fs::read_to_string(filename)?;
        (content, filename.clone())
    };

    // Check for debug flag
    let debug = args.contains(&"--debug".to_string());

    // Create parser
    let mut parser = FullPerlParser::new();

    // Parse the input
    match parser.parse_to_sexp(&input) {
        Ok(sexp) => {
            if debug {
                eprintln!("✓ Successfully parsed {}", filename);
                eprintln!("  Input size: {} bytes", input.len());
            }

            // Output S-expression
            println!("{}", sexp);

            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Parse error in {}: {}", filename, e);
            std::process::exit(1);
        }
    }
}

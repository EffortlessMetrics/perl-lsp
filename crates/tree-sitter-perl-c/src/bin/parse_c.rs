use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <perl_file>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    let source_code = match fs::read_to_string(filename) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_perl_c::language()).expect("Error loading Perl C grammar");

    match parser.parse(&source_code, None) {
        Some(_tree) => {
            // Success - just exit with 0
            // In a real parser we'd output the S-expression, but for benchmarking we just parse
            std::process::exit(0);
        }
        None => {
            eprintln!("Failed to parse");
            process::exit(1);
        }
    }
}

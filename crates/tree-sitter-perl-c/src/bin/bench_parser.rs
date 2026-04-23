use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::parse_perl_code;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bench_parser_c <file>");
        std::process::exit(1);
    }
    let file_path = &args[1];
    let code = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file: {}", e);
        std::process::exit(1);
    });
    let start = Instant::now();
    let result = parse_perl_code(&code);
    let duration = start.elapsed().as_micros();
    match result {
        Ok(tree) => {
            let has_error = tree.root_node().has_error();
            println!("status=success error={} duration_us={}", has_error, duration);
            // Always return success (0) - parse errors are indicated in the error field
        }
        Err(e) => {
            println!("status=failure error=true duration_us={}", duration);
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}

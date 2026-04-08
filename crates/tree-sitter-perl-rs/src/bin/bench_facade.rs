//! Benchmark binary for the `tree-sitter-perl-rs` facade.
//!
//! Measures wall-clock parse time through the tree-sitter-style ergonomic
//! facade that wraps the v3 native Rust parser (`perl-parser-core`).
//!
//! # Output format
//!
//! ```text
//! status=success error=false duration_us=N
//! ```
//!
//! This matches the output format of `bench_parser_c` and `perl-parser-bench`
//! so that `perl-ci-hygiene`'s `quick-bench` command can consume all three with
//! the same parsing logic.
//!
//! # Usage
//!
//! ```text
//! bench_facade <file>
//! ```

use std::env;
use std::fs;
use std::time::Instant;

use tree_sitter_perl_rs::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bench_facade <file>");
        std::process::exit(1);
    }
    let file_path = &args[1];
    let code = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file: {}", e);
        std::process::exit(1);
    });

    let mut parser = Parser::new();
    let start = Instant::now();
    let result = parser.parse(&code);
    let duration = start.elapsed().as_micros();

    match result {
        Some(_tree) => {
            // The v3 parser is error-tolerant; a successful parse may still
            // contain recovered error nodes. We report error=false here because
            // the facade returned a tree (same convention as bench_parser_c).
            println!("status=success error=false duration_us={}", duration);
        }
        None => {
            println!("status=failure error=true duration_us={}", duration);
            std::process::exit(1);
        }
    }
}

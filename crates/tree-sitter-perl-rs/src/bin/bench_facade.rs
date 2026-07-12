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
// Benchmark binary — println!/eprintln! are used for structured output consumed by bench harness.
#![allow(clippy::print_stderr, clippy::print_stdout)]

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
    let result = parser.parse_detailed(&code);
    let duration = start.elapsed().as_micros();
    let has_error = result.has_error();

    match result.tree {
        Some(_tree) => {
            println!("status=success error={} duration_us={}", has_error, duration);
        }
        None => {
            println!("status=failure error=true duration_us={}", duration);
            std::process::exit(1);
        }
    }
}

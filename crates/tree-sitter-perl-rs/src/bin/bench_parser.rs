//! Benchmark binary for the Rust implementation
//!
//! This binary is used by xtask to benchmark the Rust parser implementation.

use std::env;
use tree_sitter_perl::parse;

fn main() {
    let test_code = env::var("TEST_CODE").expect("TEST_CODE environment variable not set");

    // Parse the test code multiple times to get accurate timing
    for _ in 0..100 {
        let _tree = parse(&test_code).expect("Failed to parse test code");
    }
}

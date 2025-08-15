//! Benchmark binary for the Rust implementation
//!
//! This binary is used by xtask to benchmark the Rust parser implementation.

use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tree_sitter_perl::parse;
use walkdir::WalkDir;

fn process_file(file_path: &Path) -> (bool, u128) {
    let code = fs::read_to_string(file_path).expect("Failed to read file");
    let start = Instant::now();
    let result = parse(&code);
    let duration = start.elapsed().as_micros();
    match result {
        Ok(tree) => {
            let has_error = tree.root_node().has_error();
            (has_error, duration)
        }
        Err(_) => (true, duration),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bench_parser <file_or_directory>");
        std::process::exit(1);
    }
    let path = Path::new(&args[1]);

    if path.is_file() {
        let (has_error, duration) = process_file(path);
        println!("status=success error={} duration_us={}", has_error, duration);
    } else if path.is_dir() {
        let mut total_files = 0;
        let mut error_files = 0;
        let mut total_duration = 0;

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let (has_error, duration) = process_file(entry.path());
            total_files += 1;
            if has_error {
                error_files += 1;
            }
            total_duration += duration;
        }

        let success_rate = if total_files > 0 {
            (total_files - error_files) as f64 / total_files as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "total_files={} error_files={} success_rate={:.1} total_duration_us={}",
            total_files, error_files, success_rate, total_duration
        );
    } else {
        eprintln!("Path does not exist: {}", path.display());
        std::process::exit(1);
    }
}

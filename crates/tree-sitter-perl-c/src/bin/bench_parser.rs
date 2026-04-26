use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::{PerlParser, parse_perl_code};

#[derive(Clone, Copy)]
enum Mode {
    Wrapper,
    ReuseParser,
}

impl Mode {
    fn from_arg(value: &str) -> Option<Self> {
        match value {
            "wrapper" => Some(Self::Wrapper),
            "reuse-parser" => Some(Self::ReuseParser),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Wrapper => "wrapper",
            Self::ReuseParser => "reuse-parser",
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 4 {
        eprintln!("Usage: bench_parser_c <file> [wrapper|reuse-parser] [iterations]");
        std::process::exit(1);
    }
    let file_path = &args[1];
    let mode =
        args.get(2).map(String::as_str).map_or(Some(Mode::Wrapper), Mode::from_arg).unwrap_or_else(
            || {
                eprintln!("Invalid mode. Expected one of: wrapper, reuse-parser");
                std::process::exit(1);
            },
        );
    let iterations =
        args.get(3).map(String::as_str).map_or(Ok(100_usize), str::parse::<usize>).unwrap_or_else(
            |_| {
                eprintln!("Iterations must be a positive integer");
                std::process::exit(1);
            },
        );
    if iterations == 0 {
        eprintln!("Iterations must be greater than zero");
        std::process::exit(1);
    }

    let code = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file: {}", e);
        std::process::exit(1);
    });

    let start = Instant::now();
    let mut had_error_node = false;
    match mode {
        Mode::Wrapper => {
            for _ in 0..iterations {
                let tree = parse_perl_code(&code).unwrap_or_else(|e| {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                });
                had_error_node = tree.root_node().has_error();
            }
        }
        Mode::ReuseParser => {
            let mut parser = PerlParser::new().unwrap_or_else(|e| {
                eprintln!("Failed to create parser: {}", e);
                std::process::exit(1);
            });
            for _ in 0..iterations {
                let tree = parser.parse_code(&code).unwrap_or_else(|e| {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                });
                had_error_node = tree.root_node().has_error();
            }
        }
    }

    let duration = start.elapsed().as_micros();
    let avg_duration = duration as f64 / iterations as f64;
    println!(
        "status=success mode={} iterations={} error={} duration_us={} avg_us={:.2}",
        mode.as_str(),
        iterations,
        had_error_node,
        duration,
        avg_duration
    );
    // Always return success (0) - parse errors are indicated in the error field
}

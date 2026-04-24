use std::env;
use std::fs;
use std::process;
use std::time::Instant;
use tree_sitter_perl_c::{parse_perl_code, parse_perl_code_with_parser, try_create_parser};

#[derive(Copy, Clone)]
enum BenchMode {
    Wrapper,
    ReuseParser,
}

impl BenchMode {
    fn parse(mode: &str) -> Result<Self, String> {
        match mode {
            "wrapper" => Ok(Self::Wrapper),
            "reuse-parser" => Ok(Self::ReuseParser),
            other => {
                Err(format!("unknown mode '{other}', expected one of: wrapper | reuse-parser"))
            }
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Wrapper => "wrapper",
            Self::ReuseParser => "reuse-parser",
        }
    }
}

fn parse_u32(name: &str, value: &str) -> Result<u32, String> {
    value.parse::<u32>().map_err(|e| format!("invalid {name} '{value}': {e}"))
}

fn print_usage(bin: &str) {
    eprintln!("Usage: {bin} <file> [mode] [iterations]");
    eprintln!("  mode: wrapper | reuse-parser (default: wrapper)");
    eprintln!("  iterations: positive integer (default: 1)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 4 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let file_path = &args[1];
    let mode = match args.get(2) {
        Some(raw_mode) => match BenchMode::parse(raw_mode) {
            Ok(parsed_mode) => parsed_mode,
            Err(message) => {
                eprintln!("{message}");
                print_usage(&args[0]);
                process::exit(1);
            }
        },
        None => BenchMode::Wrapper,
    };

    let iterations = match args.get(3) {
        Some(raw_iterations) => match parse_u32("iterations", raw_iterations) {
            Ok(parsed_iterations) if parsed_iterations > 0 => parsed_iterations,
            Ok(_) => {
                eprintln!("iterations must be greater than 0");
                process::exit(1);
            }
            Err(message) => {
                eprintln!("{message}");
                process::exit(1);
            }
        },
        None => 1,
    };

    let code = match fs::read_to_string(file_path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Failed to read file: {e}");
            process::exit(1);
        }
    };

    let start = Instant::now();
    let mut last_has_error = false;

    match mode {
        BenchMode::Wrapper => {
            for _ in 0..iterations {
                match parse_perl_code(&code) {
                    Ok(tree) => {
                        last_has_error = tree.root_node().has_error();
                    }
                    Err(error) => {
                        fail(mode, iterations, start.elapsed().as_micros(), error.to_string());
                    }
                }
            }
        }
        BenchMode::ReuseParser => {
            let mut parser = match try_create_parser() {
                Ok(configured_parser) => configured_parser,
                Err(error) => {
                    fail(mode, iterations, start.elapsed().as_micros(), error.to_string());
                }
            };

            for _ in 0..iterations {
                match parse_perl_code_with_parser(&mut parser, &code) {
                    Ok(tree) => {
                        last_has_error = tree.root_node().has_error();
                    }
                    Err(error) => {
                        fail(mode, iterations, start.elapsed().as_micros(), error.to_string());
                    }
                }
            }
        }
    }

    let duration_us = start.elapsed().as_micros();
    println!(
        "status=success mode={} iterations={} error={} duration_us={}",
        mode.as_str(),
        iterations,
        last_has_error,
        duration_us
    );
}

fn fail(mode: BenchMode, iterations: u32, duration_us: u128, error: String) -> ! {
    println!(
        "status=failure mode={} iterations={} error=true duration_us={}",
        mode.as_str(),
        iterations,
        duration_us
    );
    eprintln!("Parse error: {error}");
    process::exit(1);
}

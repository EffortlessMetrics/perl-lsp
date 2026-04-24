use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::{parse_perl_code, try_create_parser};

#[derive(Clone, Copy, Debug)]
enum BenchMode {
    Wrapper,
    FreshParser,
    ReusedParser,
}

impl BenchMode {
    fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "wrapper" => Some(Self::Wrapper),
            "fresh-parser" => Some(Self::FreshParser),
            "reused-parser" => Some(Self::ReusedParser),
            _ => None,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 4 {
        eprintln!(
            "Usage: bench_parser_c <file> [mode] [iterations]\n\
             modes: wrapper | fresh-parser | reused-parser"
        );
        std::process::exit(1);
    }
    let file_path = &args[1];
    let mode =
        args.get(2).map(String::as_str).and_then(BenchMode::from_arg).unwrap_or(BenchMode::Wrapper);
    let iterations = args.get(3).and_then(|arg| arg.parse::<u32>().ok()).unwrap_or(1_000);
    if iterations == 0 {
        eprintln!("iterations must be > 0");
        std::process::exit(1);
    }

    let code = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file: {}", e);
        std::process::exit(1);
    });

    let start = Instant::now();
    let mut error_trees = 0_u32;
    match mode {
        BenchMode::ReusedParser => {
            let mut parser = try_create_parser().unwrap_or_else(|e| {
                eprintln!("Failed to initialize parser: {}", e);
                std::process::exit(1);
            });
            for _ in 0..iterations {
                match parser.parse(code.as_bytes(), None) {
                    Some(tree) => {
                        if tree.root_node().has_error() {
                            error_trees += 1;
                        }
                    }
                    None => {
                        let duration = start.elapsed().as_micros();
                        println!(
                            "status=failure mode={:?} iterations={} error=true duration_us={}",
                            mode, iterations, duration
                        );
                        eprintln!("Parse error: Failed to parse code");
                        std::process::exit(1);
                    }
                }
            }
        }
        BenchMode::Wrapper | BenchMode::FreshParser => {
            for _ in 0..iterations {
                let has_error = match mode {
                    BenchMode::Wrapper => parse_with_wrapper(&code),
                    BenchMode::FreshParser => parse_with_fresh_parser(&code),
                    BenchMode::ReusedParser => unreachable!(),
                };

                match has_error {
                    Ok(has_error) => {
                        if has_error {
                            error_trees += 1;
                        }
                    }
                    Err(e) => {
                        let duration = start.elapsed().as_micros();
                        println!(
                            "status=failure mode={:?} iterations={} error=true duration_us={}",
                            mode, iterations, duration
                        );
                        eprintln!("Parse error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    let duration = start.elapsed().as_micros();
    println!(
        "status=success mode={:?} iterations={} error_trees={} duration_us={} avg_us_per_iter={:.3}",
        mode,
        iterations,
        error_trees,
        duration,
        duration as f64 / f64::from(iterations),
    );
}

fn parse_with_wrapper(code: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let tree = parse_perl_code(code)?;
    Ok(tree.root_node().has_error())
}

fn parse_with_fresh_parser(code: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;
    match parser.parse(code.as_bytes(), None) {
        Some(tree) => Ok(tree.root_node().has_error()),
        None => Err("Failed to parse code".into()),
    }
}

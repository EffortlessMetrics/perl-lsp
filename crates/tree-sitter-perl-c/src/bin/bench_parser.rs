use std::env;
use std::fs;
use std::time::Instant;

use tree_sitter::{Parser, Tree};
use tree_sitter_perl_c::{parse_perl_bytes, parse_perl_code, try_create_parser};

#[derive(Clone, Copy)]
enum BenchMode {
    Cold,
    Warm,
}

impl BenchMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Warm => "warm",
        }
    }
}

#[derive(Clone, Copy)]
enum InputMode {
    Str,
    Bytes,
}

impl InputMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::Bytes => "bytes",
        }
    }
}

struct Config {
    file_path: String,
    mode: BenchMode,
    input: InputMode,
    iterations: u32,
}

fn print_usage() {
    eprintln!(
        "Usage: bench_parser_c [--mode cold|warm] [--iterations N] [--input str|bytes] <file>"
    );
    eprintln!("  --mode cold|warm    cold=create parser every iteration (default: cold)");
    eprintln!("  --iterations N       number of parse iterations (default: 1)");
    eprintln!("  --input str|bytes    parse via string or raw bytes path (default: str)");
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Config, String> {
    let mut mode = BenchMode::Cold;
    let mut input = InputMode::Str;
    let mut iterations = 1_u32;
    let mut file_path: Option<String> = None;

    let mut iter = args.into_iter();
    let _binary_name = iter.next();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mode" => {
                let value = iter.next().ok_or("missing value for --mode")?;
                mode = match value.as_str() {
                    "cold" => BenchMode::Cold,
                    "warm" => BenchMode::Warm,
                    _ => return Err(format!("invalid mode: {value}")),
                };
            }
            "--iterations" | "-n" => {
                let value = iter.next().ok_or("missing value for --iterations")?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid iteration count: {value}"))?;
                if parsed == 0 {
                    return Err("iterations must be greater than 0".to_string());
                }
                iterations = parsed;
            }
            "--input" => {
                let value = iter.next().ok_or("missing value for --input")?;
                input = match value.as_str() {
                    "str" => InputMode::Str,
                    "bytes" => InputMode::Bytes,
                    _ => return Err(format!("invalid input mode: {value}")),
                };
            }
            "--help" | "-h" => {
                return Err("help requested".to_string());
            }
            _ if arg.starts_with('-') => {
                return Err(format!("unknown flag: {arg}"));
            }
            _ => {
                if file_path.is_some() {
                    return Err("only one file path may be provided".to_string());
                }
                file_path = Some(arg);
            }
        }
    }

    let path = file_path.ok_or("missing <file> argument")?;

    Ok(Config { file_path: path, mode, input, iterations })
}

fn run_cold_str(code: &str, iterations: u32) -> Result<(bool, u128), String> {
    let mut has_error = false;
    let start = Instant::now();
    for _ in 0..iterations {
        let tree = parse_perl_code(code).map_err(|err| err.to_string())?;
        has_error |= tree.root_node().has_error();
    }
    Ok((has_error, start.elapsed().as_micros()))
}

fn run_cold_bytes(code: &[u8], iterations: u32) -> Result<(bool, u128), String> {
    let mut has_error = false;
    let start = Instant::now();
    for _ in 0..iterations {
        let tree = parse_perl_bytes(code).map_err(|err| err.to_string())?;
        has_error |= tree.root_node().has_error();
    }
    Ok((has_error, start.elapsed().as_micros()))
}

fn parse_with_reused_parser<T: AsRef<[u8]>>(
    parser: &mut Parser,
    code: T,
    previous_tree: Option<&Tree>,
) -> Result<Tree, String> {
    parser.parse(code, previous_tree).ok_or_else(|| "Failed to parse code".to_string())
}

fn run_warm_bytes(code: &[u8], iterations: u32) -> Result<(bool, u128), String> {
    let mut parser = try_create_parser().map_err(|err| err.to_string())?;
    let mut has_error = false;

    let start = Instant::now();
    let mut tree = parse_with_reused_parser(&mut parser, code, None)?;
    has_error |= tree.root_node().has_error();

    for _ in 1..iterations {
        tree = parse_with_reused_parser(&mut parser, code, Some(&tree))?;
        has_error |= tree.root_node().has_error();
    }

    Ok((has_error, start.elapsed().as_micros()))
}

fn run_warm_str(code: &str, iterations: u32) -> Result<(bool, u128), String> {
    let mut parser = try_create_parser().map_err(|err| err.to_string())?;
    let mut has_error = false;

    let start = Instant::now();
    let mut tree = parse_with_reused_parser(&mut parser, code, None)?;
    has_error |= tree.root_node().has_error();

    for _ in 1..iterations {
        tree = parse_with_reused_parser(&mut parser, code, Some(&tree))?;
        has_error |= tree.root_node().has_error();
    }

    Ok((has_error, start.elapsed().as_micros()))
}

fn run(config: &Config, code: &[u8]) -> Result<(bool, u128), String> {
    match (config.mode, config.input) {
        (BenchMode::Cold, InputMode::Str) => {
            let code_str = std::str::from_utf8(code)
                .map_err(|_| "input is not valid UTF-8 for --input str".to_string())?;
            run_cold_str(code_str, config.iterations)
        }
        (BenchMode::Cold, InputMode::Bytes) => run_cold_bytes(code, config.iterations),
        (BenchMode::Warm, InputMode::Str) => {
            let code_str = std::str::from_utf8(code)
                .map_err(|_| "input is not valid UTF-8 for --input str".to_string())?;
            run_warm_str(code_str, config.iterations)
        }
        (BenchMode::Warm, InputMode::Bytes) => run_warm_bytes(code, config.iterations),
    }
}

fn print_metrics(config: &Config, total_us: u128, has_error: bool) {
    let avg_us = total_us as f64 / config.iterations as f64;
    println!("mode={}", config.mode.as_str());
    println!("input={}", config.input.as_str());
    println!("iterations={}", config.iterations);
    println!("total_us={}", total_us);
    println!("avg_us={avg_us:.3}");
    println!("has_error={}", has_error);
}

fn main() {
    let config = match parse_args(env::args()) {
        Ok(config) => config,
        Err(error) if error == "help requested" => {
            print_usage();
            return;
        }
        Err(error) => {
            eprintln!("Argument error: {error}");
            print_usage();
            std::process::exit(1);
        }
    };

    let code = fs::read(&config.file_path).unwrap_or_else(|error| {
        eprintln!("Failed to read file: {error}");
        std::process::exit(1);
    });

    match run(&config, &code) {
        Ok((has_error, total_us)) => print_metrics(&config, total_us, has_error),
        Err(error) => {
            eprintln!("Parse error: {error}");
            std::process::exit(1);
        }
    }
}

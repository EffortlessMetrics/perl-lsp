use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::{
    parse_perl_bytes, parse_perl_bytes_with_parser, parse_perl_code, parse_perl_code_with_parser,
    try_create_parser,
};

#[derive(Clone, Copy)]
enum Mode {
    Cold,
    Warm,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Warm => "warm",
        }
    }
}

#[derive(Clone, Copy)]
enum InputKind {
    Str,
    Bytes,
}

impl InputKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Str => "str",
            Self::Bytes => "bytes",
        }
    }
}

struct Config {
    file_path: String,
    mode: Mode,
    input: InputKind,
    iterations: u64,
}

struct BenchSummary {
    mode: Mode,
    input: InputKind,
    iterations: u64,
    total_us: u128,
    avg_us: u128,
    has_error: bool,
}

fn usage() -> &'static str {
    "Usage: bench_parser_c <file> [--mode cold|warm] [--iterations N] [--input str|bytes]\n\
     Defaults: --mode cold --iterations 1 --input str"
}

fn parse_config(args: &[String]) -> Result<Config, String> {
    if args.len() < 2 {
        return Err(usage().to_string());
    }

    let file_path = args[1].clone();
    let mut mode = Mode::Cold;
    let mut input = InputKind::Str;
    let mut iterations = 1_u64;

    let mut index = 2_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                let value =
                    args.get(index + 1).ok_or_else(|| "Missing value for --mode".to_string())?;
                mode = match value.as_str() {
                    "cold" => Mode::Cold,
                    "warm" => Mode::Warm,
                    _ => return Err(format!("Invalid mode: {value}")),
                };
                index += 2;
            }
            "--iterations" | "-n" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "Missing value for --iterations".to_string())?;
                iterations = value
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid iteration count: {value}"))?;
                if iterations == 0 {
                    return Err("--iterations must be greater than 0".to_string());
                }
                index += 2;
            }
            "--input" => {
                let value =
                    args.get(index + 1).ok_or_else(|| "Missing value for --input".to_string())?;
                input = match value.as_str() {
                    "str" => InputKind::Str,
                    "bytes" => InputKind::Bytes,
                    _ => return Err(format!("Invalid input mode: {value}")),
                };
                index += 2;
            }
            "--cold" => {
                mode = Mode::Cold;
                index += 1;
            }
            "--warm" => {
                mode = Mode::Warm;
                index += 1;
            }
            "--help" | "-h" => {
                return Err(usage().to_string());
            }
            unknown => {
                return Err(format!("Unknown argument: {unknown}"));
            }
        }
    }

    Ok(Config { file_path, mode, input, iterations })
}

fn parse_cold(input: InputKind, code: &[u8]) -> Result<bool, String> {
    let tree = match input {
        InputKind::Str => {
            let as_str = std::str::from_utf8(code)
                .map_err(|err| format!("Input is not UTF-8 for --input str: {err}"))?;
            parse_perl_code(as_str).map_err(|err| err.to_string())?
        }
        InputKind::Bytes => parse_perl_bytes(code).map_err(|err| err.to_string())?,
    };
    Ok(tree.root_node().has_error())
}

fn run_benchmark(config: &Config, code: &[u8]) -> Result<BenchSummary, String> {
    let mut saw_error = false;
    let start = Instant::now();

    match config.mode {
        Mode::Cold => {
            for _ in 0..config.iterations {
                if parse_cold(config.input, code)? {
                    saw_error = true;
                }
            }
        }
        Mode::Warm => {
            let mut parser = try_create_parser().map_err(|err| err.to_string())?;
            for _ in 0..config.iterations {
                let tree = match config.input {
                    InputKind::Str => {
                        let as_str = std::str::from_utf8(code)
                            .map_err(|err| format!("Input is not UTF-8 for --input str: {err}"))?;
                        parse_perl_code_with_parser(&mut parser, as_str)
                            .map_err(|err| err.to_string())?
                    }
                    InputKind::Bytes => parse_perl_bytes_with_parser(&mut parser, code)
                        .map_err(|err| err.to_string())?,
                };
                if tree.root_node().has_error() {
                    saw_error = true;
                }
            }
        }
    }

    let total_us = start.elapsed().as_micros();
    let avg_us = total_us / u128::from(config.iterations);

    Ok(BenchSummary {
        mode: config.mode,
        input: config.input,
        iterations: config.iterations,
        total_us,
        avg_us,
        has_error: saw_error,
    })
}

fn print_summary(summary: &BenchSummary) {
    println!("mode={}", summary.mode.as_str());
    println!("input={}", summary.input.as_str());
    println!("iterations={}", summary.iterations);
    println!("total_us={}", summary.total_us);
    println!("avg_us={}", summary.avg_us);
    println!("has_error={}", summary.has_error);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match parse_config(&args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };

    let code = match fs::read(&config.file_path) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Failed to read file: {err}");
            std::process::exit(1);
        }
    };

    match run_benchmark(&config, &code) {
        Ok(summary) => {
            print_summary(&summary);
        }
        Err(err) => {
            eprintln!("Parse error: {err}");
            std::process::exit(1);
        }
    }
}

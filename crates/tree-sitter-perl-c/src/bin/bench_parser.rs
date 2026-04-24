use std::env;
use std::fs;
use std::time::Instant;

use tree_sitter::Parser;
use tree_sitter_perl_c::{parse_perl_bytes, try_create_parser};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "cold" => Some(Self::Cold),
            "warm" => Some(Self::Warm),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "str" => Some(Self::Str),
            "bytes" => Some(Self::Bytes),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct BenchConfig {
    file_path: String,
    mode: BenchMode,
    input_mode: InputMode,
    iterations: u32,
}

fn print_usage() {
    eprintln!(
        "Usage: bench_parser_c [--mode <cold|warm>] [--input <str|bytes>] [--iterations <N>] <file>"
    );
}

fn parse_args(args: &[String]) -> Result<BenchConfig, String> {
    let mut mode = BenchMode::Cold;
    let mut input_mode = InputMode::Str;
    let mut iterations = 1_u32;
    let mut file_path: Option<String> = None;

    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--mode" => {
                let value =
                    args.get(idx + 1).ok_or_else(|| "Missing value for --mode".to_string())?;
                mode = BenchMode::from_str(value)
                    .ok_or_else(|| format!("Invalid --mode value: {value}"))?;
                idx += 2;
            }
            "--input" => {
                let value =
                    args.get(idx + 1).ok_or_else(|| "Missing value for --input".to_string())?;
                input_mode = InputMode::from_str(value)
                    .ok_or_else(|| format!("Invalid --input value: {value}"))?;
                idx += 2;
            }
            "--iterations" | "-n" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| "Missing value for --iterations".to_string())?;
                let parsed_iterations = value
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid --iterations value: {value}"))?;
                if parsed_iterations == 0 {
                    return Err("--iterations must be >= 1".to_string());
                }
                iterations = parsed_iterations;
                idx += 2;
            }
            "--help" | "-h" => {
                return Err("help requested".to_string());
            }
            value if value.starts_with('-') => {
                return Err(format!("Unknown flag: {value}"));
            }
            value => {
                if file_path.is_some() {
                    return Err(format!("Unexpected positional argument: {value}"));
                }
                file_path = Some(value.to_string());
                idx += 1;
            }
        }
    }

    let file_path = file_path.ok_or_else(|| "Missing <file> argument".to_string())?;

    Ok(BenchConfig { file_path, mode, input_mode, iterations })
}

fn parse_once_with_reused_parser(
    parser: &mut Parser,
    input_mode: InputMode,
    code: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    let tree = match input_mode {
        InputMode::Str => {
            let source = std::str::from_utf8(code)?;
            parser.parse(source, None)
        }
        InputMode::Bytes => parser.parse(code, None),
    }
    .ok_or_else(|| "Failed to parse code".to_string())?;

    Ok(tree.root_node().has_error())
}

fn run_benchmark(
    config: &BenchConfig,
    code: &[u8],
) -> Result<(u128, bool), Box<dyn std::error::Error>> {
    let mut has_error = false;
    let start = Instant::now();

    match config.mode {
        BenchMode::Cold => {
            for _ in 0..config.iterations {
                let tree = match config.input_mode {
                    InputMode::Str => {
                        let source = std::str::from_utf8(code)?;
                        parse_perl_bytes(source.as_bytes())?
                    }
                    InputMode::Bytes => parse_perl_bytes(code)?,
                };
                has_error |= tree.root_node().has_error();
            }
        }
        BenchMode::Warm => {
            let mut parser = try_create_parser()?;
            for _ in 0..config.iterations {
                has_error |= parse_once_with_reused_parser(&mut parser, config.input_mode, code)?;
            }
        }
    }

    Ok((start.elapsed().as_micros(), has_error))
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let config = match parse_args(&raw_args) {
        Ok(config) => config,
        Err(message) => {
            print_usage();
            eprintln!("Error: {message}");
            std::process::exit(1);
        }
    };

    let code = match fs::read(&config.file_path) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Failed to read file: {error}");
            std::process::exit(1);
        }
    };

    let (total_us, has_error) = match run_benchmark(&config, &code) {
        Ok(result) => result,
        Err(error) => {
            println!("mode={}", config.mode.as_str());
            println!("input_mode={}", config.input_mode.as_str());
            println!("iterations={}", config.iterations);
            println!("total_us=0");
            println!("avg_us=0");
            println!("has_error=true");
            println!("status=failure");
            eprintln!("Parse error: {error}");
            std::process::exit(1);
        }
    };

    let avg_us = total_us / u128::from(config.iterations);

    println!("mode={}", config.mode.as_str());
    println!("input_mode={}", config.input_mode.as_str());
    println!("iterations={}", config.iterations);
    println!("total_us={total_us}");
    println!("avg_us={avg_us}");
    println!("has_error={has_error}");
    println!("status=success");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_cold_str_one_iteration() -> Result<(), String> {
        let args = vec!["sample.pl".to_string()];
        let config = parse_args(&args)?;
        assert_eq!(config.mode, BenchMode::Cold);
        assert_eq!(config.input_mode, InputMode::Str);
        assert_eq!(config.iterations, 1);
        assert_eq!(config.file_path, "sample.pl");
        Ok(())
    }

    #[test]
    fn parse_args_supports_warm_and_iterations() -> Result<(), String> {
        let args = vec![
            "--mode".to_string(),
            "warm".to_string(),
            "--input".to_string(),
            "bytes".to_string(),
            "-n".to_string(),
            "20".to_string(),
            "sample.pl".to_string(),
        ];
        let config = parse_args(&args)?;
        assert_eq!(config.mode, BenchMode::Warm);
        assert_eq!(config.input_mode, InputMode::Bytes);
        assert_eq!(config.iterations, 20);
        assert_eq!(config.file_path, "sample.pl");
        Ok(())
    }

    #[test]
    fn parse_args_rejects_unknown_flag() {
        let args = vec!["--unknown".to_string(), "sample.pl".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
    }
}

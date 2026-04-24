use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::{parse_perl_bytes, parse_perl_code, try_create_parser};

#[derive(Clone, Copy)]
enum Mode {
    Cold,
    Warm,
}

#[derive(Clone, Copy)]
enum InputMode {
    Str,
    Bytes,
}

struct Args {
    file_path: String,
    mode: Mode,
    input_mode: InputMode,
    iterations: u64,
}

impl Args {
    fn usage() -> &'static str {
        "Usage: bench_parser_c [--mode cold|warm] [--iterations N] [--input str|bytes] <file>"
    }

    fn parse() -> Result<Self, String> {
        let mut mode = Mode::Cold;
        let mut input_mode = InputMode::Str;
        let mut iterations: u64 = 1;
        let mut file_path: Option<String> = None;

        let mut iter = env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--mode" => {
                    let value =
                        iter.next().ok_or_else(|| "Missing value for --mode".to_string())?;
                    mode = match value.as_str() {
                        "cold" => Mode::Cold,
                        "warm" => Mode::Warm,
                        _ => {
                            return Err(format!(
                                "Invalid --mode value '{value}', expected cold|warm"
                            ));
                        }
                    };
                }
                "--iterations" | "-n" => {
                    let value =
                        iter.next().ok_or_else(|| "Missing value for --iterations".to_string())?;
                    iterations = value
                        .parse::<u64>()
                        .map_err(|_| format!("Invalid --iterations value '{value}'"))?;
                    if iterations == 0 {
                        return Err("--iterations must be >= 1".to_string());
                    }
                }
                "--input" => {
                    let value =
                        iter.next().ok_or_else(|| "Missing value for --input".to_string())?;
                    input_mode = match value.as_str() {
                        "str" => InputMode::Str,
                        "bytes" => InputMode::Bytes,
                        _ => {
                            return Err(format!(
                                "Invalid --input value '{value}', expected str|bytes"
                            ));
                        }
                    };
                }
                "--help" | "-h" => return Err(Self::usage().to_string()),
                _ if arg.starts_with('-') => {
                    return Err(format!("Unknown flag '{arg}'"));
                }
                _ => {
                    if file_path.is_some() {
                        return Err("Only one file path may be provided".to_string());
                    }
                    file_path = Some(arg);
                }
            }
        }

        let file_path = file_path.ok_or_else(|| format!("{}\nMissing <file>", Self::usage()))?;
        Ok(Self { file_path, mode, input_mode, iterations })
    }
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Cold => "cold",
        Mode::Warm => "warm",
    }
}

fn input_label(input_mode: InputMode) -> &'static str {
    match input_mode {
        InputMode::Str => "str",
        InputMode::Bytes => "bytes",
    }
}

fn print_result(
    mode: Mode,
    input_mode: InputMode,
    iterations: u64,
    total_us: u128,
    has_error: bool,
) {
    let avg_us = total_us / u128::from(iterations);
    println!("mode={}", mode_label(mode));
    println!("input={}", input_label(input_mode));
    println!("iterations={iterations}");
    println!("total_us={total_us}");
    println!("avg_us={avg_us}");
    println!("has_error={has_error}");
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            if message != Args::usage() {
                eprintln!("{}", Args::usage());
            }
            std::process::exit(1);
        }
    };

    let bytes = fs::read(&args.file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file '{}': {}", args.file_path, e);
        std::process::exit(1);
    });

    let start = Instant::now();
    let mut has_error = false;
    match args.mode {
        Mode::Cold => {
            for _ in 0..args.iterations {
                let parse_result = match args.input_mode {
                    InputMode::Str => match std::str::from_utf8(&bytes) {
                        Ok(code) => parse_perl_code(code),
                        Err(e) => {
                            eprintln!("Failed to decode UTF-8 for --input str: {}", e);
                            std::process::exit(1);
                        }
                    },
                    InputMode::Bytes => parse_perl_bytes(&bytes),
                };

                match parse_result {
                    Ok(tree) => {
                        has_error |= tree.root_node().has_error();
                    }
                    Err(e) => {
                        eprintln!("Parse error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Mode::Warm => {
            let mut parser = match try_create_parser() {
                Ok(parser) => parser,
                Err(e) => {
                    eprintln!("Failed to create parser: {}", e);
                    std::process::exit(1);
                }
            };

            for _ in 0..args.iterations {
                let parse_result = match args.input_mode {
                    InputMode::Str => match std::str::from_utf8(&bytes) {
                        Ok(code) => parser.parse(code, None),
                        Err(e) => {
                            eprintln!("Failed to decode UTF-8 for --input str: {}", e);
                            std::process::exit(1);
                        }
                    },
                    InputMode::Bytes => parser.parse(bytes.as_slice(), None),
                };

                match parse_result {
                    Some(tree) => {
                        has_error |= tree.root_node().has_error();
                    }
                    None => {
                        eprintln!("Parse error: tree-sitter returned no tree");
                        std::process::exit(1);
                    }
                }
            }
        }
    };

    let total_us = start.elapsed().as_micros();
    print_result(args.mode, args.input_mode, args.iterations, total_us, has_error);
}

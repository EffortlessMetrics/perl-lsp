use std::env;
use std::fs;
use std::time::Instant;
use tree_sitter_perl_c::{parse_perl_code, try_create_parser};

#[derive(Clone, Copy)]
enum BenchMode {
    Wrapper,
    ParserReuse,
}

impl BenchMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "wrapper" => Some(Self::Wrapper),
            "parser-reuse" => Some(Self::ParserReuse),
            _ => None,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 4 {
        eprintln!("Usage: bench_parser_c <file> [wrapper|parser-reuse] [iterations]");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let mode = args.get(2).and_then(|value| BenchMode::parse(value)).unwrap_or(BenchMode::Wrapper);
    let iterations = args
        .get(3)
        .map(|value| value.parse::<usize>())
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("Invalid iteration count: {error}");
            std::process::exit(1);
        })
        .unwrap_or(1);

    let code = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read file: {}", e);
        std::process::exit(1);
    });

    let start = Instant::now();
    let result = match mode {
        BenchMode::Wrapper => bench_wrapper(&code, iterations),
        BenchMode::ParserReuse => bench_parser_reuse(&code, iterations),
    };
    let duration = start.elapsed().as_micros();

    match result {
        Ok(has_error) => {
            let mode_name = match mode {
                BenchMode::Wrapper => "wrapper",
                BenchMode::ParserReuse => "parser-reuse",
            };
            println!(
                "status=success mode={} iterations={} error={} duration_us={}",
                mode_name, iterations, has_error, duration
            );
            // Always return success (0) - parse errors are indicated in the error field
        }
        Err(e) => {
            println!("status=failure error=true duration_us={}", duration);
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}

fn bench_wrapper(code: &str, iterations: usize) -> Result<bool, Box<dyn std::error::Error>> {
    let mut has_error = false;
    for _ in 0..iterations {
        let tree = parse_perl_code(code)?;
        has_error = tree.root_node().has_error();
    }
    Ok(has_error)
}

fn bench_parser_reuse(code: &str, iterations: usize) -> Result<bool, Box<dyn std::error::Error>> {
    let mut parser = try_create_parser()?;
    let mut has_error = false;
    for _ in 0..iterations {
        let Some(tree) = parser.parse(code.as_bytes(), None) else {
            return Err("Failed to parse code".into());
        };
        has_error = tree.root_node().has_error();
    }
    Ok(has_error)
}

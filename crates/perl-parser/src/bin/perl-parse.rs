use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

use perl_parser::{Node, ParseError, Parser};

#[derive(Default)]
struct TotalStats {
    files_parsed: usize,
    files_failed: usize,
    total_bytes: usize,
    total_time: std::time::Duration,
    total_nodes: usize,
    file_details: Vec<FileStats>,
}

struct FileStats {
    name: String,
    bytes: usize,
    time: std::time::Duration,
    nodes: usize,
    error: bool,
}

impl TotalStats {
    fn new() -> Self {
        Self::default()
    }

    fn add_file(&mut self, name: &str, bytes: usize, time: std::time::Duration, nodes: usize) {
        self.files_parsed += 1;
        self.total_bytes += bytes;
        self.total_time += time;
        self.total_nodes += nodes;
        self.file_details.push(FileStats {
            name: name.to_string(),
            bytes,
            time,
            nodes,
            error: false,
        });
    }

    fn add_error(&mut self, name: &str) {
        self.files_failed += 1;
        self.file_details.push(FileStats {
            name: name.to_string(),
            bytes: 0,
            time: std::time::Duration::ZERO,
            nodes: 0,
            error: true,
        });
    }

    fn print(&self) {
        eprintln!("\n=== Total Statistics ===");
        eprintln!("Files parsed: {}", self.files_parsed);
        eprintln!("Files failed: {}", self.files_failed);
        eprintln!(
            "Total size: {} bytes ({:.2} KB)",
            self.total_bytes,
            self.total_bytes as f64 / 1024.0
        );
        eprintln!("Total time: {:?}", self.total_time);
        eprintln!("Total nodes: {}", self.total_nodes);

        if self.files_parsed > 0 {
            let avg_speed = self.total_bytes as f64 / self.total_time.as_secs_f64() / 1_000_000.0;
            eprintln!("Average speed: {:.2} MB/s", avg_speed);
            eprintln!("Average nodes per file: {}", self.total_nodes / self.files_parsed);
        }

        if self.file_details.len() > 1 && self.file_details.len() <= 20 {
            eprintln!("\n=== File Details ===");
            for stat in &self.file_details {
                if stat.error {
                    eprintln!("{}: FAILED", stat.name);
                } else {
                    eprintln!(
                        "{}: {} bytes, {:?}, {} nodes",
                        stat.name, stat.bytes, stat.time, stat.nodes
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
struct Args {
    inputs: Vec<Input>,
    output_format: OutputFormat,
    show_stats: bool,
    pretty: bool,
    quiet: bool,
    continue_on_error: bool,
}

#[derive(Debug)]
enum Input {
    File(PathBuf),
    Stdin,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Sexp,
    Json,
    Debug,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = std::env::args().skip(1);
        let mut inputs = Vec::new();
        let mut output_format = OutputFormat::Sexp;
        let mut show_stats = false;
        let mut pretty = false;
        let mut quiet = false;
        let mut continue_on_error = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("perl-parse v{}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "-f" | "--format" => {
                    let format = args.next().ok_or("Missing format argument")?;
                    output_format = match format.as_str() {
                        "sexp" | "s-expression" => OutputFormat::Sexp,
                        "json" => OutputFormat::Json,
                        "debug" => OutputFormat::Debug,
                        _ => return Err(format!("Unknown format: {}", format)),
                    };
                }
                "-s" | "--stats" => show_stats = true,
                "-p" | "--pretty" => pretty = true,
                "-q" | "--quiet" => quiet = true,
                "-c" | "--continue" => continue_on_error = true,
                "-" => inputs.push(Input::Stdin),
                path if path.starts_with('-') => {
                    return Err(format!("Unknown option: {}", path));
                }
                path => {
                    inputs.push(Input::File(PathBuf::from(path)));
                }
            }
        }

        if inputs.is_empty() {
            inputs.push(Input::Stdin);
        }

        Ok(Args { inputs, output_format, show_stats, pretty, quiet, continue_on_error })
    }
}

fn print_help() {
    println!(
        r#"perl-parse - Parse Perl code and output the AST

USAGE:
    perl-parse [OPTIONS] [FILE...]

ARGS:
    <FILE>...    Path(s) to Perl file(s) to parse (use '-' for stdin)

OPTIONS:
    -h, --help              Print help information
    -V, --version           Print version information
    -f, --format <FORMAT>   Output format [default: sexp]
                           Possible values: sexp, json, debug
    -s, --stats            Show parsing statistics
    -p, --pretty           Pretty-print output (for JSON)
    -q, --quiet            Suppress output (useful with --stats)
    -c, --continue         Continue on error when parsing multiple files

EXAMPLES:
    # Parse a file and output S-expression
    perl-parse script.pl

    # Parse from stdin
    echo 'print "Hello"' | perl-parse -

    # Output as JSON with statistics
    perl-parse -f json -s script.pl

    # Parse multiple files, show only stats
    perl-parse -q -s *.pl

    # Pretty-print JSON output
    perl-parse -f json -p script.pl
"#
    );
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Try 'perl-parse --help' for more information.");
            std::process::exit(1);
        }
    };

    let mut total_stats = TotalStats::new();
    let mut had_error = false;

    for input in &args.inputs {
        let path_str = match input {
            Input::File(path) => path.display().to_string(),
            Input::Stdin => "<stdin>".to_string(),
        };

        if !args.quiet && args.inputs.len() > 1 {
            eprintln!("=== Parsing {} ===", path_str);
        }

        let source = match read_input(input) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("Error reading {}: {}", path_str, e);
                if args.continue_on_error {
                    had_error = true;
                    continue;
                } else {
                    std::process::exit(1);
                }
            }
        };

        let start = Instant::now();
        let mut parser = Parser::new(&source);
        let result = parser.parse();
        let parse_time = start.elapsed();

        match result {
            Ok(ast) => {
                if !args.quiet {
                    match args.output_format {
                        OutputFormat::Sexp => println!("{}", ast.to_sexp()),
                        OutputFormat::Json => {
                            let json = ast_to_json(&ast);
                            let output = if args.pretty {
                                serde_json::to_string_pretty(&json)
                            } else {
                                serde_json::to_string(&json)
                            };
                            match output {
                                Ok(s) => println!("{s}"),
                                Err(e) => eprintln!("JSON serialization error: {e}"),
                            }
                        }
                        OutputFormat::Debug => println!("{:#?}", ast),
                    }
                }

                total_stats.add_file(&path_str, source.len(), parse_time, ast.count_nodes());
            }
            Err(e) => {
                if !args.quiet {
                    eprintln!("\nError in {}:", path_str);
                    print_error(&e, &source);
                }
                if args.continue_on_error {
                    had_error = true;
                    total_stats.add_error(&path_str);
                } else {
                    std::process::exit(1);
                }
            }
        }
    }

    if args.show_stats {
        total_stats.print();
    }

    if had_error {
        std::process::exit(1);
    }
}

fn read_input(input: &Input) -> io::Result<String> {
    match input {
        Input::File(path) => fs::read_to_string(path),
        Input::Stdin => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}

fn ast_to_json(ast: &Node) -> serde_json::Value {
    // Convert AST to JSON representation
    serde_json::json!({
        "type": format!("{:?}", ast.kind).split('(').next().unwrap_or("Unknown"),
        "location": {
            "start": ast.location.start,
            "end": ast.location.end,
        },
        "sexp": ast.to_sexp(),
        "node_count": ast.count_nodes(),
    })
}

fn print_error(error: &ParseError, source: &str) {
    let mut stderr = io::stderr();

    match error {
        ParseError::UnexpectedToken { expected, found, location } => {
            let (line, col) = position_to_line_col(source, *location);
            writeln!(stderr, "Parse error: Unexpected token at line {}, column {}", line, col).ok();
            writeln!(stderr, "  Expected: {}", expected).ok();
            writeln!(stderr, "  Found: {}", found).ok();
            print_error_context(source, *location, &mut stderr);
        }
        ParseError::UnexpectedEof => {
            writeln!(stderr, "Parse error: Unexpected end of input").ok();
            if !source.is_empty() {
                print_error_context(source, source.len() - 1, &mut stderr);
            }
        }
        ParseError::SyntaxError { message, location } => {
            let (line, col) = position_to_line_col(source, *location);
            writeln!(stderr, "Parse error: {} at line {}, column {}", message, line, col).ok();
            print_error_context(source, *location, &mut stderr);
        }
        ParseError::InvalidNumber { literal } => {
            writeln!(stderr, "Parse error: Invalid number literal: {}", literal).ok();
        }
        ParseError::InvalidString => {
            writeln!(stderr, "Parse error: Invalid string literal").ok();
        }
        ParseError::UnclosedDelimiter { delimiter } => {
            writeln!(stderr, "Parse error: Unclosed delimiter: {}", delimiter).ok();
        }
        ParseError::InvalidRegex { message } => {
            writeln!(stderr, "Parse error: Invalid regex: {}", message).ok();
        }
        ParseError::LexerError { message } => {
            writeln!(stderr, "Parse error: Lexer error: {}", message).ok();
        }
        ParseError::RecursionLimit => {
            writeln!(stderr, "Parse error: Maximum recursion depth exceeded").ok();
        }
        ParseError::NestingTooDeep { depth, max_depth } => {
            writeln!(stderr, "Parse error: Nesting too deep ({} > {})", depth, max_depth).ok();
        }
    }
}

fn position_to_line_col(source: &str, position: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.chars().enumerate() {
        if i >= position {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

fn print_error_context(source: &str, position: usize, stderr: &mut io::Stderr) {
    let lines: Vec<&str> = source.lines().collect();
    let (line_num, col_num) = position_to_line_col(source, position);

    if line_num > 0 && line_num <= lines.len() {
        writeln!(stderr).ok();

        // Show previous line if available
        if line_num > 1 {
            writeln!(stderr, "  {} | {}", line_num - 1, lines[line_num - 2]).ok();
        }

        // Show error line
        writeln!(stderr, "  {} | {}", line_num, lines[line_num - 1]).ok();

        // Show error pointer
        write!(stderr, "  {} | ", " ".repeat(line_num.to_string().len())).ok();
        writeln!(stderr, "{}^", " ".repeat(col_num - 1)).ok();

        // Show next line if available
        if line_num < lines.len() {
            writeln!(stderr, "  {} | {}", line_num + 1, lines[line_num]).ok();
        }
    }
}

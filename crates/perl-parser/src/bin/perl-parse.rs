use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Instant;

use perl_parser::{Parser, Node};

#[derive(Debug)]
struct Args {
    input: Input,
    output_format: OutputFormat,
    show_stats: bool,
    pretty: bool,
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
        let mut input = None;
        let mut output_format = OutputFormat::Sexp;
        let mut show_stats = false;
        let mut pretty = false;

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
                "-" => input = Some(Input::Stdin),
                path => {
                    if input.is_some() {
                        return Err("Multiple input files specified".to_string());
                    }
                    input = Some(Input::File(PathBuf::from(path)));
                }
            }
        }

        let input = input.unwrap_or(Input::Stdin);

        Ok(Args {
            input,
            output_format,
            show_stats,
            pretty,
        })
    }
}

fn print_help() {
    println!(
        r#"perl-parse - Parse Perl code and output the AST

USAGE:
    perl-parse [OPTIONS] [FILE]

ARGS:
    <FILE>    Path to Perl file to parse (use '-' for stdin)

OPTIONS:
    -h, --help              Print help information
    -V, --version           Print version information
    -f, --format <FORMAT>   Output format [default: sexp]
                           Possible values: sexp, json, debug
    -s, --stats            Show parsing statistics
    -p, --pretty           Pretty-print output (for JSON)

EXAMPLES:
    # Parse a file and output S-expression
    perl-parse script.pl

    # Parse from stdin
    echo 'print "Hello"' | perl-parse -

    # Output as JSON with statistics
    perl-parse -f json -s script.pl

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

    let source = match read_input(&args.input) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            std::process::exit(1);
        }
    };

    let start = Instant::now();
    let mut parser = Parser::new(&source);
    let result = parser.parse();
    let parse_time = start.elapsed();

    match result {
        Ok(ast) => {
            match args.output_format {
                OutputFormat::Sexp => println!("{}", ast.to_sexp()),
                OutputFormat::Json => {
                    let json = ast_to_json(&ast);
                    if args.pretty {
                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                    } else {
                        println!("{}", serde_json::to_string(&json).unwrap());
                    }
                }
                OutputFormat::Debug => println!("{:#?}", ast),
            }

            if args.show_stats {
                eprintln!("\n=== Parse Statistics ===");
                eprintln!("File size: {} bytes", source.len());
                eprintln!("Parse time: {:?}", parse_time);
                eprintln!("Speed: {:.2} MB/s", source.len() as f64 / parse_time.as_secs_f64() / 1_000_000.0);
                eprintln!("Nodes: {}", count_nodes(&ast));
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
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

fn count_nodes(ast: &Node) -> usize {
    use perl_parser::NodeKind;
    
    // Count all nodes recursively
    let mut count = 1; // Count this node
    
    // Count children based on node kind
    match &ast.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                count += count_nodes(stmt);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                count += count_nodes(stmt);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            count += count_nodes(left);
            count += count_nodes(right);
        }
        NodeKind::Unary { operand, .. } => {
            count += count_nodes(operand);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            count += count_nodes(condition);
            count += count_nodes(then_expr);
            count += count_nodes(else_expr);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            count += count_nodes(condition);
            count += count_nodes(then_branch);
            for (cond, branch) in elsif_branches {
                count += count_nodes(cond);
                count += count_nodes(branch);
            }
            if let Some(else_b) = else_branch {
                count += count_nodes(else_b);
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                count += count_nodes(arg);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            count += count_nodes(object);
            for arg in args {
                count += count_nodes(arg);
            }
        }
        NodeKind::IndirectCall { object, args, .. } => {
            count += count_nodes(object);
            for arg in args {
                count += count_nodes(arg);
            }
        }
        NodeKind::Return { value } => {
            if let Some(val) = value {
                count += count_nodes(val);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            count += count_nodes(lhs);
            count += count_nodes(rhs);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            count += count_nodes(variable);
            if let Some(init) = initializer {
                count += count_nodes(init);
            }
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            for var in variables {
                count += count_nodes(var);
            }
            if let Some(init) = initializer {
                count += count_nodes(init);
            }
        }
        _ => {} // Leaf nodes
    }
    
    count
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
        "node_count": count_nodes(ast),
    })
}
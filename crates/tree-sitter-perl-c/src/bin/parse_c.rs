use std::env;
use std::fs;
use std::process;

struct CliOptions {
    filename: String,
    print_root_kind: bool,
    print_has_error: bool,
    print_sexp: bool,
}

fn print_usage(program: &str) {
    eprintln!("Usage: {program} [--root-kind] [--has-error] [--sexp] <perl_file>");
    eprintln!("\nOptions:");
    eprintln!("  --root-kind   Print root node kind");
    eprintln!("  --has-error   Print whether the parse tree contains error nodes");
    eprintln!("  --sexp        Print the root node s-expression");
}

fn parse_args() -> Result<CliOptions, String> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "parse_c".to_string());

    let mut filename: Option<String> = None;
    let mut print_root_kind = false;
    let mut print_has_error = false;
    let mut print_sexp = false;

    for arg in args {
        match arg.as_str() {
            "--root-kind" => print_root_kind = true,
            "--has-error" => print_has_error = true,
            "--sexp" => print_sexp = true,
            "--help" | "-h" => {
                print_usage(&program);
                process::exit(0);
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Unknown option: {arg}"));
            }
            _ => {
                if filename.is_some() {
                    return Err("Expected exactly one input file".to_string());
                }
                filename = Some(arg);
            }
        }
    }

    let filename = filename.ok_or_else(|| "Missing input file".to_string())?;

    Ok(CliOptions { filename, print_root_kind, print_has_error, print_sexp })
}

fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            let program = env::args().next().unwrap_or_else(|| "parse_c".to_string());
            print_usage(&program);
            process::exit(1);
        }
    };

    let bytes = match fs::read(&options.filename) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Failed to read '{}': {error}", options.filename);
            process::exit(1);
        }
    };

    let tree = match tree_sitter_perl_c::parse_perl_bytes(&bytes) {
        Ok(tree) => tree,
        Err(error) => {
            eprintln!("Failed to parse '{}': {error}", options.filename);
            process::exit(1);
        }
    };

    let root = tree.root_node();
    let has_error = root.has_error();

    if options.print_root_kind {
        println!("root_kind={}", root.kind());
    }

    if options.print_has_error {
        println!("has_error={has_error}");
    }

    if options.print_sexp {
        println!("{}", root.to_sexp());
    }

    if has_error {
        let start = root.start_position();
        let end = root.end_position();
        eprintln!(
            "Parse completed with error nodes in '{}'. root_kind={} bytes={} span={}:{}-{}:{}",
            options.filename,
            root.kind(),
            bytes.len(),
            start.row,
            start.column,
            end.row,
            end.column
        );
        process::exit(1);
    }
}

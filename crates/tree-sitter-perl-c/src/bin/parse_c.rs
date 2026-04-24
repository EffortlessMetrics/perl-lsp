use std::env;
use std::fs;
use std::process;

#[derive(Default)]
struct CliOptions {
    path: Option<String>,
    show_root_kind: bool,
    show_has_error: bool,
    show_sexp: bool,
}

impl CliOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();
        let mut saw_program_name = false;

        for arg in args {
            if !saw_program_name {
                saw_program_name = true;
                continue;
            }

            match arg.as_str() {
                "--root-kind" => options.show_root_kind = true,
                "--has-error" => options.show_has_error = true,
                "--sexp" => options.show_sexp = true,
                "-h" | "--help" => return Err(String::new()),
                flag if flag.starts_with('-') => {
                    return Err(format!("Unknown flag: {flag}"));
                }
                _ => {
                    if options.path.is_some() {
                        return Err("Expected a single input file path".to_owned());
                    }
                    options.path = Some(arg);
                }
            }
        }

        if options.path.is_none() {
            return Err("Missing required <perl_file> argument".to_owned());
        }

        Ok(options)
    }
}

fn print_usage(program_name: &str) {
    eprintln!("Usage: {program_name} [--root-kind] [--has-error] [--sexp] <perl_file>");
}

fn find_first_error_node(root: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            return Some(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.has_error() || child.is_error() || child.is_missing() {
                stack.push(child);
            }
        }
    }

    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program_name = args.first().cloned().unwrap_or_else(|| "parse_c".to_owned());
    let options = match CliOptions::parse(args) {
        Ok(options) => options,
        Err(error) if error.is_empty() => {
            print_usage(&program_name);
            process::exit(0);
        }
        Err(error) => {
            eprintln!("{error}");
            print_usage(&program_name);
            process::exit(1);
        }
    };

    let filename = options.path.as_deref().unwrap_or_default();
    let source_code = match fs::read(filename) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_perl_c::language()).unwrap_or_else(|e| {
        eprintln!("Error loading Perl C grammar: {:?}", e);
        process::exit(1);
    });

    match parser.parse(source_code.as_slice(), None) {
        Some(tree) => {
            let root = tree.root_node();
            let has_error = root.has_error();

            if options.show_root_kind {
                println!("root_kind={}", root.kind());
            }
            if options.show_has_error {
                println!("has_error={has_error}");
            }
            if options.show_sexp {
                println!("{}", root.to_sexp());
            }

            if has_error {
                if let Some(error_node) = find_first_error_node(root) {
                    let start = error_node.start_position();
                    let end = error_node.end_position();
                    eprintln!(
                        "Parse contains syntax errors: first error node kind={} bytes={}..{} row={}..{} column_bytes={}..{}",
                        error_node.kind(),
                        error_node.start_byte(),
                        error_node.end_byte(),
                        start.row + 1,
                        end.row + 1,
                        start.column,
                        end.column
                    );
                } else {
                    eprintln!("Parse contains syntax errors");
                }

                process::exit(1);
            } else {
                std::process::exit(0);
            }
        }
        None => {
            eprintln!("Failed to parse");
            process::exit(1);
        }
    }
}

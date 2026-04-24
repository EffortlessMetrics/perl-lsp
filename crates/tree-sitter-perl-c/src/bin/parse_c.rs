use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug)]
struct CliOptions {
    file: PathBuf,
    show_root_kind: bool,
    show_has_error: bool,
    show_sexp: bool,
}

impl CliOptions {
    fn parse() -> Result<Self, String> {
        let mut file: Option<PathBuf> = None;
        let mut show_root_kind = false;
        let mut show_has_error = false;
        let mut show_sexp = false;

        for arg in env::args_os().skip(1) {
            match arg.to_str() {
                Some("--root-kind") => show_root_kind = true,
                Some("--has-error") => show_has_error = true,
                Some("--sexp") => show_sexp = true,
                Some("--help") | Some("-h") => return Err(String::new()),
                Some(flag) if flag.starts_with('-') => {
                    return Err(format!("Unknown flag: {flag}"));
                }
                _ => {
                    if file.is_some() {
                        return Err("Only one input file can be provided".to_string());
                    }
                    file = Some(PathBuf::from(arg));
                }
            }
        }

        match file {
            Some(file) => Ok(Self { file, show_root_kind, show_has_error, show_sexp }),
            None => Err("Missing input file".to_string()),
        }
    }
}

fn usage(program: &str) -> String {
    format!(
        "Usage: {program} [--root-kind] [--has-error] [--sexp] <perl_file>\n\n\
         Parses Perl source bytes using the tree-sitter C grammar.\n\
         Exit status is 0 on clean parse and 1 on parse failure or syntax errors."
    )
}

fn format_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn parse_diagnostics(tree: &tree_sitter::Tree) -> Vec<String> {
    let mut stack = vec![tree.root_node()];
    let mut diagnostics = Vec::new();

    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let start = node.start_position();
            let end = node.end_position();
            diagnostics.push(format!(
                "{} node `{}` at bytes {}..{} (line {}, byte_col {} to line {}, byte_col {})",
                if node.is_missing() { "Missing" } else { "Error" },
                node.kind(),
                node.start_byte(),
                node.end_byte(),
                start.row + 1,
                start.column + 1,
                end.row + 1,
                end.column + 1
            ));
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    diagnostics
}

fn print_error_and_exit(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(1);
}

fn main() {
    let program = env::args()
        .next()
        .unwrap_or_else(|| OsString::from("parse_c").to_string_lossy().into_owned());

    let options = match CliOptions::parse() {
        Ok(opts) => opts,
        Err(err) if err.is_empty() => {
            eprintln!("{}", usage(&program));
            process::exit(1);
        }
        Err(err) => {
            eprintln!("{err}\n\n{}", usage(&program));
            process::exit(1);
        }
    };

    let source_bytes = match fs::read(&options.file) {
        Ok(bytes) => bytes,
        Err(err) => {
            print_error_and_exit(&format!("Failed to read `{}`: {err}", format_path(&options.file)))
        }
    };

    let tree = match tree_sitter_perl_c::parse_perl_bytes(&source_bytes) {
        Ok(tree) => tree,
        Err(err) => print_error_and_exit(&format!(
            "Failed to parse `{}`: {err}",
            format_path(&options.file)
        )),
    };

    let root = tree.root_node();

    if options.show_root_kind {
        println!("{}", root.kind());
    }
    if options.show_has_error {
        println!("{}", root.has_error());
    }
    if options.show_sexp {
        println!("{}", root.to_sexp());
    }

    let diagnostics = parse_diagnostics(&tree);
    if diagnostics.is_empty() {
        process::exit(0);
    }

    eprintln!("Found {} parse issue(s) in `{}`:", diagnostics.len(), format_path(&options.file));
    for diagnostic in diagnostics {
        eprintln!("- {diagnostic}");
    }
    eprintln!("Tip: rerun with `--sexp` to inspect the full parse tree.");
    process::exit(1);
}

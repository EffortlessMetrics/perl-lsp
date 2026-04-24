use std::env;
use std::fmt;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, Copy)]
struct CliOptions {
    show_root_kind: bool,
    show_has_error: bool,
    show_sexp: bool,
}

#[derive(Debug)]
struct CliArgs {
    filename: PathBuf,
    options: CliOptions,
}

#[derive(Debug)]
enum CliError {
    MissingFilePath,
    UnexpectedArgument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingFilePath => write!(f, "missing <perl_file> argument"),
            CliError::UnexpectedArgument(arg) => write!(f, "unexpected argument: {arg}"),
        }
    }
}

fn usage(program_name: &str) -> String {
    format!(
        "Usage: {program_name} [--root-kind] [--has-error] [--sexp] <perl_file>\n\nOptions:\n  --root-kind  Print the root node kind\n  --has-error  Print whether the root node reports parse errors\n  --sexp       Print the tree-sitter s-expression"
    )
}

fn parse_args<I>(args: I) -> Result<CliArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut filename: Option<PathBuf> = None;
    let mut options = CliOptions { show_root_kind: false, show_has_error: false, show_sexp: false };

    for arg in args {
        match arg.as_str() {
            "--root-kind" => options.show_root_kind = true,
            "--has-error" => options.show_has_error = true,
            "--sexp" => options.show_sexp = true,
            _ if arg.starts_with('-') => return Err(CliError::UnexpectedArgument(arg)),
            _ => {
                if filename.is_some() {
                    return Err(CliError::UnexpectedArgument(arg));
                }
                filename = Some(PathBuf::from(arg));
            }
        }
    }

    let filename = filename.ok_or(CliError::MissingFilePath)?;

    Ok(CliArgs { filename, options })
}

fn run(cli_args: CliArgs) -> Result<i32, String> {
    let source_code = std::fs::read(&cli_args.filename).map_err(|error| {
        format!("failed to read '{}': {error}", cli_args.filename.to_string_lossy())
    })?;

    let tree = tree_sitter_perl_c::parse_perl_bytes(&source_code).map_err(|error| {
        format!("failed to parse '{}': {error}", cli_args.filename.to_string_lossy())
    })?;

    let root_node = tree.root_node();
    let has_error = root_node.has_error();

    if cli_args.options.show_root_kind {
        println!("root_kind: {}", root_node.kind());
    }

    if cli_args.options.show_has_error {
        println!("has_error: {has_error}");
    }

    if cli_args.options.show_sexp {
        println!("{}", root_node.to_sexp());
    }

    if has_error {
        eprintln!(
            "parse completed but contains syntax errors; re-run with --sexp for detailed triage"
        );
        Ok(1)
    } else {
        Ok(0)
    }
}

fn main() {
    let mut args = env::args();
    let program_name = args.next().unwrap_or_else(|| String::from("parse_c"));

    let cli_args = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!();
            eprintln!("{}", usage(&program_name));
            process::exit(1);
        }
    };

    let exit_code = match run(cli_args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("Error: {message}");
            1
        }
    };

    process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::{CliError, parse_args};

    #[test]
    fn parse_args_accepts_all_output_flags() -> Result<(), String> {
        let args = vec![
            String::from("--root-kind"),
            String::from("--has-error"),
            String::from("--sexp"),
            String::from("fixture.pl"),
        ];

        let parsed = parse_args(args).map_err(|error| error.to_string())?;

        if parsed.filename.to_string_lossy() != "fixture.pl" {
            return Err(String::from("expected fixture.pl file argument"));
        }

        if !parsed.options.show_root_kind {
            return Err(String::from("expected --root-kind to be enabled"));
        }

        if !parsed.options.show_has_error {
            return Err(String::from("expected --has-error to be enabled"));
        }

        if !parsed.options.show_sexp {
            return Err(String::from("expected --sexp to be enabled"));
        }

        Ok(())
    }

    #[test]
    fn parse_args_requires_file() -> Result<(), String> {
        let args = vec![String::from("--root-kind")];

        match parse_args(args) {
            Err(CliError::MissingFilePath) => Ok(()),
            Ok(_) => Err(String::from("expected missing-file error")),
            Err(error) => Err(format!("unexpected error: {error}")),
        }
    }

    #[test]
    fn parse_args_rejects_unknown_flag() -> Result<(), String> {
        let args = vec![String::from("--unknown"), String::from("fixture.pl")];

        match parse_args(args) {
            Err(CliError::UnexpectedArgument(arg)) if arg == "--unknown" => Ok(()),
            Ok(_) => Err(String::from("expected unknown-flag error")),
            Err(error) => Err(format!("unexpected error: {error}")),
        }
    }
}

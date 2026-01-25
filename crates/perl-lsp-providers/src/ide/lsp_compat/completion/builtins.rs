//! Built-in function completion for Perl
//!
//! Provides completion for Perl built-in functions with signatures.

use crate::ide::lsp_compat::completion::{context::CompletionContext, items::CompletionItem};
use std::collections::HashSet;

/// Create the builtins HashSet
pub fn create_builtins() -> HashSet<&'static str> {
    [
        // I/O
        "print",
        "printf",
        "say",
        "sprintf",
        "open",
        "close",
        "read",
        "write",
        "seek",
        "tell",
        "binmode",
        "eof",
        "fileno",
        "flock",
        // String
        "chomp",
        "chop",
        "chr",
        "ord",
        "lc",
        "uc",
        "lcfirst",
        "ucfirst",
        "length",
        "substr",
        "index",
        "rindex",
        "split",
        "join",
        "reverse",
        "sprintf",
        "quotemeta",
        // Array
        "push",
        "pop",
        "shift",
        "unshift",
        "splice",
        "grep",
        "map",
        "sort",
        "reverse",
        // Hash
        "keys",
        "values",
        "each",
        "exists",
        "delete",
        // Math
        "abs",
        "atan2",
        "cos",
        "sin",
        "exp",
        "log",
        "sqrt",
        "int",
        "rand",
        "srand",
        // File tests
        "-r",
        "-w",
        "-x",
        "-o",
        "-R",
        "-W",
        "-X",
        "-O",
        "-e",
        "-z",
        "-s",
        "-f",
        "-d",
        "-l",
        "-p",
        "-S",
        "-b",
        "-c",
        "-t",
        "-u",
        "-g",
        "-k",
        "-T",
        "-B",
        "-M",
        "-A",
        "-C",
        // System
        "system",
        "exec",
        "fork",
        "wait",
        "waitpid",
        "kill",
        "sleep",
        "alarm",
        "getpid",
        "getppid",
        // Time
        "time",
        "localtime",
        "gmtime",
        // Misc
        "caller",
        "die",
        "warn",
        "eval",
        "exit",
        "require",
        "use",
        "no",
        "import",
        "unimport",
        "bless",
        "ref",
        "tied",
        "untie",
        "pack",
        "unpack",
        "vec",
        "study",
        "pos",
        "qr",
    ]
    .into_iter()
    .collect()
}

/// Add built-in function completions
pub fn add_builtin_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    builtins: &HashSet<&'static str>,
) {
    for builtin in builtins {
        if builtin.starts_with(&context.prefix) {
            let (insert_text, detail) = match *builtin {
                "print" => ("print ", "print FILEHANDLE LIST"),
                "open" => ("open(my $fh, '<', )", "open FILEHANDLE, MODE, FILENAME"),
                "push" => ("push(@, )", "push ARRAY, LIST"),
                "map" => ("map { } ", "map BLOCK LIST"),
                "grep" => ("grep { } ", "grep BLOCK LIST"),
                "sort" => ("sort { } ", "sort BLOCK LIST"),
                _ => (*builtin, "built-in function"),
            };

            completions.push(CompletionItem {
                label: builtin.to_string(),
                kind: crate::ide::lsp_compat::completion::items::CompletionItemKind::Function,
                detail: Some(detail.to_string()),
                documentation: None,
                insert_text: Some(insert_text.to_string()),
                sort_text: Some(format!("3_{}", builtin)),
                filter_text: Some(builtin.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }
}

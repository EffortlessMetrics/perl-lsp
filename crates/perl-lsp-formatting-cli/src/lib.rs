//! Perltidy command-line argument construction.
//!
//! This crate has a single responsibility: transform [`FormattingOptions`]
//! into deterministic `perltidy` CLI arguments used by LSP formatters.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_lsp_formatting_types::FormattingOptions;

/// Build perltidy CLI arguments from formatting options.
#[must_use]
pub fn build_perltidy_args(options: &FormattingOptions) -> Vec<String> {
    let mut args = vec!["-st".to_string(), "-se".to_string()];

    if options.insert_spaces {
        args.push(format!("-et={}", options.tab_size));
        args.push(format!("-i={}", options.tab_size));
    } else {
        args.push("-dt".to_string());
        args.push(format!("-i={}", options.tab_size));
    }

    args
}

#[cfg(test)]
mod tests {
    use super::build_perltidy_args;
    use perl_lsp_formatting_types::FormattingOptions;

    #[test]
    fn builds_space_indentation_args() {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(false),
            trim_final_newlines: Some(true),
        };

        assert_eq!(build_perltidy_args(&options), vec!["-st", "-se", "-et=4", "-i=4"]);
    }

    #[test]
    fn builds_tab_indentation_args() {
        let options = FormattingOptions {
            tab_size: 2,
            insert_spaces: false,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(false),
            trim_final_newlines: Some(true),
        };

        assert_eq!(build_perltidy_args(&options), vec!["-st", "-se", "-dt", "-i=2"]);
    }
}

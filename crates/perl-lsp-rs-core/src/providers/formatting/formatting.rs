//! Code formatting support using Perl::Tidy for Perl parsing workflow pipeline.

pub use crate::providers::formatting_types::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingOptions,
};

/// Re-export PerlTidyConfig from perl-lsp-perltidy for convenience.
pub use perl_lsp_perltidy::PerlTidyConfig;

/// Count the number of UTF-16 code units in `s`.
///
/// LSP positions use UTF-16 code units (see Language Server Protocol spec Â§3.1).
/// Characters in the Basic Multilingual Plane (U+0000â€“U+FFFF) count as 1 unit;
/// supplementary-plane characters (U+10000 and above) count as 2 units.
fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| if c as u32 >= 0x10000 { 2 } else { 1 }).sum()
}

/// Formatting error.
#[derive(Debug, thiserror::Error)]
pub enum FormattingError {
    #[error(
        "perltidy not found: {0}\n\nTo install perltidy:\n  - Recommended: cpanm Perl::Tidy\n  - CPAN: cpan Perl::Tidy\n  - Debian/Ubuntu: apt-get install perltidy\n  - RedHat/Fedora: yum install perltidy\n  - macOS: brew install perltidy\n  - Windows: cpanm Perl::Tidy"
    )]
    /// perltidy executable not found on system PATH.
    PerltidyNotFound(String),

    /// Error occurred during perltidy execution.
    ///
    /// This usually means perltidy ran but reported a problem â€” check that the
    /// Perl code is syntactically valid, or inspect the perltidy output below.
    #[error("perltidy error (check Perl syntax): {0}")]
    PerltidyError(String),

    /// I/O error during file operations.
    #[error("IO error: {0}")]
    IoError(String),
}

impl FormattingError {
    /// Return a stable machine-readable error kind string for structured LSP error data.
    ///
    /// Used by LSP handlers to populate the JSON-RPC error `data` field so that
    /// clients (e.g. the VSCode extension) can present targeted remediation actions.
    #[must_use]
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::PerltidyNotFound(_) => "perltidy_not_found",
            Self::PerltidyError(_) => "perltidy_error",
            Self::IoError(_) => "io_error",
        }
    }
}

/// Code formatter using perltidy.
pub struct FormattingProvider<R> {
    /// Subprocess runtime for executing perltidy.
    runtime: R,
    /// Optional custom perltidy path.
    perltidy_path: Option<String>,
    /// Optional perltidy configuration.
    perltidy_config: Option<PerlTidyConfig>,
}

impl<R> FormattingProvider<R> {
    /// Create a new formatting provider with the given runtime.
    pub fn new(runtime: R) -> Self {
        Self { runtime, perltidy_path: None, perltidy_config: None }
    }

    /// Set a custom perltidy path.
    pub fn with_perltidy_path(mut self, path: String) -> Self {
        self.perltidy_path = Some(path);
        self
    }

    /// Set perltidy configuration.
    pub fn with_perltidy_config(mut self, config: PerlTidyConfig) -> Self {
        self.perltidy_config = Some(config);
        self
    }
}

impl<R: perl_subprocess_runtime::SubprocessRuntime> FormattingProvider<R> {
    /// Format the entire Perl script document with perltidy integration.
    pub fn format_document(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<FormattedDocument, FormattingError> {
        let formatted = match self.run_perltidy(content, options) {
            Ok(formatted) => formatted,
            Err(FormattingError::PerltidyNotFound(message)) => {
                let rust_only_formatted = apply_lsp_whitespace_options(content, options);
                if rust_only_formatted == content {
                    return Err(FormattingError::PerltidyNotFound(message));
                }
                rust_only_formatted
            }
            Err(other) => return Err(other),
        };
        let formatted = apply_lsp_whitespace_options(&formatted, options);

        if formatted == content {
            return Ok(FormattedDocument { text: formatted, edits: vec![] });
        }

        Ok(FormattedDocument {
            text: formatted.clone(),
            edits: vec![FormatTextEdit {
                range: FormatRange::whole_document(content),
                new_text: formatted,
            }],
        })
    }

    /// Format a specific range in the document.
    pub fn format_range(
        &self,
        content: &str,
        range: &FormatRange,
        options: &FormattingOptions,
    ) -> Result<FormattedDocument, FormattingError> {
        let lines: Vec<&str> = content.lines().collect();
        let start_line = range.start.line as usize;
        let end_line = (range.end.line as usize).min(lines.len().saturating_sub(1));

        if start_line >= lines.len() {
            return Ok(FormattedDocument { text: content.to_string(), edits: vec![] });
        }

        if end_line < start_line {
            return Ok(FormattedDocument { text: content.to_string(), edits: vec![] });
        }

        let text_to_format = lines[start_line..=end_line].join("\n");
        let formatted = self.run_perltidy(&text_to_format, options)?;

        if formatted == text_to_format {
            return Ok(FormattedDocument { text: content.to_string(), edits: vec![] });
        }

        let start_char = 0;
        let end_char = utf16_len(lines[end_line]) as u32;

        Ok(FormattedDocument {
            text: content.to_string(),
            edits: vec![FormatTextEdit {
                range: FormatRange::new(
                    FormatPosition::new(start_line as u32, start_char),
                    FormatPosition::new(end_line as u32, end_char),
                ),
                new_text: formatted,
            }],
        })
    }

    fn run_perltidy(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<String, FormattingError> {
        let mut args = vec!["-st".to_string(), "-se".to_string()];

        // If we have a perltidy config, use it to generate args
        if let Some(ref config) = self.perltidy_config {
            // Use config's to_args() but merge with LSP options for tab size/indent
            let mut config_args = config.to_args();

            // If profile is set, use only the profile (perltidy will read everything from there)
            if config.profile.is_some() {
                args.extend(config_args);
            } else {
                // Merge LSP options with config options
                // LSP options take precedence for indent-related settings

                // Remove any conflicting args from config_args that LSP options will override
                config_args.retain(|arg| {
                    !arg.starts_with("-i=")
                        && !arg.starts_with("--indent-columns=")
                        && !arg.starts_with("-et")
                        && !arg.starts_with("-dt")
                        && !arg.starts_with("--tabs")
                        && !arg.starts_with("--notabs")
                });

                args.extend(config_args);

                // Apply LSP formatting options for indentation
                if options.insert_spaces {
                    args.push(format!("-et={}", options.tab_size));
                    args.push(format!("-i={}", options.tab_size));
                } else {
                    args.push("-dt".to_string());
                    args.push(format!("-i={}", options.tab_size));
                }
            }
        } else {
            // Fallback to LSP options only
            if options.insert_spaces {
                args.push(format!("-et={}", options.tab_size));
                args.push(format!("-i={}", options.tab_size));
            } else {
                args.push("-dt".to_string());
                args.push(format!("-i={}", options.tab_size));
            }
        }

        let perltidy_cmd = self.perltidy_path.as_deref().unwrap_or("perltidy");

        let output = self
            .runtime
            .run_command(
                perltidy_cmd,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                Some(content.as_bytes()),
            )
            .map_err(|error| FormattingError::PerltidyNotFound(error.message))?;

        if !output.success() {
            return Err(FormattingError::PerltidyError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn apply_lsp_whitespace_options(content: &str, options: &FormattingOptions) -> String {
    let mut output = content.to_string();

    if options.trim_trailing_whitespace.unwrap_or(false) {
        output = trim_trailing_whitespace(&output);
    }

    if options.trim_final_newlines.unwrap_or(false) {
        while output.ends_with('\n') {
            output.pop();
        }
    }

    if options.insert_final_newline.unwrap_or(false) && !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn trim_trailing_whitespace(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if let Some(without_nl) = line.strip_suffix('\n') {
            let trimmed = without_nl.trim_end_matches([' ', '\t']);
            result.push_str(trimmed);
            result.push('\n');
        } else {
            result.push_str(line.trim_end_matches([' ', '\t']));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

    struct MissingPerltidyRuntime;

    impl SubprocessRuntime for MissingPerltidyRuntime {
        fn run_command(
            &self,
            _program: &str,
            _args: &[&str],
            _stdin: Option<&[u8]>,
        ) -> std::result::Result<SubprocessOutput, SubprocessError> {
            Err(SubprocessError::new("perltidy missing"))
        }
    }

    #[test]
    fn format_document_uses_rust_whitespace_fallback_when_perltidy_missing() -> Result<()> {
        let provider = FormattingProvider::new(MissingPerltidyRuntime);
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        let formatted = provider.format_document("my $x = 1;   \n\n\n", &options)?;
        assert_eq!(formatted.edits.len(), 1);
        assert_eq!(formatted.edits[0].new_text, "my $x = 1;\n");
        Ok(())
    }

    #[test]
    fn format_document_keeps_perltidy_not_found_error_when_no_rust_fallback_changes() {
        let provider = FormattingProvider::new(MissingPerltidyRuntime);
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        };

        let result = provider.format_document("my $x = 1;\n", &options);
        assert!(matches!(result, Err(FormattingError::PerltidyNotFound(_))));
    }
    #[test]
    fn apply_lsp_whitespace_options_trim_final_newlines_removes_all_trailing_newlines() {
        // Regression: previous implementation used ends_with("

") which left
        // one trailing newline. LSP trimFinalNewlines must remove ALL trailing newlines.
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: Some(true),
        };
        let r = apply_lsp_whitespace_options("content
", &options);
        assert_eq!(r, "content");
        let r = apply_lsp_whitespace_options("content

", &options);
        assert_eq!(r, "content");
        let r = apply_lsp_whitespace_options("content


", &options);
        assert_eq!(r, "content");
    }
}}

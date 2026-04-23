//! Code formatting support using Perl::Tidy for Perl parsing workflow pipeline.

pub use crate::providers::formatting_types::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingOptions,
};

/// Re-export PerlTidyConfig from perl-lsp-perltidy for convenience.
pub use perl_lsp_perltidy::PerlTidyConfig;

/// Count the number of UTF-16 code units in `s`.
///
/// LSP positions use UTF-16 code units (see Language Server Protocol spec §3.1).
/// Characters in the Basic Multilingual Plane (U+0000–U+FFFF) count as 1 unit;
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
    /// This usually means perltidy ran but reported a problem — check that the
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
        let formatted =
            self.apply_lsp_whitespace_options(self.run_perltidy(content, options)?, options, true);

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
        let formatted = self.apply_lsp_whitespace_options(
            self.run_perltidy(&text_to_format, options)?,
            options,
            false,
        );

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

    fn apply_lsp_whitespace_options(
        &self,
        mut formatted: String,
        options: &FormattingOptions,
        full_document: bool,
    ) -> String {
        if options.trim_trailing_whitespace.unwrap_or(false) {
            let had_terminal_newline = formatted.ends_with('\n');
            let trimmed_lines = formatted
                .lines()
                .map(|line| line.trim_end_matches([' ', '\t', '\r']))
                .collect::<Vec<_>>();
            formatted = trimmed_lines.join("\n");
            if had_terminal_newline {
                formatted.push('\n');
            }
        }

        if full_document {
            if options.trim_final_newlines.unwrap_or(false) {
                // LSP spec: "Trim all newlines after the final non-newline character."
                // Pop every trailing '\n' (and a preceding '\r' if CRLF) so zero remain.
                while formatted.ends_with('\n') {
                    formatted.pop();
                    if formatted.ends_with('\r') {
                        formatted.pop();
                    }
                }
            }

            if options.insert_final_newline.unwrap_or(false)
                && !formatted.is_empty()
                && !formatted.ends_with('\n')
            {
                formatted.push('\n');
            }
        }

        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

    #[test]
    fn format_document_honors_lsp_whitespace_options() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success("my $x = 1;   \nmy $y = 2;\t\t\n\n\n"));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        let result = provider.format_document("my $x = 1;\nmy $y = 2;", &options)?;

        assert_eq!(result.edits.len(), 1);
        assert_eq!(result.text, "my $x = 1;\nmy $y = 2;\n");
        Ok(())
    }

    #[test]
    fn format_range_trims_trailing_whitespace_when_requested()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success("sub test {\n    return $x;   \n}\n"));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(false),
            trim_final_newlines: Some(true),
        };

        let content = "my $x = 1;\nsub test{return$x;}\nmy $y = 2;";
        let range = FormatRange::new(FormatPosition::new(1, 0), FormatPosition::new(1, 20));
        let result = provider.format_range(content, &range, &options)?;

        assert_eq!(result.edits.len(), 1);
        assert_eq!(result.edits[0].new_text, "sub test {\n    return $x;\n}\n");
        Ok(())
    }

    // Regression: `trim_final_newlines` previously stopped when a single trailing
    // `\n` remained (used `ends_with("\n\n")`), producing output that still had one
    // trailing newline even when the caller asked for zero. Per the LSP spec
    // (`trimFinalNewlines`): "Trim all newlines after the final non-newline
    // character." With `insert_final_newline=false` this must remove every
    // trailing newline.
    #[test]
    fn trim_final_newlines_removes_all_trailing_newlines() -> Result<(), Box<dyn std::error::Error>>
    {
        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success("foo;\n\n\n"));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(false),
            insert_final_newline: Some(false),
            trim_final_newlines: Some(true),
        };

        let result = provider.format_document("foo;", &options)?;

        assert_eq!(result.text, "foo;", "expected zero trailing newlines");
        Ok(())
    }

    // Regression: CRLF inputs. `trim_trailing_whitespace` should strip `\r`
    // (not just spaces/tabs) so the line doesn't retain a bare carriage return
    // after perltidy emits CRLF-normalized output.
    #[test]
    fn trim_trailing_whitespace_handles_crlf_inputs() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success("foo;   \r\nbar;\t\r\n"));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(false),
            trim_final_newlines: Some(false),
        };

        let result = provider.format_document("foo;\nbar;", &options)?;

        assert!(
            !result.text.contains('\r'),
            "expected no carriage returns in trimmed output, got {:?}",
            result.text
        );
        assert_eq!(result.text, "foo;\nbar;\n");
        Ok(())
    }

    // Empty-document path: all flags set must no-op without panicking and
    // without inserting a spurious trailing newline (the `insert_final_newline`
    // branch is gated on `!formatted.is_empty()`).
    #[test]
    fn empty_document_is_noop_with_all_flags_set() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = MockSubprocessRuntime::new();
        runtime.add_response(MockResponse::success(""));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        let result = provider.format_document("", &options)?;

        assert_eq!(result.text, "", "empty document must stay empty");
        assert!(result.edits.is_empty(), "no edits expected for empty document");
        Ok(())
    }

    // Idempotence: applying the LSP whitespace pass twice must yield the same
    // output as applying it once. Catches accumulator-style bugs where repeated
    // application corrupts the text.
    #[test]
    fn lsp_whitespace_options_are_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = MockSubprocessRuntime::new();
        // Same perltidy output for both passes.
        runtime.add_response(MockResponse::success("my $x = 1;   \nmy $y = 2;\n\n\n"));
        runtime.add_response(MockResponse::success("my $x = 1;\nmy $y = 2;\n"));
        let provider = FormattingProvider::new(runtime);

        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        let first = provider.format_document("my $x = 1;\nmy $y = 2;", &options)?;
        let second = provider.format_document(&first.text, &options)?;

        assert_eq!(first.text, "my $x = 1;\nmy $y = 2;\n");
        assert_eq!(second.text, first.text, "second pass must match first");
        Ok(())
    }
}

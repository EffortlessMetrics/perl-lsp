//! Code formatting support using Perl::Tidy for Perl parsing workflow pipeline.

pub use perl_lsp_formatting_types::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingOptions,
};

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
}

impl<R> FormattingProvider<R> {
    /// Create a new formatting provider with the given runtime.
    pub fn new(runtime: R) -> Self {
        Self { runtime, perltidy_path: None }
    }

    /// Set a custom perltidy path.
    pub fn with_perltidy_path(mut self, path: String) -> Self {
        self.perltidy_path = Some(path);
        self
    }
}

impl<R: perl_lsp_tooling::SubprocessRuntime> FormattingProvider<R> {
    /// Format the entire Perl script document with perltidy integration.
    pub fn format_document(
        &self,
        content: &str,
        options: &FormattingOptions,
    ) -> Result<FormattedDocument, FormattingError> {
        let formatted = self.run_perltidy(content, options)?;

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
        let end_char = lines[end_line].len() as u32;

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

        if options.insert_spaces {
            args.push(format!("-et={}", options.tab_size));
            args.push(format!("-i={}", options.tab_size));
        } else {
            args.push("-dt".to_string());
            args.push(format!("-i={}", options.tab_size));
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

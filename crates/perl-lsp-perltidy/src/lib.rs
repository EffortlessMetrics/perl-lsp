//! Perltidy integration for code formatting.
//!
//! This crate isolates Perl formatting concerns behind a small API so the
//! broader tooling crate can focus on composition rather than formatter
//! implementation details.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_subprocess_runtime::SubprocessRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Configuration for perltidy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlTidyConfig {
    /// Maximum line length.
    pub maximum_line_length: Option<u32>,
    /// Indent size (spaces).
    pub indent_columns: Option<u32>,
    /// Use tabs instead of spaces.
    pub tabs: Option<bool>,
    /// Opening brace on same line.
    pub opening_brace_on_new_line: Option<bool>,
    /// Cuddled else.
    pub cuddled_else: Option<bool>,
    /// Space after keyword.
    pub space_after_keyword: Option<bool>,
    /// Add trailing commas.
    pub add_trailing_commas: Option<bool>,
    /// Vertical alignment.
    pub vertical_alignment: Option<bool>,
    /// Block comment indentation.
    pub block_comment_indentation: Option<u32>,
    /// Custom perltidyrc file path.
    pub profile: Option<String>,
    /// Additional command line arguments.
    pub extra_args: Vec<String>,
}

impl Default for PerlTidyConfig {
    fn default() -> Self {
        Self {
            maximum_line_length: Some(80),
            indent_columns: Some(4),
            tabs: Some(false),
            opening_brace_on_new_line: Some(false),
            cuddled_else: Some(true),
            space_after_keyword: Some(true),
            add_trailing_commas: Some(false),
            vertical_alignment: Some(true),
            block_comment_indentation: Some(0),
            profile: None,
            extra_args: Vec::new(),
        }
    }
}

impl PerlTidyConfig {
    /// Create a config for PBP (Perl Best Practices) style.
    #[must_use]
    pub fn pbp() -> Self {
        Self {
            maximum_line_length: Some(78),
            indent_columns: Some(4),
            tabs: Some(false),
            opening_brace_on_new_line: Some(false),
            cuddled_else: Some(false),
            space_after_keyword: Some(true),
            add_trailing_commas: Some(true),
            vertical_alignment: Some(true),
            block_comment_indentation: Some(0),
            profile: None,
            extra_args: vec!["--perl-best-practices".to_string()],
        }
    }

    /// Create a config for GNU style.
    #[must_use]
    pub fn gnu() -> Self {
        Self {
            maximum_line_length: Some(79),
            indent_columns: Some(2),
            tabs: Some(false),
            opening_brace_on_new_line: Some(true),
            cuddled_else: Some(false),
            space_after_keyword: Some(true),
            add_trailing_commas: Some(false),
            vertical_alignment: Some(false),
            block_comment_indentation: Some(2),
            profile: None,
            extra_args: vec!["--gnu-style".to_string()],
        }
    }

    /// Convert the configuration to `perltidy` command-line arguments.
    #[must_use]
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(profile) = &self.profile {
            args.push(format!("--profile={profile}"));
            return args;
        }

        if let Some(len) = self.maximum_line_length {
            args.push(format!("--maximum-line-length={len}"));
        }

        if let Some(indent) = self.indent_columns {
            args.push(format!("--indent-columns={indent}"));
        }

        if let Some(tabs) = self.tabs {
            if tabs {
                args.push("--tabs".to_string());
            } else {
                args.push("--notabs".to_string());
            }
        }

        if let Some(brace) = self.opening_brace_on_new_line {
            if brace {
                args.push("--opening-brace-on-new-line".to_string());
            } else {
                args.push("--opening-brace-always-on-right".to_string());
            }
        }

        if let Some(cuddle) = self.cuddled_else {
            if cuddle {
                args.push("--cuddled-else".to_string());
            } else {
                args.push("--nocuddled-else".to_string());
            }
        }

        if let Some(space) = self.space_after_keyword {
            if space {
                args.push("--space-after-keyword".to_string());
            } else {
                args.push("--nospace-after-keyword".to_string());
            }
        }

        if let Some(comma) = self.add_trailing_commas {
            if comma {
                args.push("--add-trailing-commas".to_string());
            } else {
                args.push("--no-add-trailing-commas".to_string());
            }
        }

        if let Some(align) = self.vertical_alignment {
            if align {
                args.push("--vertical-alignment".to_string());
            } else {
                args.push("--no-vertical-alignment".to_string());
            }
        }

        if let Some(indent) = self.block_comment_indentation {
            args.push(format!("--block-comment-indentation={indent}"));
        }

        args.extend(self.extra_args.clone());
        args
    }
}

/// Perltidy formatter.
pub struct PerlTidyFormatter {
    config: PerlTidyConfig,
    cache: HashMap<String, String>,
    runtime: Arc<dyn SubprocessRuntime>,
}

impl PerlTidyFormatter {
    /// Creates a new formatter with the given configuration and runtime.
    #[must_use]
    pub fn new(config: PerlTidyConfig, runtime: Arc<dyn SubprocessRuntime>) -> Self {
        Self { config, cache: HashMap::new(), runtime }
    }

    /// Creates a new formatter with the OS subprocess runtime (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn with_os_runtime(config: PerlTidyConfig) -> Self {
        use perl_subprocess_runtime::OsSubprocessRuntime;
        Self::new(config, Arc::new(OsSubprocessRuntime::new()))
    }

    /// Format Perl code.
    pub fn format(&mut self, code: &str) -> Result<String, String> {
        if let Some(cached) = self.cache.get(code) {
            return Ok(cached.clone());
        }

        let mut args = self.config.to_args();
        args.push("-st".to_string());
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let output = self
            .runtime
            .run_command("perltidy", &args_refs, Some(code.as_bytes()))
            .map_err(|e| e.message)?;

        if !output.success() {
            return Err(format!("Perltidy failed: {}", output.stderr_lossy()));
        }

        let formatted = String::from_utf8(output.stdout)
            .map_err(|e| format!("Invalid UTF-8 from perltidy: {e}"))?;
        self.cache.insert(code.to_string(), formatted.clone());
        Ok(formatted)
    }

    /// Format a file in place.
    pub fn format_file(&self, file_path: &Path) -> Result<(), String> {
        let mut args = self.config.to_args();
        args.push("--".to_string());
        args.push(file_path.to_string_lossy().into_owned());
        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let output =
            self.runtime.run_command("perltidy", &args_refs, None).map_err(|e| e.message)?;

        if !output.success() {
            return Err(format!("Perltidy failed: {}", output.stderr_lossy()));
        }

        Ok(())
    }

    /// Clear any memoized formatting results.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Format a range of code.
    pub fn format_range(
        &mut self,
        code: &str,
        start_line: u32,
        end_line: u32,
    ) -> Result<String, String> {
        let lines: Vec<&str> = code.lines().collect();

        if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
            return Err("Line range out of bounds".to_string());
        }

        let range_code = lines[start_line as usize..=end_line as usize].join("\n");
        let formatted_range = self.format(&range_code)?;

        let mut result = Vec::new();
        if start_line > 0 {
            result.extend_from_slice(&lines[0..start_line as usize]);
        }
        result.extend(formatted_range.lines());
        if (end_line as usize) < lines.len() - 1 {
            result.extend_from_slice(&lines[(end_line as usize + 1)..]);
        }

        Ok(result.join("\n"))
    }

    /// Get formatting suggestions without applying them.
    pub fn get_suggestions(&mut self, code: &str) -> Result<Vec<FormatSuggestion>, String> {
        let formatted = self.format(code)?;
        if formatted == code {
            return Ok(Vec::new());
        }

        let orig_lines: Vec<&str> = code.lines().collect();
        let fmt_lines: Vec<&str> = formatted.lines().collect();
        let mut suggestions = Vec::new();

        for (i, (orig, fmt)) in orig_lines.iter().zip(fmt_lines.iter()).enumerate() {
            if orig != fmt {
                suggestions.push(FormatSuggestion {
                    line: i as u32,
                    original: (*orig).to_string(),
                    formatted: (*fmt).to_string(),
                    description: "Line formatting change".to_string(),
                });
            }
        }

        Ok(suggestions)
    }
}

/// A formatting suggestion.
#[derive(Debug, Clone)]
pub struct FormatSuggestion {
    /// Zero-based line number where the change applies.
    pub line: u32,
    /// Original line content before formatting.
    pub original: String,
    /// Suggested formatted line content.
    pub formatted: String,
    /// Human-readable description of the formatting change.
    pub description: String,
}

/// Built-in formatter for when `perltidy` is unavailable.
pub struct BuiltInFormatter {
    config: PerlTidyConfig,
}

impl BuiltInFormatter {
    /// Creates a new built-in formatter with the given configuration.
    #[must_use]
    pub fn new(config: PerlTidyConfig) -> Self {
        Self { config }
    }

    /// Apply basic indentation-based formatting without invoking `perltidy`.
    #[must_use]
    pub fn format(&self, code: &str) -> String {
        let mut result = String::new();
        let mut indent_level: i32 = 0;
        let indent_str = if self.config.tabs.unwrap_or(false) {
            "\t".to_string()
        } else {
            " ".repeat(self.config.indent_columns.unwrap_or(4) as usize)
        };

        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('}') || trimmed.starts_with(')') || trimmed.starts_with(']') {
                indent_level = indent_level.saturating_sub(1);
            }

            if !trimmed.is_empty() {
                for _ in 0..indent_level {
                    result.push_str(&indent_str);
                }
                result.push_str(trimmed);
            }
            result.push('\n');

            if trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[') {
                indent_level += 1;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};
    use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};
    use perl_tdd_support::{must, must_some};

    #[test]
    fn config_to_args_includes_core_flags() {
        let args = PerlTidyConfig::default().to_args();
        assert!(args.contains(&"--maximum-line-length=80".to_string()));
        assert!(args.contains(&"--indent-columns=4".to_string()));
        assert!(args.contains(&"--notabs".to_string()));
    }

    #[test]
    fn pbp_preset_sets_best_practices_flag() {
        let args = PerlTidyConfig::pbp().to_args();
        assert!(args.contains(&"--perl-best-practices".to_string()));
        assert!(args.contains(&"--maximum-line-length=78".to_string()));
    }

    #[test]
    fn builtin_formatter_indents_block_contents() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("if ($x) {\nprint $x;\n}\n");
        assert!(formatted.contains("    print"));
    }

    #[test]
    fn formatter_with_mock_runtime_formats_code() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"my $x = 1;\n".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let result = formatter.format("my $x=1;");
        assert_eq!(must(result), "my $x = 1;\n");

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perltidy");
        assert!(invocations[0].args.contains(&"-st".to_string()));
    }

    #[test]
    fn formatter_caches_repeat_requests() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let first = formatter.format("original");
        let second = formatter.format("original");
        assert_eq!(must(first), must(second));
        assert_eq!(runtime.invocations().len(), 1);
    }

    #[test]
    fn formatter_surfaces_runtime_failures() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::failure(b"syntax error".to_vec(), 1));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.format("invalid code");
        assert!(result.is_err());
        assert!(result.err().is_some_and(|msg| msg.contains("syntax error")));
    }

    #[test]
    fn format_file_uses_argument_separator() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(Vec::new()));
        let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let result = formatter.format_file(Path::new("test.pl"));
        assert!(result.is_ok());

        let invocations = runtime.invocations();
        let sep_pos = must_some(invocations[0].args.iter().position(|arg| arg == "--"));
        let file_pos = must_some(invocations[0].args.iter().position(|arg| arg == "test.pl"));
        assert!(sep_pos < file_pos);
    }

    // --- Basic formatting request tests ---

    #[test]
    fn format_passes_code_via_stdin() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"use strict;\n".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let _result = must(formatter.format("use strict;"));

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].stdin, Some(b"use strict;".to_vec()));
    }

    #[test]
    fn format_appends_stdout_flag() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"output\n".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let _ = must(formatter.format("input"));

        let invocations = runtime.invocations();
        let last_config_arg = invocations[0].args.last().map(String::as_str);
        assert_eq!(last_config_arg, Some("-st"));
    }

    #[test]
    fn format_returns_perltidy_output_verbatim() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let expected = "my $x = 1;\nmy $y = 2;\n";
        runtime.add_response(MockResponse::success(expected.as_bytes().to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = must(formatter.format("my $x=1;\nmy $y=2;"));
        assert_eq!(result, expected);
    }

    #[test]
    fn format_handles_empty_input() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = must(formatter.format(""));
        assert_eq!(result, "");
    }

    // --- Range formatting tests ---

    #[test]
    fn format_range_formats_selected_lines() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        // The range formatter will extract line 1 ("my $y=2;") and format it
        runtime.add_response(MockResponse::success(b"my $y = 2;".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "my $x = 1;\nmy $y=2;\nmy $z = 3;";
        let result = must(formatter.format_range(code, 1, 1));

        // Line 0 and line 2 preserved, line 1 formatted
        assert!(result.contains("my $x = 1;"));
        assert!(result.contains("my $y = 2;"));
        assert!(result.contains("my $z = 3;"));
    }

    #[test]
    fn format_range_preserves_lines_before_range() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "line0\nline1\nline2\nline3";
        let result = must(formatter.format_range(code, 2, 2));

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "line0");
        assert_eq!(lines[1], "line1");
    }

    #[test]
    fn format_range_preserves_lines_after_range() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "line0\nline1\nline2\nline3";
        let result = must(formatter.format_range(code, 1, 1));

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(*lines.last().unwrap(), "line3");
    }

    #[test]
    fn format_range_multiline_range() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"a\nb".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "header\nline1\nline2\nfooter";
        let result = must(formatter.format_range(code, 1, 2));

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "header");
        assert_eq!(lines[1], "a");
        assert_eq!(lines[2], "b");
        assert_eq!(lines[3], "footer");
    }

    #[test]
    fn format_range_start_line_out_of_bounds() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "one\ntwo\nthree";
        let result = formatter.format_range(code, 10, 12);

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("out of bounds"));
    }

    #[test]
    fn format_range_end_line_out_of_bounds() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "one\ntwo\nthree";
        let result = formatter.format_range(code, 0, 10);

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("out of bounds"));
    }

    #[test]
    fn format_range_first_line_only() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted_first".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "line0\nline1\nline2";
        let result = must(formatter.format_range(code, 0, 0));

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "formatted_first");
        assert_eq!(lines[1], "line1");
    }

    #[test]
    fn format_range_last_line_only() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted_last".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "line0\nline1\nline2";
        let result = must(formatter.format_range(code, 2, 2));

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "line0");
        assert_eq!(lines[1], "line1");
        assert_eq!(lines[2], "formatted_last");
    }

    // --- Format on type (built-in formatter) tests ---

    #[test]
    fn builtin_formatter_dedents_closing_braces() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("sub foo {\nreturn 1;\n}\n");

        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[0], "sub foo {");
        assert_eq!(lines[1], "    return 1;");
        assert_eq!(lines[2], "}");
    }

    #[test]
    fn builtin_formatter_handles_nested_blocks() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("if ($a) {\nif ($b) {\nprint 1;\n}\n}\n");

        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[0], "if ($a) {");
        assert_eq!(lines[1], "    if ($b) {");
        assert_eq!(lines[2], "        print 1;");
        assert_eq!(lines[3], "    }");
        assert_eq!(lines[4], "}");
    }

    #[test]
    fn builtin_formatter_preserves_empty_lines() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("line1\n\nline2\n");

        assert!(formatted.contains("\n\n"));
    }

    #[test]
    fn builtin_formatter_uses_tabs_when_configured() {
        let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
        let formatter = BuiltInFormatter::new(config);
        let formatted = formatter.format("sub foo {\nreturn 1;\n}\n");

        assert!(formatted.contains("\treturn 1;"));
    }

    #[test]
    fn builtin_formatter_respects_indent_columns() {
        let config = PerlTidyConfig { indent_columns: Some(2), ..PerlTidyConfig::default() };
        let formatter = BuiltInFormatter::new(config);
        let formatted = formatter.format("if (1) {\nprint;\n}\n");

        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[1], "  print;");
    }

    #[test]
    fn builtin_formatter_handles_parens_and_brackets() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("my @arr = (\n1,\n2,\n);\n");

        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines[1], "    1,");
        assert_eq!(lines[2], "    2,");
        assert_eq!(lines[3], ");");
    }

    #[test]
    fn builtin_formatter_handles_empty_input() {
        let formatter = BuiltInFormatter::new(PerlTidyConfig::default());
        let formatted = formatter.format("");
        assert_eq!(formatted, "");
    }

    // --- Missing perltidy binary handling ---

    /// A mock runtime that always returns an error, simulating a missing binary.
    struct MissingBinaryRuntime;

    impl SubprocessRuntime for MissingBinaryRuntime {
        fn run_command(
            &self,
            program: &str,
            _args: &[&str],
            _stdin: Option<&[u8]>,
        ) -> Result<SubprocessOutput, SubprocessError> {
            Err(SubprocessError::new(format!(
                "Failed to start {program}: No such file or directory"
            )))
        }
    }

    #[test]
    fn format_returns_error_when_binary_missing() {
        let runtime = Arc::new(MissingBinaryRuntime);
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.format("my $x = 1;");

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("Failed to start perltidy"));
    }

    #[test]
    fn format_file_returns_error_when_binary_missing() {
        let runtime = Arc::new(MissingBinaryRuntime);
        let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.format_file(Path::new("test.pl"));

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("Failed to start perltidy"));
    }

    #[test]
    fn format_range_returns_error_when_binary_missing() {
        let runtime = Arc::new(MissingBinaryRuntime);
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let code = "line0\nline1\nline2";
        let result = formatter.format_range(code, 1, 1);

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("Failed to start perltidy"));
    }

    #[test]
    fn get_suggestions_returns_error_when_binary_missing() {
        let runtime = Arc::new(MissingBinaryRuntime);
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.get_suggestions("my $x = 1;");

        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("Failed to start perltidy"));
    }

    // --- Perltidy configuration (.perltidyrc) tests ---

    #[test]
    fn config_with_profile_uses_only_profile_flag() {
        let config = PerlTidyConfig {
            profile: Some("/home/user/.perltidyrc".to_string()),
            ..PerlTidyConfig::default()
        };
        let args = config.to_args();

        assert_eq!(args.len(), 1);
        assert_eq!(args[0], "--profile=/home/user/.perltidyrc");
    }

    #[test]
    fn config_with_profile_ignores_other_settings() {
        let config = PerlTidyConfig {
            maximum_line_length: Some(120),
            indent_columns: Some(8),
            tabs: Some(true),
            profile: Some(".perltidyrc".to_string()),
            ..PerlTidyConfig::default()
        };
        let args = config.to_args();

        // Only profile flag should be present; all others suppressed
        assert_eq!(args.len(), 1);
        assert!(args[0].starts_with("--profile="));
    }

    #[test]
    fn config_profile_path_passed_to_perltidy() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));
        let config = PerlTidyConfig {
            profile: Some("/project/.perltidyrc".to_string()),
            ..PerlTidyConfig::default()
        };
        let mut formatter = PerlTidyFormatter::new(config, runtime.clone());

        let _ = must(formatter.format("code"));

        let invocations = runtime.invocations();
        assert!(invocations[0].args.contains(&"--profile=/project/.perltidyrc".to_string()));
    }

    // --- GNU preset tests ---

    #[test]
    fn gnu_preset_sets_gnu_style_flag() {
        let args = PerlTidyConfig::gnu().to_args();
        assert!(args.contains(&"--gnu-style".to_string()));
    }

    #[test]
    fn gnu_preset_uses_two_space_indent() {
        let args = PerlTidyConfig::gnu().to_args();
        assert!(args.contains(&"--indent-columns=2".to_string()));
    }

    #[test]
    fn gnu_preset_opens_brace_on_new_line() {
        let args = PerlTidyConfig::gnu().to_args();
        assert!(args.contains(&"--opening-brace-on-new-line".to_string()));
    }

    // --- Config flag generation tests ---

    #[test]
    fn config_tabs_true_generates_tabs_flag() {
        let config = PerlTidyConfig { tabs: Some(true), ..PerlTidyConfig::default() };
        let args = config.to_args();
        assert!(args.contains(&"--tabs".to_string()));
        assert!(!args.contains(&"--notabs".to_string()));
    }

    #[test]
    fn config_cuddled_else_false_generates_nocuddled() {
        let config = PerlTidyConfig { cuddled_else: Some(false), ..PerlTidyConfig::default() };
        let args = config.to_args();
        assert!(args.contains(&"--nocuddled-else".to_string()));
    }

    #[test]
    fn config_space_after_keyword_false() {
        let config =
            PerlTidyConfig { space_after_keyword: Some(false), ..PerlTidyConfig::default() };
        let args = config.to_args();
        assert!(args.contains(&"--nospace-after-keyword".to_string()));
    }

    #[test]
    fn config_add_trailing_commas_true() {
        let config =
            PerlTidyConfig { add_trailing_commas: Some(true), ..PerlTidyConfig::default() };
        let args = config.to_args();
        assert!(args.contains(&"--add-trailing-commas".to_string()));
    }

    #[test]
    fn config_vertical_alignment_false() {
        let config =
            PerlTidyConfig { vertical_alignment: Some(false), ..PerlTidyConfig::default() };
        let args = config.to_args();
        assert!(args.contains(&"--no-vertical-alignment".to_string()));
    }

    #[test]
    fn config_extra_args_appended() {
        let config = PerlTidyConfig {
            extra_args: vec!["--custom-flag".to_string(), "--another".to_string()],
            ..PerlTidyConfig::default()
        };
        let args = config.to_args();
        assert!(args.contains(&"--custom-flag".to_string()));
        assert!(args.contains(&"--another".to_string()));
    }

    #[test]
    fn config_none_fields_omit_flags() {
        let config = PerlTidyConfig {
            maximum_line_length: None,
            indent_columns: None,
            tabs: None,
            opening_brace_on_new_line: None,
            cuddled_else: None,
            space_after_keyword: None,
            add_trailing_commas: None,
            vertical_alignment: None,
            block_comment_indentation: None,
            profile: None,
            extra_args: Vec::new(),
        };
        let args = config.to_args();
        assert!(args.is_empty());
    }

    // --- get_suggestions tests ---

    #[test]
    fn get_suggestions_returns_empty_when_code_unchanged() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let code = "my $x = 1;\n";
        runtime.add_response(MockResponse::success(code.as_bytes().to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let suggestions = must(formatter.get_suggestions(code));
        assert!(suggestions.is_empty());
    }

    #[test]
    fn get_suggestions_returns_changed_lines() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let original = "my $x=1;\nmy $y = 2;\n";
        let formatted = "my $x = 1;\nmy $y = 2;\n";
        runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let suggestions = must(formatter.get_suggestions(original));
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].line, 0);
        assert_eq!(suggestions[0].original, "my $x=1;");
        assert_eq!(suggestions[0].formatted, "my $x = 1;");
    }

    #[test]
    fn get_suggestions_reports_multiple_changes() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        let original = "my $a=1;\nmy $b=2;\nmy $c=3;\n";
        let formatted = "my $a = 1;\nmy $b=2;\nmy $c = 3;\n";
        runtime.add_response(MockResponse::success(formatted.as_bytes().to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let suggestions = must(formatter.get_suggestions(original));
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].line, 0);
        assert_eq!(suggestions[1].line, 2);
    }

    // --- clear_cache tests ---

    #[test]
    fn clear_cache_forces_re_invocation() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"result1\n".to_vec()));
        runtime.add_response(MockResponse::success(b"result2\n".to_vec()));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());

        let first = must(formatter.format("code"));
        assert_eq!(first, "result1\n");

        formatter.clear_cache();

        let second = must(formatter.format("code"));
        assert_eq!(second, "result2\n");

        // Two invocations: cache was cleared between them
        assert_eq!(runtime.invocations().len(), 2);
    }

    // --- Invalid UTF-8 output handling ---

    #[test]
    fn format_returns_error_on_invalid_utf8() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        // Invalid UTF-8 bytes
        runtime.add_response(MockResponse::success(vec![0xFF, 0xFE, 0x00]));
        let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.format("code");
        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("Invalid UTF-8"));
    }

    // --- format_file failure handling ---

    #[test]
    fn format_file_returns_error_on_perltidy_failure() {
        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::failure(b"can't open file".to_vec(), 2));
        let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

        let result = formatter.format_file(Path::new("/nonexistent/file.pl"));
        assert!(result.is_err());
        let err = perl_tdd_support::must_err(result);
        assert!(err.contains("can't open file"));
    }

    // --- Serde roundtrip test ---

    #[test]
    fn config_serializes_and_deserializes() -> Result<(), Box<dyn std::error::Error>> {
        let config = PerlTidyConfig::default();
        let json = serde_json::to_string(&config)?;
        let restored: PerlTidyConfig = serde_json::from_str(&json)?;
        assert_eq!(restored.maximum_line_length, config.maximum_line_length);
        assert_eq!(restored.indent_columns, config.indent_columns);
        assert_eq!(restored.tabs, config.tabs);
        Ok(())
    }
}

//! Perltidy integration for code formatting
//!
//! This module provides integration with perltidy for automatic code formatting
//! and beautification of Perl code.

use super::subprocess_runtime::SubprocessRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Configuration for perltidy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlTidyConfig {
    /// Maximum line length
    pub maximum_line_length: Option<u32>,
    /// Indent size (spaces)
    pub indent_columns: Option<u32>,
    /// Use tabs instead of spaces
    pub tabs: Option<bool>,
    /// Opening brace on same line
    pub opening_brace_on_new_line: Option<bool>,
    /// Cuddled else
    pub cuddled_else: Option<bool>,
    /// Space after keyword
    pub space_after_keyword: Option<bool>,
    /// Add trailing commas
    pub add_trailing_commas: Option<bool>,
    /// Vertical alignment
    pub vertical_alignment: Option<bool>,
    /// Block comment indentation
    pub block_comment_indentation: Option<u32>,
    /// Custom perltidyrc file path
    pub profile: Option<String>,
    /// Additional command line arguments
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
    /// Create a config for PBP (Perl Best Practices) style
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

    /// Create a config for GNU style
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

    /// Convert to perltidy command line arguments
    fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(profile) = &self.profile {
            args.push(format!("--profile={}", profile));
            // If using a profile, don't add other options
            return args;
        }

        if let Some(len) = self.maximum_line_length {
            args.push(format!("--maximum-line-length={}", len));
        }

        if let Some(indent) = self.indent_columns {
            args.push(format!("--indent-columns={}", indent));
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
            args.push(format!("--block-comment-indentation={}", indent));
        }

        // Add extra args
        args.extend(self.extra_args.clone());

        args
    }
}

/// Perltidy formatter
pub struct PerlTidyFormatter {
    /// Configuration settings for perltidy invocation.
    config: PerlTidyConfig,
    /// Cache mapping source code to formatted output.
    cache: HashMap<String, String>,
    /// Subprocess runtime for executing perltidy
    runtime: Arc<dyn SubprocessRuntime>,
}

impl PerlTidyFormatter {
    /// Creates a new formatter with the given configuration and runtime.
    pub fn new(config: PerlTidyConfig, runtime: Arc<dyn SubprocessRuntime>) -> Self {
        Self { config, cache: HashMap::new(), runtime }
    }

    /// Creates a new formatter with the OS subprocess runtime (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_os_runtime(config: PerlTidyConfig) -> Self {
        use super::subprocess_runtime::OsSubprocessRuntime;
        Self::new(config, Arc::new(OsSubprocessRuntime::new()))
    }

    /// Format Perl code
    pub fn format(&mut self, code: &str) -> Result<String, String> {
        // Check cache
        if let Some(cached) = self.cache.get(code) {
            return Ok(cached.clone());
        }

        // Build argument list
        let mut args: Vec<String> = self.config.to_args();
        args.push("-st".to_string()); // Output to stdout

        // Convert to &str slice for the runtime
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Run perltidy via the runtime
        let output = self
            .runtime
            .run_command("perltidy", &args_refs, Some(code.as_bytes()))
            .map_err(|e| e.message)?;

        if !output.success() {
            return Err(format!("Perltidy failed: {}", output.stderr_lossy()));
        }

        let formatted = String::from_utf8(output.stdout)
            .map_err(|e| format!("Invalid UTF-8 from perltidy: {}", e))?;

        // Cache result
        self.cache.insert(code.to_string(), formatted.clone());

        Ok(formatted)
    }

    /// Format a file in place
    pub fn format_file(&self, file_path: &Path) -> Result<(), String> {
        // Build argument list
        let mut args: Vec<String> = self.config.to_args();
        // SECURITY: Add `--` to prevent argument injection via filenames starting with `-`
        // (e.g., a file named `-rf` would otherwise be interpreted as a flag)
        args.push("--".to_string());
        args.push(file_path.to_string_lossy().into_owned());

        // Convert to &str slice for the runtime
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Run perltidy via the runtime
        let output =
            self.runtime.run_command("perltidy", &args_refs, None).map_err(|e| e.message)?;

        if !output.success() {
            return Err(format!("Perltidy failed: {}", output.stderr_lossy()));
        }

        Ok(())
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Format a range of code
    pub fn format_range(
        &mut self,
        code: &str,
        start_line: u32,
        end_line: u32,
    ) -> Result<String, String> {
        // Split code into lines
        let lines: Vec<&str> = code.lines().collect();

        if start_line as usize >= lines.len() || end_line as usize >= lines.len() {
            return Err("Line range out of bounds".to_string());
        }

        // Extract the range to format
        let range_code = lines[start_line as usize..=end_line as usize].join("\n");

        // Format the range
        let formatted_range = self.format(&range_code)?;

        // Reconstruct the full code
        let mut result = Vec::new();

        // Add lines before range
        if start_line > 0 {
            result.extend_from_slice(&lines[0..start_line as usize]);
        }

        // Add formatted range
        result.extend(formatted_range.lines());

        // Add lines after range
        if (end_line as usize) < lines.len() - 1 {
            result.extend_from_slice(&lines[(end_line as usize + 1)..]);
        }

        Ok(result.join("\n"))
    }

    /// Get formatting suggestions without applying them
    pub fn get_suggestions(&mut self, code: &str) -> Result<Vec<FormatSuggestion>, String> {
        let formatted = self.format(code)?;

        if formatted == code {
            return Ok(Vec::new());
        }

        // Compare original and formatted to generate suggestions
        let mut suggestions = Vec::new();

        let orig_lines: Vec<&str> = code.lines().collect();
        let fmt_lines: Vec<&str> = formatted.lines().collect();

        for (i, (orig, fmt)) in orig_lines.iter().zip(fmt_lines.iter()).enumerate() {
            if orig != fmt {
                suggestions.push(FormatSuggestion {
                    line: i as u32,
                    original: orig.to_string(),
                    formatted: fmt.to_string(),
                    description: "Line formatting change".to_string(),
                });
            }
        }

        Ok(suggestions)
    }
}

/// A formatting suggestion
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

/// Built-in formatter for when perltidy is not available
pub struct BuiltInFormatter {
    /// Configuration settings controlling formatting behavior.
    config: PerlTidyConfig,
}

impl BuiltInFormatter {
    /// Creates a new built-in formatter with the given configuration.
    pub fn new(config: PerlTidyConfig) -> Self {
        Self { config }
    }

    /// Basic formatting without perltidy
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

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(')') || trimmed.starts_with(']') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add indentation
            if !trimmed.is_empty() {
                for _ in 0..indent_level {
                    result.push_str(&indent_str);
                }
                result.push_str(trimmed);
            }
            result.push('\n');

            // Increase indent for opening braces
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
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_config_to_args() {
        let config = PerlTidyConfig::default();
        let args = config.to_args();

        assert!(args.contains(&"--maximum-line-length=80".to_string()));
        assert!(args.contains(&"--indent-columns=4".to_string()));
        assert!(args.contains(&"--notabs".to_string()));
    }

    #[test]
    fn test_pbp_config() {
        let config = PerlTidyConfig::pbp();
        let args = config.to_args();

        assert!(args.contains(&"--perl-best-practices".to_string()));
        assert!(args.contains(&"--maximum-line-length=78".to_string()));
    }

    #[test]
    fn test_builtin_formatter() {
        let config = PerlTidyConfig::default();
        let formatter = BuiltInFormatter::new(config);

        let code = "if ($x) {\nprint $x;\n}\n";
        let formatted = formatter.format(code);

        assert!(formatted.contains("    print")); // Should be indented
    }

    #[test]
    fn test_formatter_with_mock_runtime() {
        use super::super::subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"my $x = 1;\n".to_vec()));

        let config = PerlTidyConfig::default();
        let mut formatter = PerlTidyFormatter::new(config, runtime.clone());

        let result = formatter.format("my $x=1;");
        assert!(result.is_ok());
        assert_eq!(must(result), "my $x = 1;\n");

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perltidy");
        assert!(invocations[0].args.contains(&"-st".to_string()));
    }

    #[test]
    fn test_formatter_caching() {
        use super::super::subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"formatted\n".to_vec()));

        let config = PerlTidyConfig::default();
        let mut formatter = PerlTidyFormatter::new(config, runtime.clone());

        // First call should invoke runtime
        let result1 = formatter.format("original");
        assert!(result1.is_ok());

        // Second call should use cache, not invoke runtime again
        let result2 = formatter.format("original");
        assert!(result2.is_ok());
        assert_eq!(must(result1), must(result2));

        // Only one invocation should have occurred
        assert_eq!(runtime.invocations().len(), 1);
    }

    #[test]
    fn test_formatter_error_handling() {
        use super::super::subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::failure(b"syntax error".to_vec(), 1));

        let config = PerlTidyConfig::default();
        let mut formatter = PerlTidyFormatter::new(config, runtime);

        let result = formatter.format("invalid code");
        match result {
            Err(e) => assert!(format!("{:?}", e).contains("syntax error")),
            Ok(_) => {
                must(Err::<(), _>("Expected error, got Ok"));
            }
        }
    }

    #[test]
    fn test_format_file_with_mock_runtime() {
        use super::super::subprocess_runtime::mock::{MockResponse, MockSubprocessRuntime};

        let runtime = Arc::new(MockSubprocessRuntime::new());
        runtime.add_response(MockResponse::success(b"".to_vec()));

        let config = PerlTidyConfig::default();
        let formatter = PerlTidyFormatter::new(config, runtime.clone());

        let result = formatter.format_file(Path::new("test.pl"));
        assert!(result.is_ok());

        let invocations = runtime.invocations();
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].program, "perltidy");

        // Ensure argument separator is used for security
        assert!(invocations[0].args.contains(&"--".to_string()));
        // Ensure the separator comes before the file path
        let sep_pos = must_some(invocations[0].args.iter().position(|a| a == "--"));
        let file_pos = must_some(invocations[0].args.iter().position(|a| a == "test.pl"));
        assert!(sep_pos < file_pos, "-- separator must come before file path");
    }
}

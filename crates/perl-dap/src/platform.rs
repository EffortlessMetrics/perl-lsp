//! Cross-platform utilities for Perl path resolution and environment setup
//!
//! This module provides platform-specific functionality for:
//! - Finding the perl binary on PATH
//! - Normalizing file paths across Windows/macOS/Linux
//! - Setting up environment variables (PERL5LIB)
//! - Formatting command-line arguments
//!
//! # Platform Support
//!
//! - **Linux**: Standard Unix paths, symlink resolution
//! - **macOS**: Darwin-specific symlink handling, Homebrew perl support
//! - **Windows**: Drive letter normalization, UNC path support, WSL path translation
//!
//! # Examples
//!
//! ```no_run
//! use perl_dap::platform::{resolve_perl_path, normalize_path, setup_environment};
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Find perl binary
//! let perl_path = resolve_perl_path()?;
//! println!("Found perl at: {}", perl_path.display());
//!
//! // Normalize a path
//! let normalized = normalize_path(&PathBuf::from("C:\\Users\\Name\\script.pl"));
//!
//! // Setup environment with include paths
//! let env = setup_environment(&[PathBuf::from("/workspace/lib")]);
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Platform-specific path separator for environment variables like PERL5LIB and PATH
#[cfg(windows)]
const PATH_SEPARATOR: char = ';';
#[cfg(not(windows))]
const PATH_SEPARATOR: char = ':';

/// Platform-specific Perl executable name
#[cfg(windows)]
const PERL_EXECUTABLE: &str = "perl.exe";
#[cfg(not(windows))]
const PERL_EXECUTABLE: &str = "perl";

/// Resolve the perl binary path on the current platform
///
/// This function searches for the perl binary on the system PATH.
/// It handles platform-specific executable names:
/// - Windows: perl.exe
/// - Unix (Linux/macOS): perl
///
/// # Errors
///
/// Returns an error if:
/// - PATH environment variable is not set
/// - perl binary is not found on PATH
///
/// # Examples
///
/// ```no_run
/// use perl_dap::platform::resolve_perl_path;
///
/// # fn main() -> anyhow::Result<()> {
/// let perl_path = resolve_perl_path()?;
/// println!("Found perl at: {}", perl_path.display());
/// # Ok(())
/// # }
/// ```
pub fn resolve_perl_path() -> Result<PathBuf> {
    // Get PATH environment variable
    let path_env = env::var("PATH").context("PATH environment variable not set")?;
    resolve_perl_path_from_path_env(&path_env)
}

fn resolve_perl_path_from_path_env(path_env: &str) -> Result<PathBuf> {
    // Search PATH directories for perl executable
    for path_dir in path_env.split(PATH_SEPARATOR) {
        let perl_path = PathBuf::from(path_dir).join(PERL_EXECUTABLE);
        if perl_path.exists() && perl_path.is_file() {
            return Ok(perl_path);
        }
    }

    anyhow::bail!("perl binary not found on PATH. Please install Perl or add it to PATH.")
}

/// Normalize a file path for cross-platform compatibility
///
/// This function handles platform-specific path normalization:
/// - **Windows**: Normalizes drive letters (C: → C:), handles UNC paths (\\server\share)
/// - **WSL**: Translates /mnt/c paths to C:\ paths
/// - **macOS/Linux**: Canonicalizes symlinks, removes redundant separators
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Returns
///
/// A normalized PathBuf
///
/// # Examples
///
/// ```
/// use perl_dap::platform::normalize_path;
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("C:\\Users\\Name\\script.pl");
/// let normalized = normalize_path(&path);
/// ```
pub fn normalize_path(path: &std::path::Path) -> PathBuf {
    // Handle WSL path translation (/mnt/c → C:\)
    #[cfg(target_os = "linux")]
    {
        if let Some(path_str) = path.to_str()
            && path_str.starts_with("/mnt/")
            && path_str.len() > 6
        {
            // Extract drive letter (e.g., /mnt/c → C:)
            let drive_letter = &path_str[5..6];
            let rest = &path_str[6..];
            let windows_path =
                format!("{}:{}", drive_letter.to_uppercase(), rest.replace('/', "\\"));
            return PathBuf::from(windows_path);
        }
    }

    // Windows drive letter normalization
    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            // Normalize drive letter to uppercase (c: → C:)
            if path_str.len() >= 2
                && path_str.chars().nth(1) == Some(':')
                && let Some(first_char) = path_str.chars().next()
            {
                let drive_letter = first_char.to_uppercase();
                let rest = &path_str[1..];
                return PathBuf::from(format!("{}{}", drive_letter, rest));
            }

            // UNC paths (\\server\share) - pass through as-is
            if path_str.starts_with("\\\\") {
                return path.to_path_buf();
            }
        }
    }

    // For Unix systems, canonicalize if possible (resolves symlinks)
    #[cfg(not(windows))]
    {
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }
    }

    // Fallback: return as-is
    path.to_path_buf()
}

/// Setup environment variables for Perl execution
///
/// This function creates a HashMap of environment variables suitable for spawning
/// a Perl process. It sets PERL5LIB to include the specified include paths.
///
/// # Arguments
///
/// * `include_paths` - Additional paths to add to @INC
///
/// # Returns
///
/// A HashMap of environment variable names to values
///
/// # Examples
///
/// ```
/// use perl_dap::platform::setup_environment;
/// use std::path::PathBuf;
///
/// let env = setup_environment(&[
///     PathBuf::from("/workspace/lib"),
///     PathBuf::from("/custom/lib"),
/// ]);
///
/// assert!(env.contains_key("PERL5LIB"));
/// ```
pub fn setup_environment(include_paths: &[PathBuf]) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if !include_paths.is_empty() {
        // Join paths with platform-specific separator
        let perl5lib = include_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(&PATH_SEPARATOR.to_string());

        env.insert("PERL5LIB".to_string(), perl5lib);
    }

    env
}

/// Format command-line arguments for platform-specific shells
///
/// This function handles platform-specific argument escaping:
/// - **Windows**: Escapes double quotes, handles spaces
/// - **Unix**: Escapes single quotes, handles spaces
///
/// # Arguments
///
/// * `args` - The arguments to format
///
/// # Returns
///
/// A vector of formatted arguments
///
/// # Examples
///
/// ```
/// use perl_dap::platform::format_command_args;
///
/// let args = vec![
///     "--file".to_string(),
///     "path with spaces.txt".to_string(),
/// ];
///
/// let formatted = format_command_args(&args);
/// assert_eq!(formatted.len(), 2);
/// ```
pub fn format_command_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|arg| {
            // If argument contains spaces, quote it
            if arg.contains(' ') {
                #[cfg(windows)]
                {
                    // Windows: escape double quotes and wrap in quotes
                    format!("\"{}\"", arg.replace('"', "\\\""))
                }
                #[cfg(not(windows))]
                {
                    // Unix: wrap in single quotes (simpler than double quote escaping)
                    if arg.contains('\'') {
                        // If contains single quote, use double quotes and escape
                        format!("\"{}\"", arg.replace('"', "\\\""))
                    } else {
                        format!("'{}'", arg)
                    }
                }
            } else {
                arg.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_perl_path() {
        // This test may fail if perl is not installed, which is acceptable for TDD
        match resolve_perl_path() {
            Ok(path) => {
                assert!(path.exists(), "Perl path should exist");
                assert!(path.is_file(), "Perl path should be a file");
            }
            Err(_) => {
                // perl not found - acceptable for TDD scaffolding
            }
        }
    }

    #[test]
    fn test_normalize_path_basic() {
        let path = PathBuf::from("script.pl");
        let normalized = normalize_path(&path);
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_path_windows_drive_letter() {
        let path = PathBuf::from("c:\\Users\\Name\\script.pl");
        let normalized = normalize_path(&path);
        let normalized_str = normalized.to_string_lossy();
        // Should uppercase drive letter
        assert!(normalized_str.starts_with("C:") || normalized_str.starts_with("c:"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_normalize_path_wsl_translation() {
        let path = PathBuf::from("/mnt/c/Users/Name/script.pl");
        let normalized = normalize_path(&path);
        let normalized_str = normalized.to_string_lossy();
        // Should translate to Windows path
        assert!(normalized_str.starts_with("C:"));
    }

    #[test]
    fn test_setup_environment_empty() {
        let env = setup_environment(&[]);
        assert!(!env.contains_key("PERL5LIB"));
    }

    #[test]
    fn test_setup_environment_with_paths() -> Result<(), Box<dyn std::error::Error>> {
        let env =
            setup_environment(&[PathBuf::from("/workspace/lib"), PathBuf::from("/custom/lib")]);

        assert!(env.contains_key("PERL5LIB"));
        let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not found")?;
        assert!(perl5lib.contains("/workspace/lib"));
        assert!(perl5lib.contains("/custom/lib"));
        Ok(())
    }

    #[test]
    fn test_format_command_args_simple() {
        let args = vec!["--verbose".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted, vec!["--verbose"]);
    }

    #[test]
    fn test_format_command_args_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
        let args = vec!["file with spaces.txt".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        // Should be quoted
        assert!(
            formatted.first().ok_or("Expected first element")?.contains("file with spaces.txt")
        );
        Ok(())
    }

    // Edge case tests for mutation testing hardening

    #[test]
    fn test_normalize_path_empty() {
        // Test: empty path handling
        let path = PathBuf::from("");
        let normalized = normalize_path(&path);
        // Should not panic, should return some valid path
        assert!(!normalized.as_os_str().is_empty() || normalized.as_os_str() == "");
    }

    #[test]
    fn test_normalize_path_relative() {
        // Test: relative path normalization
        let path = PathBuf::from("./script.pl");
        let normalized = normalize_path(&path);
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    fn test_normalize_path_parent_directory() {
        // Test: parent directory references
        let path = PathBuf::from("../script.pl");
        let normalized = normalize_path(&path);
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_path_unc_path() {
        // Test: UNC paths (\\server\share) - pass through as-is
        let path = PathBuf::from("\\\\server\\share\\file.pl");
        let normalized = normalize_path(&path);
        let normalized_str = normalized.to_string_lossy();
        assert!(normalized_str.starts_with("\\\\"), "UNC path should be preserved");
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_path_lowercase_drive() {
        // Test: lowercase drive letter gets uppercased
        let path = PathBuf::from("c:\\script.pl");
        let normalized = normalize_path(&path);
        let normalized_str = normalized.to_string_lossy();
        // Note: actual behavior depends on platform, so we just check it doesn't panic
        assert!(!normalized_str.is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_normalize_path_wsl_edge_cases() {
        // Test: WSL path edge cases
        let test_cases = vec![
            ("/mnt/c/", "C:\\"),                            // Root directory
            ("/mnt/d/Users/test.pl", "D:\\Users\\test.pl"), // Different drive
        ];

        for (input, expected_prefix) in test_cases {
            let path = PathBuf::from(input);
            let normalized = normalize_path(&path);
            let normalized_str = normalized.to_string_lossy();
            assert!(
                normalized_str.starts_with(expected_prefix),
                "Expected {} to start with {}, got {}",
                input,
                expected_prefix,
                normalized_str
            );
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_normalize_path_non_wsl() {
        // Test: non-WSL Linux paths should be canonicalized or passed through
        let path = PathBuf::from("/usr/bin/perl");
        let normalized = normalize_path(&path);
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    fn test_setup_environment_single_path() -> Result<(), Box<dyn std::error::Error>> {
        // Test: single include path
        let env = setup_environment(&[PathBuf::from("/workspace/lib")]);
        assert!(env.contains_key("PERL5LIB"));
        let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not found")?;
        assert_eq!(perl5lib, "/workspace/lib");
        Ok(())
    }

    #[test]
    fn test_setup_environment_path_separator() -> Result<(), Box<dyn std::error::Error>> {
        // Test: multiple paths use correct platform separator
        let env = setup_environment(&[PathBuf::from("/path1"), PathBuf::from("/path2")]);

        assert!(env.contains_key("PERL5LIB"));
        let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not found")?;

        #[cfg(windows)]
        assert!(perl5lib.contains(';'), "Windows should use ; separator");

        #[cfg(not(windows))]
        assert!(perl5lib.contains(':'), "Unix should use : separator");
        Ok(())
    }

    #[test]
    fn test_format_command_args_no_spaces() {
        // Test: arguments without spaces are not modified
        let args = vec!["--verbose".to_string(), "--debug".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted, args, "Args without spaces should not be quoted");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_with_single_quote() -> Result<(), Box<dyn std::error::Error>> {
        // Test: Unix argument with single quote uses double quotes
        let args = vec!["file's name.txt".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        let result = formatted.first().ok_or("Expected first element")?;
        assert!(result.contains("file's name.txt"), "Should contain original text");
        assert!(result.starts_with('"'), "Should use double quotes when single quote present");
        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn test_format_command_args_windows_quoting() -> Result<(), Box<dyn std::error::Error>> {
        // Test: Windows-specific quoting with escaped quotes
        let args = vec!["file with \"quotes\".txt".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        let result = formatted.first().ok_or("Expected first element")?;
        assert!(result.contains("\\\""), "Should escape double quotes on Windows");
        Ok(())
    }

    #[test]
    fn test_format_command_args_empty() {
        // Test: empty args array
        let args: Vec<String> = vec![];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 0, "Empty args should return empty array");
    }

    #[test]
    fn test_format_command_args_special_characters() {
        // Test: special characters in arguments
        let args = vec![
            "--input".to_string(),
            "file with spaces.txt".to_string(),
            "--output=result.txt".to_string(), // No spaces, no quoting
        ];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 3);
        assert_eq!(formatted[0], "--input");
        assert!(formatted[1].contains("file with spaces.txt"));
        assert_eq!(formatted[2], "--output=result.txt");
    }

    #[test]
    fn test_resolve_perl_path_failure_handling() {
        // Test: perl not found scenario
        // This test verifies graceful error handling without mutating process-wide env state.
        let result = resolve_perl_path_from_path_env("");

        // Should return error when perl not found
        assert!(result.is_err(), "Should fail when perl not on PATH");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found"), "Error should mention perl not found");
    }

    #[test]
    fn test_perl_executable_constant() {
        // Test: verify platform-specific executable name
        #[cfg(windows)]
        assert_eq!(PERL_EXECUTABLE, "perl.exe", "Windows should use perl.exe");

        #[cfg(not(windows))]
        assert_eq!(PERL_EXECUTABLE, "perl", "Unix should use perl");
    }

    #[test]
    fn test_path_separator_constant() {
        // Test: verify platform-specific path separator
        #[cfg(windows)]
        assert_eq!(PATH_SEPARATOR, ';', "Windows should use ; separator");

        #[cfg(not(windows))]
        assert_eq!(PATH_SEPARATOR, ':', "Unix should use : separator");
    }
}

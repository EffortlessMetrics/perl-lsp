//! Comprehensive unit tests for perl-dap-platform.

use perl_dap::command_args::format_command_args;
use perl_dap::platform::{normalize_path, resolve_perl_path, setup_environment};
use perl_tdd_support::must_some;
use std::path::PathBuf;

// ── resolve_perl_path ──────────────────────────────────────────────

#[test]
fn resolve_perl_path_returns_existing_file_when_perl_is_installed() -> Result<(), anyhow::Error> {
    // This test is environment-dependent; skip gracefully if perl isn't on PATH.
    if let Ok(path) = resolve_perl_path() {
        assert!(path.exists(), "resolved path should exist");
        assert!(path.is_file(), "resolved path should be a file");
        let filename = must_some(path.file_name()).to_string_lossy().to_string();
        assert!(filename.starts_with("perl"), "filename should start with 'perl', got: {filename}");
    }
    Ok(())
}

// ── normalize_path ─────────────────────────────────────────────────

#[test]
fn normalize_path_preserves_non_empty_relative_path() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("script.pl");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty(), "normalized path should not be empty");
    Ok(())
}

#[test]
fn normalize_path_handles_absolute_path() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("/usr/bin/perl");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty(), "normalized absolute path should not be empty");
    Ok(())
}

#[test]
fn normalize_path_handles_dot_path() -> Result<(), anyhow::Error> {
    let input = PathBuf::from(".");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty(), "normalized dot path should not be empty");
    Ok(())
}

#[test]
fn normalize_path_handles_empty_component() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("a//b");
    let normalized = normalize_path(&input);
    assert!(
        !normalized.as_os_str().is_empty(),
        "normalized path with double slash should not be empty"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_converts_wsl_mnt_path_to_windows_style() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("/mnt/c/Users/test/script.pl");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("C:"), "WSL /mnt/c path should convert to C: drive, got: {s}");
    assert!(s.contains('\\'), "WSL converted path should use backslashes, got: {s}");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_converts_wsl_mnt_lowercase_drive() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("/mnt/d/Projects/lib");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(
        s.starts_with("D:"),
        "WSL /mnt/d path should convert to D: drive (uppercase), got: {s}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_does_not_convert_non_wsl_mnt_path() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("/home/user/project");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(!s.contains('\\'), "non-WSL path should not get backslashes, got: {s}");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_short_mnt_path_no_conversion() -> Result<(), anyhow::Error> {
    // "/mnt/" is only 5 chars, plus 1 for drive letter = 6; path_str.len() > 6 check
    // means "/mnt/c" (len 6) should NOT trigger conversion
    let input = PathBuf::from("/mnt/c");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    // The path is exactly 6 chars, so the > 6 check means it won't convert
    assert!(!s.contains(':'), "path of exactly 6 chars should not be converted, got: {s}");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_canonicalizes_existing_path() -> Result<(), anyhow::Error> {
    // Canonicalize should resolve ".." for existing paths
    let input = PathBuf::from("/tmp/./");
    let normalized = normalize_path(&input);
    assert!(normalized.is_absolute(), "canonicalized existing path should be absolute");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_returns_original_for_nonexistent_path() -> Result<(), anyhow::Error> {
    let input = PathBuf::from("/nonexistent/deeply/nested/path/script.pl");
    let normalized = normalize_path(&input);
    // canonicalize will fail, so we fall through to returning the path as-is
    assert_eq!(
        normalized,
        PathBuf::from("/nonexistent/deeply/nested/path/script.pl"),
        "non-existent path should be returned unchanged"
    );
    Ok(())
}

// ── setup_environment ──────────────────────────────────────────────

#[test]
fn setup_environment_empty_paths_returns_no_perl5lib() -> Result<(), anyhow::Error> {
    let env = setup_environment(&[]);
    assert!(!env.contains_key("PERL5LIB"), "empty include paths should not set PERL5LIB");
    assert!(env.is_empty(), "env map should be empty with no paths");
    Ok(())
}

#[test]
fn setup_environment_single_path() -> Result<(), anyhow::Error> {
    let paths = [PathBuf::from("/workspace/lib")];
    let env = setup_environment(&paths);
    let perl5lib = must_some(env.get("PERL5LIB"));
    assert_eq!(perl5lib, "/workspace/lib", "single path should be set directly");
    Ok(())
}

#[test]
fn setup_environment_multiple_paths_joined_by_separator() -> Result<(), anyhow::Error> {
    let paths = [
        PathBuf::from("/workspace/lib"),
        PathBuf::from("/custom/lib"),
        PathBuf::from("/vendor/lib"),
    ];
    let env = setup_environment(&paths);
    let perl5lib = must_some(env.get("PERL5LIB"));

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts.len(), 3, "should have three path components");
    assert_eq!(parts[0], "/workspace/lib");
    assert_eq!(parts[1], "/custom/lib");
    assert_eq!(parts[2], "/vendor/lib");
    Ok(())
}

#[test]
fn setup_environment_preserves_path_with_spaces() -> Result<(), anyhow::Error> {
    let paths = [PathBuf::from("/my workspace/lib")];
    let env = setup_environment(&paths);
    let perl5lib = must_some(env.get("PERL5LIB"));
    assert!(perl5lib.contains("my workspace"), "path with spaces should be preserved");
    Ok(())
}

#[test]
fn setup_environment_only_sets_perl5lib() -> Result<(), anyhow::Error> {
    let paths = [PathBuf::from("/lib")];
    let env = setup_environment(&paths);
    assert_eq!(env.len(), 1, "should only contain PERL5LIB");
    assert!(env.contains_key("PERL5LIB"));
    Ok(())
}

#[test]
fn setup_environment_duplicate_paths_preserved_as_is() -> Result<(), anyhow::Error> {
    // setup_environment does not deduplicate — duplicates pass through and
    // Perl resolves them at runtime. This documents the contract explicitly.
    let paths = [PathBuf::from("/lib/a"), PathBuf::from("/lib/a"), PathBuf::from("/lib/b")];
    let env = setup_environment(&paths);
    let perl5lib = must_some(env.get("PERL5LIB"));

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts.len(), 3, "duplicates are NOT removed; all three entries present");
    assert_eq!(parts[0], "/lib/a");
    assert_eq!(parts[1], "/lib/a");
    assert_eq!(parts[2], "/lib/b");
    Ok(())
}

#[test]
fn setup_environment_empty_string_path_is_included() -> Result<(), anyhow::Error> {
    // PathBuf::from("") is a valid (if degenerate) path. The function must not
    // panic; it inserts PERL5LIB with an empty component.
    let paths = [PathBuf::from("")];
    let env = setup_environment(&paths);
    // Non-empty slice → PERL5LIB must be set (even if the value is "")
    assert!(env.contains_key("PERL5LIB"), "single empty-string path still triggers PERL5LIB");
    Ok(())
}

// ── format_command_args ────────────────────────────────────────────

#[test]
fn format_command_args_no_spaces_unchanged() -> Result<(), anyhow::Error> {
    let args = vec!["simple".to_string(), "args".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted, vec!["simple", "args"]);
    Ok(())
}

#[test]
fn format_command_args_empty_input() -> Result<(), anyhow::Error> {
    let args: Vec<String> = vec![];
    let formatted = format_command_args(&args);
    assert!(formatted.is_empty(), "empty args should produce empty result");
    Ok(())
}

#[test]
fn format_command_args_with_spaces_gets_quoted() -> Result<(), anyhow::Error> {
    let args = vec!["file with spaces.pl".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    let formatted_arg = &formatted[0];
    assert!(formatted_arg.contains("file with spaces.pl"), "original content should be preserved");
    // On all platforms, args with spaces get some form of quoting
    assert!(
        formatted_arg.starts_with('\'') || formatted_arg.starts_with('"'),
        "arg with spaces should be quoted, got: {formatted_arg}"
    );
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_single_quotes_simple_space() -> Result<(), anyhow::Error> {
    let args = vec!["hello world".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "'hello world'", "unix: simple space arg should be single-quoted");
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_double_quotes_when_single_quote_present() -> Result<(), anyhow::Error> {
    let args = vec!["it's a file".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(
        formatted[0], "\"it's a file\"",
        "unix: arg with single quote and spaces should use double quotes"
    );
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_escapes_double_quotes_inside() -> Result<(), anyhow::Error> {
    let args = vec!["say \"hello\" world".to_string()];
    let formatted = format_command_args(&args);
    // No single quote, so uses single-quote wrapping
    assert_eq!(
        formatted[0], "'say \"hello\" world'",
        "unix: arg with double quotes but no single quotes should use single-quote wrapping"
    );
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_both_quote_types_with_space() -> Result<(), anyhow::Error> {
    let args = vec!["it's \"complex\" stuff".to_string()];
    let formatted = format_command_args(&args);
    // Contains single quote → falls to double-quote branch, escapes inner double quotes
    assert_eq!(
        formatted[0], "\"it's \\\"complex\\\" stuff\"",
        "unix: arg with both quote types should double-quote with escaping"
    );
    Ok(())
}

#[test]
fn format_command_args_no_space_with_special_chars_unchanged() -> Result<(), anyhow::Error> {
    let args = vec!["--flag=value".to_string(), "-v".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "--flag=value");
    assert_eq!(formatted[1], "-v");
    Ok(())
}

#[test]
fn format_command_args_mixed_args() -> Result<(), anyhow::Error> {
    let args = vec![
        "perl".to_string(),
        "-I".to_string(),
        "/my lib/path".to_string(),
        "script.pl".to_string(),
    ];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 4);
    assert_eq!(formatted[0], "perl", "no-space arg unchanged");
    assert_eq!(formatted[1], "-I", "no-space arg unchanged");
    assert!(formatted[2].contains("/my lib/path"), "space arg should be quoted");
    assert_eq!(formatted[3], "script.pl", "no-space arg unchanged");
    Ok(())
}

#[test]
fn format_command_args_single_empty_string() -> Result<(), anyhow::Error> {
    let args = vec![String::new()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    #[cfg(windows)]
    assert_eq!(formatted[0], "\"\"", "empty string should be quoted to survive shell splitting");
    #[cfg(not(windows))]
    assert_eq!(formatted[0], "''", "empty string should be quoted to survive shell splitting");
    Ok(())
}

#[test]
fn format_command_args_preserves_order() -> Result<(), anyhow::Error> {
    let args: Vec<String> = (0..5).map(|i| format!("arg{i}")).collect();
    let formatted = format_command_args(&args);
    for (i, arg) in formatted.iter().enumerate() {
        assert_eq!(arg, &format!("arg{i}"), "order should be preserved at index {i}");
    }
    Ok(())
}

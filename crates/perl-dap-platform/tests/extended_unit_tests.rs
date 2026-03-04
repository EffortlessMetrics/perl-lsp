//! Extended unit tests for perl-dap-platform.
//! Covers edge cases, error conditions, and additional scenarios.

use perl_dap_platform::{format_command_args, normalize_path, setup_environment};
use std::path::PathBuf;

// ════════════════════════════════════════════════════════════════════════════════
// normalize_path Extended Tests
// ════════════════════════════════════════════════════════════════════════════════

#[test]
fn normalize_path_double_slash_in_middle() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/usr//bin//perl");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_parent_directory_reference() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("../src/lib.rs");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_multiple_parent_refs() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("../.././../../src");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_dot_current_dir() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("./");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_dot_prefixed_file() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("./script.pl");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/usr/bin/");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_no_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/usr/bin");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_single_slash() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[test]
fn normalize_path_very_long_path() -> Result<(), Box<dyn std::error::Error>> {
    let long_path = (0..50).map(|i| format!("dir{}", i)).collect::<Vec<_>>().join("/");
    let input = PathBuf::from(format!("/{}", long_path));
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty());
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_mnt_uppercase_drive() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/mnt/E/MyProject");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("E:"), "should uppercase drive letter, got: {s}");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_mnt_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/mnt/c/Program Files/MyApp");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.contains("Program Files"), "should preserve spaces");
    assert!(s.starts_with("C:"), "should have drive letter");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_mnt_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("/mnt/c/Users/test-user_123/file.pl");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.contains("test-user_123"), "should preserve special chars in path");
    Ok(())
}

#[cfg(windows)]
#[test]
fn normalize_path_windows_drive_lowercase_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("c:\\Users\\test");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("C:"), "drive letter should be uppercase, got: {s}");
    Ok(())
}

#[cfg(windows)]
#[test]
fn normalize_path_windows_already_uppercase() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("D:\\MyProject\\src");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("D:"), "uppercase drive should remain, got: {s}");
    Ok(())
}

#[cfg(windows)]
#[test]
fn normalize_path_windows_unc_path() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("\\\\server\\share\\file");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("\\\\"), "UNC path should be preserved");
    Ok(())
}

#[cfg(windows)]
#[test]
fn normalize_path_windows_forward_slash_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let input = PathBuf::from("C:/Users/test/file.pl");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy().to_string();
    assert!(s.starts_with("C:"), "should handle forward slashes");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════════
// setup_environment Extended Tests
// ════════════════════════════════════════════════════════════════════════════════

#[test]
fn setup_environment_very_long_path_list() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<PathBuf> = (0..100).map(|i| PathBuf::from(format!("/path/lib{}", i))).collect();
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let count = perl5lib.split(sep).count();
    assert_eq!(count, 100, "should handle 100 paths");
    Ok(())
}

#[test]
fn setup_environment_paths_with_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/用户/lib"), PathBuf::from("/datos/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();
    assert!(perl5lib.contains("用户"), "should preserve unicode characters");
    Ok(())
}

#[test]
fn setup_environment_paths_with_dots() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/path/./lib"), PathBuf::from("/path/../other/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();
    assert!(!perl5lib.is_empty());
    Ok(())
}

#[test]
fn setup_environment_duplicate_paths() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/lib"), PathBuf::from("/lib"), PathBuf::from("/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts.len(), 3, "duplicates should be preserved");
    Ok(())
}

#[test]
fn setup_environment_path_order_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [
        PathBuf::from("/first"),
        PathBuf::from("/second"),
        PathBuf::from("/third"),
        PathBuf::from("/fourth"),
    ];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts[0], "/first");
    assert_eq!(parts[1], "/second");
    assert_eq!(parts[2], "/third");
    assert_eq!(parts[3], "/fourth");
    Ok(())
}

#[test]
fn setup_environment_path_with_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/path/")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();
    assert!(perl5lib.contains("/path/"), "trailing slash should be preserved");
    Ok(())
}

#[test]
fn setup_environment_absolute_and_relative_mix() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/absolute/lib"), PathBuf::from("relative/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();
    assert!(perl5lib.contains("/absolute/lib"));
    assert!(perl5lib.contains("relative/lib"));
    Ok(())
}

#[test]
fn setup_environment_single_dot_path() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from(".")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or("PERL5LIB not set")?.clone();
    assert_eq!(perl5lib, ".");
    Ok(())
}

#[test]
fn setup_environment_env_does_not_contain_other_vars() -> Result<(), Box<dyn std::error::Error>> {
    let paths = [PathBuf::from("/lib")];
    let env = setup_environment(&paths);
    assert!(!env.contains_key("PATH"));
    assert!(!env.contains_key("HOME"));
    assert!(!env.contains_key("USER"));
    assert_eq!(env.len(), 1, "should only contain PERL5LIB");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════════
// format_command_args Extended Tests
// ════════════════════════════════════════════════════════════════════════════════

#[test]
fn format_command_args_many_args_without_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = (0..50).map(|i| format!("arg{}", i)).collect();
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 50);
    for (i, arg) in formatted.iter().enumerate() {
        assert_eq!(arg, &format!("arg{}", i));
    }
    Ok(())
}

#[test]
fn format_command_args_many_args_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = (0..30).map(|i| format!("arg with spaces {}", i)).collect();
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 30);
    for arg in formatted.iter() {
        assert!(arg.starts_with('\'') || arg.starts_with('"'), "should be quoted");
    }
    Ok(())
}

#[test]
fn format_command_args_tab_character() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["file\twith\ttabs".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    // Tab is not a space, so shouldn't be quoted
    assert!(!formatted[0].starts_with('\'') && !formatted[0].starts_with('"'));
    Ok(())
}

#[test]
fn format_command_args_newline_in_arg() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["line1\nline2".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    // No space, so shouldn't be quoted
    assert_eq!(formatted[0], "line1\nline2");
    Ok(())
}

#[test]
fn format_command_args_trailing_space() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["trailing ".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    // Has space, should be quoted
    assert!(
        formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
        "arg with trailing space should be quoted"
    );
    Ok(())
}

#[test]
fn format_command_args_leading_space() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec![" leading".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    assert!(
        formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
        "arg with leading space should be quoted"
    );
    Ok(())
}

#[test]
fn format_command_args_only_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["   ".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    assert!(
        formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
        "arg with only spaces should be quoted"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn format_command_args_windows_backslash() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["C:\\Program Files\\MyApp".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    // Has space, should be quoted
    assert!(formatted[0].contains("Program Files"), "should preserve path content");
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_double_quotes_only() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["say hello world".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "'say hello world'");
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_escape_double_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["test \"quoted\" value".to_string()];
    let formatted = format_command_args(&args);
    // Has space and double quotes but no single quote
    assert!(formatted[0].starts_with('\''), "should use single quotes");
    assert!(formatted[0].contains("\"quoted\""), "should preserve inner double quotes");
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn format_command_args_unix_multiple_single_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["it's can't won't".to_string()];
    let formatted = format_command_args(&args);
    // Contains single quotes with spaces, should use double quotes
    assert!(
        formatted[0].starts_with('"'),
        "should use double quotes when contains single quotes with space"
    );
    Ok(())
}

#[test]
fn format_command_args_arg_with_equals() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["--option=value with spaces".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    assert!(formatted[0].contains("--option=value with spaces"));
    Ok(())
}

#[test]
fn format_command_args_percentage_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["100%".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "100%");
    Ok(())
}

#[test]
fn format_command_args_dollar_sign() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["$variable".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "$variable");
    Ok(())
}

#[test]
fn format_command_args_asterisk() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["*.pl".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "*.pl");
    Ok(())
}

#[test]
fn format_command_args_path_like_no_space() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["/usr/local/bin/perl".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "/usr/local/bin/perl");
    Ok(())
}

#[test]
fn format_command_args_unicode_no_space() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["文件.pl".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], "文件.pl");
    Ok(())
}

#[test]
fn format_command_args_unicode_with_space() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["我的 文件.pl".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    assert!(
        formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
        "should quote unicode with space"
    );
    Ok(())
}

#[test]
fn format_command_args_alternating_spaces_no_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec![
        "no space".to_string(),
        "nospace".to_string(),
        "also spaced".to_string(),
        "compact".to_string(),
    ];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 4);
    assert!(formatted[0].starts_with('\'') || formatted[0].starts_with('"'), "first arg has space");
    assert!(!formatted[1].starts_with('\'') && !formatted[1].starts_with('"'));
    assert!(formatted[2].starts_with('\'') || formatted[2].starts_with('"'), "third arg has space");
    assert!(!formatted[3].starts_with('\'') && !formatted[3].starts_with('"'));
    Ok(())
}

#[test]
fn format_command_args_very_long_arg_no_space() -> Result<(), Box<dyn std::error::Error>> {
    let long_arg = "a".repeat(1000);
    let args = vec![long_arg.clone()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted[0], long_arg);
    Ok(())
}

#[test]
fn format_command_args_very_long_arg_with_space() -> Result<(), Box<dyn std::error::Error>> {
    let long_arg = format!("{} {}", "a".repeat(500), "b".repeat(500));
    let args = vec![long_arg.clone()];
    let formatted = format_command_args(&args);
    assert!(formatted[0].contains(&long_arg), "should preserve long arg content");
    assert!(
        formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
        "should quote long arg with space"
    );
    Ok(())
}

#[test]
fn format_command_args_null_like_string() -> Result<(), Box<dyn std::error::Error>> {
    let args = vec!["\0test".to_string()];
    let formatted = format_command_args(&args);
    assert_eq!(formatted.len(), 1);
    // No space in this case
    assert_eq!(formatted[0], "\0test");
    Ok(())
}

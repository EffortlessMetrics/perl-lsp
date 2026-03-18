//! Shell-specific helpers for Perl DAP process launch.

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(windows)]
const PATH_SEPARATOR: char = ';';
#[cfg(not(windows))]
const PATH_SEPARATOR: char = ':';

/// Setup environment variables for Perl execution.
pub fn setup_environment(include_paths: &[PathBuf]) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if !include_paths.is_empty() {
        let perl5lib = include_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(&PATH_SEPARATOR.to_string());

        env.insert("PERL5LIB".to_string(), perl5lib);
    }

    env
}

/// Format command-line arguments for platform-specific shells.
pub fn format_command_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|arg| {
            if arg.contains(' ') {
                #[cfg(windows)]
                {
                    format!("\"{}\"", arg.replace('"', "\\\""))
                }
                #[cfg(not(windows))]
                {
                    if arg.contains('\'') {
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

    // ── setup_environment ──────────────────────────────────────

    #[test]
    fn test_setup_environment_empty() {
        let env = setup_environment(&[]);
        assert!(!env.contains_key("PERL5LIB"));
    }

    #[test]
    fn test_setup_environment_with_paths() {
        let env =
            setup_environment(&[PathBuf::from("/workspace/lib"), PathBuf::from("/custom/lib")]);
        assert!(env.contains_key("PERL5LIB"));
    }

    #[test]
    fn test_setup_environment_single_path_value_matches() {
        let env = setup_environment(&[PathBuf::from("/my/lib")]);
        assert_eq!(env.get("PERL5LIB").map(String::as_str), Some("/my/lib"));
    }

    #[test]
    fn test_setup_environment_multiple_paths_joined_with_separator() -> Result<(), String> {
        let env = setup_environment(&[PathBuf::from("/a"), PathBuf::from("/b")]);
        let perl5lib = env.get("PERL5LIB").ok_or_else(|| "PERL5LIB must be set".to_string())?;
        assert!(perl5lib.contains("/a"), "first path present in PERL5LIB");
        assert!(perl5lib.contains("/b"), "second path present in PERL5LIB");
        #[cfg(not(windows))]
        assert_eq!(perl5lib.as_str(), "/a:/b");
        #[cfg(windows)]
        assert_eq!(perl5lib.as_str(), "/a;/b");
        Ok(())
    }

    #[test]
    fn test_setup_environment_only_sets_perl5lib() {
        let env = setup_environment(&[PathBuf::from("/lib")]);
        assert_eq!(env.len(), 1);
        assert!(env.contains_key("PERL5LIB"));
    }

    // ── format_command_args ────────────────────────────────────

    #[test]
    fn test_format_command_args_with_spaces() {
        let args = vec!["file with spaces.txt".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert!(formatted[0].contains("file with spaces.txt"));
    }

    #[test]
    fn test_format_command_args_without_spaces_passthrough() {
        let args = vec!["simple".to_string(), "/path/to/file.pl".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted, args, "args without spaces pass through unchanged");
    }

    #[test]
    fn test_format_command_args_empty_returns_empty() {
        let formatted = format_command_args(&[]);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_command_args_space_arg_is_quoted() {
        let args = vec!["plain".to_string(), "has space".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0], "plain", "no-space arg unchanged");
        assert!(formatted[1].contains("has space"), "space arg contains original text");
        assert_ne!(formatted[1], "has space", "space arg must be wrapped in quotes");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_space_no_single_quote_uses_single_quotes() {
        let args = vec!["arg with space".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "'arg with space'");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_space_with_single_quote_uses_double_quotes() {
        let args = vec!["it's here".to_string()];
        let formatted = format_command_args(&args);
        assert!(formatted[0].starts_with('"'), "double-quoted when arg contains single quote");
        assert!(formatted[0].ends_with('"'), "double-quoted when arg contains single quote");
        assert!(formatted[0].contains("it's here"), "original content preserved");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_inner_double_quote_escaped_in_double_quoted_form() {
        let args = vec!["say \"hello\" it's".to_string()];
        let formatted = format_command_args(&args);
        assert!(formatted[0].contains(r#"\""#), "inner double quotes should be escaped as \\\" ");
    }

    // ── shell metacharacter escaping edge cases ────────────────

    #[test]
    fn test_format_command_args_newline_in_argument() {
        // Newlines are whitespace, so the arg contains a "space-like" char.
        // The function only checks for literal ' ', so newlines pass through unquoted.
        let args = vec!["line1\nline2".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        // No literal space, so it passes through unchanged.
        assert_eq!(formatted[0], "line1\nline2");
    }

    #[test]
    fn test_format_command_args_carriage_return_in_argument() {
        let args = vec!["before\rafter".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        // No literal space, so CR passes through unchanged.
        assert_eq!(formatted[0], "before\rafter");
    }

    #[test]
    fn test_format_command_args_crlf_in_argument() {
        let args = vec!["line1\r\nline2".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0], "line1\r\nline2");
    }

    #[test]
    fn test_format_command_args_tab_in_argument() {
        let args = vec!["col1\tcol2".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        // Tab is not a space, so it passes through unchanged.
        assert_eq!(formatted[0], "col1\tcol2");
    }

    #[test]
    fn test_format_command_args_nul_byte_in_argument() {
        let args = vec!["before\0after".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0], "before\0after");
    }

    #[test]
    fn test_format_command_args_newline_with_space_triggers_quoting() {
        // When an arg has both a newline AND a space, quoting is triggered.
        let args = vec!["line1\n line2".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert!(
            formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
            "arg with space and newline should be quoted"
        );
        assert!(formatted[0].contains("line1\n line2"), "original content preserved inside quotes");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_single_quote_without_space_passthrough() {
        // Single quote alone (no space) passes through unquoted.
        let args = vec!["it's".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "it's");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_double_quote_without_space_passthrough() {
        let args = vec![r#"say"hello""#.to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], r#"say"hello""#, "no space means no quoting");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_nested_single_inside_double_with_space() {
        // Space + single quote => double-quoting path on Unix.
        let args = vec!["it's a \"test\"".to_string()];
        let formatted = format_command_args(&args);
        assert!(formatted[0].starts_with('"'), "double-quoted because of single quote");
        assert!(formatted[0].ends_with('"'), "double-quoted because of single quote");
        // Inner double quotes must be escaped.
        assert!(formatted[0].contains(r#"\""#), "inner double quotes escaped");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_only_quotes_mixed_with_space() {
        let args = vec!["' \" '".to_string()];
        let formatted = format_command_args(&args);
        // Contains space AND single quote => double-quote path.
        assert!(formatted[0].starts_with('"'), "opens with double quote");
        assert!(formatted[0].ends_with('"'), "closes with double quote");
    }

    #[test]
    fn test_format_command_args_backslash_without_space() {
        let args = vec![r"C:\Users\test".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], r"C:\Users\test", "backslash without space passes through");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_backslash_with_space_single_quoted() {
        let args = vec![r"path\ with space".to_string()];
        let formatted = format_command_args(&args);
        // Has space, no single quote => single-quoted on Unix.
        assert_eq!(formatted[0], r"'path\ with space'");
    }

    #[test]
    fn test_format_command_args_trailing_backslash() {
        let args = vec![r"trailing\".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], r"trailing\", "trailing backslash preserved");
    }

    #[test]
    fn test_format_command_args_multiple_consecutive_backslashes() {
        let args = vec![r"a\\b\\\\c".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], r"a\\b\\\\c");
    }

    #[test]
    fn test_format_command_args_utf8_no_space() {
        let args = vec!["\u{00e9}l\u{00e8}ve".to_string()]; // eleve with accents
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "\u{00e9}l\u{00e8}ve", "UTF-8 without space passes through");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_utf8_with_space_single_quoted() {
        let args = vec!["\u{00e9}l\u{00e8}ve file".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "'\u{00e9}l\u{00e8}ve file'");
    }

    #[test]
    fn test_format_command_args_cjk_characters() {
        let args = vec!["\u{4f60}\u{597d}\u{4e16}\u{754c}".to_string()]; // nihao shijie
        let formatted = format_command_args(&args);
        assert_eq!(
            formatted[0], "\u{4f60}\u{597d}\u{4e16}\u{754c}",
            "CJK characters without space pass through"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn test_format_command_args_cjk_with_space() {
        let args = vec!["\u{4f60}\u{597d} \u{4e16}\u{754c}".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "'\u{4f60}\u{597d} \u{4e16}\u{754c}'");
    }

    #[test]
    fn test_format_command_args_emoji_characters() {
        let args = vec!["\u{1f600}\u{1f680}".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], "\u{1f600}\u{1f680}", "emoji without space passes through");
    }

    #[test]
    fn test_format_command_args_empty_string_argument() {
        let args = vec!["".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0], "", "empty string passes through unchanged");
    }

    #[test]
    fn test_format_command_args_multiple_empty_strings() {
        let args = vec!["".to_string(), "".to_string(), "".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 3);
        for arg in &formatted {
            assert_eq!(arg, "", "each empty string preserved");
        }
    }

    #[test]
    fn test_format_command_args_single_space_argument() {
        let args = vec![" ".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert_ne!(formatted[0], " ", "single space should be quoted");
        assert!(formatted[0].contains(' '), "quoted form still contains the space");
    }

    #[test]
    fn test_format_command_args_multiple_spaces_argument() {
        let args = vec!["   ".to_string()];
        let formatted = format_command_args(&args);
        assert_ne!(formatted[0], "   ", "multiple spaces should be quoted");
    }

    #[test]
    fn test_format_command_args_very_long_argument_no_space() {
        let long_arg = "a".repeat(10_000);
        let args = vec![long_arg.clone()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted[0], long_arg, "long arg without space passes through");
    }

    #[test]
    fn test_format_command_args_very_long_argument_with_space() {
        let long_arg = format!("{} {}", "a".repeat(5_000), "b".repeat(5_000));
        let args = vec![long_arg.clone()];
        let formatted = format_command_args(&args);
        assert_ne!(formatted[0], long_arg, "long arg with space should be quoted");
        assert!(formatted[0].contains(&long_arg), "original content preserved inside quotes");
    }

    #[test]
    fn test_format_command_args_control_characters_bell_escape() {
        // BEL (\x07) and ESC (\x1b) are control characters.
        let args = vec!["start\x07middle\x1bend".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(
            formatted[0], "start\x07middle\x1bend",
            "control chars without space pass through"
        );
    }

    #[test]
    fn test_format_command_args_mixed_control_and_space() {
        let args = vec!["ctrl\x07 here".to_string()];
        let formatted = format_command_args(&args);
        assert!(
            formatted[0].starts_with('\'') || formatted[0].starts_with('"'),
            "space triggers quoting even with control characters present"
        );
    }

    #[test]
    fn test_format_command_args_shell_metacharacters_no_space() {
        // Shell metacharacters without spaces pass through (function only quotes on space).
        let args = vec![
            "$HOME".to_string(),
            "`whoami`".to_string(),
            "$(id)".to_string(),
            "foo|bar".to_string(),
            "a;b".to_string(),
            "x&y".to_string(),
            "a>b".to_string(),
            "a<b".to_string(),
        ];
        let formatted = format_command_args(&args);
        for (i, arg) in args.iter().enumerate() {
            assert_eq!(&formatted[i], arg, "metachar arg without space passes through: {arg}");
        }
    }

    #[test]
    fn test_format_command_args_glob_characters_no_space() {
        let args = vec!["*.pl".to_string(), "file?.txt".to_string(), "[abc].pm".to_string()];
        let formatted = format_command_args(&args);
        for (i, arg) in args.iter().enumerate() {
            assert_eq!(&formatted[i], arg, "glob without space passes through: {arg}");
        }
    }

    #[test]
    fn test_format_command_args_preserves_order_and_count() {
        let args: Vec<String> = (0..20).map(|i| format!("arg_{i}")).collect();
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 20);
        for (i, arg) in formatted.iter().enumerate() {
            assert_eq!(arg, &format!("arg_{i}"));
        }
    }
}

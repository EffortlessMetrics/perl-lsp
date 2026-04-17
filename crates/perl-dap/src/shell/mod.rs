//! Shell-specific helpers for Perl DAP process launch.

pub use crate::command_args::format_command_args;
pub use crate::platform::setup_environment;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
}

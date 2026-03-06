//! Platform-aware shell argument formatting used by `perl-dap`.

/// Format command-line arguments for platform-specific shells.
#[must_use]
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
    use super::format_command_args;

    #[test]
    fn leaves_simple_args_unmodified() {
        let args = vec!["plain".to_string(), "--flag".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted, args);
    }

    #[test]
    fn quotes_args_with_spaces() {
        let args = vec!["file with spaces.txt".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        assert!(formatted[0].contains("file with spaces.txt"));
        assert_ne!(formatted[0], "file with spaces.txt");
    }
}

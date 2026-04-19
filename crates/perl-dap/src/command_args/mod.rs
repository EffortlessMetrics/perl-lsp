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

    #[test]
    fn empty_slice_returns_empty_vec() {
        let args: Vec<String> = vec![];
        let formatted = format_command_args(&args);
        assert!(formatted.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn double_quote_escapes_arg_with_space_and_single_quote() {
        // An arg containing both a space and a single-quote cannot be
        // single-quote-wrapped, so it must be double-quote-escaped instead.
        let args = vec!["it's a file".to_string()];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 1);
        // Must be wrapped in double-quotes (not single-quotes).
        assert!(
            formatted[0].starts_with('"'),
            "expected double-quote prefix, got: {}",
            formatted[0]
        );
        assert!(
            formatted[0].ends_with('"'),
            "expected double-quote suffix, got: {}",
            formatted[0]
        );
        // The original text must be preserved inside the wrapper.
        assert!(
            formatted[0].contains("it's a file"),
            "original text not found in: {}",
            formatted[0]
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn mixed_args_are_each_handled_independently() {
        let args = vec![
            "plain".to_string(),         // no space → unchanged
            "has spaces".to_string(),    // space, no single-quote → single-quote wrap
            "it's here now".to_string(), // space + single-quote → double-quote wrap
        ];
        let formatted = format_command_args(&args);
        assert_eq!(formatted.len(), 3);

        // Plain arg is unchanged.
        assert_eq!(formatted[0], "plain");

        // Space-only arg is single-quote-wrapped.
        assert_eq!(formatted[1], "'has spaces'");

        // Space+single-quote arg is double-quote-wrapped.
        assert!(
            formatted[2].starts_with('"'),
            "expected double-quote prefix, got: {}",
            formatted[2]
        );
        assert!(
            formatted[2].ends_with('"'),
            "expected double-quote suffix, got: {}",
            formatted[2]
        );
        assert!(
            formatted[2].contains("it's here now"),
            "original text not found in: {}",
            formatted[2]
        );
    }
}

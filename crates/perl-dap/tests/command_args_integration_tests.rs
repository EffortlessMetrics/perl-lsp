//! Integration tests for `perl-dap-command-args`.
//!
//! Verifies that `format_command_args` correctly quotes arguments containing
//! spaces while passing through arguments without spaces unchanged. Covers
//! empty input, trivial single-arg cases, platform-specific quoting (Unix
//! single/double quotes), UTF-8 and emoji handling, shell-sensitive characters,
//! very long arguments, and argument-order preservation.

use perl_dap::command_args::format_command_args;

// ── Empty and trivial inputs ────────────────────────────────────────

#[test]
fn empty_args_returns_empty_vec() {
    let result = format_command_args(&[]);
    assert!(result.is_empty());
}

#[test]
fn single_empty_string_arg_is_quoted_to_preserve_emptiness() {
    let args = vec![String::new()];
    let result = format_command_args(&args);
    #[cfg(windows)]
    assert_eq!(result, vec!["\"\""]);
    #[cfg(not(windows))]
    assert_eq!(result, vec!["''"]);
}

#[test]
fn single_simple_arg_unchanged() {
    let args = vec!["hello".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result, vec!["hello"]);
}

// ── Arguments without spaces pass through ───────────────────────────

#[test]
fn flag_style_args_pass_through() {
    let args =
        vec!["--verbose".to_string(), "-I/usr/lib".to_string(), "--output=result.txt".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result, args);
}

#[test]
fn path_without_spaces_passes_through() {
    let args = vec!["/usr/local/bin/perl".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result, vec!["/usr/local/bin/perl"]);
}

// ── Arguments with spaces get quoted ────────────────────────────────

#[test]
fn arg_with_leading_space_is_quoted() {
    let args = vec![" leading".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], " leading", "should be quoted");
    assert!(result[0].contains(" leading"), "original text preserved");
}

#[test]
fn arg_with_trailing_space_is_quoted() {
    let args = vec!["trailing ".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "trailing ", "should be quoted");
    assert!(result[0].contains("trailing "), "original text preserved");
}

#[test]
fn arg_with_multiple_spaces_is_quoted() {
    let args = vec!["a b c d".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "a b c d");
    assert!(result[0].contains("a b c d"));
}

#[test]
fn arg_that_is_only_spaces_is_quoted() {
    let args = vec!["   ".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "   ");
}

// ── Very long arguments ─────────────────────────────────────────────

#[test]
fn very_long_arg_without_spaces_passes_through() {
    let long_arg = "x".repeat(10_000);
    let args = vec![long_arg.clone()];
    let result = format_command_args(&args);
    assert_eq!(result[0], long_arg);
}

#[test]
fn very_long_arg_with_spaces_is_quoted() {
    let long_arg = format!("{} {}", "a".repeat(5_000), "b".repeat(5_000));
    let args = vec![long_arg.clone()];
    let result = format_command_args(&args);
    assert_ne!(result[0], long_arg, "space-containing arg must be quoted");
    assert!(result[0].contains(&long_arg), "original text preserved");
}

// ── UTF-8 handling ──────────────────────────────────────────────────

#[test]
fn utf8_arg_without_spaces_passes_through() {
    let args = vec!["\u{00e9}l\u{00e8}ve".to_string()]; // "eleve" with accents
    let result = format_command_args(&args);
    assert_eq!(result[0], "\u{00e9}l\u{00e8}ve");
}

#[test]
fn utf8_arg_with_spaces_is_quoted() {
    let args = vec!["\u{00e9}l\u{00e8}ve du monde".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "\u{00e9}l\u{00e8}ve du monde");
    assert!(result[0].contains("\u{00e9}l\u{00e8}ve du monde"));
}

#[test]
fn cjk_characters_without_spaces_pass_through() {
    let args = vec!["\u{6d4b}\u{8bd5}".to_string()]; // Chinese "test"
    let result = format_command_args(&args);
    assert_eq!(result[0], "\u{6d4b}\u{8bd5}");
}

#[test]
fn emoji_arg_without_spaces_passes_through() {
    let args = vec!["\u{1f600}\u{1f601}".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "\u{1f600}\u{1f601}");
}

#[test]
fn emoji_arg_with_spaces_is_quoted() {
    let args = vec!["\u{1f600} smile".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "\u{1f600} smile");
    assert!(result[0].contains("\u{1f600} smile"));
}

// ── Shell-sensitive characters ──────────────────────────────────────

#[test]
fn semicolon_without_space_passes_through() {
    // Semicolons without spaces do NOT trigger quoting (by design).
    let args = vec!["a;b".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "a;b");
}

#[test]
fn pipe_without_space_passes_through() {
    let args = vec!["a|b".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "a|b");
}

#[test]
fn backtick_without_space_passes_through() {
    let args = vec!["`cmd`".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "`cmd`");
}

#[test]
fn dollar_sign_without_space_passes_through() {
    let args = vec!["$HOME".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "$HOME");
}

// ── Platform-specific quoting on non-Windows ────────────────────────

#[cfg(not(windows))]
mod unix_quoting {
    use perl_dap::command_args::format_command_args;

    #[test]
    fn space_only_arg_uses_single_quotes() {
        let args = vec!["hello world".to_string()];
        let result = format_command_args(&args);
        assert_eq!(result[0], "'hello world'");
    }

    #[test]
    fn arg_with_space_and_single_quote_uses_double_quotes() {
        let args = vec!["it's a test".to_string()];
        let result = format_command_args(&args);
        assert!(result[0].starts_with('"'));
        assert!(result[0].ends_with('"'));
        assert!(result[0].contains("it's a test"));
    }

    #[test]
    fn arg_with_space_single_quote_and_double_quote_escapes_inner_double_quote() {
        let args = vec!["it's a \"test\"".to_string()];
        let result = format_command_args(&args);
        assert!(result[0].starts_with('"'));
        assert!(result[0].ends_with('"'));
        // The inner double quotes should be escaped.
        assert!(result[0].contains(r#"\""#), "inner double quotes should be escaped");
    }

    #[test]
    fn arg_with_space_and_double_quote_but_no_single_quote_uses_single_quotes() {
        // Space + double-quote but no single-quote => single-quote wrapping (no escaping needed).
        let args = vec!["say \"hello\"".to_string()];
        let result = format_command_args(&args);
        assert_eq!(result[0], "'say \"hello\"'");
    }

    #[test]
    fn newline_without_space_is_quoted() {
        // Newline is whitespace and must be quoted to avoid shell token splitting.
        let args = vec!["line1\nline2".to_string()];
        let result = format_command_args(&args);
        assert_eq!(result[0], "'line1\nline2'");
    }
}

// ── Many arguments ──────────────────────────────────────────────────

#[test]
fn many_args_each_handled_independently() {
    let args: Vec<String> =
        (0..100).map(|i| if i % 2 == 0 { format!("arg{i}") } else { format!("arg {i}") }).collect();

    let result = format_command_args(&args);
    assert_eq!(result.len(), 100);

    for (i, formatted) in result.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(formatted, &format!("arg{i}"), "even index should be unquoted");
        } else {
            assert_ne!(formatted, &format!("arg {i}"), "odd index should be quoted");
            assert!(formatted.contains(&format!("arg {i}")), "original text preserved");
        }
    }
}

#[test]
fn preserves_argument_order() {
    let args: Vec<String> = vec!["first".into(), "second arg".into(), "third".into()];
    let result = format_command_args(&args);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], "first");
    assert!(result[1].contains("second arg"));
    assert_eq!(result[2], "third");
}

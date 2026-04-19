//! Integration tests for `perl-dap-shell`.
//!
//! Covers both `setup_environment` (PERL5LIB construction from include paths)
//! and `format_command_args` (space-aware quoting). Tests include empty input,
//! single/multiple paths, Unicode paths, special characters, platform-specific
//! separators, and combined usage in a typical DAP launch scenario.

use perl_dap::shell::{format_command_args, setup_environment};
use std::path::PathBuf;

// ════════════════════════════════════════════════════════════════════
// setup_environment
// ════════════════════════════════════════════════════════════════════

// ── Empty input ─────────────────────────────────────────────────────

#[test]
fn empty_include_paths_produces_no_env_vars() {
    let env = setup_environment(&[]);
    assert!(env.is_empty());
    assert!(!env.contains_key("PERL5LIB"));
}

// ── Single path ─────────────────────────────────────────────────────

#[test]
fn single_path_sets_perl5lib_exactly() {
    let env = setup_environment(&[PathBuf::from("/home/user/lib")]);
    assert_eq!(env.len(), 1);
    assert_eq!(
        env.get("PERL5LIB").map(String::as_str),
        Some("/home/user/lib")
    );
}

// ── Multiple paths ──────────────────────────────────────────────────

#[test]
fn multiple_paths_joined_with_platform_separator() -> Result<(), String> {
    let env = setup_environment(&[
        PathBuf::from("/a"),
        PathBuf::from("/b"),
        PathBuf::from("/c"),
    ]);

    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;

    #[cfg(not(windows))]
    assert_eq!(perl5lib, "/a:/b:/c");

    #[cfg(windows)]
    assert_eq!(perl5lib, "/a;/b;/c");

    Ok(())
}

#[test]
fn path_order_is_preserved() -> Result<(), String> {
    let paths: Vec<PathBuf> = (0..10)
        .map(|i| PathBuf::from(format!("/path{i}")))
        .collect();
    let env = setup_environment(&paths);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;

    let parts: Vec<&str> = if cfg!(windows) {
        perl5lib.split(';').collect()
    } else {
        perl5lib.split(':').collect()
    };

    assert_eq!(parts.len(), 10);
    for (i, part) in parts.iter().enumerate() {
        assert_eq!(*part, format!("/path{i}"));
    }

    Ok(())
}

// ── Unicode paths ───────────────────────────────────────────────────

#[test]
fn unicode_path_is_preserved() -> Result<(), String> {
    let env = setup_environment(&[PathBuf::from("/home/\u{00fc}ser/lib")]);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;
    assert!(
        perl5lib.contains("\u{00fc}ser"),
        "Unicode character preserved"
    );
    Ok(())
}

#[test]
fn cjk_path_is_preserved() -> Result<(), String> {
    let env = setup_environment(&[PathBuf::from("/\u{6d4b}\u{8bd5}/lib")]);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;
    assert!(
        perl5lib.contains("\u{6d4b}\u{8bd5}"),
        "CJK characters preserved"
    );
    Ok(())
}

#[test]
fn emoji_in_path_is_preserved() -> Result<(), String> {
    let env = setup_environment(&[PathBuf::from("/\u{1f4c1}/lib")]);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;
    assert!(perl5lib.contains('\u{1f4c1}'), "Emoji in path preserved");
    Ok(())
}

// ── Paths with special characters ───────────────────────────────────

#[test]
fn path_with_spaces_is_included_as_is() -> Result<(), String> {
    let env = setup_environment(&[PathBuf::from("/path with spaces/lib")]);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;
    assert_eq!(perl5lib, "/path with spaces/lib");
    Ok(())
}

#[test]
fn path_with_equals_sign_is_included() -> Result<(), String> {
    let env = setup_environment(&[PathBuf::from("/opt/perl=5.36/lib")]);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;
    assert!(perl5lib.contains("perl=5.36"));
    Ok(())
}

// ── Only PERL5LIB is set ───────────────────────────────────────────

#[test]
fn only_perl5lib_key_present() {
    let env = setup_environment(&[PathBuf::from("/lib1"), PathBuf::from("/lib2")]);
    assert_eq!(env.len(), 1);
    let keys: Vec<&String> = env.keys().collect();
    assert_eq!(keys, vec!["PERL5LIB"]);
}

// ── Very many paths ─────────────────────────────────────────────────

#[test]
fn many_paths_all_included() -> Result<(), String> {
    let paths: Vec<PathBuf> = (0..500)
        .map(|i| PathBuf::from(format!("/path/{i}")))
        .collect();
    let env = setup_environment(&paths);
    let perl5lib = env
        .get("PERL5LIB")
        .ok_or_else(|| "PERL5LIB should be set".to_string())?;

    let sep = if cfg!(windows) { ';' } else { ':' };
    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts.len(), 500);

    Ok(())
}

// ════════════════════════════════════════════════════════════════════
// format_command_args
// ════════════════════════════════════════════════════════════════════

#[test]
fn format_empty_args() {
    let result = format_command_args(&[]);
    assert!(result.is_empty());
}

#[test]
fn format_simple_arg_unchanged() {
    let args = vec!["perl".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result, vec!["perl"]);
}

#[test]
fn format_spaced_arg_is_quoted() {
    let args = vec!["my script.pl".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "my script.pl");
    assert!(result[0].contains("my script.pl"));
}

#[cfg(not(windows))]
#[test]
fn format_space_only_uses_single_quotes() {
    let args = vec!["hello world".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "'hello world'");
}

#[cfg(not(windows))]
#[test]
fn format_space_and_single_quote_uses_double_quotes() {
    let args = vec!["it's here".to_string()];
    let result = format_command_args(&args);
    assert!(result[0].starts_with('"'));
    assert!(result[0].ends_with('"'));
    assert!(result[0].contains("it's here"));
}

// ── UTF-8 in args ───────────────────────────────────────────────────

#[test]
fn utf8_arg_without_spaces_passes_through() {
    let args = vec!["\u{00e9}l\u{00e8}ve".to_string()];
    let result = format_command_args(&args);
    assert_eq!(result[0], "\u{00e9}l\u{00e8}ve");
}

#[test]
fn utf8_arg_with_spaces_is_quoted() {
    let args = vec!["caf\u{00e9} au lait".to_string()];
    let result = format_command_args(&args);
    assert_ne!(result[0], "caf\u{00e9} au lait");
    assert!(result[0].contains("caf\u{00e9} au lait"));
}

// ── Combined: environment + args ────────────────────────────────────

#[test]
fn environment_and_args_work_together() {
    // This test verifies both functions can be used together in a
    // typical DAP launch scenario.
    let env = setup_environment(&[PathBuf::from("/workspace/lib")]);
    let args = format_command_args(&[
        "perl".to_string(),
        "-d".to_string(),
        "my script.pl".to_string(),
    ]);

    assert!(env.contains_key("PERL5LIB"));
    assert_eq!(args.len(), 3);
    assert_eq!(args[0], "perl");
    assert_eq!(args[1], "-d");
    assert!(args[2].contains("my script.pl"));
}

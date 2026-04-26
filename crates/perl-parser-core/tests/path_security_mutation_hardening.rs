//! Mutation hardening tests for `path_security.rs`.
//!
//! Covers the following mutation targets:
//!
//! * `validate_workspace_path` — boolean guards on `starts_with`, null-byte
//!   / control-char predicate, `path != "/"` in absolute-path guard.
//! * `sanitize_completion_path_input` — `ParentDir` → `None`, root-dir
//!   guard `path != "/"`, backslash `..` string check.
//! * `is_hidden_or_forbidden_entry_name` — `len() > 1` off-by-one.
//! * `is_safe_completion_filename` — `len() > 255` off-by-one, control
//!   character predicate.
//! * `build_completion_path` — `dir_part == "."` branch, trailing slash.
//! * `split_completion_path_components` — `!dir.is_empty()` guard.

use perl_parser_core::path_security::{
    build_completion_path, is_hidden_or_forbidden_entry_name, is_safe_completion_filename,
    sanitize_completion_path_input, split_completion_path_components, validate_workspace_path,
};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// validate_workspace_path — null-byte / control-char guard
//
// Mutation: flip `is_control() && c != '\t'` to `|| c != '\t'` or remove the
// null-byte branch entirely.  Both would allow poisoned paths through.
// ---------------------------------------------------------------------------

#[test]
fn null_byte_in_path_returns_invalid_characters_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let result = validate_workspace_path(&PathBuf::from("file\0.pm"), ws);
    assert!(
        matches!(
            result,
            Err(perl_parser_core::path_security::WorkspacePathError::InvalidPathCharacters)
        ),
        "null byte must produce InvalidPathCharacters, got {result:?}"
    );
    Ok(())
}

#[test]
fn newline_in_path_returns_invalid_characters_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let result = validate_workspace_path(&PathBuf::from("lib\nfoo.pm"), ws);
    assert!(
        matches!(
            result,
            Err(perl_parser_core::path_security::WorkspacePathError::InvalidPathCharacters)
        ),
        "newline must produce InvalidPathCharacters, got {result:?}"
    );
    Ok(())
}

#[test]
fn tab_in_path_is_not_rejected_as_invalid_characters() -> TestResult {
    // The implementation explicitly allows \t. A mutation that changes
    // `c != '\t'` would incorrectly reject tabs.
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let result = validate_workspace_path(&PathBuf::from("lib/file\t.pm"), ws);
    assert!(
        !matches!(
            result,
            Err(perl_parser_core::path_security::WorkspacePathError::InvalidPathCharacters)
        ),
        "tab must NOT produce InvalidPathCharacters (tabs are explicitly allowed)"
    );
    Ok(())
}

#[test]
fn bell_char_in_path_returns_invalid_characters_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let result = validate_workspace_path(&PathBuf::from("lib/\x07file.pm"), ws);
    assert!(
        matches!(
            result,
            Err(perl_parser_core::path_security::WorkspacePathError::InvalidPathCharacters)
        ),
        "BEL char must produce InvalidPathCharacters"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_workspace_path — `starts_with` workspace containment guard
//
// Mutation: remove the `!` in `!canonical.starts_with(...)` or
// `!final_path.starts_with(...)`.
// ---------------------------------------------------------------------------

#[test]
fn path_outside_workspace_returns_outside_workspace_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let result = validate_workspace_path(&PathBuf::from("../../etc/passwd"), ws);
    assert!(result.is_err(), "escaping path must be rejected, got {result:?}");
    // Must NOT be InvalidPathCharacters — it's a traversal, not a char issue.
    assert!(
        !matches!(
            result,
            Err(perl_parser_core::path_security::WorkspacePathError::InvalidPathCharacters)
        ),
        "traversal should not be reported as InvalidPathCharacters"
    );
    Ok(())
}

#[test]
fn safe_relative_path_resolves_under_workspace() -> TestResult {
    let temp = tempfile::tempdir()?;
    let ws = temp.path();

    let resolved = validate_workspace_path(&PathBuf::from("lib/Foo.pm"), ws)?;

    // The resolved path must start with the workspace root.
    let canonical_ws = ws.canonicalize()?;
    assert!(
        resolved.starts_with(&canonical_ws),
        "resolved path '{resolved:?}' must be under workspace '{canonical_ws:?}'"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// sanitize_completion_path_input — ParentDir guard and "/" exception
//
// Mutation: remove the `Component::ParentDir => return None` arm, or flip
// `path != "/"` to `path == "/"`.
// ---------------------------------------------------------------------------

#[test]
fn sanitize_parent_traversal_returns_none() {
    assert!(sanitize_completion_path_input("../secret").is_none(), "'../secret' must return None");
    assert!(sanitize_completion_path_input("..").is_none(), "'..' must return None");
    assert!(
        sanitize_completion_path_input("foo/../bar").is_none(),
        "'foo/../bar' must return None"
    );
}

#[test]
fn sanitize_root_slash_is_the_only_allowed_absolute_path() {
    // "/" is the single allowed absolute path (the special-case guard).
    let result = sanitize_completion_path_input("/");
    assert_eq!(result, Some("/".to_string()), "'/' must be allowed");

    // Any other absolute path must be rejected.
    assert!(
        sanitize_completion_path_input("/etc/passwd").is_none(),
        "'/etc/passwd' must return None"
    );
    assert!(sanitize_completion_path_input("/usr").is_none(), "'/usr' must return None");
}

#[test]
fn sanitize_null_byte_returns_none() {
    assert!(sanitize_completion_path_input("lib/Foo\0.pm").is_none(), "null byte must return None");
}

#[test]
fn sanitize_valid_relative_path_returns_some() {
    let result = sanitize_completion_path_input("lib/Foo/Bar.pm");
    assert_eq!(result, Some("lib/Foo/Bar.pm".to_string()));
}

#[test]
fn sanitize_backslash_windows_traversal_returns_none() {
    // The string check `path.contains("../")` / `path.contains("..\\")`
    // must fire on Windows-style traversal.
    assert!(
        sanitize_completion_path_input(r"..\secret").is_none(),
        r"'..\secret' must return None"
    );
}

#[test]
fn sanitize_empty_path_returns_some_empty() {
    // Empty string is explicitly allowed.
    assert_eq!(sanitize_completion_path_input(""), Some(String::new()));
}

// ---------------------------------------------------------------------------
// is_hidden_or_forbidden_entry_name — `len() > 1` boundary
//
// Mutation: change `> 1` to `>= 1` (allows "."), or `> 0` (same).
// ---------------------------------------------------------------------------

#[test]
fn single_dot_is_not_hidden() {
    assert!(!is_hidden_or_forbidden_entry_name("."), "'.' must NOT be considered hidden");
}

#[test]
fn dotgit_is_hidden() {
    assert!(is_hidden_or_forbidden_entry_name(".git"), "'.git' must be hidden");
}

#[test]
fn dotfile_single_char_after_dot_is_hidden() {
    // ".x" — length is 2, so `len() > 1` passes.
    assert!(is_hidden_or_forbidden_entry_name(".x"), "'.x' (len 2) must be hidden");
}

#[test]
fn normal_dirname_is_not_hidden() {
    assert!(!is_hidden_or_forbidden_entry_name("lib"));
    assert!(!is_hidden_or_forbidden_entry_name("src"));
    assert!(!is_hidden_or_forbidden_entry_name("t"));
}

#[test]
fn forbidden_build_dirs_are_hidden() {
    for name in &["node_modules", "target", "build", ".cargo", ".rustup"] {
        assert!(is_hidden_or_forbidden_entry_name(name), "'{name}' must be in the forbidden list");
    }
}

// ---------------------------------------------------------------------------
// is_safe_completion_filename — len boundary and control char predicate
//
// Mutations: `len() > 255` → `>= 255` (rejects 255-char names incorrectly),
// `is_control()` predicate flip.
// ---------------------------------------------------------------------------

#[test]
fn filename_255_chars_is_safe() {
    let name = "a".repeat(255);
    assert!(is_safe_completion_filename(&name), "255-char name must be safe");
}

#[test]
fn filename_256_chars_is_not_safe() {
    let name = "b".repeat(256);
    assert!(!is_safe_completion_filename(&name), "256-char name must be rejected");
}

#[test]
fn filename_exactly_at_boundary_254_is_safe() {
    let name = "c".repeat(254);
    assert!(is_safe_completion_filename(&name), "254-char name must be safe");
}

#[test]
fn empty_filename_is_not_safe() {
    assert!(!is_safe_completion_filename(""), "empty name must be rejected");
}

#[test]
fn null_byte_in_filename_is_not_safe() {
    assert!(!is_safe_completion_filename("foo\0bar"), "null byte must be rejected");
}

#[test]
fn control_char_in_filename_is_not_safe() {
    assert!(!is_safe_completion_filename("foo\x07bar"), "BEL must be rejected");
    assert!(!is_safe_completion_filename("foo\nbar"), "LF must be rejected");
    assert!(!is_safe_completion_filename("foo\rbar"), "CR must be rejected");
}

#[test]
fn normal_filename_is_safe() {
    assert!(is_safe_completion_filename("Module.pm"));
    assert!(is_safe_completion_filename("Foo_Bar.pl"));
    assert!(is_safe_completion_filename("test-suite.t"));
}

#[test]
fn windows_reserved_names_are_not_safe() {
    for name in &["CON", "PRN", "AUX", "NUL"] {
        assert!(!is_safe_completion_filename(name), "{name} must be rejected");
        // Case-insensitive
        let lower = name.to_lowercase();
        assert!(!is_safe_completion_filename(&lower), "{lower} must be rejected");
    }
    for i in 1u8..=9 {
        let com = format!("COM{i}");
        let lpt = format!("LPT{i}");
        assert!(!is_safe_completion_filename(&com), "{com} must be rejected");
        assert!(!is_safe_completion_filename(&lpt), "{lpt} must be rejected");
    }
}

// ---------------------------------------------------------------------------
// build_completion_path — `dir_part == "."` branch and trailing slash
//
// Mutation: flip `dir_part == "."` to `dir_part != "."`.
// ---------------------------------------------------------------------------

#[test]
fn build_path_dot_dir_file_has_no_prefix() {
    let result = build_completion_path(".", "Foo.pm", false);
    assert_eq!(result, "Foo.pm", "dir='.' file must produce plain filename, got '{result}'");
}

#[test]
fn build_path_dot_dir_subdir_has_no_prefix_but_slash() {
    let result = build_completion_path(".", "lib", true);
    assert_eq!(result, "lib/", "dir='.' subdir must produce 'lib/', got '{result}'");
}

#[test]
fn build_path_non_dot_dir_prefixes_correctly() {
    let result = build_completion_path("lib", "Foo.pm", false);
    assert_eq!(result, "lib/Foo.pm", "lib/Foo.pm must be produced");
}

#[test]
fn build_path_directory_entry_gets_trailing_slash() {
    let result = build_completion_path("lib", "Sub", true);
    assert!(result.ends_with('/'), "directory entry must end with '/', got '{result}'");
    assert_eq!(result, "lib/Sub/");
}

#[test]
fn build_path_file_entry_has_no_trailing_slash() {
    let result = build_completion_path("lib", "Foo.pm", false);
    assert!(!result.ends_with('/'), "file entry must not end with '/'");
}

#[test]
fn build_path_strips_trailing_slash_on_dir_part() {
    let result = build_completion_path("lib/", "Foo.pm", false);
    assert_eq!(result, "lib/Foo.pm", "trailing slash on dir_part must be stripped");
}

// ---------------------------------------------------------------------------
// split_completion_path_components — `!dir.is_empty()` guard
//
// Mutation: flip to `dir.is_empty()`.
// ---------------------------------------------------------------------------

#[test]
fn split_nested_path_returns_correct_dir_and_file() {
    let (dir, file) = split_completion_path_components("lib/Foo/Bar");
    assert_eq!(dir, "lib/Foo", "dir component must be 'lib/Foo'");
    assert_eq!(file, "Bar", "file component must be 'Bar'");
}

#[test]
fn split_single_filename_returns_dot_dir() {
    let (dir, file) = split_completion_path_components("Module.pm");
    assert_eq!(dir, ".", "bare filename must produce dir='.'");
    assert_eq!(file, "Module.pm");
}

#[test]
fn split_single_slash_returns_dot_dir() {
    // "lib/" → rsplit_once('/') = Some(("lib", "")) — dir is "lib" (not empty)
    let (dir, file) = split_completion_path_components("lib/");
    assert_eq!(dir, "lib");
    assert_eq!(file, "");
}

#[test]
fn split_path_with_leading_slash_only_returns_dot_dir() {
    // "/" → rsplit_once('/') = Some(("", "")) — dir IS empty, so fallback fires:
    // returns (".", path) where path is the original input "/".
    let (dir, file) = split_completion_path_components("/");
    assert_eq!(dir, ".", "leading-slash-only must produce dir='.'");
    assert_eq!(file, "/");
}

//! Tests verifying that perl-lsp-launcher enforces the same print-statement lint rules
//! as sibling crates in the workspace (perl-lsp, perl-dap, etc.).
//!
//! This test module exists because GitHub issue #3224 identified inconsistent lint
//! enforcement across the workspace. The perl-lsp-launcher crate was missing the
//! `#![deny(clippy::print_stderr, clippy::print_stdout)]` directive that all other
//! library crates have.
//!
//! These tests verify:
//! 1. The lint denial directive is present in lib.rs
//! 2. The lint allow directive for test code is present in lib.rs
//! 3. The intentional `startup_banner` exception is explicitly annotated

use std::fs;

/// Returns the path to perl-lsp-launcher's lib.rs source file.
fn perl_lsp_launcher_lib_path() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("src/lib.rs")
}

/// Returns the source text of perl-lsp-launcher's lib.rs.
#[allow(clippy::expect_used)]
fn read_lib_source() -> String {
    fs::read_to_string(perl_lsp_launcher_lib_path())
        .expect("perl-lsp-launcher/src/lib.rs must exist and be readable")
}

/// Returns the line number (1-indexed) of the first line matching `pattern` in lib.rs,
/// or None if not found.
fn find_line_number(source: &str, pattern: &str) -> Option<usize> {
    source.lines().position(|line| line.contains(pattern)).map(|pos| pos + 1) // Convert to 1-indexed
}

#[test]
fn test_lib_has_deny_print_stderr_directive() {
    let source = read_lib_source();
    let pattern = "#![deny(clippy::print_stderr, clippy::print_stdout)]";
    let found = find_line_number(&source, pattern);

    let failure_msg = format!(
        "perl-lsp-launcher/src/lib.rs is missing lint enforcement directive:\n\
         Expected to find: {}\n\
         \n\
         All sibling library crates (perl-lsp, perl-dap, perl-lsp-transport,\n\
         perl-lsp-protocol, perl-semantic-analyzer, perl-corpus) have this directive.\n\
         perl-lsp-launcher is the only one missing it.\n\
         \n\
         This was identified in issue #3224 as the root cause of inconsistent enforcement.",
        pattern
    );

    assert!(found.is_some(), "{}", failure_msg);
}

#[test]
fn test_lib_has_cfg_attr_allow_in_test_mode() {
    let source = read_lib_source();
    let pattern = "#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]";
    let found = find_line_number(&source, pattern);

    let failure_msg = format!(
        "perl-lsp-launcher/src/lib.rs is missing test-mode suppression directive:\n\
         Expected to find: {}\n\
         \n\
         This directive allows tests to use print statements without triggering the deny lint.\n\
         Sibling crates have this directive immediately after the deny directive.",
        pattern
    );

    assert!(found.is_some(), "{}", failure_msg);
}

#[test]
fn test_deny_and_allow_directives_appear_after_unsafe_code_deny() {
    let source = read_lib_source();

    // The deny(unsafe_code) is at line 8 in the current file
    let unsafe_code_line = find_line_number(&source, "#![deny(unsafe_code)]");
    assert!(unsafe_code_line.is_some(), "lib.rs must contain #![deny(unsafe_code)]");
    let unsafe_code_line = unsafe_code_line.unwrap();

    // The print lint directives must appear after the unsafe_code deny
    let deny_print_line =
        find_line_number(&source, "#![deny(clippy::print_stderr, clippy::print_stdout)]");
    assert!(
        deny_print_line.is_some(),
        "lib.rs must contain #![deny(clippy::print_stderr, clippy::print_stdout)]"
    );
    let deny_print_line = deny_print_line.unwrap();

    let failure_msg = format!(
        "Lint enforcement directives must appear after #![deny(unsafe_code)] (line {}).\n\
         Found #![deny(unsafe_code)] at line {}.\n\
         Found #![deny(clippy::print_stderr, clippy::print_stdout)] at line {} (expected > {}).",
        unsafe_code_line, unsafe_code_line, deny_print_line, unsafe_code_line
    );

    assert!(deny_print_line > unsafe_code_line, "{}", failure_msg);
}

#[test]
fn test_startup_banner_has_allow_print_stderr_annotation() {
    let source = read_lib_source();

    // Find the startup_banner function declaration
    let fn_line = find_line_number(&source, "pub fn startup_banner");
    assert!(fn_line.is_some(), "Could not find 'pub fn startup_banner' in lib.rs");
    let fn_line = fn_line.unwrap();

    // The #[allow(clippy::print_stderr)] must appear on the line immediately before
    // the function declaration (or within a few lines before it)
    let lines: Vec<&str> = source.lines().collect();
    let allow_line = lines[..fn_line.saturating_sub(1)] // Lines before function
        .iter()
        .rev() // Reverse to search backwards from function
        .take(5) // Look back at most 5 lines
        .position(|line| line.contains("#[allow(clippy::print_stderr)]"));

    let failure_msg = format!(
        "startup_banner function at line {} is missing #[allow(clippy::print_stderr)] annotation.\n\
         \n\
         This function intentionally uses eprintln! to emit the startup banner before\n\
         the tracing subscriber is configured. The exception must be explicitly annotated\n\
         so the lint is machine-checkable rather than relying on undocumented behavior.\n\
         \n\
         Without this annotation, the deny directive from issue #3224 would trigger a\n\
         compile error on the intentional eprintln! call at runtime.",
        fn_line
    );

    assert!(allow_line.is_some(), "{}", failure_msg);
}

#[test]
fn test_startup_banner_documentation_explains_exception() {
    let source = read_lib_source();

    // The startup_banner should have a doc comment explaining why it uses eprintln!
    let fn_line = find_line_number(&source, "pub fn startup_banner");
    assert!(fn_line.is_some(), "Could not find 'pub fn startup_banner' in lib.rs");
    let fn_line = fn_line.unwrap();

    let lines: Vec<&str> = source.lines().collect();

    // Look backwards from the function for the most recent doc comment.
    // Unlike the annotation test, we use take() + filter() here instead of
    // take_while(), because the #[allow(...)] attribute line sits between the
    // doc comment block and the function declaration.
    let doc_comment_lines: Vec<&&str> = lines[..fn_line.saturating_sub(1)]
        .iter()
        .rev()
        .take(10) // Look back at most 10 lines
        .filter(|line| line.starts_with("///") || line.starts_with("//!"))
        .collect();

    let has_tracing_explanation = doc_comment_lines.iter().any(|line| {
        line.contains("tracing") && (line.contains("before") || line.contains("fires"))
    });

    let has_visibility_explanation =
        doc_comment_lines.iter().any(|line| line.contains("stderr") || line.contains("visible"));

    let failure_msg = "startup_banner function doc comment must explain why eprintln! is used\n\
         (i.e., it fires before tracing is configured and must be visible regardless\n\
         of whether --log is active).\n\
         \n\
         The doc comment should mention:\n\
         1. That it fires before the LSP handshake/tracing subscriber\n\
         2. That it writes to stderr directly (not through tracing)\n\
         3. That it remains visible regardless of --log flag\n\
         \n\
         These constraints are documented in issue #3224 and the ADR for work-1efc01c2."
        .to_string();

    assert!(has_tracing_explanation || has_visibility_explanation, "{failure_msg}");
}

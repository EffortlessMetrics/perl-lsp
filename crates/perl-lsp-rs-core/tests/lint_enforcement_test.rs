//! Tests verifying that perl-lsp-rs-core enforces print-statement lint rules.
//!
//! GitHub issue #3224 identified inconsistent print-statement lint enforcement
//! across the workspace. This test verifies that the `perl-lsp-rs-core` crate
//! (which absorbed `perl-lsp-launcher`) carries:
//!
//! - workspace-level `print_stderr = "deny"` / `print_stdout = "deny"` in Cargo.toml
//! - `#![cfg_attr(test, allow(...))]` to suppress in test code
//! - `#[expect(clippy::print_stderr)]` on `startup_banner` (the one intentional exception)
//!
//! These tests read the actual source files via `CARGO_MANIFEST_DIR` so they would
//! catch any future accidental removal of the directives.

use std::fs;

/// Returns the path to perl-lsp-rs-core's `lib.rs`.
fn lib_rs_path() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("src/lib.rs")
}

/// Returns the path to the runtime launcher module.
fn launcher_mod_path() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("src/runtime/launcher/mod.rs")
}

#[allow(clippy::expect_used)]
fn read_source(path: &std::path::Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("source file must be readable: {}", path.display()))
}

fn find_line_number(source: &str, pattern: &str) -> Option<usize> {
    source.lines().position(|line| line.contains(pattern)).map(|pos| pos + 1)
}

#[test]
fn test_workspace_cargo_has_print_deny() {
    // Navigate from manifest dir up to workspace root
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest
        .ancestors()
        .skip(1) // skip the crate dir
        .find(|p| p.join("Cargo.toml").exists() && p.join("Cargo.lock").exists())
        .expect("workspace root Cargo.toml must be readable");
    let cargo_toml = std::fs::read_to_string(workspace_root.join("Cargo.toml"))
        .expect("workspace Cargo.toml must be readable");
    assert!(
        cargo_toml.contains("print_stderr = \"deny\""),
        "workspace Cargo.toml must contain print_stderr = \"deny\" in [workspace.lints.clippy]"
    );
    assert!(
        cargo_toml.contains("print_stdout = \"deny\""),
        "workspace Cargo.toml must contain print_stdout = \"deny\" in [workspace.lints.clippy]"
    );
}

#[test]
fn test_lib_has_cfg_attr_allow_in_test_mode() {
    let source = read_source(&lib_rs_path());
    let pattern = "#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]";
    assert!(
        find_line_number(&source, pattern).is_some(),
        "perl-lsp-rs-core/src/lib.rs is missing test-mode suppression directive:\n  {pattern}\n\n\
         Without this, test helpers that use eprintln!/println! would fail to compile."
    );
}

#[test]
fn test_startup_banner_has_allow_annotation() {
    let source = read_source(&launcher_mod_path());
    // The expect annotation must appear before the function definition
    let allow_line = find_line_number(&source, "#[expect(clippy::print_stderr,");
    let fn_line = find_line_number(&source, "pub fn startup_banner(");

    assert!(
        allow_line.is_some(),
        "src/runtime/launcher/mod.rs: startup_banner is missing its \
         #[expect(clippy::print_stderr, reason = ...)] annotation.\n\
         The eprintln! in startup_banner fires before the tracing subscriber is configured \
         and is the one intentional exception in this crate."
    );

    if let (Some(allow), Some(func)) = (allow_line, fn_line) {
        assert!(
            allow < func,
            "The #[expect(clippy::print_stderr, ...)] annotation (line {allow}) must appear \
             before pub fn startup_banner (line {func})."
        );
    }
}

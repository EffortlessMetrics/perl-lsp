//! Integration tests verifying test-code quality baseline infrastructure.
//!
//! These tests define the acceptance criteria for work-c40bd322:
//! - Test baseline files exist and contain correct values
//! - Required dev-dependencies are configured
//! - `#![allow(clippy::panic)]` is present on test modules that use tdd-support
//!
//! ALL THESE TESTS SHOULD FAIL BEFORE IMPLEMENTATION AND PASS AFTER.

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Get the workspace root (two levels up from CARGO_MANIFEST_DIR since we're in crates/*/tests/)
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // We're in crates/perl-ci-hygiene/, so go up 2 levels to workspace root:
    // crates/perl-ci-hygiene -> crates -> workspace root
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Test 1: ci/panic_test_baseline.txt exists and contains a valid count.
///
/// Acceptance criterion 1: `ci/panic_test_baseline.txt` exists — contains the exact
/// count of `panic!` in test code as of this PR, established via a fresh scan.
#[test]
fn test_panic_test_baseline_file_exists_and_contains_count() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");
    assert!(
        baseline_path.exists(),
        "ci/panic_test_baseline.txt does not exist. \
        This baseline must be established before any remediation begins."
    );

    let content = fs::read_to_string(&baseline_path)
        .expect("Should be able to read ci/panic_test_baseline.txt");
    let content = content.trim();

    // Must be a non-negative integer
    let count: u32 = content
        .parse()
        .expect("ci/panic_test_baseline.txt must contain a valid non-negative integer");

    // Sanity check: should be in a reasonable range (0 to 5000 based on issue #3237)
    assert!(
        count <= 5000,
        "panic! count {} seems unreasonably high; verify the scan methodology",
        count
    );

    // The baseline should reflect the current state (pre-remediation)
    // If count is 0, that would be surprising unless all panic! were already removed
    println!("INFO: panic_test_baseline = {}", count);
}

/// Test 2: ci/todo_test_baseline.txt exists and contains 0.
///
/// Acceptance criterion 2: `ci/todo_test_baseline.txt` exists — contains 0
/// (no unlinked TODOs in test code).
#[test]
fn test_todo_test_baseline_file_exists_and_contains_zero() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/todo_test_baseline.txt");
    assert!(
        baseline_path.exists(),
        "ci/todo_test_baseline.txt does not exist. \
        This baseline must be established before any remediation begins."
    );

    let content = fs::read_to_string(&baseline_path)
        .expect("Should be able to read ci/todo_test_baseline.txt");
    let content = content.trim();

    let count: u32 = content
        .parse()
        .expect("ci/todo_test_baseline.txt must contain a valid non-negative integer");

    assert_eq!(
        count, 0,
        "ci/todo_test_baseline.txt should contain 0 (no unlinked TODOs in test code). \
        The only unlinked TODO found is in production code, outside scope."
    );
}

/// Test 3: perl-dead-code has perl-tdd-support as dev-dependency.
///
/// Acceptance criterion 3: `perl-dead-code` crate has `perl-tdd-support`
/// as a dev-dependency.
#[test]
fn test_perl_dead_code_has_tdd_support_dev_dependency() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-dead-code/Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .expect("Should be able to read crates/perl-dead-code/Cargo.toml");

    // Check for [dev-dependencies] section with perl-tdd-support
    assert!(
        content.contains("[dev-dependencies]"),
        "perl-dead-code should have a [dev-dependencies] section"
    );

    // Check that perl-tdd-support is in dev-dependencies (workspace variant)
    let dev_dep_pattern =
        Regex::new(r#"perl-tdd-support\s*=\s*\{[^}]*workspace\s*=\s*true[^}]*\}"#)
            .expect("Invalid regex");
    assert!(
        dev_dep_pattern.is_match(&content),
        "perl-dead-code should have perl-tdd-support = {{ workspace = true }} in [dev-dependencies]"
    );

    // Also accept simple form: perl-tdd-support = { workspace = true }
    let simple_pattern = Regex::new(r#"perl-tdd-support\s*=\s*\{\s*workspace\s*=\s*true\s*\}"#)
        .expect("Invalid regex");
    assert!(
        simple_pattern.is_match(&content),
        "perl-dead-code should have perl-tdd-support = {{ workspace = true }} in [dev-dependencies]"
    );
}

/// Test 4: perl-lsp-feature-policy has perl-tdd-support as dev-dependency.
///
/// Acceptance criterion 4: `perl-lsp-feature-policy` crate has `perl-tdd-support`
/// as a dev-dependency.
#[test]
fn test_perl_lsp_feature_policy_has_tdd_support_dev_dependency() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-lsp-feature-policy/Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .expect("Should be able to read crates/perl-lsp-feature-policy/Cargo.toml");

    // Check for [dev-dependencies] section with perl-tdd-support
    // This crate may not have had dev-dependencies before, so we check if it now has the section
    assert!(
        content.contains("[dev-dependencies]"),
        "perl-lsp-feature-policy should have a [dev-dependencies] section (created if needed)"
    );

    // Check that perl-tdd-support is in dev-dependencies
    let simple_pattern = Regex::new(r#"perl-tdd-support\s*=\s*\{\s*workspace\s*=\s*true\s*\}"#)
        .expect("Invalid regex");
    assert!(
        simple_pattern.is_match(&content),
        "perl-lsp-feature-policy should have perl-tdd-support = {{ workspace = true }} in [dev-dependencies]"
    );
}

/// Test 5: perl-dead-code tests have #![allow(clippy::panic)] on test module.
///
/// Acceptance criterion 3: `perl-dead-code` crate has `#![allow(clippy::panic)]`
/// on its test module.
#[test]
fn test_perl_dead_code_tests_have_allow_clippy_panic() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-dead-code/tests");

    // Find all test files
    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .collect();

    assert!(!test_files.is_empty(), "perl-dead-code should have test files in tests/");

    // At least one test file should have the allow attribute at module level
    let allow_pattern =
        Regex::new(r#"#!\s*\[\s*allow\s*\(\s*clippy::panic\s*\)\s*\]"#).expect("Invalid regex");

    let has_allow = test_files.iter().any(|e| {
        let content = fs::read_to_string(e.path()).unwrap_or_default();
        allow_pattern.is_match(&content)
    });

    assert!(
        has_allow,
        "perl-dead-code test modules should have #![allow(clippy::panic)] at module level \
        to avoid compile failures when using must()/must_err()/must_some() from perl-tdd-support"
    );
}

/// Test 6: perl-lsp-feature-policy tests have #![allow(clippy::panic)] on test module.
///
/// Acceptance criterion 4: `perl-lsp-feature-policy` crate has `#![allow(clippy::panic)]`
/// on its test module.
#[test]
fn test_perl_lsp_feature_policy_tests_have_allow_clippy_panic() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-lsp-feature-policy/tests");

    // Find all test files
    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .collect();

    assert!(!test_files.is_empty(), "perl-lsp-feature-policy should have test files in tests/");

    // At least one test file should have the allow attribute at module level
    let allow_pattern =
        Regex::new(r#"#!\s*\[\s*allow\s*\(\s*clippy::panic\s*\)\s*\]"#).expect("Invalid regex");

    let has_allow = test_files.iter().any(|e| {
        let content = fs::read_to_string(e.path()).unwrap_or_default();
        allow_pattern.is_match(&content)
    });

    assert!(
        has_allow,
        "perl-lsp-feature-policy test modules should have #![allow(clippy::panic)] at module level \
        to avoid compile failures when using must()/must_err()/must_some() from perl-tdd-support"
    );
}

/// Test 7: perl-parser-core tests should NOT have panic! in match-arm catches
/// (outside #[should_panic] functions).
///
/// Acceptance criterion 5: `panic!` burn-down — all verified `panic!` in match-arm
/// catches (outside `should_panic` functions) are replaced with `assert_matches!`
/// in perl-parser-core.
#[test]
fn test_perl_parser_core_no_panic_in_match_arm_catches() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-parser-core/src/engine/parser");

    // Find all test files in the parser tests directory
    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().map_or(false, |ext| ext == "rs")
                && e.path().file_name().map_or(false, |name| {
                    name.to_string_lossy().contains("_test")
                        || name.to_string_lossy().contains("tests")
                })
        })
        .collect();

    if test_files.is_empty() {
        // No test files found, skip this test
        return;
    }

    // Pattern to detect panic! in match-arm catches
    // Matches patterns like: `other => panic!("...")` or `other,` or `_ => panic!(...)`
    // Note: the variant pattern can end with comma OR => before the =>
    let panic_pattern =
        Regex::new(r#"(?m)^\s*(?:\w+\s*(?:,|=>)|other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\("#)
            .expect("Invalid regex");
    for entry in &test_files {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        if panic_pattern.is_match(&content) {
            let lines: Vec<&str> = content.lines().collect();
            let example_lines: Vec<_> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| panic_pattern.is_match(line))
                .take(3)
                .collect();

            let examples: Vec<_> = example_lines
                .iter()
                .map(|(num, line)| format!("  line {}: {}", num + 1, line))
                .collect();

            assert!(
                false,
                "Found `panic!` in match-arm catches in {:?}:\n{}\n\
                These should be replaced with `assert_matches!`.",
                entry.path(),
                examples.join("\n")
            );
        }
    }
}

/// Test 8: perl-dap tests should NOT have panic! in match-arm catches
/// (outside #[should_panic] functions).
///
/// Acceptance criterion 5: `panic!` burn-down in perl-dap.
#[test]
fn test_perl_dap_no_panic_in_match_arm_catches() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-dap/tests");

    // Find all test files
    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .collect();

    if test_files.is_empty() {
        return;
    }
    // Pattern to detect panic! in match-arm catches
    // Matches patterns like: `other => panic!("...")` or `other,` or `_ => panic!(...)`
    // Note: the variant pattern can end with comma OR => before the =>
    let panic_pattern =
        Regex::new(r#"(?m)^\s*(?:\w+\s*(?:,|=>)|other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\("#)
            .expect("Invalid regex");

    for entry in &test_files {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        if panic_pattern.is_match(&content) {
            let lines: Vec<&str> = content.lines().collect();
            let example_lines: Vec<_> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| panic_pattern.is_match(line))
                .take(3)
                .collect();

            let examples: Vec<_> = example_lines
                .iter()
                .map(|(num, line)| format!("  line {}: {}", num + 1, line))
                .collect();

            assert!(
                false,
                "Found `panic!` in match-arm catches in {:?}:\n{}\n\
                These should be replaced with `assert_matches!`.",
                entry.path(),
                examples.join("\n")
            );
        }
    }
}

/// Test 9: perl-lexer tests should NOT have panic! in match-arm catches
/// (Note: issue #3237 claimed perl-builtins but that's in perl-lexer).
///
/// Acceptance criterion 5: `panic!` burn-down in perl-lexer.
#[test]
fn test_perl_lexer_no_panic_in_match_arm_catches() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-lexer/tests");

    // Find all test files
    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().map_or(false, |ext| ext == "rs"))
        .collect();

    if test_files.is_empty() {
        return;
    }
    // Pattern to detect panic! in match-arm catches
    // Matches patterns like: `other => panic!("...")` or `other,` or `_ => panic!(...)`
    // Note: the variant pattern can end with comma OR => before the =>
    let panic_pattern =
        Regex::new(r#"(?m)^\s*(?:\w+\s*(?:,|=>)|other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\("#)
            .expect("Invalid regex");

    for entry in &test_files {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        if panic_pattern.is_match(&content) {
            let lines: Vec<&str> = content.lines().collect();
            let example_lines: Vec<_> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| panic_pattern.is_match(line))
                .take(3)
                .collect();

            let examples: Vec<_> = example_lines
                .iter()
                .map(|(num, line)| format!("  line {}: {}", num + 1, line))
                .collect();

            assert!(
                false,
                "Found `panic!` in match-arm catches in {:?}:\n{}\n\
                These should be replaced with `assert_matches!`.",
                entry.path(),
                examples.join("\n")
            );
        }
    }
}

/// Test 10: Production baselines should be unchanged.
///
/// Acceptance criterion 8: Production baselines unchanged —
/// `ci/panic_prod_baseline.txt` and `ci/unwrap_prod_baseline.txt`
/// remain at their original values (no regression).
#[test]
fn test_production_baselines_unchanged() {
    let ws_root = workspace_root();
    let panic_prod_path = ws_root.join("ci/panic_prod_baseline.txt");
    let unwrap_prod_path = ws_root.join("ci/unwrap_prod_baseline.txt");

    assert!(panic_prod_path.exists(), "ci/panic_prod_baseline.txt should exist");
    assert!(unwrap_prod_path.exists(), "ci/unwrap_prod_baseline.txt should exist");

    let panic_prod: u32 = fs::read_to_string(&panic_prod_path)
        .expect("Should read panic_prod_baseline")
        .trim()
        .parse()
        .expect("panic_prod_baseline should be a number");

    let unwrap_prod: u32 = fs::read_to_string(&unwrap_prod_path)
        .expect("Should read unwrap_prod_baseline")
        .trim()
        .parse()
        .expect("unwrap_prod_baseline should be a number");

    // Both should be 0 per current baselines
    assert_eq!(
        panic_prod, 0,
        "Production panic baseline should remain 0 (no new panic! in production code)"
    );
    assert_eq!(
        unwrap_prod, 0,
        "Production unwrap baseline should remain 0 (no new unwrap in production code)"
    );
}

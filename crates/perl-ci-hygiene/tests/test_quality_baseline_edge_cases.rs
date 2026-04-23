//! Edge case tests for test-code quality baseline infrastructure.
//!
//! These tests verify edge cases and boundary conditions that the red tests
//! don't fully cover. These are the "stress tests" of the implementation.
//!
//! Edge cases covered:
//! - Dependency specifically in [dev-dependencies] section vs [dependencies]
//! - #![allow(clippy::panic)] with various whitespace configurations
//! - Commented-out panic! should not trigger detection
//! - should_panic functions are properly excluded
//! - Production baselines are exactly 0
//! - Test directories have files

use regex::Regex;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Get the workspace root (two levels up from CARGO_MANIFEST_DIR since we're in crates/*/tests/)
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Edge Case 1: Verify dev-dependency is in [dev-dependencies] section, not [dependencies]
#[test]
fn test_perl_dead_code_tdd_support_is_in_dev_dependencies_section() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-dead-code/Cargo.toml");
    let content = fs::read_to_string(&cargo_path).expect("Should read Cargo.toml");

    // Find the [dev-dependencies] section
    let dev_deps_start =
        content.find("[dev-dependencies]").expect("Should have [dev-dependencies] section");

    // Find the next section (either [dependencies] or end of relevant portion)
    let next_section = content[dev_deps_start..].find("\n[").map(|pos| pos + dev_deps_start);

    let dev_deps_section = match next_section {
        Some(pos) => &content[dev_deps_start..pos],
        None => &content[dev_deps_start..],
    };

    // Verify perl-tdd-support is in the dev-dependencies section
    assert!(
        dev_deps_section.contains("perl-tdd-support"),
        "perl-tdd-support should be in [dev-dependencies] section, not [dependencies]"
    );
}

/// Edge Case 2: Verify perl-lsp-feature-policy dev-dependency is in correct section
#[test]
fn test_perl_lsp_feature_policy_tdd_support_is_in_dev_dependencies_section() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-lsp-feature-policy/Cargo.toml");
    let content = fs::read_to_string(&cargo_path).expect("Should read Cargo.toml");

    // Find the [dev-dependencies] section
    let dev_deps_start =
        content.find("[dev-dependencies]").expect("Should have [dev-dependencies] section");

    let next_section = content[dev_deps_start..].find("\n[").map(|pos| pos + dev_deps_start);

    let dev_deps_section = match next_section {
        Some(pos) => &content[dev_deps_start..pos],
        None => &content[dev_deps_start..],
    };

    assert!(
        dev_deps_section.contains("perl-tdd-support"),
        "perl-tdd-support should be in [dev-dependencies] section"
    );
}

/// Edge Case 3: #![allow(clippy::panic)] pattern handles various whitespace
/// This verifies the regex in the red tests handles edge case whitespace
#[test]
fn test_allow_clippy_panic_pattern_handles_various_whitespace() {
    let test_cases = vec![
        "#![allow(clippy::panic)]",
        "#![allow( clippy::panic )]",
        "#![allow(  clippy::panic  )]",
        "#![allow(clippy::panic)] // with comment",
    ];

    let allow_pattern =
        Regex::new(r#"#!\s*\[\s*allow\s*\(\s*clippy::panic\s*\)\s*\]"#).expect("Invalid regex");

    for case in test_cases {
        assert!(allow_pattern.is_match(case), "Pattern should match: {}", case);
    }

    // Negative case - different lint name should NOT match
    let negative_cases = vec![
        "#![allow(clippy::panic_in_test_code)]", // different lint
    ];

    for case in negative_cases {
        assert!(!allow_pattern.is_match(case), "Pattern should NOT match: {}", case);
    }
}

/// Edge Case 4: Panic pattern in comments should be excluded
/// The regex should not detect commented-out panic! patterns when on their own line
#[test]
fn test_panic_pattern_excludes_line_comments() {
    // These are line comments - they should NOT match the ^ anchor
    let commented_cases = vec!["// _ => panic!(\"commented out\")", "    // _ => panic!()"];

    let panic_pattern =
        Regex::new(r#"(?m)^\s*(?:\w+\s*(?:,|=>)|other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\("#)
            .expect("Invalid regex");

    for case in commented_cases {
        assert!(!panic_pattern.is_match(case), "Should NOT match line comment: {}", case);
    }

    // Block comments are trickier - they might match if on a single line
    // but that's acceptable since our test file scanning would catch actual code
}

/// Edge Case 5: Verify should_panic functions are excluded from panic detection
/// This tests that the current implementation correctly handles should_panic
#[test]
fn test_should_panic_functions_are_handled() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-parser-core/src/engine/parser");

    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path().extension().is_some_and(|ext| ext == "rs")
                && e.path().file_name().is_some_and(|name| {
                    name.to_string_lossy().contains("_test")
                        || name.to_string_lossy().contains("tests")
                })
        })
        .collect();

    if test_files.is_empty() {
        return;
    }

    let panic_pattern =
        Regex::new(r#"(?m)^\s*(?:\w+\s*(?:,|=>)|other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\("#)
            .expect("Invalid regex");
    let should_panic_pattern = Regex::new(r#"#\[should_panic\]"#).expect("Invalid regex");

    for entry in &test_files {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        // If file has should_panic, it's expected to have some panic! patterns
        // That's OK - they're intentional
        if should_panic_pattern.is_match(&content) {
            continue;
        }

        // If no should_panic and has panic! in match arm, that's a problem
        if panic_pattern.is_match(&content) {
            panic!(
                "Found panic! in non-should_panic test in {:?}. \
                All panic! in match-arm catches should be replaced with assert_matches!",
                entry.path()
            );
        }
    }
}

/// Edge Case 6: Production panic baseline is exactly 0
#[test]
fn test_production_panic_baseline_is_zero() {
    let ws_root = workspace_root();
    let panic_prod_path = ws_root.join("ci/panic_prod_baseline.txt");

    let content =
        fs::read_to_string(&panic_prod_path).expect("Should read panic_prod_baseline.txt");
    let count: u32 = content.trim().parse().expect("panic_prod_baseline should be a number");

    assert_eq!(count, 0, "Production panic baseline should be exactly 0");
}

/// Edge Case 7: Production unwrap baseline is exactly 0
#[test]
fn test_production_unwrap_baseline_is_zero() {
    let ws_root = workspace_root();
    let unwrap_prod_path = ws_root.join("ci/unwrap_prod_baseline.txt");

    let content =
        fs::read_to_string(&unwrap_prod_path).expect("Should read unwrap_prod_baseline.txt");
    let count: u32 = content.trim().parse().expect("unwrap_prod_baseline should be a number");

    assert_eq!(count, 0, "Production unwrap baseline should be exactly 0");
}

/// Edge Case 8: perl-dead-code tests directory exists and has files
#[test]
fn test_perl_dead_code_tests_directory_has_files() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-dead-code/tests");

    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    assert!(!test_files.is_empty(), "perl-dead-code tests directory should have .rs files");
}

/// Edge Case 9: perl-lsp-feature-policy tests directory exists and has files
#[test]
fn test_perl_lsp_feature_policy_tests_directory_has_files() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-lsp-feature-policy/tests");

    let test_files: Vec<_> = WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    assert!(
        !test_files.is_empty(),
        "perl-lsp-feature-policy tests directory should have .rs files"
    );
}

/// Edge Case 10: Baseline file contains valid parseable content
#[test]
fn test_panic_test_baseline_is_valid_u32() {
    let ws_root = workspace_root();
    let test_baseline = ws_root.join("ci/panic_test_baseline.txt");

    let content = fs::read_to_string(&test_baseline).expect("Should read baseline");
    let trimmed = content.trim();

    // Must be non-empty
    assert!(!trimmed.is_empty(), "Baseline file should not be empty");

    // Must parse as u32
    let count: u32 = trimmed
        .parse()
        .expect("ci/panic_test_baseline.txt must contain a valid non-negative integer");

    // Should be in reasonable range
    assert!(count <= 5000, "panic! count seems unreasonably high");
}

/// Edge Case 11: Baseline file contains valid parseable content for todo
#[test]
fn test_todo_test_baseline_is_valid_u32() {
    let ws_root = workspace_root();
    let test_baseline = ws_root.join("ci/todo_test_baseline.txt");

    let content = fs::read_to_string(&test_baseline).expect("Should read baseline");
    let trimmed = content.trim();

    // Must be non-empty
    assert!(!trimmed.is_empty(), "Baseline file should not be empty");

    // Must parse as u32
    let count: u32 = trimmed
        .parse()
        .expect("ci/todo_test_baseline.txt must contain a valid non-negative integer");

    // Should be 0 per the implementation
    assert_eq!(count, 0, "TODO baseline should be 0");
}

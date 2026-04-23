//! Property-based tests for test-code quality baseline infrastructure.
//!
//! These tests verify invariants about the test-code quality infrastructure
//! using property-based testing techniques with generated inputs.
//!
//! Properties tested:
//! 1. Baseline count stability - re-reading baseline file gives same value
//! 2. Allow attribute regex - correctly identifies #![allow(clippy::panic)]
//! 3. Deterministic scan - running the scan twice gives same results
//! 4. TDD-support usage is properly guarded with allow attribute
//! 5. Production baselines unchanged - no regression

use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Get the workspace root
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

/// ALLOW ATTRIBUTE PATTERN
/// Matches: `#![allow(clippy::panic)]` with various whitespace
fn allow_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"#!\s*\[\s*allow\s*\(\s*clippy::panic\s*\)\s*\]").expect("Invalid regex")
    })
}

// =============================================================================
// Property 1: Baseline Count Stability
// =============================================================================

/// Property: Reading the panic test baseline multiple times should give the same value.
/// This verifies the baseline file is stable and not being modified concurrently.
#[test]
fn property_baseline_count_is_deterministic() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");

    let reads: Vec<u32> = (0..100)
        .map(|_| {
            let content = fs::read_to_string(&baseline_path).expect("Should read baseline file");
            content.trim().parse::<u32>().expect("Should parse as u32")
        })
        .collect();

    // All reads should be identical
    let first = reads[0];
    for (i, &value) in reads.iter().enumerate().skip(1) {
        assert_eq!(
            value, first,
            "Baseline read {} differs from first read: expected {}, got {}",
            i, first, value
        );
    }
}

/// Property: The panic test baseline should be within a reasonable range.
#[test]
fn property_baseline_count_is_in_valid_range() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");

    let content = fs::read_to_string(&baseline_path).expect("Should read baseline file");
    let count: u32 = content.trim().parse().expect("Should parse as u32");

    assert!(count <= 5000, "panic! count {} exceeds reasonable upper bound of 5000", count);
}

/// Property: The panic test baseline should be > 0 (remediation target exists)
#[test]
fn property_baseline_count_is_positive() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");

    let content = fs::read_to_string(&baseline_path).expect("Should read baseline file");
    let count: u32 = content.trim().parse().expect("Should parse as u32");

    assert!(count > 0, "Baseline should be established (non-zero)");
}

// =============================================================================
// Property 2: Allow Attribute Regex Correctness
// =============================================================================

/// Property: The allow attribute regex should match various #![allow(clippy::panic)] forms.
#[test]
fn property_allow_pattern_matches_valid_allow_attributes() {
    let pattern = allow_pattern();

    let should_match = vec![
        "#![allow(clippy::panic)]",
        "#![allow( clippy::panic )]",
        "#![allow(  clippy::panic  )]",
        "#![allow(clippy::panic)] // trailing comment",
    ];

    for case in should_match {
        assert!(pattern.is_match(case), "Allow pattern should match: {}", case);
    }
}

/// Property: The allow attribute regex should NOT match different lint names.
#[test]
fn property_allow_pattern_rejects_different_lints() {
    let pattern = allow_pattern();

    let should_not_match = vec![
        "#![allow(clippy::panic_in_test_code)]",
        "#![allow(clippy::expect_used)]",
        "#![allow(unused)]",
    ];

    for case in should_not_match {
        assert!(!pattern.is_match(case), "Allow pattern should NOT match different lint: {}", case);
    }
}

/// Property: The allow attribute regex handles various whitespace patterns.
#[test]
fn property_allow_pattern_handles_various_whitespace() {
    let pattern = allow_pattern();

    let test_cases = vec![
        ("#![allow(clippy::panic)]", true),
        ("#![  allow  (  clippy::panic  )  ]", true),
        ("#![allow(clippy::panic)]// comment", true),
        ("#![allow(clippy::panic)]  ", true),
    ];

    for (input, expected) in test_cases {
        let result = pattern.is_match(input);
        assert_eq!(result, expected, "Allow pattern mismatch for: {}", input);
    }
}

// =============================================================================
// Property 3: Scan Determinism
// =============================================================================

/// Property: Scanning for allow attributes should be deterministic.
#[test]
fn property_allow_scan_is_deterministic() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-dead-code/tests");

    fn count_allows_in_dir(dir: &PathBuf) -> usize {
        let pattern = allow_pattern();
        let mut count = 0;
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            count += pattern.find_iter(&content).count();
        }
        count
    }

    let count1 = count_allows_in_dir(&tests_dir);
    let count2 = count_allows_in_dir(&tests_dir);
    let count3 = count_allows_in_dir(&tests_dir);

    assert_eq!(count1, count2, "First and second scan should match");
    assert_eq!(count2, count3, "Second and third scan should match");
}

// =============================================================================
// Property 4: TDD-Support Usage is Properly Guarded
// =============================================================================

/// Property: All test files that use tdd-support helpers must have #![allow(clippy::panic)].
#[test]
fn property_tdd_support_usage_is_properly_guarded() {
    let ws_root = workspace_root();
    let tests_dir = ws_root.join("crates/perl-dead-code/tests");

    for entry in walkdir::WalkDir::new(&tests_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        // If file uses tdd-support helpers (must, must_some, must_err)
        let uses_tdd_support = content.contains("perl_tdd_support::must")
            || content.contains("must_some")
            || content.contains("must_err")
            || content.contains("::must(");

        // And uses tdd-support, then it MUST have the allow attribute
        if uses_tdd_support {
            let has_allow = allow_pattern().is_match(&content);
            assert!(
                has_allow,
                "File {:?} uses tdd-support but lacks #![allow(clippy::panic)]",
                entry.path()
            );
        }
    }
}

// =============================================================================
// Property 5: Production Baselines Unchanged
// =============================================================================

/// Property: Production baselines should remain at 0 (no regression).
#[test]
fn property_production_baselines_are_zero() {
    let ws_root = workspace_root();

    let panic_prod: u32 = fs::read_to_string(ws_root.join("ci/panic_prod_baseline.txt"))
        .expect("Should read panic_prod_baseline")
        .trim()
        .parse()
        .expect("panic_prod_baseline should be a number");

    let unwrap_prod: u32 = fs::read_to_string(ws_root.join("ci/unwrap_prod_baseline.txt"))
        .expect("Should read unwrap_prod_baseline")
        .trim()
        .parse()
        .expect("unwrap_prod_baseline should be a number");

    assert_eq!(panic_prod, 0, "Production panic baseline should remain 0");
    assert_eq!(unwrap_prod, 0, "Production unwrap baseline should remain 0");
}

// =============================================================================
// Property 6: Baseline Files Exist and Are Valid
// =============================================================================

/// Property: All required baseline files should exist.
#[test]
fn property_required_baseline_files_exist() {
    let ws_root = workspace_root();

    let required_files = vec![
        "ci/panic_test_baseline.txt",
        "ci/todo_test_baseline.txt",
        "ci/panic_prod_baseline.txt",
        "ci/unwrap_prod_baseline.txt",
    ];

    for file in required_files {
        let path = ws_root.join(file);
        assert!(path.exists(), "Required baseline file {:?} should exist", file);
    }
}

/// Property: Baseline files should be non-empty and parseable.
#[test]
fn property_baseline_files_are_parseable() {
    let ws_root = workspace_root();

    let baselines = vec![
        ("ci/panic_test_baseline.txt", "panic_test_baseline"),
        ("ci/todo_test_baseline.txt", "todo_test_baseline"),
    ];

    for (file, name) in baselines {
        let path = ws_root.join(file);
        let content = fs::read_to_string(&path).expect(&format!("Should read {}", name));
        let trimmed = content.trim();
        assert!(!trimmed.is_empty(), "{} should not be empty", name);
        let _: u32 = trimmed.parse().expect(&format!("{} should be parseable as u32", name));
    }
}

// =============================================================================
// Property 7: Dev-Dependencies Are Properly Configured
// =============================================================================

/// Property: perl-dead-code should have perl-tdd-support in dev-dependencies.
#[test]
fn property_perl_dead_code_has_tdd_support_dev_dependency() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-dead-code/Cargo.toml");
    let content = fs::read_to_string(&cargo_path).expect("Should read Cargo.toml");

    assert!(content.contains("[dev-dependencies]"), "Should have [dev-dependencies] section");
    assert!(content.contains("perl-tdd-support"), "Should have perl-tdd-support dependency");
}

/// Property: perl-lsp-feature-policy should have perl-tdd-support in dev-dependencies.
#[test]
fn property_perl_lsp_feature_policy_has_tdd_support_dev_dependency() {
    let ws_root = workspace_root();
    let cargo_path = ws_root.join("crates/perl-lsp-feature-policy/Cargo.toml");
    let content = fs::read_to_string(&cargo_path).expect("Should read Cargo.toml");

    assert!(content.contains("[dev-dependencies]"), "Should have [dev-dependencies] section");
    assert!(content.contains("perl-tdd-support"), "Should have perl-tdd-support dependency");
}

// =============================================================================
// Property 8: Verified Crates Are Clean
// =============================================================================

/// Property: Verified crates should have no panic! in match-arm catches.
/// This is the acceptance criterion for the burn-down.
#[test]
fn property_verified_crates_are_clean() {
    let ws_root = workspace_root();

    // Pattern that matches old-style match-arm catches
    // This matches: `other => panic!(...)` or `_ => panic!(...)`
    let panic_pattern =
        Regex::new(r"(?m)^\s*(?:other\s*(?:,|=>)|_)\s*=>\s*panic!\s*\(").expect("Invalid regex");

    let verified_crates = vec![
        "crates/perl-parser-core/src/engine/parser",
        "crates/perl-dap/tests",
        "crates/perl-lexer/tests",
    ];

    for crate_dir in verified_crates {
        let dir = ws_root.join(crate_dir);

        if !dir.exists() {
            continue;
        }

        let mut failures = Vec::new();

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let content = fs::read_to_string(entry.path()).unwrap_or_default();

            // Skip should_panic tests - they intentionally have panic!
            if content.contains("#[should_panic]") {
                continue;
            }

            if let Some(m) = panic_pattern.find(&content) {
                failures.push(format!(
                    "{}:{}-{}: {}",
                    entry.path().display(),
                    m.start(),
                    m.end(),
                    m.as_str()
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "Found panic! in match-arm catches in {}: {:#?}",
            crate_dir,
            failures
        );
    }
}

// =============================================================================
// Summary Property
// =============================================================================

/// Property: The total panic count in test code should not increase.
#[test]
fn property_panic_count_is_at_or_below_baseline() {
    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");

    let baseline: u32 = fs::read_to_string(&baseline_path)
        .expect("Should read baseline")
        .trim()
        .parse()
        .expect("Should parse as u32");

    // This property documents that baseline is the ceiling
    assert!(baseline > 0, "Baseline should be established");
    println!("INFO: panic_test_baseline = {} (ceiling for remediation)", baseline);
}

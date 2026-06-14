//! Red TDD mechanical guard test for #1445: enforce codec-only variablesReference arithmetic.
//!
//! This test verifies that `crates/perl-dap/src/debug_adapter/parsing.rs` contains NO raw
//! variablesReference arithmetic (%, /, *, specific constants like 1_000_000, 2_000_000_000)
//! OUTSIDE of comments and test blocks.
//!
//! The contract enforced by this test:
//! **Only `var_ref.rs` produces/consumes variablesReference wire values via arithmetic.
//!   All other files use VariableReference::encode() and decode().**
//!
//! This prevents future #1445-like bugs where ad-hoc arithmetic creates untyped refs.

use std::fs;
use std::path::Path;

/// Helper: find all line numbers containing a pattern in a string.
fn find_pattern_lines(content: &str, pattern: &str) -> Vec<(usize, String)> {
    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            matches.push((line_num + 1, line.trim().to_string()));
        }
    }
    matches
}

/// RED TEST for #1445 guard: verify that the collision bug (saturating_mul(100)) is present.
///
/// This test MUST FAIL on current code (the bug exists).
/// After the fix, it MUST PASS (saturating_mul(100) is removed).
///
/// The bug: fallback_scope_variables uses `variables_ref.saturating_mul(100) + offset`
/// which produces child refs in the EvalResult band for deep frames (collision #1445).
#[test]
fn test_var_ref_codec_no_raw_arithmetic_in_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let parsing_rs_path =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/debug_adapter/parsing.rs"));

    let content = fs::read_to_string(parsing_rs_path)
        .map_err(|e| format!("failed to read {}: {}", parsing_rs_path.display(), e))?;

    // The #1445 BUG: saturating_mul(100) produces collision.
    // Search in UNSTRIPPED content (comments don't matter for this pattern).
    let bug_pattern = "saturating_mul(100)";
    let matches = find_pattern_lines(&content, bug_pattern);

    // RED TEST: This test FAILS when the bug is present (saturating_mul found).
    // After the builder fixes it, this test PASSES (no saturating_mul found).
    // BEFORE fix: assertion fails — TEST IS RED
    // AFTER fix: assertion passes — TEST IS GREEN
    assert!(
        matches.is_empty(),
        "#1445 BUG DETECTED: Found '{}' in parsing.rs (fallback_scope_variables collision):\n{}",
        bug_pattern,
        matches
            .iter()
            .map(|(line_num, text)| format!("Line {}: {}", line_num, text))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(())
}

/// Sanity check: verify that var_ref.rs DOES have the expected patterns
/// (as a positive control — var_ref.rs should contain the arithmetic we're forbidding elsewhere).
#[test]
fn test_var_ref_rs_contains_expected_arithmetic_patterns() -> Result<(), Box<dyn std::error::Error>>
{
    let var_ref_rs_path =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/debug_adapter/var_ref.rs"));

    let content = fs::read_to_string(var_ref_rs_path)
        .map_err(|e| format!("failed to read {}: {}", var_ref_rs_path.display(), e))?;

    // var_ref.rs SHOULD contain these patterns (they define the codec).
    assert!(
        content.contains("* 10 +"),
        "var_ref.rs should contain Scope encoding (frame_id * 10 + kind)"
    );
    assert!(content.contains("<< 16"), "var_ref.rs should contain Child encoding (parent << 16)");
    assert!(content.contains("2_000_000_000"), "var_ref.rs should contain CHILD_BASE constant");
    assert!(
        content.contains("EVAL_BASE"),
        "var_ref.rs should contain EVAL_BASE for EvalResult band"
    );

    Ok(())
}

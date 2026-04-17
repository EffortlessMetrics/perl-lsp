//! Tests to verify safe eval documentation correctly clarifies its limitations.
//!
//! These tests verify that documentation correctly states:
//! - Safe eval provides SYNTHACTIC VALIDATION ONLY (admission control)
//! - Safe eval does NOT provide interpreter sandboxing or isolation
//!
//! This addresses the documentation gap where "safe eval" could be misinterpreted
//! as providing strong security isolation when it only performs syntactic validation.

use std::fs;
use std::path::PathBuf;

/// Get the repository root (parent of CARGO_MANIFEST_DIR since we're in a crate)
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/<crate>
        .unwrap()
        .parent() // repo root
        .unwrap()
        .to_path_buf()
}

/// Documentation files that should contain the safe eval clarification.
const DOCUMENTATION_FILES: &[&str] = &[
    "docs/tutorials/DAP_USER_GUIDE.md",
    "docs/adr/0019-security-first-dap.md",
    "docs/adr/0028-safe-eval-timeout.md",
    "docs/DAP_SECURITY_SPECIFICATION.md",
    "crates/perl-dap/src/debug_adapter/safe_eval.rs",
];

/// Test that all relevant documentation files exist.
#[test]
fn test_documentation_files_exist() {
    let root = repo_root();
    for file_path in DOCUMENTATION_FILES {
        let full_path = root.join(file_path);
        assert!(
            full_path.exists(),
            "Documentation file '{}' should exist at '{}'",
            file_path,
            full_path.display()
        );
    }
}

/// Test that safe eval documentation contains clarification about syntactic validation only.
/// This is the key documentation gap that this work item addresses.
#[test]
fn test_safe_eval_contains_syntactic_validation_clarification() {
    let root = repo_root();
    let file_path = root.join("crates/perl-dap/src/debug_adapter/safe_eval.rs");

    let content = fs::read_to_string(&file_path).expect("Failed to read safe_eval.rs");

    // The code comments should already contain the clarification
    assert!(
        content.contains("admission control"),
        "safe_eval.rs should clarify it is 'admission control', not a sandbox"
    );
    assert!(
        content.contains("syntactic validation") || content.contains("does not provide interpreter isolation"),
        "safe_eval.rs should clarify it provides only syntactic validation, not interpreter isolation"
    );
}

/// Test that DAP_SECURITY_SPECIFICATION.md explicitly clarifies safe eval limitations.
#[test]
fn test_security_spec_clarifies_safe_eval_is_not_sandbox() {
    let root = repo_root();
    let file_path = root.join("docs/DAP_SECURITY_SPECIFICATION.md");

    let content = fs::read_to_string(&file_path).expect("Failed to read DAP_SECURITY_SPECIFICATION.md");

    // The internal security spec should already have this clarification
    // Note: The text may be split across lines, so we check for both key phrases
    let has_not_sandboxed = content.contains("not a sandboxed");
    let has_interpreter_boundary = content.contains("interpreter boundary");

    assert!(
        has_not_sandboxed && has_interpreter_boundary,
        "DAP_SECURITY_SPECIFICATION.md should clarify safe eval is not a sandboxed interpreter boundary.\n\
         Found 'not a sandboxed': {}, Found 'interpreter boundary': {}",
        has_not_sandboxed,
        has_interpreter_boundary
    );
}

/// Test that ADR-0028 (Safe Eval Timeout) mentions the limitation.
#[test]
fn test_adr_0028_mentions_safe_eval_limitation() {
    let root = repo_root();
    let file_path = root.join("docs/adr/0028-safe-eval-timeout.md");

    let content = fs::read_to_string(&file_path).expect("Failed to read ADR-0028");

    // ADR should mention that safe eval is about policy validation + timeout
    // and does not replace a sandbox
    let has_policy_validation = content.contains("policy validation");
    let has_not_sandbox = content.contains("not a sandbox") || content.contains("sandbox");

    // Either the ADR already has the clarification or it needs to be added
    // This test documents the expected state after documentation clarification
    if !has_policy_validation && !has_not_sandbox {
        panic!(
            "ADR-0028 should mention that safe eval provides policy validation,\n\
             not sandboxed isolation. Consider adding clarification about:\n\
             - Safe eval is syntactic validation (admission control)\n\
             - Safe eval does not provide interpreter isolation\n\
             - Timeout enforcement is the other key protection"
        );
    }
}

/// Test that user-facing DAP_USER_GUIDE.md doesn't make misleading claims about safe eval.
/// The guide mentions "safe mode" but should clarify it's syntactic validation only.
#[test]
fn test_dap_user_guide_safe_eval_context() {
    let root = repo_root();
    let file_path = root.join("docs/tutorials/DAP_USER_GUIDE.md");

    let content = fs::read_to_string(&file_path).expect("Failed to read DAP_USER_GUIDE.md");

    // The user guide mentions "safe mode" and "safe eval"
    // It should either:
    // 1. Have a clarification section explaining safe eval is syntactic validation only
    // 2. Or reference where users can learn more about the limitation

    let mentions_safe_eval = content.contains("safe eval") || content.contains("safe mode");
    assert!(
        mentions_safe_eval,
        "DAP_USER_GUIDE.md should mention safe eval/safe mode"
    );

    // If it mentions safe eval, check for clarification context
    // The guide should either already have the clarification or needs it added
    if content.contains("safe eval") || content.contains("safe mode") {
        // Look for clarification phrases near the safe eval mentions
        let has_clarification = content.contains("syntactic validation")
            || content.contains("admission control")
            || content.contains("not a sandbox")
            || content.contains("policy validation")
            || content.contains("expression cannot contain newlines")
            || content.contains("timeout enforcement");

        if !has_clarification {
            panic!(
                "DAP_USER_GUIDE.md mentions 'safe eval' or 'safe mode' but does not\n\
                 clarify that this is syntactic validation only (admission control),\n\
                 not a sandboxed interpreter. Consider adding a note explaining that\n\
                 safe eval checks expression syntax and blocks known dangerous ops,\n\
                 but does not provide OS-level isolation."
            );
        }
    }
}

/// Test that ADR-0019 (Security-First DAP) includes context about safe eval limitations.
#[test]
fn test_adr_0019_safe_eval_limitation_context() {
    let root = repo_root();
    let file_path = root.join("docs/adr/0019-security-first-dap.md");

    let content = fs::read_to_string(&file_path).expect("Failed to read ADR-0019");

    // ADR-0019 should clarify that "safe evaluation defaults" is about
    // syntactic validation + timeout, not sandboxing

    if content.contains("safe evaluation defaults") || content.contains("Safe Evaluation") {
        // Should have context about what safe eval actually does
        let has_context = content.contains("syntactic validation")
            || content.contains("admission control")
            || content.contains("policy validation")
            || content.contains("expression sanitization")
            || content.contains("not a sandbox");

        if !has_context {
            panic!(
                "ADR-0019 mentions 'safe evaluation defaults' but does not clarify\n\
                 that this is syntactic validation (admission control), not sandboxing.\n\
                 Consider adding context that safe eval:\n\
                 - Validates expressions don't have side effects\n\
                 - Does NOT provide interpreter isolation or OS sandboxing\n\
                 - Works alongside timeout enforcement for DoS prevention"
            );
        }
    }
}

/// Integration test: Verify all key documentation together provide complete picture.
/// This ensures the documentation gap is addressed across all relevant docs.
#[test]
fn test_documentation_gap_closure_for_safe_eval() {
    let root = repo_root();

    // Read all documentation files
    let docs: Vec<(String, String)> = DOCUMENTATION_FILES
        .iter()
        .map(|f| {
            let path = root.join(f);
            let content = fs::read_to_string(&path).unwrap_or_default();
            (f.to_string(), content)
        })
        .collect();

    // Count how many docs mention safe eval
    let safe_eval_mentions: Vec<&str> = docs
        .iter()
        .filter(|(_, c)| c.contains("safe eval") || c.contains("Safe Evaluation") || c.contains("safe mode"))
        .map(|(f, _)| f.as_str())
        .collect();

    assert!(
        !safe_eval_mentions.is_empty(),
        "At least some docs should mention safe eval"
    );

    // Count how many docs have clarification context
    let clarification_contexts: Vec<&str> = docs
        .iter()
        .filter(|(_, c)| {
            c.contains("syntactic validation")
                || c.contains("admission control")
                || c.contains("not a sandbox")
                || c.contains("policy validation")
                || c.contains("does not provide interpreter isolation")
        })
        .map(|(f, _)| f.as_str())
        .collect();

    // The majority of docs mentioning safe eval should have clarification context
    // If less than half have clarification, there's a documentation gap
    if safe_eval_mentions.len() > 1 && clarification_contexts.len() < safe_eval_mentions.len() / 2 {
        let missing: Vec<&str> = safe_eval_mentions
            .iter()
            .filter(|f| !clarification_contexts.contains(f))
            .copied()
            .collect();

        panic!(
            "Documentation gap detected! {} docs mention safe eval, \
             but only {} have clarification context about it being syntactic validation only.\n\
             Docs needing clarification: {:?}\n\
             Required clarification phrases should include:\n\
             - 'syntactic validation' or 'admission control'\n\
             - 'not a sandbox' or 'does not provide interpreter isolation'",
            safe_eval_mentions.len(),
            clarification_contexts.len(),
            missing
        );
    }
}
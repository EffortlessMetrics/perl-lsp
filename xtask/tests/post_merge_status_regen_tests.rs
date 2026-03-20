//! Post-merge CURRENT_STATUS.md regeneration validation tests.
//!
//! Tests for issue #2296: infra: centralize CURRENT_STATUS.md rendering (post-merge regeneration).
//!
//! Validates:
//! - The `policy_checks` gate no longer blocks PRs with a `--check` on CURRENT_STATUS.md.
//! - A post-merge workflow exists to auto-regenerate CURRENT_STATUS.md on push to master.
//! - The GATE_REGISTRY.toml policy gate command does not require a freshness check either.

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    // Walk up from the manifest directory to the workspace root.
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // xtask is at <root>/xtask -- go up one level
    dir.pop();
    dir
}

/// The `policy_checks` gate in gate-policy.yaml must not run
/// `update-current-status.py --check` as part of a PR merge gate.
/// That check is now handled post-merge by the dedicated workflow.
#[test]
fn test_policy_checks_gate_does_not_block_on_stale_status() {
    let root = project_root();
    let gate_policy_path = root.join(".ci/gate-policy.yaml");
    let content =
        fs::read_to_string(&gate_policy_path).expect("should be able to read .ci/gate-policy.yaml");

    // Find the policy_checks block
    let policy_block_start = content
        .find("name: policy_checks")
        .expect("policy_checks gate must exist in gate-policy.yaml");

    // Extract the policy_checks gate section (up to next gate entry or end of file)
    let policy_section = &content[policy_block_start..];
    let section_end =
        policy_section[1..].find("\n  - name:").map(|i| i + 1).unwrap_or(policy_section.len());
    let policy_section = &policy_section[..section_end];

    // The --check on update-current-status.py must NOT appear in the policy_checks gate.
    // Stale CURRENT_STATUS.md is regenerated post-merge, not blocked in PRs.
    assert!(
        !policy_section.contains("update-current-status.py --check"),
        "policy_checks gate must not run `update-current-status.py --check`.\n\
         This check causes PR merge conflicts. Regeneration is now post-merge.\n\
         Found in gate-policy.yaml policy_checks section:\n{}",
        policy_section
    );
}

/// The GATE_REGISTRY.toml policy gate must not require CURRENT_STATUS.md freshness check.
#[test]
fn test_gate_registry_policy_does_not_require_status_check() {
    let root = project_root();
    let registry_path = root.join(".ci/GATE_REGISTRY.toml");
    let content =
        fs::read_to_string(&registry_path).expect("should be able to read .ci/GATE_REGISTRY.toml");

    // Find the policy gate section
    let policy_start =
        content.find("id = \"policy\"").expect("policy gate must exist in GATE_REGISTRY.toml");

    let policy_section = &content[policy_start..];
    let section_end =
        policy_section[1..].find("\n[[gate]]").map(|i| i + 1).unwrap_or(policy_section.len());
    let policy_section = &policy_section[..section_end];

    assert!(
        !policy_section.contains("update-current-status.py --check"),
        "GATE_REGISTRY.toml policy gate must not require CURRENT_STATUS.md freshness check.\n\
         Regeneration is handled post-merge, not blocked in PRs.\n\
         Found in GATE_REGISTRY.toml policy section:\n{}",
        policy_section
    );
}

/// A post-merge workflow must exist that regenerates CURRENT_STATUS.md on push to master.
#[test]
fn test_post_merge_status_workflow_exists() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    assert!(
        workflow_path.exists(),
        "Missing post-merge status workflow at .github/workflows/post-merge-status.yml.\n\
         This workflow is required to auto-regenerate CURRENT_STATUS.md after merges.\n\
         See issue #2296."
    );
}

/// The post-merge workflow must trigger on push to master.
#[test]
fn test_post_merge_workflow_triggers_on_push_to_master() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    let content =
        fs::read_to_string(&workflow_path).expect("post-merge-status.yml should be readable");

    assert!(content.contains("push:"), "post-merge-status.yml must have a push trigger");
    assert!(
        content.contains("master"),
        "post-merge-status.yml push trigger must include master branch"
    );
}

/// The post-merge workflow must run the update-status write command.
#[test]
fn test_post_merge_workflow_runs_status_update() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    let content =
        fs::read_to_string(&workflow_path).expect("post-merge-status.yml should be readable");

    // The workflow must invoke either the xtask command or the python script with --write
    let runs_status_update = content.contains("update-status --write")
        || content.contains("update-current-status.py --write");

    assert!(
        runs_status_update,
        "post-merge-status.yml must run status update with --write flag.\n\
         Expected one of: `update-status --write` or `update-current-status.py --write`.\n\
         Workflow content:\n{}",
        content
    );
}

/// The post-merge workflow must auto-commit changed files.
#[test]
fn test_post_merge_workflow_auto_commits() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    let content =
        fs::read_to_string(&workflow_path).expect("post-merge-status.yml should be readable");

    assert!(
        content.contains("git commit") || content.contains("git push"),
        "post-merge-status.yml must commit and push regenerated CURRENT_STATUS.md.\n\
         Workflow content:\n{}",
        content
    );
}

//! Post-merge status regeneration validation tests.
//!
//! Tests for issue #2801: split monolithic CURRENT_STATUS.md into modular status files.
//! Tests for issue #2296: infra: centralize CURRENT_STATUS.md rendering (post-merge regeneration).
//!
//! Validates:
//! - The `policy_checks` gate no longer blocks PRs with a `--check` on CURRENT_STATUS.md.
//! - A post-merge workflow exists to auto-regenerate the status subsystem files.
//! - The GATE_REGISTRY.toml policy gate command does not require a freshness check.
//! - The modular structure (4 generated files + stable stub) is correctly wired.

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
fn test_policy_checks_gate_does_not_block_on_stale_status() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let gate_policy_path = root.join(".ci/gate-policy.yaml");
    let content = fs::read_to_string(&gate_policy_path)?;

    // Find the policy_checks block
    let policy_block_start = content
        .find("name: policy_checks")
        .ok_or("policy_checks gate must exist in gate-policy.yaml")?;

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
    Ok(())
}

/// The GATE_REGISTRY.toml policy gate must not require CURRENT_STATUS.md freshness check.
#[test]
fn test_gate_registry_policy_does_not_require_status_check()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let registry_path = root.join(".ci/GATE_REGISTRY.toml");
    let content = fs::read_to_string(&registry_path)?;

    // Find the policy gate section
    let policy_start =
        content.find("id = \"policy\"").ok_or("policy gate must exist in GATE_REGISTRY.toml")?;

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
    Ok(())
}

/// A post-merge workflow must exist that regenerates status subsystem files on push to master.
#[test]
fn test_post_merge_status_workflow_exists() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    assert!(
        workflow_path.exists(),
        "Missing post-merge status workflow at .github/workflows/post-merge-status.yml.\n\
         This workflow is required to auto-regenerate status files after merges.\n\
         See issue #2296 and issue #2801."
    );
}

/// The post-merge workflow must trigger on push to master.
#[test]
fn test_post_merge_workflow_triggers_on_push_to_master() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");
    let content = fs::read_to_string(&workflow_path)?;

    assert!(content.contains("push:"), "post-merge-status.yml must have a push trigger");
    assert!(
        content.contains("master"),
        "post-merge-status.yml push trigger must include master branch"
    );
    Ok(())
}

/// The post-merge workflow must run the update-status write command.
#[test]
fn test_post_merge_workflow_runs_status_update() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");
    let content = fs::read_to_string(&workflow_path)?;

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
    Ok(())
}

/// The post-merge workflow must auto-commit changed files.
#[test]
fn test_post_merge_workflow_auto_commits() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");
    let content = fs::read_to_string(&workflow_path)?;

    assert!(
        content.contains("git commit") || content.contains("git push"),
        "post-merge-status.yml must commit and push regenerated status files.\n\
         Workflow content:\n{}",
        content
    );
    Ok(())
}

/// The post-merge workflow must commit files in docs/project/status/ — NOT CURRENT_STATUS.md alone.
#[test]
fn test_post_merge_workflow_commits_status_directory() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    let content =
        fs::read_to_string(&workflow_path).expect("post-merge-status.yml should be readable");

    // The workflow must add files from the status/ subdirectory.
    assert!(
        content.contains("docs/project/status/"),
        "post-merge-status.yml must commit files in docs/project/status/ (modular structure).\n\
         After issue #2801, CURRENT_STATUS.md is a stable stub — status/ files are generated.\n\
         Workflow content:\n{}",
        content
    );
}

/// The post-merge workflow must NOT git-add CURRENT_STATUS.md (it is now human-owned stable stub).
#[test]
fn test_post_merge_workflow_does_not_add_current_status() {
    let root = project_root();
    let workflow_path = root.join(".github/workflows/post-merge-status.yml");

    let content =
        fs::read_to_string(&workflow_path).expect("post-merge-status.yml should be readable");

    // The workflow must not try to commit CURRENT_STATUS.md — it's a stable human-owned stub now.
    assert!(
        !content.contains("git add docs/project/CURRENT_STATUS.md"),
        "post-merge-status.yml must not git-add CURRENT_STATUS.md.\n\
         After issue #2801, CURRENT_STATUS.md is a stable human-owned stub.\n\
         Generated metrics are in docs/project/status/*.md\n\
         Workflow content:\n{}",
        content
    );
}

/// The stub CURRENT_STATUS.md must not contain any <!-- BEGIN: --> markers.
/// If it does, `xtask update-status --check` will try to patch it and fail.
#[test]
fn test_stub_current_status_has_no_begin_markers() {
    let root = project_root();
    let stub_path = root.join("docs/project/CURRENT_STATUS.md");

    let content = fs::read_to_string(&stub_path)
        .expect("docs/project/CURRENT_STATUS.md should exist as a stub");

    assert!(
        !content.contains("<!-- BEGIN:"),
        "CURRENT_STATUS.md must not contain <!-- BEGIN: --> markers.\n\
         It is now a stable stub — generated content belongs in docs/project/status/*.md\n\
         Remove all <!-- BEGIN: ... --> blocks from CURRENT_STATUS.md."
    );
}

/// All subsystem status files must contain their expected marker blocks.
///
/// This is an integration-level gate: if any marker is missing the xtask
/// `update-status` command will fail with "Expected 1 match ... got 0".
#[test]
fn test_subsystem_files_have_markers() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let status_dir = root.join("docs/project/status");

    let lsp = fs::read_to_string(status_dir.join("lsp.md"))?;
    assert!(lsp.contains("<!-- BEGIN: LSP_COVERAGE -->"), "lsp.md missing LSP_COVERAGE block");
    assert!(
        lsp.contains("<!-- BEGIN: LSP_METRICS_BULLETS -->"),
        "lsp.md missing LSP_METRICS_BULLETS block"
    );
    assert!(
        lsp.contains("<!-- BEGIN: COMPLIANCE_TABLE -->"),
        "lsp.md missing COMPLIANCE_TABLE block"
    );

    let tests_md = fs::read_to_string(status_dir.join("tests.md"))?;
    assert!(
        tests_md.contains("<!-- BEGIN: TESTS_TABLE_ROWS -->"),
        "tests.md missing TESTS_TABLE_ROWS block"
    );
    assert!(
        tests_md.contains("<!-- BEGIN: TESTS_METRICS_BULLETS -->"),
        "tests.md missing TESTS_METRICS_BULLETS block"
    );

    let parser = fs::read_to_string(status_dir.join("parser.md"))?;
    assert!(
        parser.contains("<!-- BEGIN: PARSER_TRACKING_TABLE -->"),
        "parser.md missing PARSER_TRACKING_TABLE block"
    );
    assert!(
        parser.contains("<!-- BEGIN: PARSER_METRICS_BULLETS -->"),
        "parser.md missing PARSER_METRICS_BULLETS block"
    );
    assert!(
        parser.contains("<!-- BEGIN: PARSER_NODEKIND_ROW -->"),
        "parser.md missing PARSER_NODEKIND_ROW block"
    );
    assert!(
        parser.contains("<!-- BEGIN: PARSER_RELIABILITY_ROW -->"),
        "parser.md missing PARSER_RELIABILITY_ROW block"
    );
    assert!(
        parser.contains("<!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->"),
        "parser.md missing PARSER_STRICT_CLEAN_ROW block"
    );

    let quality = fs::read_to_string(status_dir.join("quality.md"))?;
    assert!(
        quality.contains("<!-- BEGIN: QUALITY_METRICS_BULLETS -->"),
        "quality.md missing QUALITY_METRICS_BULLETS block"
    );
    assert!(
        quality.contains("<!-- BEGIN: QUALITY_CRATE_TABLE -->"),
        "quality.md missing QUALITY_CRATE_TABLE block"
    );

    let dap = fs::read_to_string(status_dir.join("dap.md"))?;
    assert!(
        dap.contains("<!-- BEGIN: DAP_TEST_COUNTS -->"),
        "dap.md missing DAP_TEST_COUNTS block"
    );

    let workspace_md = fs::read_to_string(status_dir.join("workspace.md"))?;
    assert!(
        workspace_md.contains("<!-- BEGIN: WORKSPACE_STALE_RATE -->"),
        "workspace.md missing WORKSPACE_STALE_RATE block"
    );
    assert!(
        workspace_md.contains("<!-- BEGIN: WORKSPACE_SLO_TABLE -->"),
        "workspace.md missing WORKSPACE_SLO_TABLE block"
    );
    assert!(
        workspace_md.contains("<!-- BEGIN: WORKSPACE_MULTIROOT -->"),
        "workspace.md missing WORKSPACE_MULTIROOT block"
    );
    assert!(
        workspace_md.contains("<!-- BEGIN: WORKSPACE_FIXTURES -->"),
        "workspace.md missing WORKSPACE_FIXTURES block"
    );
    assert!(
        workspace_md.contains("<!-- BEGIN: WORKSPACE_METRICS_BULLETS -->"),
        "workspace.md missing WORKSPACE_METRICS_BULLETS block"
    );

    Ok(())
}

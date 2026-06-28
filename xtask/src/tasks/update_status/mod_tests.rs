// Coordination-layer tests for replace_block helpers and file-existence checks.

use super::*;

#[test]
fn test_replace_block() -> Result<()> {
    let input = "before\n<!-- BEGIN: X -->\nold content\n<!-- END: X -->\nafter";
    let result = replace_block(input, "<!-- BEGIN: X -->", "<!-- END: X -->", "new content")?;
    assert_eq!(result, "before\n<!-- BEGIN: X -->\nnew content\n<!-- END: X -->\nafter");
    Ok(())
}

#[test]
fn test_replace_block_missing_marker() {
    let input = "no markers here";
    let result = replace_block(input, "<!-- BEGIN: X -->", "<!-- END: X -->", "new");
    assert!(result.is_err());
}

#[test]
fn test_update_status_repro_commands_are_write_only() {
    let repros = [
        "cargo xtask update-status --write --only lsp",
        "cargo xtask update-status --write --only tests",
        "cargo xtask update-status --write --only parser",
        "cargo xtask update-status --write --only quality",
        "cargo xtask update-status --write --only dap",
        "cargo xtask update-status --write --only workspace",
    ];

    for repro in repros {
        assert!(
            repro.starts_with("cargo xtask update-status --write --only "),
            "repro command should be a directly runnable write-only subsystem command: {repro}"
        );
    }
}

#[test]
fn test_parser_status_refreshes_accuracy_artifact_before_rendering() -> Result<()> {
    let root = project_root()?;
    let path = root.join("xtask/src/tasks/update_status/mod.rs");
    let content =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let refresh_index = content
        .find("refresh_default_artifact_for_status(&root)?")
        .ok_or_else(|| eyre!("parser subsystem does not refresh accuracy artifact"))?;
    let collect_index = content
        .find("let parser_metrics = parser::collect_parser_metrics(&root);")
        .ok_or_else(|| eyre!("parser subsystem does not collect parser metrics"))?;
    assert!(
        refresh_index < collect_index,
        "parser status must refresh target/metrics/parser_accuracy.json before rendering"
    );
    Ok(())
}

/// The subsystem status files, UX planning scaffold, DAP scorecard, and workspace scorecard must exist.
#[test]
fn test_subsystem_files_exist() -> Result<()> {
    let root = project_root()?;
    let status_dir = root.join("docs/project/status");
    for name in &[
        "lsp.md",
        "tests.md",
        "parser.md",
        "quality.md",
        "editor_ux.json",
        "editor_ux.schema.json",
        "dap.md",
        "workspace.md",
    ] {
        let path = status_dir.join(name);
        assert!(path.exists(), "subsystem file missing: {}", path.display());
    }
    Ok(())
}

/// The stub CURRENT_STATUS.md must NOT contain any <!-- BEGIN: --> markers.
#[test]
fn test_stub_has_no_begin_markers() -> Result<()> {
    let root = project_root()?;
    let stub_path = root.join("docs/project/CURRENT_STATUS.md");
    let content = fs::read_to_string(&stub_path).context("reading CURRENT_STATUS.md")?;
    assert!(
        !content.contains("<!-- BEGIN:"),
        "CURRENT_STATUS.md must not contain <!-- BEGIN: --> markers (it is now a stable stub). \
         Generated content belongs in docs/project/status/*.md"
    );
    Ok(())
}

/// Structural: update_status must be split into a directory module with per-subsystem files.
#[test]
fn test_update_status_is_split_into_subsystem_modules() -> Result<()> {
    let root = project_root()?;
    let status_dir = root.join("xtask/src/tasks/update_status");
    assert!(
        status_dir.exists() && status_dir.is_dir(),
        "update_status must be a directory module at xtask/src/tasks/update_status/ \
         (refactor issue #4174: split from monolithic update_status.rs)"
    );
    let runtime_modules = [
        "cmd.rs",
        "dap.rs",
        "editor_ux.rs",
        "flaky.rs",
        "lsp.rs",
        "mod.rs",
        "parser.rs",
        "parser/accuracy.rs",
        "parser/failure.rs",
        "parser/render.rs",
        "quality.rs",
        "tests.rs",
        "token/mod.rs",
        "token/source.rs",
        "workspace.rs",
    ];

    for name in runtime_modules {
        let path = status_dir.join(name);
        assert!(
            path.exists(),
            "subsystem module {name} missing at xtask/src/tasks/update_status/{name}"
        );
        let content =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let loc = content.lines().count();
        assert!(loc <= 400, "module {name} has {loc} LOC — exceeds 400-line anti-regression gate");
    }
    Ok(())
}

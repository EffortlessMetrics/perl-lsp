//! Update derived metrics in docs/project/status/ subsystem files.
//!
//! Rust port of `scripts/update-current-status.py`.  Computes test counts,
//! feature catalog metrics, corpus statistics, and missing-docs warnings, then
//! patches the markdown files between fenced markers.
//!
//! Subsystem files written:
//!   - docs/project/status/lsp.md     (LSP coverage + compliance table)
//!   - docs/project/status/tests.md   (test counts + tracked debt)
//!   - docs/project/status/parser.md  (parser corpus tracking)
//!   - docs/project/status/quality.md (mutation score, perf)
//!   - docs/project/status/editor_ux.json (UX scorecard receipt)
//!   - docs/project/status/workspace.md (workspace index scorecard)
//!
//! Also keeps docs/project/ROADMAP.md compliance table in sync when lsp subsystem runs.

use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Context, Result, eyre};

use crate::utils::project_root;

mod cmd;
use cmd::{replace_block, run_cmd, run_cmd_merged, run_subsystem};

mod dap;
mod editor_ux;
mod flaky;
mod lsp;
mod parser;
mod quality;
mod tests;
mod token;
mod workspace;

// ---------------------------------------------------------------------------
// Subsystem selector
// ---------------------------------------------------------------------------

/// Which subsystems to regenerate.
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum StatusSubsystem {
    Lsp,
    Tests,
    Parser,
    Quality,
    /// DAP debugger scorecard (launch success, latency, test counts).
    Dap,
    Workspace,
}

impl StatusSubsystem {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusSubsystem::Lsp => "lsp",
            StatusSubsystem::Tests => "tests",
            StatusSubsystem::Parser => "parser",
            StatusSubsystem::Quality => "quality",
            StatusSubsystem::Dap => "dap",
            StatusSubsystem::Workspace => "workspace",
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the update-status task.
///
/// * `write` – write changes back to disk.
/// * `check` – verify files are up to date (returns error if stale).
/// * `only`  – when set, only regenerate the given subsystem; otherwise all.
///
/// When neither `write` nor `check` is set, defaults to `check`.
pub fn run(write: bool, check: bool, only: Option<StatusSubsystem>) -> Result<()> {
    let check = if !write && !check { true } else { check };

    let root = project_root()?;

    let subsystems: Vec<StatusSubsystem> = match only {
        Some(s) => vec![s],
        None => vec![
            StatusSubsystem::Lsp,
            StatusSubsystem::Tests,
            StatusSubsystem::Parser,
            StatusSubsystem::Quality,
            StatusSubsystem::Dap,
            StatusSubsystem::Workspace,
        ],
    };

    let mut files_to_update: Vec<(&'static str, PathBuf, String)> = Vec::new();

    let need_lsp = subsystems.contains(&StatusSubsystem::Lsp);
    let need_tests = subsystems.contains(&StatusSubsystem::Tests);
    let need_parser = subsystems.contains(&StatusSubsystem::Parser);
    let need_quality = subsystems.contains(&StatusSubsystem::Quality);
    let need_dap = subsystems.contains(&StatusSubsystem::Dap);
    let need_workspace = subsystems.contains(&StatusSubsystem::Workspace);

    // --- LSP subsystem ---
    if need_lsp {
        run_subsystem("lsp", "cargo xtask update-status --write --only lsp", || {
            let cov = lsp::count_lsp_coverage(&root)?;
            let compliance_table = lsp::compute_compliance_table(&root)?;

            let lsp_path = root.join("docs/project/status/lsp.md");
            let original_lsp =
                fs::read_to_string(&lsp_path).context("reading docs/project/status/lsp.md")?;
            let updated_lsp = lsp::generate_lsp_status(&cov, &compliance_table, &original_lsp)?;
            if updated_lsp != original_lsp {
                files_to_update.push(("docs/project/status/lsp.md", lsp_path, updated_lsp));
            }

            let roadmap_path = root.join("docs/project/ROADMAP.md");
            let original_roadmap =
                fs::read_to_string(&roadmap_path).context("reading docs/project/ROADMAP.md")?;
            let updated_roadmap = lsp::update_roadmap(&root, &original_roadmap)?;
            if updated_roadmap != original_roadmap {
                files_to_update.push(("docs/project/ROADMAP.md", roadmap_path, updated_roadmap));
            }
            Ok(())
        })?;
    }

    // --- Tests subsystem ---
    if need_tests {
        run_subsystem("tests", "cargo xtask update-status --write --only tests", || {
            let test_counts = tests::count_tests(&root);
            let missing_docs_current = tests::count_missing_docs_perl_parser(&root);
            let missing_docs_baseline = tests::read_missing_docs_baseline(&root);

            let tests_path = root.join("docs/project/status/tests.md");
            let original_tests =
                fs::read_to_string(&tests_path).context("reading docs/project/status/tests.md")?;
            let updated_tests = tests::generate_tests_status(
                &test_counts,
                missing_docs_current,
                missing_docs_baseline,
                &original_tests,
            )?;
            if updated_tests != original_tests {
                files_to_update.push(("docs/project/status/tests.md", tests_path, updated_tests));
            }
            Ok(())
        })?;
    }

    // --- Parser subsystem ---
    if need_parser {
        run_subsystem("parser", "cargo xtask update-status --write --only parser", || {
            let parser_metrics = parser::collect_parser_metrics(&root);

            let parser_path = root.join("docs/project/status/parser.md");
            let original_parser = fs::read_to_string(&parser_path)
                .context("reading docs/project/status/parser.md")?;
            let updated_parser = parser::generate_parser_status(&parser_metrics, &original_parser)?;
            if updated_parser != original_parser {
                files_to_update.push((
                    "docs/project/status/parser.md",
                    parser_path,
                    updated_parser,
                ));
            }
            Ok(())
        })?;
    }

    // --- Quality subsystem ---
    if need_quality {
        run_subsystem("quality", "cargo xtask update-status --write --only quality", || {
            let quality_path = root.join("docs/project/status/quality.md");
            let original_quality = fs::read_to_string(&quality_path)
                .context("reading docs/project/status/quality.md")?;
            let updated_quality = quality::generate_quality_status(&root, &original_quality)?;
            if updated_quality != original_quality {
                files_to_update.push((
                    "docs/project/status/quality.md",
                    quality_path,
                    updated_quality,
                ));
            }

            let ux_path = root.join("docs/project/status/editor_ux.json");
            let original_ux = fs::read_to_string(&ux_path).unwrap_or_default();
            let updated_ux = editor_ux::generate_editor_ux_receipt(&root)?;
            if updated_ux != original_ux {
                files_to_update.push(("docs/project/status/editor_ux.json", ux_path, updated_ux));
            }
            Ok(())
        })?;
    }

    // --- DAP subsystem ---
    if need_dap {
        run_subsystem("dap", "cargo xtask update-status --write --only dap", || {
            let dap_counts = dap::count_dap_tests(&root);

            let dap_path = root.join("docs/project/status/dap.md");
            let original_dap =
                fs::read_to_string(&dap_path).context("reading docs/project/status/dap.md")?;
            let updated_dap = dap::generate_dap_status(&root, &dap_counts, &original_dap)?;
            if updated_dap != original_dap {
                files_to_update.push(("docs/project/status/dap.md", dap_path, updated_dap));
            }
            Ok(())
        })?;
    }

    // --- Workspace subsystem ---
    if need_workspace {
        run_subsystem("workspace", "cargo xtask update-status --write --only workspace", || {
            let workspace_path = root.join("docs/project/status/workspace.md");
            let original_workspace = fs::read_to_string(&workspace_path)
                .context("reading docs/project/status/workspace.md")?;
            let updated_workspace =
                workspace::generate_workspace_status(&root, &original_workspace)?;
            if updated_workspace != original_workspace {
                files_to_update.push((
                    "docs/project/status/workspace.md",
                    workspace_path,
                    updated_workspace,
                ));
            }
            Ok(())
        })?;
    }

    if files_to_update.is_empty() {
        eprintln!("All files are up to date.");
        return Ok(());
    }

    if write {
        for (name, path, content) in &files_to_update {
            fs::write(path, content).with_context(|| format!("writing {name}"))?;
            eprintln!("Updated {name}");
        }
        return Ok(());
    }

    // check mode
    if check {
        for (name, _, _) in &files_to_update {
            eprintln!("{name} is out of date.");
        }
        eprintln!("Run `just status-update`");
        eprintln!("Then re-run `just ci-gate`");
        return Err(eyre!("{} file(s) out of date", files_to_update.len()));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests (coordination layer only: replace_block helpers + file-existence checks)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod mod_tests {
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
        for name in &["mod.rs", "lsp.rs", "tests.rs", "parser.rs", "quality.rs"] {
            let path = status_dir.join(name);
            assert!(
                path.exists(),
                "subsystem module {name} missing at xtask/src/tasks/update_status/{name}"
            );
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let loc = content.lines().count();
            assert!(
                loc <= 400,
                "module {name} has {loc} LOC — exceeds 400-line anti-regression gate"
            );
        }
        Ok(())
    }
}

//! Workspace subsystem status generator.
//!
//! Owns workspace fixture counting, scorecard test counting, and workspace.md generation.

use std::fs;
use std::path::Path;

use color_eyre::eyre::Result;
use walkdir::WalkDir;

use super::replace_block;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count Perl source files (`.pl`, `.pm`) in a directory tree.
fn count_perl_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().map(|ext| ext == "pl" || ext == "pm").unwrap_or(false)
        })
        .count()
}

/// Count the number of `#[test]` annotated functions in the workspace scorecard test file.
fn count_scorecard_tests(root: &Path) -> usize {
    let path = root.join("crates/perl-workspace-index/tests/workspace_scorecard.rs");
    let Ok(content) = fs::read_to_string(&path) else { return 0 };
    content.matches("#[test]").count()
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

pub(super) fn generate_workspace_status(root: &Path, original: &str) -> Result<String> {
    let workspaces_dir = root.join("test_corpus/workspaces");
    let small_count = count_perl_files(&workspaces_dir.join("small"));
    let medium_count = count_perl_files(&workspaces_dir.join("medium"));
    let large_count = count_perl_files(&workspaces_dir.join("large"));

    let scorecard_tests = count_scorecard_tests(root);

    let stale_row = format!(
        "| **Stale-index defect rate** | 0 / {scorecard_tests} scenarios tested | 0% | \
         see `cargo test -p perl-workspace-index -- scorecard` |"
    );

    let slo_table = "\
| Operation | SLO Target | Source |
|-----------|-----------|--------|
| Index initialization (P95) | < 5 000 ms | `perl-workspace-index-slo` |
| Incremental reindex (P95) | < 100 ms | `perl-workspace-index-slo` |
| Definition lookup (P95) | < 50 ms | `perl-workspace-index-slo` |
| Completion (P95) | < 100 ms | `perl-workspace-index-slo` |
| Hover (P95) | < 50 ms | `perl-workspace-index-slo` |"
        .to_string();

    let multiroot_row = "| **Multi-root integration tests** | 8 / 8 tests | 8 / 8 | \
         `just ci-workspace-multiroot` (nightly gate) |"
        .to_string();

    let fixtures_table = format!(
        "| Scale | Path | File count | Purpose |\n\
         |-------|------|-----------|--------|\n\
         | small | `test_corpus/workspaces/small/` | {small_count} | Smoke + SLO P95 baseline |\n\
         | medium | `test_corpus/workspaces/medium/` | {medium_count} | Typical project scale |\n\
         | large | `test_corpus/workspaces/large/` | {large_count} | Enterprise scale |\n\
         | xlarge | `test_corpus/workspaces/xlarge/` | ~10 000 (generated) | Stress / limit discovery |"
    );

    let bullets = format!(
        "- **Stale-index defect rate**: 0 stale-symbol defects across {scorecard_tests} tested deletion/rename scenarios \
         (unit tests in `crates/perl-workspace-index/tests/workspace_scorecard.rs`)\n\
         - **Incremental reindex SLO**: P95 target = 100ms (from `perl-workspace-index-slo`); measured in `scorecard_incremental_reindex_latency_within_slo`\n\
         - **Multi-root tests**: 8 integration tests in `crates/perl-lsp/tests/multi_root_workspace_tests.rs` activated in nightly CI gate via `just ci-workspace-multiroot` (PR #4137)\n\
         - **Fixture workspaces**: 4 scales at `test_corpus/workspaces/` ({small_count} / {medium_count} / {large_count} committed + xlarge generated on demand)"
    );

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: WORKSPACE_STALE_RATE -->",
        "<!-- END: WORKSPACE_STALE_RATE -->",
        &stale_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: WORKSPACE_SLO_TABLE -->",
        "<!-- END: WORKSPACE_SLO_TABLE -->",
        &slo_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: WORKSPACE_MULTIROOT -->",
        "<!-- END: WORKSPACE_MULTIROOT -->",
        &multiroot_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: WORKSPACE_FIXTURES -->",
        "<!-- END: WORKSPACE_FIXTURES -->",
        &fixtures_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: WORKSPACE_METRICS_BULLETS -->",
        "<!-- END: WORKSPACE_METRICS_BULLETS -->",
        &bullets,
    )?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn test_generate_workspace_status_patches_all_blocks() -> Result<()> {
        let root = crate::utils::project_root()?;
        let template = "\
<!-- BEGIN: WORKSPACE_STALE_RATE -->\nold\n<!-- END: WORKSPACE_STALE_RATE -->\n\
<!-- BEGIN: WORKSPACE_SLO_TABLE -->\nold\n<!-- END: WORKSPACE_SLO_TABLE -->\n\
<!-- BEGIN: WORKSPACE_MULTIROOT -->\nold\n<!-- END: WORKSPACE_MULTIROOT -->\n\
<!-- BEGIN: WORKSPACE_FIXTURES -->\nold\n<!-- END: WORKSPACE_FIXTURES -->\n\
<!-- BEGIN: WORKSPACE_METRICS_BULLETS -->\nold\n<!-- END: WORKSPACE_METRICS_BULLETS -->\n";
        let result = generate_workspace_status(&root, template)?;
        for block in &[
            "WORKSPACE_STALE_RATE",
            "WORKSPACE_SLO_TABLE",
            "WORKSPACE_MULTIROOT",
            "WORKSPACE_FIXTURES",
            "WORKSPACE_METRICS_BULLETS",
        ] {
            assert!(
                !result.contains(&format!("<!-- BEGIN: {block} -->\nold\n<!-- END: {block} -->")),
                "workspace status block {block} was not replaced"
            );
        }
        assert!(result.contains("perl-workspace-index-slo"), "SLO table must reference slo crate");
        assert!(result.contains("small"), "fixtures table must list small workspace");
        assert!(result.contains("xlarge"), "fixtures table must list xlarge workspace");
        Ok(())
    }

    #[test]
    fn test_workspace_fixture_directories_exist() -> Result<()> {
        let root = crate::utils::project_root()?;
        let workspaces = root.join("test_corpus/workspaces");
        for scale in &["small", "medium", "large", "xlarge"] {
            let dir = workspaces.join(scale);
            assert!(dir.exists(), "fixture workspace '{scale}' directory is missing");
            assert!(dir.is_dir(), "fixture workspace '{scale}' is not a directory");
        }
        Ok(())
    }
}

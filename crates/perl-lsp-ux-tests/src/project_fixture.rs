//! Shared helpers for source-backed editor UX fixture projects.

use crate::{ScenarioConfig, UxHarness};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REAL_PROJECTS_DIR: &str = "test_corpus/real_projects";
const MOJOLICIOUS_SKELETON: &str = "mojolicious_skeleton";
const DANCER2_SKELETON: &str = "dancer2_skeleton";
const CATALYST_SKELETON: &str = "catalyst_skeleton";

/// Source file loaded from a committed real-project UX fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProjectFixtureFile {
    /// Fixture-relative path using `/` separators.
    pub relative_path: String,
    /// UTF-8 source text for the fixture file.
    pub content: String,
}

/// Resolve the repository workspace root from this crate's manifest location.
pub fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("CARGO_MANIFEST_DIR must be nested under the workspace root")
}

/// Load all Perl source files under the Mojolicious skeleton UX fixture.
pub fn load_mojolicious_fixture_files() -> Result<Vec<ProjectFixtureFile>> {
    load_real_project_fixture_files(MOJOLICIOUS_SKELETON)
}

/// Load all Perl source files under the Dancer2 sample UX fixture.
pub fn load_dancer2_fixture_files() -> Result<Vec<ProjectFixtureFile>> {
    load_real_project_fixture_files(DANCER2_SKELETON)
}

/// Load all Perl source files under the Catalyst sample UX fixture.
pub fn load_catalyst_fixture_files() -> Result<Vec<ProjectFixtureFile>> {
    load_real_project_fixture_files(CATALYST_SKELETON)
}

/// Build a workspace-enabled scenario config seeded with fixture files.
pub fn fixture_scenario_config(files: &[ProjectFixtureFile]) -> ScenarioConfig {
    files.iter().fold(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1"),
        |config, file| config.with_file(&file.relative_path, &file.content),
    )
}

/// Create a UX harness seeded with fixture files and workspace indexing enabled.
pub fn create_fixture_harness(files: &[ProjectFixtureFile]) -> Result<UxHarness> {
    UxHarness::new(fixture_scenario_config(files))
}

/// Open every fixture file in a harness.
pub fn open_all_fixture_files(harness: &UxHarness, files: &[ProjectFixtureFile]) -> Result<()> {
    for file in files {
        harness.open_file(&file.relative_path, &file.content)?;
    }
    Ok(())
}

/// Find fixture content by fixture-relative path.
pub fn fixture_content<'a>(
    files: &'a [ProjectFixtureFile],
    relative_path: &str,
) -> Result<&'a str> {
    files
        .iter()
        .find(|file| file.relative_path == relative_path)
        .map(|file| file.content.as_str())
        .with_context(|| format!("missing fixture file {relative_path}"))
}

fn load_real_project_fixture_files(fixture_name: &str) -> Result<Vec<ProjectFixtureFile>> {
    let root = workspace_root()?.join(REAL_PROJECTS_DIR).join(fixture_name);
    let mut files = Vec::new();
    collect_perl_files(&root, &root, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn collect_perl_files(root: &Path, dir: &Path, files: &mut Vec<ProjectFixtureFile>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry.with_context(|| format!("reading an entry under {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_perl_files(root, &path, files)?;
        } else if is_perl_source(&path) {
            let relative_path = path
                .strip_prefix(root)
                .with_context(|| format!("stripping fixture root from {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            files.push(ProjectFixtureFile { relative_path, content });
        }
    }
    Ok(())
}

fn is_perl_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "pm" | "pl" | "t"))
}

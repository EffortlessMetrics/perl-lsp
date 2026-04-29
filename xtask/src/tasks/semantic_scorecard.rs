use crate::utils::project_root;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FIXTURE_MANIFEST: &str =
    "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/manifest.json";
const DEFAULT_OUTPUT: &str = "target/receipts/metrics/semantic_scorecard.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/semantic_scorecard.md";

const AVAILABLE_FACT_ROWS: &[&str] = &[
    "declaration_facts",
    "occurrence_facts",
    "export_facts",
    "definition_candidates",
    "reference_edges",
];

const UNAVAILABLE_FACT_ROWS: &[&str] = &[
    "import_specs",
    "package_graph",
    "rename_plan",
    "safe_delete_plan",
];

#[derive(Debug, Deserialize)]
struct SemanticManifest {
    fixture_family_version: u32,
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    family: String,
    /// Relative path to the fixture file (reserved for future harness use).
    #[allow(dead_code)]
    path: String,
}

#[derive(Debug, Serialize)]
struct ScorecardRow {
    status: &'static str,
    total_facts: usize,
    exact_facts: usize,
    high_confidence_facts: usize,
    heuristic_facts: usize,
    dynamic_boundary_facts: usize,
    fixture_family_coverage: usize,
}

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    fixture_family_version: u32,
    fixture_count: usize,
    fixture_ids: Vec<String>,
    rows: BTreeMap<String, ScorecardRow>,
    notes: &'static str,
}

pub fn run(manifest: Option<PathBuf>, output: Option<PathBuf>, status_md: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let manifest_path = root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let manifest = load_manifest(&manifest_path)?;
    let artifact = build_artifact(manifest);

    write_json(&output_path, &artifact)?;
    if let Some(parent) = status_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&status_path, render_status_markdown(&artifact))
        .with_context(|| format!("writing {}", status_path.display()))?;

    println!("semantic scorecard updated: {}", output_path.display());
    println!("status page updated: {}", status_path.display());
    Ok(())
}

fn load_manifest(path: &Path) -> Result<SemanticManifest> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut parsed: SemanticManifest =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    parsed.fixtures.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(parsed)
}

fn build_artifact(manifest: SemanticManifest) -> Artifact {
    let fixture_ids = manifest.fixtures.iter().map(|fixture| fixture.id.clone()).collect::<Vec<_>>();
    let fixture_family_coverage = manifest
        .fixtures
        .iter()
        .map(|fixture| fixture.family.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let mut rows = BTreeMap::new();
    for &row in AVAILABLE_FACT_ROWS {
        rows.insert(
            row.to_string(),
            ScorecardRow {
                status: "available_zero",
                total_facts: 0,
                exact_facts: 0,
                high_confidence_facts: 0,
                heuristic_facts: 0,
                dynamic_boundary_facts: 0,
                fixture_family_coverage,
            },
        );
    }

    for &row in UNAVAILABLE_FACT_ROWS {
        rows.insert(
            row.to_string(),
            ScorecardRow {
                status: "unavailable",
                total_facts: 0,
                exact_facts: 0,
                high_confidence_facts: 0,
                heuristic_facts: 0,
                dynamic_boundary_facts: 0,
                fixture_family_coverage: 0,
            },
        );
    }

    Artifact {
        schema_version: 2,
        measured_at: "deterministic-fixture-scorecard-v1",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        rows,
        notes: "Wave 2 scorecard rows are deterministic and adapter-safe: available rows emit zero counts until fact adapters are wired; future rows are explicitly unavailable.",
    }
}

fn write_json(path: &Path, artifact: &Artifact) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(artifact)?;
    fs::write(path, format!("{payload}\n")).with_context(|| format!("writing {}", path.display()))
}

fn render_status_markdown(artifact: &Artifact) -> String {
    let mut text = String::new();
    text.push_str("# Semantic Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&format!("Fixture family version: `{}`  \n", artifact.fixture_family_version));
    text.push_str(&format!("Fixtures loaded: `{}`\n\n", artifact.fixture_count));
    text.push_str("## Fixture IDs\n\n");
    for id in &artifact.fixture_ids {
        text.push_str(&format!("- `{id}`\n"));
    }

    text.push_str("\n## Semantic Facts Coverage\n\n");
    text.push_str("| Row | Status | Total | Exact | High confidence | Heuristic | Dynamic boundary | Fixture-family coverage |\n");
    text.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for (row_name, row) in &artifact.rows {
        text.push_str(&format!(
            "| {row_name} | {} | {} | {} | {} | {} | {} | {} |\n",
            row.status,
            row.total_facts,
            row.exact_facts,
            row.high_confidence_facts,
            row.heuristic_facts,
            row.dynamic_boundary_facts,
            row.fixture_family_coverage,
        ));
    }

    text.push('\n');
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_artifact_emits_all_scorecard_rows() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase { id: "b".to_string(), family: "x".to_string(), path: "b.pl".to_string() },
                FixtureCase { id: "a".to_string(), family: "x".to_string(), path: "a.pl".to_string() },
                FixtureCase { id: "c".to_string(), family: "y".to_string(), path: "c.pl".to_string() },
            ],
        };
        let artifact = build_artifact(manifest);

        assert_eq!(artifact.measured_at, "deterministic-fixture-scorecard-v1");
        assert_eq!(artifact.fixture_ids, vec!["b".to_string(), "a".to_string(), "c".to_string()]);
        assert_eq!(artifact.fixture_count, 3);

        assert_eq!(artifact.rows.len(), AVAILABLE_FACT_ROWS.len() + UNAVAILABLE_FACT_ROWS.len());
        for &row_name in AVAILABLE_FACT_ROWS {
            let row = artifact.rows.get(row_name).ok_or_else(|| color_eyre::eyre::eyre!("missing row"))?;
            assert_eq!(row.status, "available_zero");
            assert_eq!(row.total_facts, 0);
            assert_eq!(row.fixture_family_coverage, 2);
        }

        for &row_name in UNAVAILABLE_FACT_ROWS {
            let row = artifact.rows.get(row_name).ok_or_else(|| color_eyre::eyre::eyre!("missing row"))?;
            assert_eq!(row.status, "unavailable");
            assert_eq!(row.total_facts, 0);
            assert_eq!(row.fixture_family_coverage, 0);
        }

        Ok(())
    }

    #[test]
    fn manifest_load_sorts_fixtures() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(
            tmp.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"z","family":"f","path":"z.pl"},{"id":"a","family":"f","path":"a.pl"}]}"#,
        )?;
        let parsed = load_manifest(tmp.path())?;
        assert_eq!(parsed.fixtures[0].id, "a");
        assert_eq!(parsed.fixtures[1].id, "z");
        Ok(())
    }
}

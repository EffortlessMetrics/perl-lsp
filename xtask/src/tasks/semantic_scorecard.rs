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

const SUPPORTED_ROW_KEYS: &[&str] = &[
    "declaration_facts",
    "occurrence_facts",
    "export_facts",
    "definition_candidates",
    "reference_edges",
];

const UNAVAILABLE_ROW_KEYS: &[&str] = &[
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
    #[allow(dead_code)]
    path: String,
}

#[derive(Debug, Serialize, Default)]
struct FactCoverage {
    total: usize,
    exact: usize,
    high_confidence: usize,
    heuristic: usize,
    dynamic_boundary: usize,
}

#[derive(Debug, Serialize)]
struct Row {
    status: &'static str,
    available: bool,
    coverage: FactCoverage,
}

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    fixture_family_version: u32,
    fixture_count: usize,
    fixture_ids: Vec<String>,
    fixture_family_coverage: BTreeMap<String, usize>,
    rows: BTreeMap<String, Row>,
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
    let mut family_coverage = BTreeMap::new();
    for fixture in &manifest.fixtures {
        *family_coverage.entry(fixture.family.clone()).or_insert(0) += 1;
    }

    let mut rows = BTreeMap::new();
    for &metric in SUPPORTED_ROW_KEYS {
        rows.insert(
            metric.to_string(),
            Row {
                status: "adapter_pending",
                available: true,
                coverage: FactCoverage::default(),
            },
        );
    }
    for &metric in UNAVAILABLE_ROW_KEYS {
        rows.insert(
            metric.to_string(),
            Row {
                status: "unavailable",
                available: false,
                coverage: FactCoverage::default(),
            },
        );
    }

    Artifact {
        schema_version: 2,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        fixture_family_coverage: family_coverage,
        rows,
        notes: "Semantic fact adapters are optional in v1: supported rows emit deterministic zero coverage until adapters land; future-plan rows are explicitly marked unavailable.",
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

    text.push_str("\n## Fixture family coverage\n\n| Family | Fixture count |\n|---|---:|\n");
    for (family, count) in &artifact.fixture_family_coverage {
        text.push_str(&format!("| {family} | {count} |\n"));
    }

    text.push_str("\n## Rows\n\n");
    text.push_str("| Row | Status | Available | Total | Exact | High confidence | Heuristic | Dynamic boundary |\n");
    text.push_str("|---|---|---|---:|---:|---:|---:|---:|\n");
    for (row_key, row) in &artifact.rows {
        text.push_str(&format!(
            "| {row_key} | {} | {} | {} | {} | {} | {} | {} |\n",
            row.status,
            if row.available { "yes" } else { "no" },
            row.coverage.total,
            row.coverage.exact,
            row.coverage.high_confidence,
            row.coverage.heuristic,
            row.coverage.dynamic_boundary
        ));
    }

    text.push_str("\n");
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_artifact_emits_v1_rows_and_fixture_family_coverage() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase { id: "b".to_string(), family: "f2".to_string(), path: "b.pl".to_string() },
                FixtureCase { id: "a".to_string(), family: "f1".to_string(), path: "a.pl".to_string() },
                FixtureCase { id: "c".to_string(), family: "f2".to_string(), path: "c.pl".to_string() },
            ],
        };
        let artifact = build_artifact(manifest);

        assert_eq!(artifact.schema_version, 2);
        assert_eq!(artifact.fixture_ids, vec!["b", "a", "c"]);
        assert_eq!(artifact.fixture_family_coverage.get("f1"), Some(&1));
        assert_eq!(artifact.fixture_family_coverage.get("f2"), Some(&2));

        for key in SUPPORTED_ROW_KEYS {
            let row = artifact.rows.get(*key).ok_or_else(|| color_eyre::eyre::eyre!("missing row"))?;
            assert!(row.available);
            assert_eq!(row.status, "adapter_pending");
            assert_eq!(row.coverage.total, 0);
        }
        for key in UNAVAILABLE_ROW_KEYS {
            let row = artifact.rows.get(*key).ok_or_else(|| color_eyre::eyre::eyre!("missing row"))?;
            assert!(!row.available);
            assert_eq!(row.status, "unavailable");
            assert_eq!(row.coverage.high_confidence, 0);
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

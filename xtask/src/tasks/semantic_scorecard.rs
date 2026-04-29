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

const FEATURE_ROWS: &[(&str, bool)] = &[
    ("declaration_facts", true),
    ("occurrence_facts", true),
    ("export_facts", true),
    ("definition_candidates", true),
    ("reference_edges", true),
    ("import_specs", false),
    ("package_graph", false),
    ("rename_plan", false),
    ("safe_delete_plan", false),
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

#[derive(Debug, Serialize)]
struct CoverageBreakdown {
    exact_facts: usize,
    high_confidence_facts: usize,
    heuristic_facts: usize,
    dynamic_boundary_facts: usize,
}

#[derive(Debug, Serialize)]
struct ScorecardRow {
    available: bool,
    status: &'static str,
    fixture_family_coverage: BTreeMap<String, usize>,
    total_facts: usize,
    coverage: CoverageBreakdown,
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

pub fn run(
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    status_md: Option<PathBuf>,
) -> Result<()> {
    let root = project_root()?;
    let manifest_path =
        root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
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
    let fixture_ids =
        manifest.fixtures.iter().map(|fixture| fixture.id.clone()).collect::<Vec<_>>();
    let mut families = BTreeMap::<String, usize>::new();
    for fixture in &manifest.fixtures {
        *families.entry(fixture.family.clone()).or_default() += 1;
    }

    let mut rows = BTreeMap::new();
    for &(row_name, available) in FEATURE_ROWS {
        rows.insert(
            row_name.to_string(),
            ScorecardRow {
                available,
                status: if available { "adapter_missing_or_empty" } else { "unavailable" },
                fixture_family_coverage: if available { families.clone() } else { BTreeMap::new() },
                total_facts: 0,
                coverage: CoverageBreakdown {
                    exact_facts: 0,
                    high_confidence_facts: 0,
                    heuristic_facts: 0,
                    dynamic_boundary_facts: 0,
                },
            },
        );
    }

    Artifact {
        schema_version: 1,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        rows,
        notes: "Wave 2 scorecard shape is live. Rows remain deterministic when adapters are missing by emitting unavailable/empty counts.",
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

    text.push_str(
        "\n## Semantic Fact Coverage\n\n| Row | Availability | Status | Total facts | exact | high-confidence | heuristic | dynamic-boundary |\n|---|---|---|---:|---:|---:|---:|---:|\n",
    );
    for (row_name, row) in &artifact.rows {
        let availability = if row.available { "available" } else { "unavailable" };
        text.push_str(&format!(
            "| {row_name} | {availability} | {} | {} | {} | {} | {} | {} |\n",
            row.status,
            row.total_facts,
            row.coverage.exact_facts,
            row.coverage.high_confidence_facts,
            row.coverage.heuristic_facts,
            row.coverage.dynamic_boundary_facts
        ));
    }

    text.push_str("\n## Fixture-family coverage\n\n");
    for (row_name, row) in &artifact.rows {
        if !row.available {
            continue;
        }
        text.push_str(&format!("### `{row_name}`\n"));
        if row.fixture_family_coverage.is_empty() {
            text.push_str("- no fixtures counted\n");
        } else {
            for (family, count) in &row.fixture_family_coverage {
                text.push_str(&format!("- `{family}`: {count}\n"));
            }
        }
        text.push('\n');
    }

    text.push_str("\n");
    text.push_str(artifact.notes);
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify build_artifact preserves insertion order of fixture IDs and emits
    /// the semantic scorecard rows with deterministic empty counts.
    #[test]
    fn build_artifact_preserves_fixture_order_and_emits_all_metrics() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase {
                    id: "b".to_string(),
                    family: "x".to_string(),
                    path: "b.pl".to_string(),
                },
                FixtureCase {
                    id: "a".to_string(),
                    family: "x".to_string(),
                    path: "a.pl".to_string(),
                },
            ],
        };
        let artifact = build_artifact(manifest);

        // Fixture IDs are preserved in input order (sorting is load_manifest's job).
        assert_eq!(artifact.measured_at, "deterministic-fixture-baseline");
        assert_eq!(artifact.fixture_ids, vec!["b".to_string(), "a".to_string()]);
        assert_eq!(artifact.fixture_count, 2);

        assert_eq!(artifact.rows.len(), FEATURE_ROWS.len(), "row count must match FEATURE_ROWS");
        for &(row_name, available) in FEATURE_ROWS {
            let row = artifact.rows.get(row_name).ok_or_else(|| {
                color_eyre::eyre::eyre!("expected row '{row_name}' to be present")
            })?;
            assert_eq!(row.available, available, "availability mismatch for '{row_name}'");
            assert_eq!(
                row.status,
                if available { "adapter_missing_or_empty" } else { "unavailable" },
                "unexpected status for '{row_name}'"
            );
            assert_eq!(row.total_facts, 0, "row '{row_name}' should default to zero facts");
        }

        // Rows are in alphabetical order (BTreeMap) — spot-check boundary keys.
        let keys: Vec<&str> = artifact.rows.keys().map(String::as_str).collect();
        assert_eq!(keys.first().copied(), Some("declaration_facts"));
        assert_eq!(keys.last().copied(), Some("safe_delete_plan"));

        Ok(())
    }

    /// Verify load_manifest sorts fixtures by id, making the pipeline deterministic
    /// regardless of the order fixtures appear in the JSON file.
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

    /// Verify that the full pipeline (load_manifest -> build_artifact) is stable
    /// across two calls with the same manifest data in a different order: the
    /// fixture IDs in the artifact must be identical both times.
    #[test]
    fn full_pipeline_is_stable_across_orderings() -> Result<()> {
        let tmp_fwd = tempfile::NamedTempFile::new()?;
        let tmp_rev = tempfile::NamedTempFile::new()?;
        // Same two fixtures, different JSON order.
        fs::write(
            tmp_fwd.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"alpha","family":"f","path":"a.pl"},{"id":"beta","family":"f","path":"b.pl"}]}"#,
        )?;
        fs::write(
            tmp_rev.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"beta","family":"f","path":"b.pl"},{"id":"alpha","family":"f","path":"a.pl"}]}"#,
        )?;

        let artifact_fwd = build_artifact(load_manifest(tmp_fwd.path())?);
        let artifact_rev = build_artifact(load_manifest(tmp_rev.path())?);

        // Both should produce the same sorted fixture list.
        assert_eq!(
            artifact_fwd.fixture_ids, artifact_rev.fixture_ids,
            "fixture IDs must be identical regardless of input order"
        );
        assert_eq!(artifact_fwd.fixture_ids, vec!["alpha".to_string(), "beta".to_string()]);
        Ok(())
    }
}

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FIXTURE_MANIFEST: &str =
    "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/manifest.json";
const DEFAULT_OUTPUT: &str = "target/receipts/metrics/semantic_scorecard.json";
const DEFAULT_STATUS_MD: &str = "docs/project/status/semantic_scorecard.md";

const AVAILABLE_ROWS: &[&str] = &[
    "declaration_facts",
    "occurrence_facts",
    "export_facts",
    "definition_candidates",
    "reference_edges",
];

const UNAVAILABLE_ROWS: &[&str] =
    &["import_specs", "package_graph", "rename_plan", "safe_delete_plan"];

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
struct FactConfidenceBreakdown {
    exact_facts: usize,
    high_confidence_facts: usize,
    heuristic_facts: usize,
    dynamic_boundary_facts: usize,
}

#[derive(Debug, Serialize)]
struct FactRow {
    status: &'static str,
    total_facts: usize,
    fixture_coverage: FixtureCoverage,
    confidence_breakdown: FactConfidenceBreakdown,
}

#[derive(Debug, Serialize)]
struct UnavailableRow {
    status: &'static str,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct FixtureCoverage {
    covered_fixture_count: usize,
    total_fixture_count: usize,
    covered_fixture_families: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    schema_version: u32,
    measured_at: &'static str,
    subsystem: &'static str,
    fixture_family_version: u32,
    fixture_count: usize,
    fixture_ids: Vec<String>,
    fixture_families: Vec<String>,
    fact_rows: BTreeMap<String, FactRow>,
    unavailable_rows: BTreeMap<String, UnavailableRow>,
    notes: &'static str,
}

pub fn run(
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    status_md: Option<PathBuf>,
    check: bool,
) -> Result<()> {
    let root = project_root()?;
    let manifest_path =
        root.join(manifest.unwrap_or_else(|| PathBuf::from(DEFAULT_FIXTURE_MANIFEST)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));
    let status_path = root.join(status_md.unwrap_or_else(|| PathBuf::from(DEFAULT_STATUS_MD)));

    let manifest = load_manifest(&manifest_path)?;
    let artifact = build_artifact(manifest);

    let payload = serialize_json(&artifact)?;
    let status_markdown = render_status_markdown(&artifact);

    if check {
        verify_file_matches(&output_path, &payload)?;
        verify_file_matches(&status_path, &status_markdown)?;
        println!("semantic scorecard check passed: outputs are current");
        return Ok(());
    }

    write_json(&output_path, &payload)?;
    if let Some(parent) = status_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&status_path, status_markdown)
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
    let fixture_families =
        manifest.fixtures.iter().map(|fixture| fixture.family.clone()).collect::<Vec<_>>();

    let coverage = FixtureCoverage {
        covered_fixture_count: fixture_ids.len(),
        total_fixture_count: fixture_ids.len(),
        covered_fixture_families: fixture_families.clone(),
    };

    let mut fact_rows = BTreeMap::new();
    for &row in AVAILABLE_ROWS {
        fact_rows.insert(
            row.to_string(),
            FactRow {
                status: "adapter_missing",
                total_facts: 0,
                fixture_coverage: FixtureCoverage {
                    covered_fixture_count: coverage.covered_fixture_count,
                    total_fixture_count: coverage.total_fixture_count,
                    covered_fixture_families: coverage.covered_fixture_families.clone(),
                },
                confidence_breakdown: FactConfidenceBreakdown {
                    exact_facts: 0,
                    high_confidence_facts: 0,
                    heuristic_facts: 0,
                    dynamic_boundary_facts: 0,
                },
            },
        );
    }

    let mut unavailable_rows = BTreeMap::new();
    for &row in UNAVAILABLE_ROWS {
        unavailable_rows.insert(
            row.to_string(),
            UnavailableRow { status: "unavailable", reason: "planned for future scorecard waves" },
        );
    }

    Artifact {
        schema_version: 2,
        measured_at: "deterministic-fixture-baseline",
        subsystem: "semantic",
        fixture_family_version: manifest.fixture_family_version,
        fixture_count: fixture_ids.len(),
        fixture_ids,
        fixture_families,
        fact_rows,
        unavailable_rows,
        notes: "Wave 2 shape: fact rows are deterministic and stay useful when adapters are unavailable.",
    }
}

fn serialize_json(artifact: &Artifact) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(artifact)?))
}

fn write_json(path: &Path, payload: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", path.display()))?;
    }
    fs::write(path, payload).with_context(|| format!("writing {}", path.display()))
}

fn verify_file_matches(path: &Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if actual != expected {
        bail!(
            "{} is stale; run `cargo xtask semantic-scorecard` to refresh generated outputs",
            path.display()
        );
    }
    Ok(())
}

fn render_status_markdown(artifact: &Artifact) -> String {
    let mut text = String::new();
    text.push_str("# Semantic Scorecard\n\n");
    text.push_str(&format!("Measured: `{}`  \n", artifact.measured_at));
    text.push_str(&format!("Fixture family version: `{}`  \n", artifact.fixture_family_version));
    text.push_str(&format!("Fixtures loaded: `{}`\n\n", artifact.fixture_count));

    text.push_str("## Fact Coverage\n\n");
    text.push_str(
        "| Row | Status | Facts | Coverage | Exact | High | Heuristic | Dynamic boundary |\n",
    );
    text.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for (row_name, row) in &artifact.fact_rows {
        text.push_str(&format!(
            "| {row_name} | {} | {} | {}/{} | {} | {} | {} | {} |\n",
            row.status,
            row.total_facts,
            row.fixture_coverage.covered_fixture_count,
            row.fixture_coverage.total_fixture_count,
            row.confidence_breakdown.exact_facts,
            row.confidence_breakdown.high_confidence_facts,
            row.confidence_breakdown.heuristic_facts,
            row.confidence_breakdown.dynamic_boundary_facts
        ));
    }

    text.push_str("\n## Unavailable Rows\n\n| Row | Status | Reason |\n|---|---|---|\n");
    for (row_name, row) in &artifact.unavailable_rows {
        text.push_str(&format!("| {row_name} | {} | {} |\n", row.status, row.reason));
    }

    text.push_str("\n## Fixture IDs\n\n");
    for id in &artifact.fixture_ids {
        text.push_str(&format!("- `{id}`\n"));
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
    fn build_artifact_emits_wave2_row_shape() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![
                FixtureCase {
                    id: "b".to_string(),
                    family: "family b".to_string(),
                    path: "b.pl".to_string(),
                },
                FixtureCase {
                    id: "a".to_string(),
                    family: "family a".to_string(),
                    path: "a.pl".to_string(),
                },
            ],
        };
        let artifact = build_artifact(manifest);

        assert_eq!(artifact.schema_version, 2);
        assert_eq!(artifact.fixture_count, 2);
        assert_eq!(artifact.fixture_ids, vec!["b".to_string(), "a".to_string()]);

        assert_eq!(artifact.fact_rows.len(), AVAILABLE_ROWS.len());
        for row_name in AVAILABLE_ROWS {
            let row = artifact.fact_rows.get(*row_name).expect("row should exist");
            assert_eq!(row.status, "adapter_missing");
            assert_eq!(row.total_facts, 0);
            assert_eq!(row.fixture_coverage.covered_fixture_count, 2);
            assert_eq!(row.fixture_coverage.total_fixture_count, 2);
            assert_eq!(row.confidence_breakdown.exact_facts, 0);
            assert_eq!(row.confidence_breakdown.high_confidence_facts, 0);
            assert_eq!(row.confidence_breakdown.heuristic_facts, 0);
            assert_eq!(row.confidence_breakdown.dynamic_boundary_facts, 0);
        }

        assert_eq!(artifact.unavailable_rows.len(), UNAVAILABLE_ROWS.len());
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

    #[test]
    fn scorecard_json_includes_wave2_top_level_keys() -> Result<()> {
        let manifest = SemanticManifest {
            fixture_family_version: 1,
            fixtures: vec![FixtureCase {
                id: "fixture_a".to_string(),
                family: "family a".to_string(),
                path: "a.pl".to_string(),
            }],
        };

        let artifact = build_artifact(manifest);
        let value: serde_json::Value = serde_json::to_value(&artifact)?;

        assert!(value.get("fact_rows").is_some(), "fact_rows should exist");
        assert!(value.get("unavailable_rows").is_some(), "unavailable_rows should exist");
        assert!(value.get("fixture_families").is_some(), "fixture_families should exist");
        Ok(())
    }

    /// Verify that the full pipeline (load_manifest -> build_artifact) is stable
    /// across two calls with the same manifest data in a different order: fixture IDs
    /// and fixture_families must be identical and sorted regardless of JSON input order.
    #[test]
    fn full_pipeline_is_stable_across_orderings() -> Result<()> {
        let tmp_fwd = tempfile::NamedTempFile::new()?;
        let tmp_rev = tempfile::NamedTempFile::new()?;
        // Same two fixtures, different JSON order.
        fs::write(
            tmp_fwd.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"alpha","family":"family alpha","path":"a.pl"},{"id":"beta","family":"family beta","path":"b.pl"}]}"#,
        )?;
        fs::write(
            tmp_rev.path(),
            r#"{"fixture_family_version":1,"fixtures":[{"id":"beta","family":"family beta","path":"b.pl"},{"id":"alpha","family":"family alpha","path":"a.pl"}]}"#,
        )?;

        let artifact_fwd = build_artifact(load_manifest(tmp_fwd.path())?);
        let artifact_rev = build_artifact(load_manifest(tmp_rev.path())?);

        // Fixture IDs must be identical and sorted regardless of input order.
        assert_eq!(
            artifact_fwd.fixture_ids, artifact_rev.fixture_ids,
            "fixture IDs must be identical regardless of input order"
        );
        assert_eq!(artifact_fwd.fixture_ids, vec!["alpha".to_string(), "beta".to_string()]);

        // fixture_families must be co-indexed with fixture_ids (same sort order).
        assert_eq!(
            artifact_fwd.fixture_families, artifact_rev.fixture_families,
            "fixture_families must be identical regardless of input order"
        );
        assert_eq!(
            artifact_fwd.fixture_families,
            vec!["family alpha".to_string(), "family beta".to_string()]
        );
        Ok(())
    }

    #[test]
    fn verify_file_matches_detects_drift() -> Result<()> {
        let tmp = tempfile::NamedTempFile::new()?;
        fs::write(tmp.path(), "actual\n")?;
        let err = verify_file_matches(tmp.path(), "expected\n").expect_err("must fail on drift");
        assert!(err.to_string().contains("is stale"));
        Ok(())
    }
}

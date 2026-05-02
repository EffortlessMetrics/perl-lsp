//! Parser accuracy scorecard contract and denominator inventory.
//!
//! The first implementation slice deliberately emits denominator rows and
//! insufficient-data metric rows only. Accuracy scoring layers are added in
//! later slices once their gold fixtures and extractors exist.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const DEFAULT_MANIFEST: &str = "crates/perl-corpus/fixtures/parser_accuracy/manifest.json";
const DEFAULT_OUTPUT: &str = "target/metrics/parser_accuracy.json";

#[derive(Debug, Clone, Deserialize)]
struct ParserAccuracyManifest {
    schema_version: u32,
    fixtures: Vec<FixtureMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureMetadata {
    id: String,
    family: String,
    label_mode: LabelMode,
    source_path: String,
    scored_lines: u64,
    scored_symbols: u64,
    fully_labeled_regions: u64,
    partial_labeled_regions: u64,
    unknown_regions: u64,
    negative_regions: u64,
    dynamic_boundaries: u64,
    unsupported_constructs: u64,
    real_project_file: bool,
    generated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LabelMode {
    Full,
    Partial,
    Unknown,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParserAccuracyArtifact {
    schema_version: u32,
    subsystem: &'static str,
    generated_at: String,
    commit: String,
    cadence: Cadence,
    denominator: Denominator,
    families: Vec<FamilySummary>,
    metrics: Vec<MetricRow>,
    failure_packets: Vec<FailurePacket>,
    gold_drift: GoldDrift,
    metric_runtime: MetricRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cadence {
    Pr,
    MergeGate,
    Nightly,
    Release,
}

impl Cadence {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "pr" => Ok(Self::Pr),
            "merge_gate" => Ok(Self::MergeGate),
            "nightly" => Ok(Self::Nightly),
            "release" => Ok(Self::Release),
            other => bail!("unsupported parser accuracy cadence '{other}'"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Denominator {
    fixture_count: u64,
    fixture_family_count: u64,
    scored_line_count: u64,
    scored_symbol_count: u64,
    fully_labeled_region_count: u64,
    partial_labeled_region_count: u64,
    unknown_region_count: u64,
    negative_region_count: u64,
    dynamic_boundary_case_count: u64,
    unsupported_construct_case_count: u64,
    real_project_file_count: u64,
    generated_fixture_count: u64,
    hand_labeled_fixture_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FamilySummary {
    family: String,
    fixture_count: u64,
    label_modes: Vec<LabelMode>,
    scored_line_count: u64,
    scored_symbol_count: u64,
    dynamic_boundary_case_count: u64,
    unsupported_construct_case_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum MetricRow {
    Measured {
        metric: String,
        value: f64,
        sample_count: u64,
        direction: Direction,
        confidence: Confidence,
        cadence: Cadence,
    },
    InsufficientData {
        metric: String,
        reason: String,
        sample_count: u64,
        confidence: Confidence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Confidence {
    High,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FailurePacket {
    failure_kind: String,
    likely_layer: String,
    fixture_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct GoldDrift {
    schema_error_count: u64,
    span_error_count: u64,
    duplicate_symbol_id_count: u64,
    missing_resolves_to_target_count: u64,
    changed_line_count: u64,
    changed_symbol_count: u64,
    removed_expectation_count: u64,
    added_expectation_count: u64,
    dynamic_expectation_change_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct MetricRuntime {
    runtime_ms: f64,
    timeout_count: u64,
    flake_count: u64,
    artifact_size_bytes: u64,
    ci_runner_failure_count: u64,
    orphan_process_count: u64,
    cache_hit_rate: Option<f64>,
}

/// Run `cargo xtask metrics parser-accuracy`.
pub fn run(
    json: bool,
    check: bool,
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    cadence: &str,
) -> Result<()> {
    let root = project_root()?;
    let cadence = Cadence::parse(cadence)?;
    let manifest_path = manifest.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let output_path = output.unwrap_or_else(|| root.join(DEFAULT_OUTPUT));
    let start = Instant::now();

    let manifest = read_manifest(&root, &manifest_path)?;
    let mut artifact = build_artifact(&root, &manifest, cadence)?;
    artifact.metric_runtime.runtime_ms = start.elapsed().as_secs_f64() * 1000.0;
    settle_artifact_size(&mut artifact)?;

    if check {
        validate_artifact_contract(&artifact)?;
        println!(
            "parser accuracy artifact check passed: {} fixtures across {} families",
            artifact.denominator.fixture_count, artifact.denominator.fixture_family_count
        );
        return Ok(());
    }

    if json {
        write_artifact(&output_path, &artifact)?;
        println!("parser accuracy artifact written: {}", output_path.display());
    } else {
        print_summary(&artifact);
    }

    Ok(())
}

fn read_manifest(root: &Path, path: &Path) -> Result<ParserAccuracyManifest> {
    let manifest_path = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading parser accuracy manifest {}", manifest_path.display()))?;
    let manifest: ParserAccuracyManifest = serde_json::from_str(&raw)
        .with_context(|| format!("parsing parser accuracy manifest {}", manifest_path.display()))?;
    if manifest.schema_version != 1 {
        bail!("unsupported parser accuracy manifest schema_version {}", manifest.schema_version);
    }
    for fixture in &manifest.fixtures {
        let source_path = root.join(&fixture.source_path);
        if !source_path.exists() {
            bail!(
                "parser accuracy fixture '{}' source does not exist: {}",
                fixture.id,
                source_path.display()
            );
        }
    }
    Ok(manifest)
}

fn build_artifact(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    cadence: Cadence,
) -> Result<ParserAccuracyArtifact> {
    let denominator = compute_denominator(manifest);
    let families = summarize_families(manifest);
    let fixture_count = denominator.fixture_count as f64;
    let metrics = vec![
        MetricRow::Measured {
            metric: "denominator_fixture_count".to_string(),
            value: fixture_count,
            sample_count: denominator.fixture_count,
            direction: Direction::Neutral,
            confidence: Confidence::High,
            cadence,
        },
        insufficient("line_construct_f1", "line-level gold scorer is not wired yet"),
        insufficient("ast_node_kind_f1", "AST structural gold scorer is not wired yet"),
        insufficient("symbol_decl_f1", "symbol gold scorer is not wired yet"),
    ];

    Ok(ParserAccuracyArtifact {
        schema_version: 1,
        subsystem: "parser_accuracy",
        generated_at: Utc::now().to_rfc3339(),
        commit: git_commit(root),
        cadence,
        denominator,
        families,
        metrics,
        failure_packets: Vec::new(),
        gold_drift: GoldDrift::default(),
        metric_runtime: MetricRuntime::default(),
    })
}

fn compute_denominator(manifest: &ParserAccuracyManifest) -> Denominator {
    let mut families = BTreeSet::new();
    let mut denominator =
        Denominator { fixture_count: manifest.fixtures.len() as u64, ..Denominator::default() };

    for fixture in &manifest.fixtures {
        families.insert(fixture.family.clone());
        denominator.scored_line_count += fixture.scored_lines;
        denominator.scored_symbol_count += fixture.scored_symbols;
        denominator.fully_labeled_region_count += fixture.fully_labeled_regions;
        denominator.partial_labeled_region_count += fixture.partial_labeled_regions;
        denominator.unknown_region_count += fixture.unknown_regions;
        denominator.negative_region_count += fixture.negative_regions;
        denominator.dynamic_boundary_case_count += fixture.dynamic_boundaries;
        denominator.unsupported_construct_case_count += fixture.unsupported_constructs;
        if fixture.real_project_file {
            denominator.real_project_file_count += 1;
        }
        if fixture.generated {
            denominator.generated_fixture_count += 1;
        } else {
            denominator.hand_labeled_fixture_count += 1;
        }
    }

    denominator.fixture_family_count = families.len() as u64;
    denominator
}

fn summarize_families(manifest: &ParserAccuracyManifest) -> Vec<FamilySummary> {
    #[derive(Default)]
    struct Accumulator {
        fixture_count: u64,
        label_modes: BTreeSet<LabelMode>,
        scored_line_count: u64,
        scored_symbol_count: u64,
        dynamic_boundary_case_count: u64,
        unsupported_construct_case_count: u64,
    }

    let mut by_family = BTreeMap::<String, Accumulator>::new();
    for fixture in &manifest.fixtures {
        let entry = by_family.entry(fixture.family.clone()).or_default();
        entry.fixture_count += 1;
        entry.label_modes.insert(fixture.label_mode);
        entry.scored_line_count += fixture.scored_lines;
        entry.scored_symbol_count += fixture.scored_symbols;
        entry.dynamic_boundary_case_count += fixture.dynamic_boundaries;
        entry.unsupported_construct_case_count += fixture.unsupported_constructs;
    }

    by_family
        .into_iter()
        .map(|(family, entry)| FamilySummary {
            family,
            fixture_count: entry.fixture_count,
            label_modes: entry.label_modes.into_iter().collect(),
            scored_line_count: entry.scored_line_count,
            scored_symbol_count: entry.scored_symbol_count,
            dynamic_boundary_case_count: entry.dynamic_boundary_case_count,
            unsupported_construct_case_count: entry.unsupported_construct_case_count,
        })
        .collect()
}

fn insufficient(metric: &str, reason: &str) -> MetricRow {
    MetricRow::InsufficientData {
        metric: metric.to_string(),
        reason: reason.to_string(),
        sample_count: 0,
        confidence: Confidence::Low,
    }
}

fn validate_artifact_contract(artifact: &ParserAccuracyArtifact) -> Result<()> {
    if artifact.schema_version != 1 {
        bail!("parser accuracy artifact schema_version must be 1");
    }
    if artifact.subsystem != "parser_accuracy" {
        bail!("parser accuracy artifact subsystem must be parser_accuracy");
    }
    if artifact.denominator.fixture_count == 0 {
        bail!("parser accuracy artifact denominator has no fixtures");
    }
    if artifact.families.is_empty() {
        bail!("parser accuracy artifact has no fixture families");
    }
    if artifact.metrics.is_empty() {
        bail!("parser accuracy artifact has no metric rows");
    }
    for metric in &artifact.metrics {
        match metric {
            MetricRow::Measured { sample_count, .. } if *sample_count == 0 => {
                bail!("measured parser accuracy metric has zero sample_count")
            }
            MetricRow::Measured { .. } | MetricRow::InsufficientData { .. } => {}
        }
    }
    Ok(())
}

fn write_artifact(path: &Path, artifact: &ParserAccuracyArtifact) -> Result<()> {
    let parent = path.parent().ok_or_else(|| eyre!("parser accuracy output path has no parent"))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating parser accuracy output dir {}", parent.display()))?;
    let json = render_artifact(artifact)?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn render_artifact(artifact: &ParserAccuracyArtifact) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(artifact)?))
}

fn settle_artifact_size(artifact: &mut ParserAccuracyArtifact) -> Result<()> {
    for _ in 0..4 {
        let size = render_artifact(artifact)?.len() as u64;
        if artifact.metric_runtime.artifact_size_bytes == size {
            return Ok(());
        }
        artifact.metric_runtime.artifact_size_bytes = size;
    }
    Ok(())
}

fn print_summary(artifact: &ParserAccuracyArtifact) {
    println!("Parser accuracy denominator");
    println!("  fixtures: {}", artifact.denominator.fixture_count);
    println!("  families: {}", artifact.denominator.fixture_family_count);
    println!("  scored lines: {}", artifact.denominator.scored_line_count);
    println!("  scored symbols: {}", artifact.denominator.scored_symbol_count);
    println!("  dynamic boundary cases: {}", artifact.denominator.dynamic_boundary_case_count);
}

fn git_commit(root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest() -> ParserAccuracyManifest {
        ParserAccuracyManifest {
            schema_version: 1,
            fixtures: vec![
                FixtureMetadata {
                    id: "package_basic".to_string(),
                    family: "packages".to_string(),
                    label_mode: LabelMode::Full,
                    source_path: "package_basic.pl".to_string(),
                    scored_lines: 2,
                    scored_symbols: 1,
                    fully_labeled_regions: 1,
                    partial_labeled_regions: 0,
                    unknown_regions: 0,
                    negative_regions: 1,
                    dynamic_boundaries: 0,
                    unsupported_constructs: 0,
                    real_project_file: false,
                    generated: false,
                },
                FixtureMetadata {
                    id: "dynamic_require_boundary".to_string(),
                    family: "dynamic_require".to_string(),
                    label_mode: LabelMode::Partial,
                    source_path: "dynamic_require.pl".to_string(),
                    scored_lines: 1,
                    scored_symbols: 1,
                    fully_labeled_regions: 0,
                    partial_labeled_regions: 1,
                    unknown_regions: 1,
                    negative_regions: 0,
                    dynamic_boundaries: 1,
                    unsupported_constructs: 0,
                    real_project_file: false,
                    generated: false,
                },
            ],
        }
    }

    #[test]
    fn denominator_counts_manifest_inventory() {
        let denominator = compute_denominator(&fixture_manifest());
        assert_eq!(denominator.fixture_count, 2);
        assert_eq!(denominator.fixture_family_count, 2);
        assert_eq!(denominator.scored_line_count, 3);
        assert_eq!(denominator.scored_symbol_count, 2);
        assert_eq!(denominator.unknown_region_count, 1);
        assert_eq!(denominator.negative_region_count, 1);
        assert_eq!(denominator.dynamic_boundary_case_count, 1);
        assert_eq!(denominator.hand_labeled_fixture_count, 2);
    }

    #[test]
    fn family_summary_groups_label_modes() -> Result<()> {
        let families = summarize_families(&fixture_manifest());
        assert_eq!(families.len(), 2);
        let dynamic = families
            .iter()
            .find(|family| family.family == "dynamic_require")
            .ok_or_else(|| color_eyre::eyre::eyre!("dynamic family should exist"))?;
        assert_eq!(dynamic.fixture_count, 1);
        assert_eq!(dynamic.label_modes, vec![LabelMode::Partial]);
        assert_eq!(dynamic.dynamic_boundary_case_count, 1);
        Ok(())
    }

    #[test]
    fn artifact_uses_insufficient_data_for_unwired_scorers() -> Result<()> {
        let root = Path::new(".");
        let artifact = build_artifact(root, &fixture_manifest(), Cadence::Pr)?;
        validate_artifact_contract(&artifact)?;
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "line_construct_f1"
            )
        }));
        Ok(())
    }
}

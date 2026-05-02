//! Parser accuracy scorecard contract and denominator inventory.
//!
//! The implementation starts with denominator rows and then adds accuracy
//! scoring layers in small, schema-valid slices.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail, eyre};
use perl_parser::{Node, NodeKind, Parser};
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
    #[serde(default)]
    line_expectations: Vec<LineExpectation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LabelMode {
    Full,
    Partial,
    Unknown,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LineExpectation {
    line: u64,
    expected_tags: BTreeSet<LineTag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LineTag {
    PackageDecl,
    SubDecl,
    MethodDecl,
    VariableDecl,
    Import,
    Export,
    FunctionCall,
    MethodCall,
    Regex,
    QuoteLike,
    HeredocOpener,
    HeredocBody,
    HeredocTerminator,
    Pod,
    FormatDecl,
    GivenWhen,
    DoWhile,
    UntilLoop,
    DynamicBoundary,
    ParseError,
    RecoveryRegion,
    UnsupportedConstruct,
}

const LINE_TAG_VOCABULARY: &[LineTag] = &[
    LineTag::PackageDecl,
    LineTag::SubDecl,
    LineTag::MethodDecl,
    LineTag::VariableDecl,
    LineTag::Import,
    LineTag::Export,
    LineTag::FunctionCall,
    LineTag::MethodCall,
    LineTag::Regex,
    LineTag::QuoteLike,
    LineTag::HeredocOpener,
    LineTag::HeredocBody,
    LineTag::HeredocTerminator,
    LineTag::Pod,
    LineTag::FormatDecl,
    LineTag::GivenWhen,
    LineTag::DoWhile,
    LineTag::UntilLoop,
    LineTag::DynamicBoundary,
    LineTag::ParseError,
    LineTag::RecoveryRegion,
    LineTag::UnsupportedConstruct,
];

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LineScore {
    line_count: u64,
    true_positive_count: u64,
    false_positive_count: u64,
    false_negative_count: u64,
    exact_match_count: u64,
    expected_parse_error_count: u64,
    false_parse_error_count: u64,
    missed_parse_error_count: u64,
    expected_dynamic_boundary_count: u64,
    correct_dynamic_boundary_count: u64,
    expected_unsupported_construct_count: u64,
    correct_unsupported_construct_count: u64,
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
    let line_score = score_manifest_line_tags(root, manifest)?;
    let mut metrics = vec![MetricRow::Measured {
        metric: "denominator_fixture_count".to_string(),
        value: fixture_count,
        sample_count: denominator.fixture_count,
        direction: Direction::Neutral,
        confidence: Confidence::High,
        cadence,
    }];
    metrics.extend(line_metrics(&line_score, cadence));
    metrics.extend([
        insufficient("ast_node_kind_f1", "AST structural gold scorer is not wired yet"),
        insufficient("symbol_decl_f1", "symbol gold scorer is not wired yet"),
    ]);

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

fn score_manifest_line_tags(root: &Path, manifest: &ParserAccuracyManifest) -> Result<LineScore> {
    let mut score = LineScore::default();
    for fixture in &manifest.fixtures {
        if fixture.line_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = fs::read_to_string(&source_path).with_context(|| {
            format!("reading parser accuracy fixture source {}", source_path.display())
        })?;
        let actual_by_line = extract_line_tags(&source);
        for expectation in &fixture.line_expectations {
            let actual = actual_by_line.get(&expectation.line).cloned().unwrap_or_default();
            score_line_tags(&expectation.expected_tags, &actual, &mut score);
        }
    }
    Ok(score)
}

fn extract_line_tags(source: &str) -> BTreeMap<u64, BTreeSet<LineTag>> {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let line_starts = line_starts(source);
    let mut by_line = BTreeMap::new();
    collect_node_line_tags(&output.ast, &line_starts, &mut by_line);
    by_line
}

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' && index + 1 < source.len() {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_for_offset(line_starts: &[usize], offset: usize) -> u64 {
    match line_starts.binary_search(&offset) {
        Ok(index) => (index + 1) as u64,
        Err(0) => 1,
        Err(index) => index as u64,
    }
}

fn collect_node_line_tags(
    node: &Node,
    line_starts: &[usize],
    by_line: &mut BTreeMap<u64, BTreeSet<LineTag>>,
) {
    if let Some(tag) = line_tag_for_node(node) {
        let line = line_for_offset(line_starts, node.location.start);
        by_line.entry(line).or_default().insert(tag);
    }
    if let NodeKind::FunctionCall { name, args } = &node.kind
        && name == "require"
        && args.first().is_some_and(|arg| matches!(arg.kind, NodeKind::Variable { .. }))
    {
        let line = line_for_offset(line_starts, node.location.start);
        by_line.entry(line).or_default().insert(LineTag::DynamicBoundary);
    }
    node.for_each_child(|child| collect_node_line_tags(child, line_starts, by_line));
}

fn line_tag_for_node(node: &Node) -> Option<LineTag> {
    match &node.kind {
        NodeKind::Package { .. } => Some(LineTag::PackageDecl),
        NodeKind::Subroutine { .. } => Some(LineTag::SubDecl),
        NodeKind::Method { .. } => Some(LineTag::MethodDecl),
        NodeKind::VariableDeclaration { .. } | NodeKind::VariableListDeclaration { .. } => {
            Some(LineTag::VariableDecl)
        }
        NodeKind::Use { .. } => Some(LineTag::Import),
        NodeKind::FunctionCall { name, .. } if name == "require" => Some(LineTag::Import),
        NodeKind::FunctionCall { .. } => Some(LineTag::FunctionCall),
        NodeKind::MethodCall { .. } => Some(LineTag::MethodCall),
        NodeKind::Regex { .. }
        | NodeKind::Match { .. }
        | NodeKind::Substitution { .. }
        | NodeKind::Transliteration { .. } => Some(LineTag::Regex),
        NodeKind::Heredoc { .. } => Some(LineTag::HeredocOpener),
        NodeKind::Format { .. } => Some(LineTag::FormatDecl),
        NodeKind::Given { .. } | NodeKind::When { .. } | NodeKind::Default { .. } => {
            Some(LineTag::GivenWhen)
        }
        NodeKind::Do { .. } => Some(LineTag::DoWhile),
        NodeKind::Error { .. } => Some(LineTag::ParseError),
        NodeKind::UnknownRest => Some(LineTag::UnsupportedConstruct),
        _ => None,
    }
}

fn score_line_tags(
    expected: &BTreeSet<LineTag>,
    actual: &BTreeSet<LineTag>,
    score: &mut LineScore,
) {
    score.line_count += 1;
    let true_positives = expected.intersection(actual).count() as u64;
    let false_positives = actual.difference(expected).count() as u64;
    let false_negatives = expected.difference(actual).count() as u64;
    score.true_positive_count += true_positives;
    score.false_positive_count += false_positives;
    score.false_negative_count += false_negatives;
    if expected == actual {
        score.exact_match_count += 1;
    }

    let expected_parse_error = expected.contains(&LineTag::ParseError);
    let actual_parse_error = actual.contains(&LineTag::ParseError);
    if expected_parse_error {
        score.expected_parse_error_count += 1;
    }
    if actual_parse_error && !expected_parse_error {
        score.false_parse_error_count += 1;
    }
    if expected_parse_error && !actual_parse_error {
        score.missed_parse_error_count += 1;
    }

    if expected.contains(&LineTag::DynamicBoundary) {
        score.expected_dynamic_boundary_count += 1;
        if actual.contains(&LineTag::DynamicBoundary) {
            score.correct_dynamic_boundary_count += 1;
        }
    }

    if expected.contains(&LineTag::UnsupportedConstruct) {
        score.expected_unsupported_construct_count += 1;
        if actual.contains(&LineTag::UnsupportedConstruct) {
            score.correct_unsupported_construct_count += 1;
        }
    }
}

fn line_metrics(score: &LineScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.line_count == 0 {
        return vec![insufficient("line_construct_f1", "line-level gold labels are not available")];
    }

    let precision_denominator = score.true_positive_count + score.false_positive_count;
    let recall_denominator = score.true_positive_count + score.false_negative_count;
    let precision = ratio(score.true_positive_count, precision_denominator);
    let recall = ratio(score.true_positive_count, recall_denominator);
    let f1 = match (precision, recall) {
        (Some(precision), Some(recall)) if precision + recall > 0.0 => {
            Some(2.0 * precision * recall / (precision + recall))
        }
        _ => None,
    };

    let mut rows = vec![
        measured_count(
            "line_construct_true_positive_count",
            score.true_positive_count,
            score.line_count,
            cadence,
        ),
        measured_count(
            "line_construct_false_positive_count",
            score.false_positive_count,
            score.line_count,
            cadence,
        ),
        measured_count(
            "line_construct_false_negative_count",
            score.false_negative_count,
            score.line_count,
            cadence,
        ),
        measured_rate(
            "line_construct_exact_match_rate",
            score.exact_match_count,
            score.line_count,
            cadence,
        ),
    ];

    rows.push(optional_measured_rate(
        "line_construct_precision",
        precision,
        precision_denominator,
        "no predicted line tags were available",
        cadence,
    ));
    rows.push(optional_measured_rate(
        "line_construct_recall",
        recall,
        recall_denominator,
        "no expected line tags were available",
        cadence,
    ));
    rows.push(optional_measured_rate(
        "line_construct_f1",
        f1,
        recall_denominator,
        "line precision or recall denominator is unavailable",
        cadence,
    ));
    rows.push(measured_rate(
        "line_error_false_positive_rate",
        score.false_parse_error_count,
        score.line_count,
        cadence,
    ));
    rows.push(optional_measured_rate(
        "line_error_false_negative_rate",
        ratio(score.missed_parse_error_count, score.expected_parse_error_count),
        score.expected_parse_error_count,
        "no expected parse-error line labels are available",
        cadence,
    ));
    rows.push(optional_measured_rate(
        "line_dynamic_boundary_correct_rate",
        ratio(score.correct_dynamic_boundary_count, score.expected_dynamic_boundary_count),
        score.expected_dynamic_boundary_count,
        "no expected dynamic-boundary line labels are available",
        cadence,
    ));
    rows.push(optional_measured_rate(
        "line_unsupported_detection_rate",
        ratio(
            score.correct_unsupported_construct_count,
            score.expected_unsupported_construct_count,
        ),
        score.expected_unsupported_construct_count,
        "no expected unsupported-construct line labels are available",
        cadence,
    ));

    rows
}

fn measured_count(metric: &str, value: u64, sample_count: u64, cadence: Cadence) -> MetricRow {
    MetricRow::Measured {
        metric: metric.to_string(),
        value: value as f64,
        sample_count,
        direction: Direction::Neutral,
        confidence: Confidence::High,
        cadence,
    }
}

fn measured_rate(metric: &str, numerator: u64, denominator: u64, cadence: Cadence) -> MetricRow {
    let value = ratio(numerator, denominator).unwrap_or(0.0);
    MetricRow::Measured {
        metric: metric.to_string(),
        value,
        sample_count: denominator,
        direction: Direction::Neutral,
        confidence: Confidence::High,
        cadence,
    }
}

fn optional_measured_rate(
    metric: &str,
    value: Option<f64>,
    sample_count: u64,
    insufficient_reason: &str,
    cadence: Cadence,
) -> MetricRow {
    match value {
        Some(value) if sample_count > 0 => MetricRow::Measured {
            metric: metric.to_string(),
            value,
            sample_count,
            direction: Direction::Neutral,
            confidence: Confidence::High,
            cadence,
        },
        _ => insufficient(metric, insufficient_reason),
    }
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 { None } else { Some(numerator as f64 / denominator as f64) }
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
    if LINE_TAG_VOCABULARY.is_empty() {
        bail!("parser accuracy line tag vocabulary must not be empty");
    }
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

    fn tags(values: &[LineTag]) -> BTreeSet<LineTag> {
        values.iter().copied().collect()
    }

    fn write_fixture_sources(root: &Path) -> Result<()> {
        fs::write(
            root.join("package_basic.pl"),
            "package Accuracy::Basic;\n\nsub answer { 42 }\n",
        )?;
        fs::write(
            root.join("dynamic_require.pl"),
            "package Accuracy::DynamicRequire;\n\nmy $module = \"Accuracy::Plugin\";\nrequire $module;\n",
        )?;
        Ok(())
    }

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
                    line_expectations: vec![
                        LineExpectation { line: 1, expected_tags: tags(&[LineTag::PackageDecl]) },
                        LineExpectation { line: 3, expected_tags: tags(&[LineTag::SubDecl]) },
                    ],
                },
                FixtureMetadata {
                    id: "dynamic_require_boundary".to_string(),
                    family: "dynamic_require".to_string(),
                    label_mode: LabelMode::Partial,
                    source_path: "dynamic_require.pl".to_string(),
                    scored_lines: 3,
                    scored_symbols: 1,
                    fully_labeled_regions: 0,
                    partial_labeled_regions: 1,
                    unknown_regions: 1,
                    negative_regions: 0,
                    dynamic_boundaries: 1,
                    unsupported_constructs: 0,
                    real_project_file: false,
                    generated: false,
                    line_expectations: vec![
                        LineExpectation { line: 1, expected_tags: tags(&[LineTag::PackageDecl]) },
                        LineExpectation { line: 3, expected_tags: tags(&[LineTag::VariableDecl]) },
                        LineExpectation {
                            line: 4,
                            expected_tags: tags(&[LineTag::Import, LineTag::DynamicBoundary]),
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn denominator_counts_manifest_inventory() {
        let denominator = compute_denominator(&fixture_manifest());
        assert_eq!(denominator.fixture_count, 2);
        assert_eq!(denominator.fixture_family_count, 2);
        assert_eq!(denominator.scored_line_count, 5);
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
    fn line_tag_vocabulary_includes_required_contract() {
        assert_eq!(LINE_TAG_VOCABULARY.len(), 22);
        assert!(LINE_TAG_VOCABULARY.contains(&LineTag::PackageDecl));
        assert!(LINE_TAG_VOCABULARY.contains(&LineTag::DynamicBoundary));
        assert!(LINE_TAG_VOCABULARY.contains(&LineTag::UnsupportedConstruct));
    }

    #[test]
    fn line_scorer_counts_false_positive_and_false_negative() {
        let expected = tags(&[LineTag::PackageDecl, LineTag::SubDecl]);
        let actual = tags(&[LineTag::PackageDecl, LineTag::MethodCall]);
        let mut score = LineScore::default();

        score_line_tags(&expected, &actual, &mut score);

        assert_eq!(score.true_positive_count, 1);
        assert_eq!(score.false_positive_count, 1);
        assert_eq!(score.false_negative_count, 1);
        assert_eq!(score.exact_match_count, 0);
    }

    #[test]
    fn line_metrics_emit_measured_scores_and_insufficient_missing_denominators() {
        let mut score = LineScore::default();
        score_line_tags(
            &tags(&[LineTag::Import, LineTag::DynamicBoundary]),
            &tags(&[LineTag::Import, LineTag::DynamicBoundary]),
            &mut score,
        );

        let metrics = line_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "line_construct_f1" && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "line_unsupported_detection_rate"
            )
        }));
    }

    #[test]
    fn artifact_uses_measured_line_scores_and_insufficient_data_for_unwired_scorers() -> Result<()>
    {
        let tmp = tempfile::tempdir()?;
        write_fixture_sources(tmp.path())?;
        let artifact = build_artifact(tmp.path(), &fixture_manifest(), Cadence::Pr)?;
        validate_artifact_contract(&artifact)?;
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, sample_count, .. }
                    if metric == "line_construct_f1"
                        && *sample_count > 0
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "ast_node_kind_f1"
            )
        }));
        Ok(())
    }
}

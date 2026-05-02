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
use perl_semantic_facts::{AnchorId, EntityId};
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};
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
    #[serde(default)]
    ast_expectations: Vec<AstExpectation>,
    #[serde(default)]
    symbol_expectations: SymbolExpectations,
    #[serde(default)]
    symbol_safety_regions: Vec<SymbolSafetyRegion>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AstExpectation {
    id: String,
    kind: String,
    line: u64,
    span_text: String,
    parent_kind: Option<String>,
    depth: Option<u64>,
    operator: Option<String>,
    parent_operator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AstPrediction {
    kind: String,
    line: u64,
    span_text: String,
    parent_kind: Option<String>,
    depth: u64,
    operator: Option<String>,
    parent_operator: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct SymbolExpectations {
    #[serde(default)]
    entities: Vec<SymbolEntityExpectation>,
    #[serde(default)]
    occurrences: Vec<SymbolOccurrenceExpectation>,
    #[serde(default)]
    edges: Vec<SymbolEdgeExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SymbolEntityExpectation {
    id: String,
    kind: String,
    canonical_name: String,
    span_text: String,
    package: Option<String>,
    scope: Option<String>,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SymbolOccurrenceExpectation {
    id: String,
    kind: String,
    canonical_name: Option<String>,
    span_text: String,
    package: Option<String>,
    scope: Option<String>,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SymbolEdgeExpectation {
    id: String,
    kind: String,
    from: String,
    to: String,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SymbolSafetyRegion {
    kind: SymbolSafetyRegionKind,
    line: u64,
    span_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SymbolSafetyRegionKind {
    Comment,
    Pod,
    String,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SymbolEntityKey {
    kind: String,
    canonical_name: String,
    span_text: String,
    package: Option<String>,
    scope: Option<String>,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SymbolOccurrenceKey {
    kind: String,
    canonical_name: Option<String>,
    span_text: String,
    package: Option<String>,
    scope: Option<String>,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SymbolEdgeKey {
    kind: String,
    from: String,
    to: String,
    provenance: String,
    confidence: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SymbolPredictions {
    entities: BTreeSet<SymbolEntityKey>,
    occurrences: BTreeSet<SymbolOccurrenceKey>,
    safety_spans: BTreeSet<SymbolSpanLocation>,
    edges: BTreeSet<SymbolEdgeKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SymbolSpanLocation {
    line: u64,
    span_text: String,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AstScore {
    expected_node_count: u64,
    predicted_node_count: u64,
    node_kind_true_positive_count: u64,
    node_kind_false_positive_count: u64,
    node_kind_false_negative_count: u64,
    span_exact_count: u64,
    span_near_count: u64,
    parent_child_expected_count: u64,
    parent_child_correct_count: u64,
    tree_depth_expected_count: u64,
    tree_depth_correct_count: u64,
    operator_precedence_expected_count: u64,
    operator_precedence_correct_count: u64,
    delimiter_pairing_expected_count: u64,
    delimiter_pairing_correct_count: u64,
    unexpected_error_node_count: u64,
    missing_expected_node_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SymbolScore {
    entity_expected_count: u64,
    entity_predicted_count: u64,
    entity_true_positive_count: u64,
    entity_false_positive_count: u64,
    entity_false_negative_count: u64,
    occurrence_expected_count: u64,
    occurrence_predicted_count: u64,
    occurrence_true_positive_count: u64,
    occurrence_false_positive_count: u64,
    occurrence_false_negative_count: u64,
    edge_expected_count: u64,
    edge_predicted_count: u64,
    edge_true_positive_count: u64,
    edge_false_positive_count: u64,
    edge_false_negative_count: u64,
    entity_by_kind: BTreeMap<String, KindScore>,
    occurrence_by_kind: BTreeMap<String, KindScore>,
    false_positive_sample_count: u64,
    false_import_count: u64,
    false_export_count: u64,
    false_exact_resolution_count: u64,
    false_dynamic_resolution_count: u64,
    dynamic_false_precision_count: u64,
    dynamic_false_precision_sample_count: u64,
    comment_safety_region_count: u64,
    pod_safety_region_count: u64,
    string_safety_region_count: u64,
    unknown_safety_region_count: u64,
    symbols_emitted_in_comments: u64,
    symbols_emitted_in_pod: u64,
    symbols_emitted_in_strings: u64,
    symbols_emitted_in_unknown_regions: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct KindScore {
    expected_count: u64,
    predicted_count: u64,
    true_positive_count: u64,
    false_positive_count: u64,
    false_negative_count: u64,
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
    let ast_score = score_manifest_ast(root, manifest)?;
    let symbol_score = score_manifest_symbols(root, manifest)?;
    let mut metrics = vec![MetricRow::Measured {
        metric: "denominator_fixture_count".to_string(),
        value: fixture_count,
        sample_count: denominator.fixture_count,
        direction: Direction::Neutral,
        confidence: Confidence::High,
        cadence,
    }];
    metrics.extend(line_metrics(&line_score, cadence));
    metrics.extend(ast_metrics(&ast_score, cadence));
    metrics.extend(symbol_metrics(&symbol_score, cadence));
    metrics.extend(safety_metrics(&line_score, &symbol_score, cadence));

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

fn score_manifest_ast(root: &Path, manifest: &ParserAccuracyManifest) -> Result<AstScore> {
    let mut score = AstScore::default();
    for fixture in &manifest.fixtures {
        if fixture.ast_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = fs::read_to_string(&source_path).with_context(|| {
            format!("reading parser accuracy fixture source {}", source_path.display())
        })?;
        let expected_lines = fixture
            .ast_expectations
            .iter()
            .map(|expectation| expectation.line)
            .collect::<BTreeSet<_>>();
        let predictions = extract_ast_predictions(&source)
            .into_iter()
            .filter(|prediction| expected_lines.contains(&prediction.line))
            .collect::<Vec<_>>();
        score_ast_expectations(&fixture.ast_expectations, &predictions, &mut score);
    }
    Ok(score)
}

fn extract_ast_predictions(source: &str) -> Vec<AstPrediction> {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let line_starts = line_starts(source);
    let mut predictions = Vec::new();
    collect_ast_predictions(&output.ast, source, &line_starts, None, None, 0, &mut predictions);
    predictions
}

fn collect_ast_predictions(
    node: &Node,
    source: &str,
    line_starts: &[usize],
    parent_kind: Option<&str>,
    parent_operator: Option<&str>,
    depth: u64,
    predictions: &mut Vec<AstPrediction>,
) {
    let operator = node_operator(node);
    if is_ast_scored_node(node) {
        predictions.push(AstPrediction {
            kind: node.kind.kind_name().to_string(),
            line: line_for_offset(line_starts, node.location.start),
            span_text: source
                .get(node.location.start..node.location.end)
                .unwrap_or_default()
                .to_string(),
            parent_kind: parent_kind.map(ToString::to_string),
            depth,
            operator: operator.map(ToString::to_string),
            parent_operator: parent_operator.map(ToString::to_string),
        });
    }

    let kind = node.kind.kind_name();
    node.for_each_child(|child| {
        collect_ast_predictions(
            child,
            source,
            line_starts,
            Some(kind),
            operator,
            depth + 1,
            predictions,
        );
    });
}

fn is_ast_scored_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Package { .. }
            | NodeKind::Subroutine { .. }
            | NodeKind::Method { .. }
            | NodeKind::VariableDeclaration { .. }
            | NodeKind::VariableListDeclaration { .. }
            | NodeKind::FunctionCall { .. }
            | NodeKind::MethodCall { .. }
            | NodeKind::Regex { .. }
            | NodeKind::Match { .. }
            | NodeKind::Substitution { .. }
            | NodeKind::Transliteration { .. }
            | NodeKind::Heredoc { .. }
            | NodeKind::Format { .. }
            | NodeKind::Binary { .. }
            | NodeKind::Error { .. }
            | NodeKind::UnknownRest
    )
}

fn node_operator(node: &Node) -> Option<&str> {
    match &node.kind {
        NodeKind::Binary { op, .. } => Some(op.as_str()),
        _ => None,
    }
}

fn score_ast_expectations(
    expectations: &[AstExpectation],
    predictions: &[AstPrediction],
    score: &mut AstScore,
) {
    score.expected_node_count += expectations.len() as u64;
    score.predicted_node_count += predictions.len() as u64;
    score.unexpected_error_node_count +=
        predictions.iter().filter(|prediction| prediction.kind == "Error").count() as u64;

    let mut matched = BTreeSet::new();
    for expectation in expectations {
        let prediction_index = predictions.iter().enumerate().find_map(|(index, prediction)| {
            if matched.contains(&index) {
                return None;
            }
            if prediction.kind == expectation.kind && prediction.line == expectation.line {
                Some(index)
            } else {
                None
            }
        });

        let Some(prediction_index) = prediction_index else {
            score.node_kind_false_negative_count += 1;
            score.missing_expected_node_count += 1;
            continue;
        };

        matched.insert(prediction_index);
        let prediction = &predictions[prediction_index];
        score.node_kind_true_positive_count += 1;
        if prediction.span_text == expectation.span_text {
            score.span_exact_count += 1;
        }
        if prediction.line == expectation.line {
            score.span_near_count += 1;
        }
        if let Some(parent_kind) = &expectation.parent_kind {
            score.parent_child_expected_count += 1;
            if prediction.parent_kind.as_ref() == Some(parent_kind) {
                score.parent_child_correct_count += 1;
            }
        }
        if let Some(depth) = expectation.depth {
            score.tree_depth_expected_count += 1;
            if prediction.depth == depth {
                score.tree_depth_correct_count += 1;
            }
        }
        if let Some(operator) = &expectation.operator {
            score.operator_precedence_expected_count += 1;
            if prediction.operator.as_ref() == Some(operator)
                && prediction.parent_operator.as_ref() == expectation.parent_operator.as_ref()
            {
                score.operator_precedence_correct_count += 1;
            }
        }
    }

    score.node_kind_false_positive_count += predictions
        .iter()
        .enumerate()
        .filter(|(index, prediction)| !matched.contains(index) && prediction.kind != "Error")
        .count() as u64;
}

fn score_manifest_symbols(root: &Path, manifest: &ParserAccuracyManifest) -> Result<SymbolScore> {
    let mut score = SymbolScore::default();
    for fixture in &manifest.fixtures {
        if fixture.symbol_expectations.entities.is_empty()
            && fixture.symbol_expectations.occurrences.is_empty()
            && fixture.symbol_expectations.edges.is_empty()
            && fixture.symbol_safety_regions.is_empty()
        {
            continue;
        }

        let source_path = root.join(&fixture.source_path);
        let source = fs::read_to_string(&source_path).with_context(|| {
            format!("reading parser accuracy fixture source {}", source_path.display())
        })?;
        let predictions = extract_symbol_predictions(&source_path, &source)?;
        score_symbol_expectations(
            &fixture.symbol_expectations,
            &predictions,
            fixture.label_mode == LabelMode::Full,
            &mut score,
        );
        score_symbol_safety_regions(&fixture.symbol_safety_regions, &predictions, &mut score);
    }
    Ok(score)
}

fn extract_symbol_predictions(source_path: &Path, source: &str) -> Result<SymbolPredictions> {
    let index = WorkspaceIndex::new();
    let source_path_text = source_path.to_string_lossy();
    index.index_file_str(&source_path_text, source).map_err(|err| {
        eyre!("indexing parser accuracy fixture {}: {err}", source_path.display())
    })?;
    let shard = index.file_fact_shard(&source_path_text).ok_or_else(|| {
        eyre!("missing canonical fact shard for parser accuracy fixture {}", source_path.display())
    })?;
    Ok(symbol_predictions_from_shard(source, &shard))
}

fn symbol_predictions_from_shard(source: &str, shard: &FileFactShard) -> SymbolPredictions {
    let anchors_by_id =
        shard.anchors.iter().map(|anchor| (anchor.id, anchor)).collect::<BTreeMap<AnchorId, _>>();
    let entities_by_id =
        shard.entities.iter().map(|entity| (entity.id, entity)).collect::<BTreeMap<EntityId, _>>();

    let mut safety_spans = BTreeSet::new();

    let entities = shard
        .entities
        .iter()
        .map(|entity| {
            let anchor = entity.anchor_id.and_then(|anchor_id| anchors_by_id.get(&anchor_id));
            if let Some(anchor) = anchor {
                safety_spans.insert(symbol_span_location(source, anchor));
            }
            SymbolEntityKey {
                kind: format!("{:?}", entity.kind),
                canonical_name: entity.canonical_name.clone(),
                span_text: anchor.map(|anchor| anchor_text(source, anchor)).unwrap_or_default(),
                package: package_from_name(&entity.canonical_name),
                scope: entity.scope_id.map(|scope| scope.0.to_string()),
                provenance: format!("{:?}", entity.provenance),
                confidence: format!("{:?}", entity.confidence),
            }
        })
        .collect();

    let occurrences = shard
        .occurrences
        .iter()
        .map(|occurrence| {
            let anchor = anchors_by_id.get(&occurrence.anchor_id);
            if let Some(anchor) = anchor {
                safety_spans.insert(symbol_span_location(source, anchor));
            }
            let canonical_name = occurrence
                .entity_id
                .and_then(|entity_id| entities_by_id.get(&entity_id))
                .map(|entity| entity.canonical_name.clone());
            SymbolOccurrenceKey {
                kind: format!("{:?}", occurrence.kind),
                package: canonical_name.as_deref().and_then(package_from_name),
                canonical_name,
                span_text: anchor.map(|anchor| anchor_text(source, anchor)).unwrap_or_default(),
                scope: occurrence.scope_id.map(|scope| scope.0.to_string()),
                provenance: format!("{:?}", occurrence.provenance),
                confidence: format!("{:?}", occurrence.confidence),
            }
        })
        .collect();

    let edges = shard
        .edges
        .iter()
        .map(|edge| SymbolEdgeKey {
            kind: format!("{:?}", edge.kind),
            from: entities_by_id
                .get(&edge.from_entity_id)
                .map(|entity| entity.canonical_name.clone())
                .unwrap_or_else(|| format!("<unknown:{}>", edge.from_entity_id.0)),
            to: entities_by_id
                .get(&edge.to_entity_id)
                .map(|entity| entity.canonical_name.clone())
                .unwrap_or_else(|| format!("<unknown:{}>", edge.to_entity_id.0)),
            provenance: format!("{:?}", edge.provenance),
            confidence: format!("{:?}", edge.confidence),
        })
        .collect();

    SymbolPredictions { entities, occurrences, safety_spans, edges }
}

fn anchor_text(source: &str, anchor: &perl_semantic_facts::AnchorFact) -> String {
    source
        .get(anchor.span_start_byte as usize..anchor.span_end_byte as usize)
        .unwrap_or_default()
        .to_string()
}

fn symbol_span_location(
    source: &str,
    anchor: &perl_semantic_facts::AnchorFact,
) -> SymbolSpanLocation {
    SymbolSpanLocation {
        line: line_for_byte(source, anchor.span_start_byte as usize),
        span_text: anchor_text(source, anchor),
    }
}

fn line_for_byte(source: &str, byte: usize) -> u64 {
    source.as_bytes().iter().take(byte.min(source.len())).filter(|byte| **byte == b'\n').count()
        as u64
        + 1
}

fn package_from_name(name: &str) -> Option<String> {
    name.rsplit_once("::").map(|(package, _)| package.to_string())
}

fn score_symbol_expectations(
    expectations: &SymbolExpectations,
    predictions: &SymbolPredictions,
    count_false_positives: bool,
    score: &mut SymbolScore,
) {
    let expected_entities =
        expectations.entities.iter().map(entity_key_from_expectation).collect::<BTreeSet<_>>();
    score.entity_expected_count += expected_entities.len() as u64;
    score.entity_predicted_count += predictions.entities.len() as u64;
    let (entity_tp, entity_fp, entity_fn) =
        score_key_sets(&expected_entities, &predictions.entities, count_false_positives);
    score.entity_true_positive_count += entity_tp;
    score.entity_false_positive_count += entity_fp;
    score.entity_false_negative_count += entity_fn;
    if count_false_positives {
        score.false_positive_sample_count += predictions.entities.len() as u64;
    }
    score_kind_sets(
        &expected_entities,
        &predictions.entities,
        count_false_positives,
        &mut score.entity_by_kind,
        |key| key.kind.as_str(),
    );

    let expected_occurrences = expectations
        .occurrences
        .iter()
        .map(occurrence_key_from_expectation)
        .collect::<BTreeSet<_>>();
    score.occurrence_expected_count += expected_occurrences.len() as u64;
    score.occurrence_predicted_count += predictions.occurrences.len() as u64;
    let (occurrence_tp, occurrence_fp, occurrence_fn) =
        score_key_sets(&expected_occurrences, &predictions.occurrences, count_false_positives);
    score.occurrence_true_positive_count += occurrence_tp;
    score.occurrence_false_positive_count += occurrence_fp;
    score.occurrence_false_negative_count += occurrence_fn;
    if count_false_positives {
        score.false_positive_sample_count += predictions.occurrences.len() as u64;
        for false_positive in predictions.occurrences.difference(&expected_occurrences) {
            match false_positive.kind.as_str() {
                "Import" => score.false_import_count += 1,
                "Export" => score.false_export_count += 1,
                _ => {}
            }
            if false_positive.canonical_name.is_some() && false_positive.confidence == "High" {
                score.false_exact_resolution_count += 1;
            }
            if false_positive.provenance == "DynamicBoundary"
                && false_positive.canonical_name.is_some()
            {
                score.false_dynamic_resolution_count += 1;
            }
        }
    }
    score.dynamic_false_precision_sample_count += expected_occurrences
        .iter()
        .filter(|expected| expected.provenance == "DynamicBoundary")
        .count() as u64;
    score.dynamic_false_precision_count +=
        count_dynamic_false_precision(&expected_occurrences, &predictions.occurrences);
    score_kind_sets(
        &expected_occurrences,
        &predictions.occurrences,
        count_false_positives,
        &mut score.occurrence_by_kind,
        |key| key.kind.as_str(),
    );

    let expected_edges =
        expectations.edges.iter().map(edge_key_from_expectation).collect::<BTreeSet<_>>();
    score.edge_expected_count += expected_edges.len() as u64;
    score.edge_predicted_count += predictions.edges.len() as u64;
    let (edge_tp, edge_fp, edge_fn) =
        score_key_sets(&expected_edges, &predictions.edges, count_false_positives);
    score.edge_true_positive_count += edge_tp;
    score.edge_false_positive_count += edge_fp;
    score.edge_false_negative_count += edge_fn;
    if count_false_positives {
        score.false_positive_sample_count += predictions.edges.len() as u64;
        score.false_exact_resolution_count += predictions
            .edges
            .difference(&expected_edges)
            .filter(|false_positive| false_positive.confidence == "High")
            .count() as u64;
    }
}

fn count_dynamic_false_precision(
    expected: &BTreeSet<SymbolOccurrenceKey>,
    predicted: &BTreeSet<SymbolOccurrenceKey>,
) -> u64 {
    expected
        .iter()
        .filter(|expected| expected.provenance == "DynamicBoundary")
        .filter(|expected| {
            predicted.iter().any(|prediction| {
                prediction.span_text == expected.span_text
                    && (prediction.canonical_name.is_some()
                        || prediction.provenance != "DynamicBoundary"
                        || prediction.confidence == "High")
            })
        })
        .count() as u64
}

fn score_symbol_safety_regions(
    regions: &[SymbolSafetyRegion],
    predictions: &SymbolPredictions,
    score: &mut SymbolScore,
) {
    for region in regions {
        let emitted = predictions.safety_spans.contains(&SymbolSpanLocation {
            line: region.line,
            span_text: region.span_text.clone(),
        });
        match region.kind {
            SymbolSafetyRegionKind::Comment => {
                score.comment_safety_region_count += 1;
                if emitted {
                    score.symbols_emitted_in_comments += 1;
                }
            }
            SymbolSafetyRegionKind::Pod => {
                score.pod_safety_region_count += 1;
                if emitted {
                    score.symbols_emitted_in_pod += 1;
                }
            }
            SymbolSafetyRegionKind::String => {
                score.string_safety_region_count += 1;
                if emitted {
                    score.symbols_emitted_in_strings += 1;
                }
            }
            SymbolSafetyRegionKind::Unknown => {
                score.unknown_safety_region_count += 1;
                if emitted {
                    score.symbols_emitted_in_unknown_regions += 1;
                }
            }
        }
    }
}

fn entity_key_from_expectation(expectation: &SymbolEntityExpectation) -> SymbolEntityKey {
    SymbolEntityKey {
        kind: expectation.kind.clone(),
        canonical_name: expectation.canonical_name.clone(),
        span_text: expectation.span_text.clone(),
        package: expectation.package.clone(),
        scope: expectation.scope.clone(),
        provenance: expectation.provenance.clone(),
        confidence: expectation.confidence.clone(),
    }
}

fn occurrence_key_from_expectation(
    expectation: &SymbolOccurrenceExpectation,
) -> SymbolOccurrenceKey {
    SymbolOccurrenceKey {
        kind: expectation.kind.clone(),
        canonical_name: expectation.canonical_name.clone(),
        span_text: expectation.span_text.clone(),
        package: expectation.package.clone(),
        scope: expectation.scope.clone(),
        provenance: expectation.provenance.clone(),
        confidence: expectation.confidence.clone(),
    }
}

fn edge_key_from_expectation(expectation: &SymbolEdgeExpectation) -> SymbolEdgeKey {
    SymbolEdgeKey {
        kind: expectation.kind.clone(),
        from: expectation.from.clone(),
        to: expectation.to.clone(),
        provenance: expectation.provenance.clone(),
        confidence: expectation.confidence.clone(),
    }
}

fn score_key_sets<T: Ord>(
    expected: &BTreeSet<T>,
    predicted: &BTreeSet<T>,
    count_false_positives: bool,
) -> (u64, u64, u64) {
    let true_positives = expected.intersection(predicted).count() as u64;
    let false_positives =
        if count_false_positives { predicted.difference(expected).count() as u64 } else { 0 };
    let false_negatives = expected.difference(predicted).count() as u64;
    (true_positives, false_positives, false_negatives)
}

fn score_kind_sets<T: Ord>(
    expected: &BTreeSet<T>,
    predicted: &BTreeSet<T>,
    count_false_positives: bool,
    by_kind: &mut BTreeMap<String, KindScore>,
    kind: impl Fn(&T) -> &str,
) {
    let kinds = expected
        .iter()
        .chain(predicted.iter())
        .map(|key| kind(key).to_string())
        .collect::<BTreeSet<_>>();
    for current_kind in kinds {
        let expected_for_kind =
            expected.iter().filter(|key| kind(key) == current_kind).collect::<BTreeSet<_>>();
        let predicted_for_kind =
            predicted.iter().filter(|key| kind(key) == current_kind).collect::<BTreeSet<_>>();
        let (true_positive_count, false_positive_count, false_negative_count) =
            score_key_sets(&expected_for_kind, &predicted_for_kind, count_false_positives);
        let entry = by_kind.entry(current_kind).or_default();
        entry.expected_count += expected_for_kind.len() as u64;
        entry.predicted_count += predicted_for_kind.len() as u64;
        entry.true_positive_count += true_positive_count;
        entry.false_positive_count += false_positive_count;
        entry.false_negative_count += false_negative_count;
    }
}

fn ast_metrics(score: &AstScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.expected_node_count == 0 {
        return vec![insufficient("ast_node_kind_f1", "AST gold labels are not available")];
    }

    let precision_denominator =
        score.node_kind_true_positive_count + score.node_kind_false_positive_count;
    let recall_denominator =
        score.node_kind_true_positive_count + score.node_kind_false_negative_count;
    let precision = ratio(score.node_kind_true_positive_count, precision_denominator);
    let recall = ratio(score.node_kind_true_positive_count, recall_denominator);
    let f1 = match (precision, recall) {
        (Some(precision), Some(recall)) if precision + recall > 0.0 => {
            Some(2.0 * precision * recall / (precision + recall))
        }
        _ => None,
    };

    vec![
        optional_measured_rate(
            "ast_node_kind_precision",
            precision,
            precision_denominator,
            "no predicted AST nodes were available",
            cadence,
        ),
        optional_measured_rate(
            "ast_node_kind_recall",
            recall,
            recall_denominator,
            "no expected AST nodes were available",
            cadence,
        ),
        optional_measured_rate(
            "ast_node_kind_f1",
            f1,
            recall_denominator,
            "AST precision or recall denominator is unavailable",
            cadence,
        ),
        measured_rate(
            "ast_node_span_exact_rate",
            score.span_exact_count,
            score.expected_node_count,
            cadence,
        ),
        measured_rate(
            "ast_node_span_near_rate",
            score.span_near_count,
            score.expected_node_count,
            cadence,
        ),
        optional_measured_rate(
            "ast_parent_child_edge_accuracy",
            ratio(score.parent_child_correct_count, score.parent_child_expected_count),
            score.parent_child_expected_count,
            "no expected AST parent-child edges are available",
            cadence,
        ),
        optional_measured_rate(
            "ast_tree_depth_accuracy",
            ratio(score.tree_depth_correct_count, score.tree_depth_expected_count),
            score.tree_depth_expected_count,
            "no expected AST tree-depth labels are available",
            cadence,
        ),
        optional_measured_rate(
            "ast_operator_precedence_accuracy",
            ratio(
                score.operator_precedence_correct_count,
                score.operator_precedence_expected_count,
            ),
            score.operator_precedence_expected_count,
            "no expected AST operator-precedence labels are available",
            cadence,
        ),
        optional_measured_rate(
            "ast_delimiter_pairing_accuracy",
            ratio(score.delimiter_pairing_correct_count, score.delimiter_pairing_expected_count),
            score.delimiter_pairing_expected_count,
            "no expected AST delimiter-pairing labels are available",
            cadence,
        ),
        measured_count(
            "ast_unexpected_error_node_count",
            score.unexpected_error_node_count,
            score.expected_node_count,
            cadence,
        ),
        measured_count(
            "ast_missing_expected_node_count",
            score.missing_expected_node_count,
            score.expected_node_count,
            cadence,
        ),
    ]
}

fn symbol_metrics(score: &SymbolScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.entity_expected_count == 0
        && score.occurrence_expected_count == 0
        && score.edge_expected_count == 0
    {
        return vec![insufficient("symbol_decl_f1", "symbol gold labels are not available")];
    }

    let mut rows = vec![
        symbol_precision_metric(
            "symbol_decl_precision",
            score.entity_true_positive_count,
            score.entity_false_positive_count,
            cadence,
        ),
        symbol_recall_metric(
            "symbol_decl_recall",
            score.entity_true_positive_count,
            score.entity_false_negative_count,
            cadence,
        ),
        symbol_f1_metric(
            "symbol_decl_f1",
            score.entity_true_positive_count,
            score.entity_false_positive_count,
            score.entity_false_negative_count,
            score.entity_expected_count,
            cadence,
        ),
        symbol_precision_metric(
            "symbol_ref_precision",
            score.occurrence_true_positive_count,
            score.occurrence_false_positive_count,
            cadence,
        ),
        symbol_recall_metric(
            "symbol_ref_recall",
            score.occurrence_true_positive_count,
            score.occurrence_false_negative_count,
            cadence,
        ),
        symbol_f1_metric(
            "symbol_ref_f1",
            score.occurrence_true_positive_count,
            score.occurrence_false_positive_count,
            score.occurrence_false_negative_count,
            score.occurrence_expected_count,
            cadence,
        ),
        symbol_precision_metric(
            "symbol_edge_precision",
            score.edge_true_positive_count,
            score.edge_false_positive_count,
            cadence,
        ),
        symbol_recall_metric(
            "symbol_edge_recall",
            score.edge_true_positive_count,
            score.edge_false_negative_count,
            cadence,
        ),
        symbol_f1_metric(
            "symbol_edge_f1",
            score.edge_true_positive_count,
            score.edge_false_positive_count,
            score.edge_false_negative_count,
            score.edge_expected_count,
            cadence,
        ),
    ];

    for &(metric, source, kind) in SYMBOL_KIND_F1_ROWS {
        let kind_score = match source {
            SymbolMetricSource::Entity => score.entity_by_kind.get(kind),
            SymbolMetricSource::Occurrence => score.occurrence_by_kind.get(kind),
        };
        rows.push(match kind_score {
            Some(kind_score) if kind_score.expected_count > 0 => symbol_f1_metric(
                metric,
                kind_score.true_positive_count,
                kind_score.false_positive_count,
                kind_score.false_negative_count,
                kind_score.expected_count,
                cadence,
            ),
            _ => insufficient(metric, "no symbol gold labels are available for this kind"),
        });
    }

    rows
}

fn safety_metrics(
    line_score: &LineScore,
    symbol_score: &SymbolScore,
    cadence: Cadence,
) -> Vec<MetricRow> {
    vec![
        optional_measured_count(
            "false_symbol_count",
            symbol_score.entity_false_positive_count + symbol_score.occurrence_false_positive_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_declaration_count",
            symbol_score.entity_false_positive_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_reference_count",
            symbol_score.occurrence_false_positive_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_import_count",
            symbol_score.false_import_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_export_count",
            symbol_score.false_export_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_parse_error_count",
            line_score.false_parse_error_count,
            line_score.line_count,
            "no line labels are available for parse-error false positives",
            cadence,
        ),
        optional_measured_count(
            "false_exact_resolution_count",
            symbol_score.false_exact_resolution_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "false_dynamic_resolution_count",
            symbol_score.false_dynamic_resolution_count,
            symbol_score.false_positive_sample_count,
            "no fully labeled symbol fixtures are available",
            cadence,
        ),
        optional_measured_count(
            "dynamic_false_precision_count",
            symbol_score.dynamic_false_precision_count,
            symbol_score.dynamic_false_precision_sample_count,
            "no dynamic-boundary symbol labels are available",
            cadence,
        ),
        optional_measured_count(
            "symbols_emitted_in_comments",
            symbol_score.symbols_emitted_in_comments,
            symbol_score.comment_safety_region_count,
            "no comment safety regions are available",
            cadence,
        ),
        optional_measured_count(
            "symbols_emitted_in_pod",
            symbol_score.symbols_emitted_in_pod,
            symbol_score.pod_safety_region_count,
            "no POD safety regions are available",
            cadence,
        ),
        optional_measured_count(
            "symbols_emitted_in_strings",
            symbol_score.symbols_emitted_in_strings,
            symbol_score.string_safety_region_count,
            "no string safety regions are available",
            cadence,
        ),
        optional_measured_count(
            "symbols_emitted_in_unknown_regions",
            symbol_score.symbols_emitted_in_unknown_regions,
            symbol_score.unknown_safety_region_count,
            "no unknown safety regions are available",
            cadence,
        ),
    ]
}

#[derive(Debug, Clone, Copy)]
enum SymbolMetricSource {
    Entity,
    Occurrence,
}

const SYMBOL_KIND_F1_ROWS: &[(&str, SymbolMetricSource, &str)] = &[
    ("symbol_decl_package_f1", SymbolMetricSource::Entity, "Package"),
    ("symbol_decl_subroutine_f1", SymbolMetricSource::Entity, "Subroutine"),
    ("symbol_decl_method_f1", SymbolMetricSource::Entity, "Method"),
    ("symbol_decl_lexical_variable_f1", SymbolMetricSource::Entity, "LexicalVariable"),
    ("symbol_decl_global_variable_f1", SymbolMetricSource::Entity, "GlobalVariable"),
    ("symbol_ref_import_f1", SymbolMetricSource::Occurrence, "Import"),
    ("symbol_ref_export_f1", SymbolMetricSource::Occurrence, "Export"),
    ("symbol_ref_typeglob_alias_f1", SymbolMetricSource::Occurrence, "TypeglobReference"),
    ("symbol_decl_generated_accessor_f1", SymbolMetricSource::Entity, "GeneratedMember"),
    ("symbol_decl_role_method_f1", SymbolMetricSource::Entity, "RoleMethod"),
    ("symbol_decl_inherited_method_f1", SymbolMetricSource::Entity, "InheritedMethod"),
    ("symbol_ref_dynamic_boundary_f1", SymbolMetricSource::Occurrence, "DynamicBoundary"),
];

fn symbol_precision_metric(
    metric: &str,
    true_positive_count: u64,
    false_positive_count: u64,
    cadence: Cadence,
) -> MetricRow {
    let denominator = true_positive_count + false_positive_count;
    optional_measured_rate(
        metric,
        ratio(true_positive_count, denominator),
        denominator,
        "no symbol predictions are available",
        cadence,
    )
}

fn symbol_recall_metric(
    metric: &str,
    true_positive_count: u64,
    false_negative_count: u64,
    cadence: Cadence,
) -> MetricRow {
    let denominator = true_positive_count + false_negative_count;
    optional_measured_rate(
        metric,
        ratio(true_positive_count, denominator),
        denominator,
        "no symbol gold labels are available",
        cadence,
    )
}

fn symbol_f1_metric(
    metric: &str,
    true_positive_count: u64,
    false_positive_count: u64,
    false_negative_count: u64,
    sample_count: u64,
    cadence: Cadence,
) -> MetricRow {
    let denominator = (2 * true_positive_count) + false_positive_count + false_negative_count;
    optional_measured_rate(
        metric,
        ratio(2 * true_positive_count, denominator),
        sample_count,
        "symbol F1 denominator is unavailable",
        cadence,
    )
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

fn optional_measured_count(
    metric: &str,
    value: u64,
    sample_count: u64,
    insufficient_reason: &str,
    cadence: Cadence,
) -> MetricRow {
    if sample_count == 0 {
        return insufficient(metric, insufficient_reason);
    }
    measured_count(metric, value, sample_count, cadence)
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
                    ast_expectations: vec![
                        AstExpectation {
                            id: "package_basic_package".to_string(),
                            kind: "Package".to_string(),
                            line: 1,
                            span_text: "package Accuracy::Basic;".to_string(),
                            parent_kind: Some("Program".to_string()),
                            depth: Some(1),
                            operator: None,
                            parent_operator: None,
                        },
                        AstExpectation {
                            id: "package_basic_subroutine".to_string(),
                            kind: "Subroutine".to_string(),
                            line: 3,
                            span_text: "sub answer { 42 }".to_string(),
                            parent_kind: Some("Program".to_string()),
                            depth: Some(1),
                            operator: None,
                            parent_operator: None,
                        },
                    ],
                    symbol_expectations: SymbolExpectations {
                        entities: vec![
                            SymbolEntityExpectation {
                                id: "package_basic_package_entity".to_string(),
                                kind: "Package".to_string(),
                                canonical_name: "Accuracy::Basic".to_string(),
                                span_text: "Accuracy::Basic".to_string(),
                                package: Some("Accuracy".to_string()),
                                scope: None,
                                provenance: "ExactAst".to_string(),
                                confidence: "High".to_string(),
                            },
                            SymbolEntityExpectation {
                                id: "package_basic_answer_entity".to_string(),
                                kind: "Subroutine".to_string(),
                                canonical_name: "Accuracy::Basic::answer".to_string(),
                                span_text: "answer".to_string(),
                                package: Some("Accuracy::Basic".to_string()),
                                scope: None,
                                provenance: "ExactAst".to_string(),
                                confidence: "High".to_string(),
                            },
                        ],
                        occurrences: vec![],
                        edges: vec![SymbolEdgeExpectation {
                            id: "package_basic_defines_answer".to_string(),
                            kind: "Defines".to_string(),
                            from: "Accuracy::Basic".to_string(),
                            to: "Accuracy::Basic::answer".to_string(),
                            provenance: "ExactAst".to_string(),
                            confidence: "High".to_string(),
                        }],
                    },
                    symbol_safety_regions: vec![],
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
                    ast_expectations: vec![
                        AstExpectation {
                            id: "dynamic_require_package".to_string(),
                            kind: "Package".to_string(),
                            line: 1,
                            span_text: "package Accuracy::DynamicRequire;".to_string(),
                            parent_kind: Some("Program".to_string()),
                            depth: Some(1),
                            operator: None,
                            parent_operator: None,
                        },
                        AstExpectation {
                            id: "dynamic_require_variable".to_string(),
                            kind: "VariableDeclaration".to_string(),
                            line: 3,
                            span_text: "my $module = \"Accuracy::Plugin\"".to_string(),
                            parent_kind: Some("Program".to_string()),
                            depth: Some(1),
                            operator: None,
                            parent_operator: None,
                        },
                        AstExpectation {
                            id: "dynamic_require_call".to_string(),
                            kind: "FunctionCall".to_string(),
                            line: 4,
                            span_text: "require $module".to_string(),
                            parent_kind: Some("ExpressionStatement".to_string()),
                            depth: Some(2),
                            operator: None,
                            parent_operator: None,
                        },
                    ],
                    symbol_expectations: SymbolExpectations::default(),
                    symbol_safety_regions: vec![],
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
    fn ast_scorer_counts_wrong_parent_child_edge() {
        let expectations = vec![AstExpectation {
            id: "sub_wrong_parent".to_string(),
            kind: "Subroutine".to_string(),
            line: 1,
            span_text: "sub answer { 42 }".to_string(),
            parent_kind: Some("Block".to_string()),
            depth: Some(1),
            operator: None,
            parent_operator: None,
        }];
        let predictions = vec![AstPrediction {
            kind: "Subroutine".to_string(),
            line: 1,
            span_text: "sub answer { 42 }".to_string(),
            parent_kind: Some("Program".to_string()),
            depth: 1,
            operator: None,
            parent_operator: None,
        }];
        let mut score = AstScore::default();

        score_ast_expectations(&expectations, &predictions, &mut score);

        assert_eq!(score.node_kind_true_positive_count, 1);
        assert_eq!(score.parent_child_expected_count, 1);
        assert_eq!(score.parent_child_correct_count, 0);
    }

    #[test]
    fn ast_metrics_emit_measured_scores_and_insufficient_missing_denominators() {
        let mut score = AstScore::default();
        score_ast_expectations(
            &[AstExpectation {
                id: "binary_precedence".to_string(),
                kind: "Binary".to_string(),
                line: 1,
                span_text: "2 * 3".to_string(),
                parent_kind: Some("Binary".to_string()),
                depth: Some(3),
                operator: Some("*".to_string()),
                parent_operator: Some("+".to_string()),
            }],
            &[AstPrediction {
                kind: "Binary".to_string(),
                line: 1,
                span_text: "2 * 3".to_string(),
                parent_kind: Some("Binary".to_string()),
                depth: 3,
                operator: Some("*".to_string()),
                parent_operator: Some("+".to_string()),
            }],
            &mut score,
        );

        let metrics = ast_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "ast_operator_precedence_accuracy"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "ast_delimiter_pairing_accuracy"
            )
        }));
    }

    #[test]
    fn symbol_scorer_counts_typeglob_hit_and_generated_accessor_gap() -> Result<()> {
        let expectations = SymbolExpectations {
            entities: vec![SymbolEntityExpectation {
                id: "generated_name".to_string(),
                kind: "GeneratedMember".to_string(),
                canonical_name: "Accuracy::GeneratedAccessor::name".to_string(),
                span_text: "name".to_string(),
                package: Some("Accuracy::GeneratedAccessor".to_string()),
                scope: None,
                provenance: "FrameworkSynthesis".to_string(),
                confidence: "Medium".to_string(),
            }],
            occurrences: vec![SymbolOccurrenceExpectation {
                id: "typeglob_alias".to_string(),
                kind: "TypeglobReference".to_string(),
                canonical_name: None,
                span_text: "*alias".to_string(),
                package: None,
                scope: None,
                provenance: "DynamicBoundary".to_string(),
                confidence: "Low".to_string(),
            }],
            edges: vec![],
        };
        let predictions = SymbolPredictions {
            entities: BTreeSet::new(),
            occurrences: [SymbolOccurrenceKey {
                kind: "TypeglobReference".to_string(),
                canonical_name: None,
                span_text: "*alias".to_string(),
                package: None,
                scope: None,
                provenance: "DynamicBoundary".to_string(),
                confidence: "Low".to_string(),
            }]
            .into_iter()
            .collect(),
            safety_spans: BTreeSet::new(),
            edges: BTreeSet::new(),
        };
        let mut score = SymbolScore::default();

        score_symbol_expectations(&expectations, &predictions, false, &mut score);

        assert_eq!(score.occurrence_true_positive_count, 1);
        assert_eq!(score.entity_false_negative_count, 1);
        let generated = score
            .entity_by_kind
            .get("GeneratedMember")
            .ok_or_else(|| eyre!("generated member score should be present"))?;
        assert_eq!(generated.false_negative_count, 1);
        Ok(())
    }

    #[test]
    fn symbol_metrics_emit_measured_kind_rows() {
        let mut score = SymbolScore::default();
        score.entity_expected_count = 1;
        score.entity_true_positive_count = 1;
        score.entity_by_kind.insert(
            "Package".to_string(),
            KindScore {
                expected_count: 1,
                predicted_count: 1,
                true_positive_count: 1,
                false_positive_count: 0,
                false_negative_count: 0,
            },
        );

        let metrics = symbol_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "symbol_decl_package_f1"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "symbol_decl_generated_accessor_f1"
            )
        }));
    }

    #[test]
    fn dynamic_false_precision_counts_exact_resolution_for_dynamic_boundary() {
        let expectations = SymbolExpectations {
            entities: vec![],
            occurrences: vec![SymbolOccurrenceExpectation {
                id: "dynamic_require".to_string(),
                kind: "FunctionCall".to_string(),
                canonical_name: None,
                span_text: "require $module".to_string(),
                package: None,
                scope: None,
                provenance: "DynamicBoundary".to_string(),
                confidence: "Low".to_string(),
            }],
            edges: vec![],
        };
        let predictions = SymbolPredictions {
            entities: BTreeSet::new(),
            occurrences: [SymbolOccurrenceKey {
                kind: "FunctionCall".to_string(),
                canonical_name: Some("Accuracy::Plugin".to_string()),
                span_text: "require $module".to_string(),
                package: Some("Accuracy".to_string()),
                scope: None,
                provenance: "ExactAst".to_string(),
                confidence: "High".to_string(),
            }]
            .into_iter()
            .collect(),
            safety_spans: BTreeSet::new(),
            edges: BTreeSet::new(),
        };
        let mut score = SymbolScore::default();

        score_symbol_expectations(&expectations, &predictions, false, &mut score);

        assert_eq!(score.dynamic_false_precision_sample_count, 1);
        assert_eq!(score.dynamic_false_precision_count, 1);
    }

    #[test]
    fn symbol_safety_regions_count_comment_pod_string_and_unknown_hits() {
        let regions = vec![
            SymbolSafetyRegion {
                kind: SymbolSafetyRegionKind::Comment,
                line: 2,
                span_text: "commented_out".to_string(),
            },
            SymbolSafetyRegion {
                kind: SymbolSafetyRegionKind::Pod,
                line: 6,
                span_text: "podded".to_string(),
            },
            SymbolSafetyRegion {
                kind: SymbolSafetyRegionKind::String,
                line: 3,
                span_text: "stringy".to_string(),
            },
            SymbolSafetyRegion {
                kind: SymbolSafetyRegionKind::Unknown,
                line: 9,
                span_text: "dynamic_name".to_string(),
            },
        ];
        let predictions = SymbolPredictions {
            entities: BTreeSet::new(),
            occurrences: BTreeSet::new(),
            safety_spans: [
                SymbolSpanLocation { line: 2, span_text: "commented_out".to_string() },
                SymbolSpanLocation { line: 3, span_text: "stringy".to_string() },
            ]
            .into_iter()
            .collect(),
            edges: BTreeSet::new(),
        };
        let mut score = SymbolScore::default();

        score_symbol_safety_regions(&regions, &predictions, &mut score);

        assert_eq!(score.symbols_emitted_in_comments, 1);
        assert_eq!(score.symbols_emitted_in_strings, 1);
        assert_eq!(score.symbols_emitted_in_pod, 0);
        assert_eq!(score.symbols_emitted_in_unknown_regions, 0);
    }

    #[test]
    fn safety_metrics_emit_dynamic_false_precision_floor_candidate() {
        let line_score =
            LineScore { line_count: 2, false_parse_error_count: 0, ..LineScore::default() };
        let symbol_score = SymbolScore {
            false_positive_sample_count: 3,
            entity_false_positive_count: 1,
            occurrence_false_positive_count: 1,
            dynamic_false_precision_sample_count: 1,
            dynamic_false_precision_count: 0,
            comment_safety_region_count: 1,
            symbols_emitted_in_comments: 0,
            ..SymbolScore::default()
        };

        let metrics = safety_metrics(&line_score, &symbol_score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "dynamic_false_precision_count"
                        && (*value - 0.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 3, .. }
                    if metric == "false_symbol_count"
                        && (*value - 2.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn artifact_uses_measured_line_ast_and_symbol_scores() -> Result<()> {
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
                MetricRow::Measured { metric, sample_count, .. }
                    if metric == "ast_node_kind_f1"
                        && *sample_count > 0
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, sample_count, .. }
                    if metric == "symbol_decl_f1"
                        && *sample_count > 0
            )
        }));
        Ok(())
    }
}

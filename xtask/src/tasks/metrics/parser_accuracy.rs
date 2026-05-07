//! Parser accuracy scorecard contract and denominator inventory.
//!
//! The implementation starts with denominator rows and then adds accuracy
//! scoring layers in small, schema-valid slices.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail, eyre};
use perl_lsp_rs_core::providers::completion::CompletionProvider;
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_lsp_rs_core::providers::navigation::definition_shadow::{
    DefinitionCutoverResult, goto_definition_cutover,
};
use perl_lsp_rs_core::providers::navigation::hover_shadow::{HoverCutoverResult, hover_cutover};
use perl_lsp_rs_core::providers::navigation::references_shadow::{
    ReferencesCutoverResult, find_references_cutover,
};
use perl_lsp_rs_core::providers::navigation::rename_shadow::{RenameCutoverResult, rename_cutover};
use perl_lsp_rs_core::providers::navigation::safe_delete_shadow::{
    SafeDeleteCutoverResult, safe_delete_cutover,
};
use perl_parser::apply_edits;
use perl_parser::edit::Edit as CoreEdit;
use perl_parser::incremental_v2::IncrementalParserV2;
use perl_parser::position::Position;
use perl_parser::{
    Edit as TextEdit, IncrementalState, Node, NodeKind, ParseError, Parser, PositionMapper,
    TokenKind, TokenStream,
};
use perl_semantic_facts::{AnchorId, EntityId};
use perl_workspace::position::Range;
use perl_workspace::semantic::queries::QueryContext;
use perl_workspace::workspace::document_store::DocumentStore;
use perl_workspace::workspace::workspace_index::{FileFactShard, WorkspaceIndex};
use serde::{Deserialize, Serialize};

use crate::allocation_tracker::{get_current_memory_usage, measure_allocations};
use crate::tasks::metrics::ratchet::MetricReceipt;
use crate::utils::project_root;

mod failure_packet;

const DEFAULT_MANIFEST: &str = "crates/perl-corpus/fixtures/parser_accuracy/manifest.json";
const DEFAULT_OUTPUT: &str = "target/metrics/parser_accuracy.json";
const DEFAULT_RATCHET_RECEIPT: &str = "target/receipts/metrics/parser_accuracy.json";
const GOLD_BASELINE: &str = ".ci/metrics/baselines/parser_accuracy_gold.json";
const FAILURE_PACKET_STATUS_RECEIPT: &str =
    "docs/project/status/parser_accuracy_failure_packets.json";
const FIXTURE_INVENTORY_STATUS_RECEIPT: &str =
    "docs/project/status/parser_accuracy_fixture_inventory.json";
const FAILURE_WORKLIST_STATUS_RECEIPT: &str =
    "docs/project/status/parser_accuracy_failure_worklist.md";
const NEXT_POINTER_STATUS_RECEIPT: &str = "docs/project/status/parser_accuracy_next.md";
const SAFETY_FLOOR_METRICS: &[(&str, f64)] =
    &[("dynamic_false_precision_count", 0.0), ("fast_path_wrong_result_count", 0.0)];
const DEFERRED_PRECISION_RECALL_CANDIDATES: &[&str] = &[
    "line_construct_precision",
    "line_construct_recall",
    "line_construct_f1",
    "ast_node_kind_precision",
    "ast_node_kind_recall",
    "ast_node_kind_f1",
    "symbol_decl_precision",
    "symbol_decl_recall",
    "symbol_decl_f1",
    "symbol_ref_precision",
    "symbol_ref_recall",
    "symbol_ref_f1",
    "symbol_edge_precision",
    "symbol_edge_recall",
    "symbol_edge_f1",
];

#[derive(Debug, Clone, Deserialize)]
struct ParserAccuracyManifest {
    schema_version: u32,
    fixtures: Vec<FixtureMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoldBaseline {
    schema_version: u32,
    expectation_signatures: BTreeSet<String>,
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
    #[serde(default)]
    recovery_expectations: Vec<RecoveryExpectation>,
    #[serde(default)]
    incremental_expectations: Vec<IncrementalExpectation>,
    #[serde(default)]
    span_expectations: Vec<SpanExpectation>,
    #[serde(default)]
    provider_expectations: ProviderExpectations,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AstDelimiterPair {
    open: char,
    close: char,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct ProviderExpectations {
    #[serde(default)]
    method_completion: Vec<MethodCompletionProviderExpectation>,
    #[serde(default)]
    diagnostics: Vec<DiagnosticProviderExpectation>,
    #[serde(default)]
    navigation: Vec<NavigationProviderExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MethodCompletionProviderExpectation {
    id: String,
    cursor_marker: String,
    expected_receiver_package: Option<String>,
    #[serde(default)]
    expected_present: Vec<String>,
    #[serde(default)]
    expected_absent: Vec<String>,
    expected_fallback: bool,
    #[serde(default)]
    import_visibility: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct DiagnosticProviderExpectation {
    id: String,
    expected_code: String,
    message_contains: String,
    expected_present: bool,
    #[serde(default)]
    dynamic_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct NavigationProviderExpectation {
    id: String,
    symbol: String,
    #[serde(default)]
    cursor_marker: Option<String>,
    #[serde(default)]
    cursor_symbol: Option<String>,
    #[serde(default)]
    expected_document_symbols: Vec<String>,
    #[serde(default)]
    expected_definition_span: Option<String>,
    #[serde(default)]
    expected_references: Vec<String>,
    #[serde(default)]
    unexpected_references: Vec<String>,
    #[serde(default)]
    hover_contains: Vec<String>,
    #[serde(default)]
    rename_new_name: Option<String>,
    #[serde(default)]
    expected_rename_safe_edit: Option<bool>,
    #[serde(default)]
    expected_rename_edit_count: Option<u64>,
    #[serde(default)]
    expected_safe_delete_blocked: Option<bool>,
    #[serde(default)]
    expected_safe_delete_blocker_count: Option<u64>,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct RecoveryExpectation {
    id: String,
    first_error_line: u64,
    error_region: LineRange,
    recovery_line: u64,
    #[serde(default)]
    post_error_line_expectations: Vec<LineExpectation>,
    #[serde(default)]
    post_error_symbol_spans: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
struct LineRange {
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct IncrementalExpectation {
    id: String,
    #[serde(default)]
    edits: Vec<IncrementalEditExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct IncrementalEditExpectation {
    old_text: String,
    new_text: String,
    #[serde(default)]
    occurrence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SpanExpectation {
    id: String,
    span_text: String,
    #[serde(default)]
    occurrence: Option<u64>,
    byte_start: usize,
    byte_end: usize,
    line_start: u64,
    line_end: u64,
    utf16_start: SpanPositionExpectation,
    utf16_end: SpanPositionExpectation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
struct SpanPositionExpectation {
    line: u32,
    character: u32,
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
    subsystem: String,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        previous: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        floor: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f64>,
        sample_count: u64,
        direction: Direction,
        confidence: Confidence,
        cadence: Cadence,
        #[serde(skip_serializing_if = "Option::is_none")]
        macro_value: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        micro_value: Option<f64>,
    },
    InsufficientData {
        metric: String,
        reason: String,
        sample_count: u64,
        confidence: Confidence,
    },
}

impl MetricRow {
    fn name(&self) -> &str {
        match self {
            MetricRow::Measured { metric, .. } | MetricRow::InsufficientData { metric, .. } => {
                metric
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    Up,
    Down,
    Flat,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    expected: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    actual: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    nearest_predictions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_next_fix: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct GoldDrift {
    schema_error_count: u64,
    span_error_count: u64,
    duplicate_symbol_id_count: u64,
    missing_resolves_to_target_count: u64,
    changed_line_count: u64,
    changed_line_sample_count: u64,
    changed_symbol_count: u64,
    removed_expectation_count: u64,
    added_expectation_count: u64,
    added_expectation_sample_count: u64,
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
    #[serde(skip_serializing_if = "is_zero_u64")]
    cache_sample_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_rss_mb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allocated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allocation_count: Option<u64>,
}

#[derive(Debug, Default)]
struct MetricSourceCache {
    sources: BTreeMap<PathBuf, String>,
    hit_count: u64,
    miss_count: u64,
}

impl MetricSourceCache {
    fn read<'a>(&'a mut self, path: &Path, label: &str) -> Result<&'a str> {
        let key = path.to_path_buf();
        match self.sources.entry(key) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                self.hit_count += 1;
                Ok(entry.into_mut().as_str())
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.miss_count += 1;
                let source = fs::read_to_string(path).with_context(|| {
                    format!("reading parser accuracy {label} {}", path.display())
                })?;
                Ok(entry.insert(source).as_str())
            }
        }
    }

    fn hit_rate(&self) -> Option<f64> {
        ratio(self.hit_count, self.hit_count + self.miss_count)
    }

    fn sample_count(&self) -> u64 {
        self.hit_count + self.miss_count
    }
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
    proof_score: ProofScore,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProofScore {
    true_positive_by_bucket: BTreeMap<ProofBucket, u64>,
    predicted_by_bucket: BTreeMap<ProofBucket, u64>,
    high_confidence_false_positive_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ProofBucket {
    Exact,
    High,
    Medium,
    Low,
    Heuristic,
    Dynamic,
}

impl ProofBucket {
    fn precision_metric(self) -> &'static str {
        match self {
            ProofBucket::Exact => "exact_fact_precision",
            ProofBucket::High => "high_confidence_precision",
            ProofBucket::Medium => "medium_confidence_precision",
            ProofBucket::Low => "low_confidence_precision",
            ProofBucket::Heuristic => "heuristic_fact_precision",
            ProofBucket::Dynamic => "dynamic_boundary_precision",
        }
    }

    fn insufficient_reason(self) -> &'static str {
        match self {
            ProofBucket::Exact => {
                "no exact fact predictions are available in fully labeled fixtures"
            }
            ProofBucket::High => {
                "no high-confidence fact predictions are available in fully labeled fixtures"
            }
            ProofBucket::Medium => {
                "no medium-confidence fact predictions are available in fully labeled fixtures"
            }
            ProofBucket::Low => {
                "no low-confidence fact predictions are available in fully labeled fixtures"
            }
            ProofBucket::Heuristic => {
                "no heuristic fact predictions are available in fully labeled fixtures"
            }
            ProofBucket::Dynamic => {
                "no dynamic-boundary fact predictions are available in fully labeled fixtures"
            }
        }
    }
}

trait ProofShape {
    fn provenance(&self) -> &str;
    fn confidence(&self) -> &str;
}

impl ProofShape for SymbolEntityKey {
    fn provenance(&self) -> &str {
        &self.provenance
    }

    fn confidence(&self) -> &str {
        &self.confidence
    }
}

impl ProofShape for SymbolOccurrenceKey {
    fn provenance(&self) -> &str {
        &self.provenance
    }

    fn confidence(&self) -> &str {
        &self.confidence
    }
}

impl ProofShape for SymbolEdgeKey {
    fn provenance(&self) -> &str {
        &self.provenance
    }

    fn confidence(&self) -> &str {
        &self.confidence
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct KindScore {
    expected_count: u64,
    predicted_count: u64,
    true_positive_count: u64,
    false_positive_count: u64,
    false_negative_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RecoveryScore {
    expectation_count: u64,
    first_error_line_correct_count: u64,
    error_region_true_positive_count: u64,
    error_region_false_positive_count: u64,
    error_region_false_negative_count: u64,
    spillover_lines: Vec<u64>,
    recovery_parse_micros: Vec<u64>,
    post_error_line_score: LineScore,
    post_error_symbol_expected_count: u64,
    post_error_symbol_found_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct IncrementalScore {
    expectation_count: u64,
    full_parse_equivalent_count: u64,
    edit_apply_equivalent_count: u64,
    no_panic_count: u64,
    no_progress_count: u64,
    timeout_count: u64,
    fallback_count: u64,
    checkpoint_hit_count: u64,
    checkpoint_miss_count: u64,
    content_hash_hit_count: u64,
    content_hash_miss_count: u64,
    semantic_fact_cache_hit_count: u64,
    semantic_fact_cache_miss_count: u64,
    workspace_shard_reuse_count: u64,
    workspace_shard_replacement_attempt_count: u64,
    unchanged_file_skip_count: u64,
    unchanged_file_index_attempt_count: u64,
    reparse_byte_ratios: Vec<f64>,
    reused_node_ratios: Vec<f64>,
    changed_range_sample_count: u64,
    changed_range_correct_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SpanScore {
    expectation_count: u64,
    byte_exact_count: u64,
    line_exact_count: u64,
    utf16_exact_count: u64,
    near_count: u64,
    invalid_count: u64,
    out_of_bounds_count: u64,
    inverted_count: u64,
    non_char_boundary_count: u64,
    crlf_sample_count: u64,
    crlf_position_error_count: u64,
    unicode_sample_count: u64,
    unicode_position_error_count: u64,
    tab_sample_count: u64,
    tab_column_mismatch_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct UnsupportedScore {
    manifest_construct_count: u64,
    family_count: u64,
    line_labeled_construct_count: u64,
    detected_count: u64,
    salvaged_count: u64,
    false_exact_count: u64,
    false_exact_sample_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MethodCompletionProviderScore {
    receiver_expected_count: u64,
    receiver_hit_count: u64,
    fallback_expected_count: u64,
    fallback_correct_count: u64,
    false_receiver_count: u64,
    relevance_assertion_count: u64,
    relevance_assertion_correct_count: u64,
    import_visibility_expected_count: u64,
    import_visibility_correct_count: u64,
    completion_query_micros: Vec<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DiagnosticProviderScore {
    dynamic_boundary_expected_absent_count: u64,
    dynamic_boundary_false_positive_count: u64,
    undefined_expected_absent_count: u64,
    undefined_false_positive_count: u64,
    undefined_expected_present_count: u64,
    undefined_false_negative_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NavigationProviderScore {
    document_symbol_expected_count: u64,
    document_symbol_returned_count: u64,
    document_symbol_span_exact_count: u64,
    goto_definition_expected_count: u64,
    goto_definition_hit_count: u64,
    goto_definition_span_exact_count: u64,
    goto_definition_false_target_count: u64,
    definition_query_micros: Vec<u64>,
    references_expected_count: u64,
    references_hit_count: u64,
    references_returned_count: u64,
    references_false_positive_count: u64,
    references_absent_assertion_count: u64,
    reference_query_micros: Vec<u64>,
    hover_expected_count: u64,
    hover_origin_correct_count: u64,
    rename_safe_edit_expected_count: u64,
    rename_safe_edit_correct_count: u64,
    safe_delete_blocker_expected_count: u64,
    safe_delete_blocker_correct_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ScaleCostScore {
    fixture_count: u64,
    file_bytes: u64,
    source_lines: u64,
    token_count: u64,
    ast_node_count: u64,
    symbol_count: u64,
    import_count: u64,
    export_count: u64,
    sub_count: u64,
    package_count: u64,
    max_nesting_depth: u64,
    max_brace_depth: u64,
    max_regex_length: u64,
    max_heredoc_body_bytes: u64,
    quote_like_count: u64,
    dynamic_boundary_count: u64,
    lex_ms: Vec<f64>,
    parse_ms: Vec<f64>,
    ast_projection_ms: Vec<f64>,
    semantic_extraction_ms: Vec<f64>,
    workspace_insert_ms: Vec<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DeterminismScore {
    fixture_count: u64,
    token_stream_stable_count: u64,
    parse_hash_stable_count: u64,
    ast_hash_stable_count: u64,
    semantic_fact_hash_stable_count: u64,
    diagnostic_hash_stable_count: u64,
    repeated_parse_stable_count: u64,
    comment_invariance_stable_count: u64,
    comment_invariance_sample_count: u64,
}

/// Run `cargo xtask metrics parser-accuracy`.
pub fn run(
    json: bool,
    check: bool,
    export_status_receipts: bool,
    manifest: Option<PathBuf>,
    output: Option<PathBuf>,
    cadence: &str,
) -> Result<()> {
    let root = project_root()?;
    let cadence = Cadence::parse(cadence)?;
    let manifest_path = manifest.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let output_path = output.unwrap_or_else(|| root.join(DEFAULT_OUTPUT));

    let (manifest, artifact) = build_status_artifact(&root, &manifest_path, cadence)?;

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
        write_ratchet_receipt(&root, &artifact)?;
        println!("parser accuracy artifact written: {}", output_path.display());
    } else if export_status_receipts {
        write_ratchet_receipt(&root, &artifact)?;
    } else {
        print_summary(&artifact);
    }

    if export_status_receipts {
        write_status_receipts(&root, &manifest, &artifact)?;
    }

    Ok(())
}

pub fn refresh_default_artifact_for_status(root: &Path) -> Result<()> {
    let manifest_path = root.join(DEFAULT_MANIFEST);
    let output_path = root.join(DEFAULT_OUTPUT);
    let (_manifest, artifact) = build_status_artifact(root, &manifest_path, Cadence::Pr)?;
    validate_artifact_contract(&artifact)?;
    write_artifact(&output_path, &artifact)?;
    write_ratchet_receipt(root, &artifact)?;
    println!("parser accuracy artifact written: {}", output_path.display());
    Ok(())
}

fn build_status_artifact(
    root: &Path,
    manifest_path: &Path,
    cadence: Cadence,
) -> Result<(ParserAccuracyManifest, ParserAccuracyArtifact)> {
    let start = Instant::now();
    let manifest = read_manifest(root, manifest_path)?;
    let rss_before = get_current_memory_usage().ok();
    let (artifact, allocation_measurement) =
        measure_allocations(|| build_artifact(root, &manifest, cadence));
    let rss_after = get_current_memory_usage().ok();
    let mut artifact = artifact?;
    artifact.metric_runtime.runtime_ms = start.elapsed().as_secs_f64() * 1000.0;
    artifact.metric_runtime.peak_rss_mb =
        measured_memory_mb(rss_before, rss_after, allocation_measurement.peak_delta_mb());
    artifact.metric_runtime.allocated_bytes = Some(allocation_measurement.allocated_bytes);
    artifact.metric_runtime.allocation_count = Some(allocation_measurement.allocation_count);
    settle_artifact_size(&mut artifact)?;
    sync_allocation_metric_rows(&mut artifact, cadence);
    sync_runtime_metric_rows(&mut artifact, cadence);
    Ok((manifest, artifact))
}

fn measured_memory_mb(
    rss_before: Option<f64>,
    rss_after: Option<f64>,
    fallback_mb: f64,
) -> Option<f64> {
    if let (Some(before), Some(after)) = (rss_before, rss_after) {
        if after > before {
            return Some(after - before);
        }
    }
    (fallback_mb > 0.0).then_some(fallback_mb)
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

fn read_gold_baseline(root: &Path) -> Result<Option<GoldBaseline>> {
    let baseline_path = root.join(GOLD_BASELINE);
    if !baseline_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&baseline_path).with_context(|| {
        format!("reading parser accuracy gold baseline {}", baseline_path.display())
    })?;
    let baseline: GoldBaseline = serde_json::from_str(&raw).with_context(|| {
        format!("parsing parser accuracy gold baseline {}", baseline_path.display())
    })?;
    if baseline.schema_version != 1 {
        bail!(
            "unsupported parser accuracy gold baseline schema_version {}",
            baseline.schema_version
        );
    }
    Ok(Some(baseline))
}

fn build_artifact(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    cadence: Cadence,
) -> Result<ParserAccuracyArtifact> {
    let denominator = compute_denominator(manifest);
    let families = summarize_families(manifest);
    let fixture_count = denominator.fixture_count as f64;
    let mut source_cache = MetricSourceCache::default();
    let line_score = score_manifest_line_tags(root, manifest, &mut source_cache)?;
    let ast_score = score_manifest_ast(root, manifest, &mut source_cache)?;
    let symbol_score = score_manifest_symbols(root, manifest, &mut source_cache)?;
    let recovery_score = score_manifest_recovery(root, manifest, &mut source_cache)?;
    let incremental_score = score_manifest_incremental(root, manifest, &mut source_cache)?;
    let span_score = score_manifest_spans(root, manifest, &mut source_cache)?;
    let unsupported_score =
        score_manifest_unsupported(root, manifest, &line_score, &mut source_cache)?;
    let method_completion_provider_score =
        score_method_completion_provider_expectations(root, manifest, &mut source_cache)?;
    let diagnostic_provider_score =
        score_diagnostic_provider_expectations(root, manifest, &mut source_cache)?;
    let navigation_provider_score =
        score_navigation_provider_expectations(root, manifest, &mut source_cache)?;
    let scale_cost_score = score_manifest_scale_cost(root, manifest, &mut source_cache)?;
    let determinism_score = score_manifest_determinism(root, manifest, &mut source_cache)?;
    let gold_drift = audit_gold_drift(root, manifest, &mut source_cache)?;
    let mut metrics = vec![measured_value(
        "denominator_fixture_count",
        fixture_count,
        denominator.fixture_count,
        cadence,
    )];
    metrics.extend(line_metrics(&line_score, cadence));
    metrics.extend(ast_metrics(&ast_score, cadence));
    metrics.extend(symbol_metrics(&symbol_score, cadence));
    metrics.extend(safety_metrics(&line_score, &symbol_score, cadence));
    metrics.extend(recovery_metrics(&recovery_score, cadence));
    metrics.extend(incremental_metrics(&incremental_score, cadence));
    metrics.extend(span_metrics(&span_score, cadence));
    metrics.extend(confidence_metrics(&symbol_score, cadence));
    metrics.extend(unsupported_metrics(&unsupported_score, cadence));
    metrics.extend(provider_impact_metrics(
        &method_completion_provider_score,
        &diagnostic_provider_score,
        &navigation_provider_score,
        cadence,
    ));
    metrics.extend(scale_metrics(&scale_cost_score, cadence));
    metrics.extend(cost_metrics(
        &scale_cost_score,
        &recovery_score,
        &method_completion_provider_score,
        &navigation_provider_score,
        cadence,
    ));
    metrics.extend(cache_reuse_metrics(&incremental_score, cadence));
    metrics.extend(determinism_metrics(&determinism_score, cadence));
    metrics.extend(gold_drift_metrics(&gold_drift, denominator.fixture_count, cadence));
    apply_safety_floor_metadata(&mut metrics);

    Ok(ParserAccuracyArtifact {
        schema_version: 1,
        subsystem: "parser_accuracy".to_string(),
        generated_at: Utc::now().to_rfc3339(),
        commit: git_commit(root),
        cadence,
        denominator,
        families,
        metrics,
        failure_packets: failure_packet::collect_failure_packets(root, manifest)?,
        gold_drift,
        metric_runtime: MetricRuntime {
            cache_hit_rate: source_cache.hit_rate(),
            cache_sample_count: source_cache.sample_count(),
            ..MetricRuntime::default()
        },
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

fn score_manifest_line_tags(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<LineScore> {
    let mut score = LineScore::default();
    for fixture in &manifest.fixtures {
        if fixture.line_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "fixture source")?;
        let actual_by_line = extract_line_tags(source);
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
        if byte == b'\n' {
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
        NodeKind::Use { .. } | NodeKind::No { .. } => Some(LineTag::Import),
        NodeKind::FunctionCall { name, .. } if name == "require" => Some(LineTag::Import),
        NodeKind::FunctionCall { .. } => Some(LineTag::FunctionCall),
        NodeKind::MethodCall { .. } => Some(LineTag::MethodCall),
        NodeKind::Eval { .. } => Some(LineTag::FunctionCall),
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

fn score_manifest_recovery(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<RecoveryScore> {
    let mut score = RecoveryScore::default();
    for fixture in &manifest.fixtures {
        if fixture.recovery_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "recovery fixture source")?;
        let (prediction, recovery_micros) = extract_recovery_prediction(&source_path, source);
        score.recovery_parse_micros.push(recovery_micros);
        for expectation in &fixture.recovery_expectations {
            score_recovery_expectation(expectation, &prediction, &mut score);
        }
    }
    Ok(score)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RecoveryPrediction {
    first_error_line: Option<u64>,
    error_region_lines: BTreeSet<u64>,
    actual_by_line: BTreeMap<u64, BTreeSet<LineTag>>,
    symbol_spans: BTreeSet<SymbolSpanLocation>,
}

fn extract_recovery_prediction(source_path: &Path, source: &str) -> (RecoveryPrediction, u64) {
    let mut parser = Parser::new(source);
    let recovery_start = Instant::now();
    let output = parser.parse_with_recovery();
    let recovery_micros = recovery_start.elapsed().as_micros() as u64;
    let line_starts = line_starts(source);
    let mut error_lines = BTreeSet::new();
    for diagnostic in &output.diagnostics {
        if let Some(location) = parse_error_location(diagnostic) {
            error_lines.insert(line_for_offset(&line_starts, location));
        }
    }
    collect_error_node_lines(&output.ast, &line_starts, &mut error_lines);

    let mut actual_by_line = BTreeMap::new();
    collect_node_line_tags(&output.ast, &line_starts, &mut actual_by_line);
    let symbol_spans = extract_symbol_predictions(source_path, source)
        .map(|predictions| predictions.safety_spans)
        .unwrap_or_default();

    (
        RecoveryPrediction {
            first_error_line: error_lines.first().copied(),
            error_region_lines: error_lines,
            actual_by_line,
            symbol_spans,
        },
        recovery_micros,
    )
}

fn parse_error_location(error: &ParseError) -> Option<usize> {
    match error {
        ParseError::UnexpectedToken { location, .. }
        | ParseError::SyntaxError { location, .. }
        | ParseError::Recovered { location, .. } => Some(*location),
        _ => None,
    }
}

fn collect_error_node_lines(node: &Node, line_starts: &[usize], lines: &mut BTreeSet<u64>) {
    if matches!(node.kind, NodeKind::Error { .. }) {
        lines.insert(line_for_offset(line_starts, node.location.start));
    }
    node.for_each_child(|child| collect_error_node_lines(child, line_starts, lines));
}

fn score_recovery_expectation(
    expectation: &RecoveryExpectation,
    prediction: &RecoveryPrediction,
    score: &mut RecoveryScore,
) {
    score.expectation_count += 1;
    if prediction.first_error_line == Some(expectation.first_error_line) {
        score.first_error_line_correct_count += 1;
    }

    let expected_region = line_range_set(expectation.error_region);
    score.error_region_true_positive_count +=
        expected_region.intersection(&prediction.error_region_lines).count() as u64;
    score.error_region_false_positive_count +=
        prediction.error_region_lines.difference(&expected_region).count() as u64;
    score.error_region_false_negative_count +=
        expected_region.difference(&prediction.error_region_lines).count() as u64;

    let actual_end = prediction
        .error_region_lines
        .iter()
        .next_back()
        .copied()
        .unwrap_or(expectation.error_region.end);
    score.spillover_lines.push(actual_end.saturating_sub(expectation.error_region.end));

    for line_expectation in &expectation.post_error_line_expectations {
        let actual =
            prediction.actual_by_line.get(&line_expectation.line).cloned().unwrap_or_default();
        score_line_tags(&line_expectation.expected_tags, &actual, &mut score.post_error_line_score);
    }

    for span in &expectation.post_error_symbol_spans {
        score.post_error_symbol_expected_count += 1;
        if prediction
            .symbol_spans
            .iter()
            .any(|actual| actual.line >= expectation.recovery_line && actual.span_text == *span)
        {
            score.post_error_symbol_found_count += 1;
        }
    }
}

fn line_range_set(range: LineRange) -> BTreeSet<u64> {
    if range.end < range.start {
        return BTreeSet::new();
    }
    (range.start..=range.end).collect()
}

fn score_manifest_incremental(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<IncrementalScore> {
    let mut score = IncrementalScore::default();
    let content_hash_index = WorkspaceIndex::new();
    for fixture in &manifest.fixtures {
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "incremental fixture source")?;
        score_content_hash_reuse_probe(&content_hash_index, &source_path, source, &mut score)?;
        if fixture.incremental_expectations.is_empty() {
            continue;
        }
        for expectation in &fixture.incremental_expectations {
            score_incremental_expectation(source, expectation, &mut score);
        }
    }
    Ok(score)
}

fn score_content_hash_reuse_probe(
    index: &WorkspaceIndex,
    source_path: &Path,
    source: &str,
    score: &mut IncrementalScore,
) -> Result<()> {
    let source_path_text = source_path.to_string_lossy();
    let first_hit = content_hash_probe_hits(index, &source_path_text, source);
    let first_semantic_hit = semantic_fact_cache_probe_hits(index, &source_path_text);
    if index.index_file_str(&source_path_text, source).is_err() {
        return Ok(());
    }
    record_content_hash_probe(score, first_hit);
    record_semantic_fact_cache_probe(score, first_semantic_hit);
    record_unchanged_file_skip_probe(score, first_hit);
    record_workspace_shard_reuse_probe(index, &source_path_text, score);

    let second_hit = content_hash_probe_hits(index, &source_path_text, source);
    let second_semantic_hit = semantic_fact_cache_probe_hits(index, &source_path_text);
    index.index_file_str(&source_path_text, source).map_err(|err| {
        eyre!(
            "re-indexing parser accuracy fixture {} for content-hash metric: {err}",
            source_path.display()
        )
    })?;
    record_content_hash_probe(score, second_hit);
    record_semantic_fact_cache_probe(score, second_semantic_hit);
    record_unchanged_file_skip_probe(score, second_hit);

    Ok(())
}

fn content_hash_probe_hits(index: &WorkspaceIndex, source_path_text: &str, source: &str) -> bool {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let content_hash = hasher.finish();
    index.file_fact_shard(source_path_text).is_some_and(|shard| shard.content_hash == content_hash)
}

fn semantic_fact_cache_probe_hits(index: &WorkspaceIndex, source_path_text: &str) -> bool {
    index.file_fact_shard(source_path_text).is_some()
}

fn record_workspace_shard_reuse_probe(
    index: &WorkspaceIndex,
    source_path_text: &str,
    score: &mut IncrementalScore,
) {
    let Some(shard) = index.file_fact_shard(source_path_text) else {
        return;
    };
    let key = DocumentStore::uri_key(&shard.source_uri);
    let replacement = index.replace_fact_shard_incremental(&key, shard);
    score.workspace_shard_replacement_attempt_count += 1;
    if replacement.content_unchanged {
        score.workspace_shard_reuse_count += 1;
    }
}

fn record_content_hash_probe(score: &mut IncrementalScore, hit: bool) {
    if hit {
        score.content_hash_hit_count += 1;
    } else {
        score.content_hash_miss_count += 1;
    }
}

fn record_semantic_fact_cache_probe(score: &mut IncrementalScore, hit: bool) {
    if hit {
        score.semantic_fact_cache_hit_count += 1;
    } else {
        score.semantic_fact_cache_miss_count += 1;
    }
}

fn record_unchanged_file_skip_probe(score: &mut IncrementalScore, skipped: bool) {
    score.unchanged_file_index_attempt_count += 1;
    if skipped {
        score.unchanged_file_skip_count += 1;
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct IncrementalExpectationResult {
    full_parse_equivalent: bool,
    edit_apply_equivalent: bool,
    fallback_used: bool,
    checkpoint_hit_count: u64,
    checkpoint_miss_count: u64,
    reparse_byte_ratio: Option<f64>,
    reused_node_ratio: Option<f64>,
    changed_range_correct: Option<bool>,
}

fn score_incremental_expectation(
    source: &str,
    expectation: &IncrementalExpectation,
    score: &mut IncrementalScore,
) {
    score.expectation_count += 1;
    let outcome =
        catch_unwind(AssertUnwindSafe(|| run_incremental_expectation(source, expectation)));

    let result = match outcome {
        Ok(Ok(result)) => {
            score.no_panic_count += 1;
            result
        }
        Ok(Err(error)) => {
            score.no_panic_count += 1;
            if error.to_string().contains("did not advance") {
                score.no_progress_count += 1;
            }
            return;
        }
        Err(_) => return,
    };

    if result.full_parse_equivalent {
        score.full_parse_equivalent_count += 1;
    }
    if result.edit_apply_equivalent {
        score.edit_apply_equivalent_count += 1;
    }
    if result.fallback_used {
        score.fallback_count += 1;
    }
    score.checkpoint_hit_count += result.checkpoint_hit_count;
    score.checkpoint_miss_count += result.checkpoint_miss_count;
    if let Some(value) = result.reparse_byte_ratio {
        score.reparse_byte_ratios.push(value);
    }
    if let Some(value) = result.reused_node_ratio {
        score.reused_node_ratios.push(value);
    }
    if let Some(correct) = result.changed_range_correct {
        score.changed_range_sample_count += 1;
        if correct {
            score.changed_range_correct_count += 1;
        }
    }
}

fn run_incremental_expectation(
    source: &str,
    expectation: &IncrementalExpectation,
) -> Result<IncrementalExpectationResult> {
    let resolved_edits = resolve_incremental_edits(source, &expectation.edits)
        .with_context(|| format!("resolving incremental expectation '{}'", expectation.id))?;
    let expected_source = apply_resolved_edits(source, &resolved_edits)?;
    let mut parser = IncrementalParserV2::new();
    let _initial_ast = parser.parse(source)?;
    for edit in &resolved_edits {
        parser.edit(edit.core_edit.clone());
    }
    let incremental_ast = parser.parse(&expected_source)?;
    let mut full_parser = Parser::new(&expected_source);
    let full_ast = full_parser.parse()?;

    let apply_result = run_incremental_apply_path(source, &resolved_edits)?;
    let total_nodes = parser.reused_nodes + parser.reparsed_nodes;
    let reused_node_ratio =
        if total_nodes == 0 { None } else { Some(parser.reused_nodes as f64 / total_nodes as f64) };

    Ok(IncrementalExpectationResult {
        full_parse_equivalent: incremental_ast == full_ast,
        edit_apply_equivalent: apply_result.edit_apply_equivalent,
        fallback_used: apply_result.fallback_used,
        checkpoint_hit_count: apply_result.checkpoint_hit_count,
        checkpoint_miss_count: apply_result.checkpoint_miss_count,
        reparse_byte_ratio: apply_result.reparse_byte_ratio,
        reused_node_ratio,
        changed_range_correct: apply_result.changed_range_correct,
    })
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedIncrementalEdit {
    start_byte: usize,
    old_end_byte: usize,
    new_end_byte: usize,
    new_text: String,
    core_edit: CoreEdit,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct IncrementalApplyResult {
    edit_apply_equivalent: bool,
    fallback_used: bool,
    checkpoint_hit_count: u64,
    checkpoint_miss_count: u64,
    reparse_byte_ratio: Option<f64>,
    changed_range_correct: Option<bool>,
}

fn run_incremental_apply_path(
    source: &str,
    resolved_edits: &[ResolvedIncrementalEdit],
) -> Result<IncrementalApplyResult> {
    let mut state = IncrementalState::new(source.to_string());
    let expected_source = apply_resolved_edits(source, resolved_edits)?;
    let mut result = IncrementalApplyResult::default();
    let mut total_reparsed_bytes = 0usize;
    let mut changed_ranges_cover_edits = true;

    for edit in resolved_edits {
        if state.find_lex_checkpoint(edit.start_byte).is_some() {
            result.checkpoint_hit_count += 1;
        } else {
            result.checkpoint_miss_count += 1;
        }
        let text_edit = TextEdit {
            start_byte: edit.start_byte,
            old_end_byte: edit.old_end_byte,
            new_end_byte: edit.new_end_byte,
            new_text: edit.new_text.clone(),
        };
        let reparse =
            apply_edits(&mut state, &[text_edit]).map_err(|error| eyre!(error.to_string()))?;
        if reparse
            .changed_ranges
            .iter()
            .any(|range| range.start == 0 && range.end == state.source.len())
        {
            result.fallback_used = true;
        }
        total_reparsed_bytes += reparse.reparsed_bytes;
        let expected_range = edit.start_byte..edit.new_end_byte;
        changed_ranges_cover_edits &= reparse
            .changed_ranges
            .iter()
            .any(|range| range.start <= expected_range.start && range.end >= expected_range.end);
    }

    result.edit_apply_equivalent = state.source == expected_source;
    if !resolved_edits.is_empty() {
        result.changed_range_correct = Some(changed_ranges_cover_edits);
    }
    if !state.source.is_empty() {
        result.reparse_byte_ratio = Some(total_reparsed_bytes as f64 / state.source.len() as f64);
    }
    Ok(result)
}

fn resolve_incremental_edits(
    source: &str,
    edits: &[IncrementalEditExpectation],
) -> Result<Vec<ResolvedIncrementalEdit>> {
    let mut current = source.to_string();
    let mut resolved = Vec::new();
    for edit in edits {
        let occurrence = edit.occurrence.unwrap_or(1);
        let start =
            find_text_occurrence(&current, &edit.old_text, occurrence).ok_or_else(|| {
                eyre!("could not find edit text '{}' occurrence {}", edit.old_text, occurrence)
            })?;
        let old_end = start + edit.old_text.len();
        let new_end = start + edit.new_text.len();
        let mut next = current.clone();
        next.replace_range(start..old_end, &edit.new_text);
        let core_edit = CoreEdit::new(
            start,
            old_end,
            new_end,
            position_at(&current, start)?,
            position_at(&current, old_end)?,
            position_at(&next, new_end)?,
        );
        resolved.push(ResolvedIncrementalEdit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: new_end,
            new_text: edit.new_text.clone(),
            core_edit,
        });
        current = next;
    }
    Ok(resolved)
}

fn apply_resolved_edits(source: &str, edits: &[ResolvedIncrementalEdit]) -> Result<String> {
    let mut current = source.to_string();
    for edit in edits {
        current.replace_range(edit.start_byte..edit.old_end_byte, &edit.new_text);
    }
    Ok(current)
}

fn find_text_occurrence(source: &str, needle: &str, occurrence: u64) -> Option<usize> {
    if needle.is_empty() || occurrence == 0 {
        return None;
    }
    let mut seen = 0u64;
    for (offset, _) in source.match_indices(needle) {
        seen += 1;
        if seen == occurrence {
            return Some(offset);
        }
    }
    None
}

fn position_at(source: &str, byte: usize) -> Result<Position> {
    if byte > source.len() || !source.is_char_boundary(byte) {
        bail!("byte offset {byte} is not a valid source boundary");
    }
    let mut position = Position::start();
    position.advance(&source[..byte]);
    Ok(position)
}

fn score_manifest_spans(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<SpanScore> {
    let mut score = SpanScore::default();
    for fixture in &manifest.fixtures {
        if fixture.span_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "span fixture source")?;
        for expectation in &fixture.span_expectations {
            score_span_expectation(source, expectation, &mut score);
        }
    }
    Ok(score)
}

fn score_manifest_unsupported(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    line_score: &LineScore,
    source_cache: &mut MetricSourceCache,
) -> Result<UnsupportedScore> {
    let mut score = UnsupportedScore {
        line_labeled_construct_count: line_score.expected_unsupported_construct_count,
        detected_count: line_score.correct_unsupported_construct_count,
        ..UnsupportedScore::default()
    };
    let mut families = BTreeSet::new();

    for fixture in &manifest.fixtures {
        if fixture.unsupported_constructs == 0 {
            continue;
        }
        score.manifest_construct_count += fixture.unsupported_constructs;
        families.insert(fixture.family.clone());

        if fixture.symbol_expectations.entities.is_empty()
            && fixture.symbol_expectations.occurrences.is_empty()
            && fixture.symbol_expectations.edges.is_empty()
        {
            continue;
        }

        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "unsupported fixture source")?;
        let predictions = extract_symbol_predictions(&source_path, source)?;
        score_unsupported_symbol_expectations(
            &fixture.symbol_expectations,
            &predictions,
            &mut score,
        );
    }

    score.family_count = families.len() as u64;
    Ok(score)
}

fn score_unsupported_symbol_expectations(
    expectations: &SymbolExpectations,
    predictions: &SymbolPredictions,
    score: &mut UnsupportedScore,
) {
    let expected_entities = expectations
        .entities
        .iter()
        .map(entity_key_from_expectation)
        .filter(is_conservative_symbol_entity)
        .collect::<BTreeSet<_>>();
    let expected_occurrences = expectations
        .occurrences
        .iter()
        .map(occurrence_key_from_expectation)
        .filter(is_conservative_symbol_occurrence)
        .collect::<BTreeSet<_>>();
    let expected_edges = expectations
        .edges
        .iter()
        .map(edge_key_from_expectation)
        .filter(is_conservative_symbol_edge)
        .collect::<BTreeSet<_>>();

    score.false_exact_sample_count +=
        (expected_entities.len() + expected_occurrences.len() + expected_edges.len()) as u64;
    score.salvaged_count += expected_entities.intersection(&predictions.entities).count() as u64;
    score.salvaged_count +=
        expected_occurrences.intersection(&predictions.occurrences).count() as u64;
    score.salvaged_count += expected_edges.intersection(&predictions.edges).count() as u64;

    for expected in &expected_entities {
        if predictions.entities.iter().any(|prediction| {
            prediction.span_text == expected.span_text
                && prediction.provenance == "ExactAst"
                && prediction.confidence == "High"
        }) {
            score.false_exact_count += 1;
        }
    }
    for expected in &expected_occurrences {
        if predictions.occurrences.iter().any(|prediction| {
            prediction.span_text == expected.span_text
                && prediction.canonical_name.is_some()
                && prediction.provenance == "ExactAst"
                && prediction.confidence == "High"
        }) {
            score.false_exact_count += 1;
        }
    }
    for expected in &expected_edges {
        if predictions.edges.iter().any(|prediction| {
            prediction.from == expected.from
                && prediction.to == expected.to
                && prediction.provenance == "ExactAst"
                && prediction.confidence == "High"
        }) {
            score.false_exact_count += 1;
        }
    }
}

fn is_conservative_symbol_entity(entity: &SymbolEntityKey) -> bool {
    entity.provenance != "ExactAst" || entity.confidence != "High"
}

fn is_conservative_symbol_occurrence(occurrence: &SymbolOccurrenceKey) -> bool {
    occurrence.provenance != "ExactAst" || occurrence.confidence != "High"
}

fn is_conservative_symbol_edge(edge: &SymbolEdgeKey) -> bool {
    edge.provenance != "ExactAst" || edge.confidence != "High"
}

fn score_method_completion_provider_expectations(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<MethodCompletionProviderScore> {
    let mut score = MethodCompletionProviderScore::default();

    for fixture in &manifest.fixtures {
        if fixture.provider_expectations.method_completion.is_empty() {
            continue;
        }

        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "provider fixture source")?;
        let provider_source = provider_completion_source(source)?;
        let index_source = provider_completion_index_source(source)?;

        let index = Arc::new(WorkspaceIndex::new());
        let source_path_text = source_path.to_string_lossy();
        index.index_file_str(&source_path_text, &index_source).map_err(|err| {
            eyre!("indexing parser accuracy provider fixture {}: {err}", source_path.display())
        })?;

        let mut parser = Parser::new(&provider_source);
        let output = parser.parse_with_recovery();
        let provider = CompletionProvider::new_with_index_and_source(
            &output.ast,
            &provider_source,
            Some(index),
        );

        for expectation in &fixture.provider_expectations.method_completion {
            let cursor = locate_cursor_marker(source, &expectation.cursor_marker)
                .with_context(|| format!("locating cursor marker for {}", expectation.id))?;
            let query_start = Instant::now();
            let completions = provider.get_completions(&provider_source, cursor);
            score.completion_query_micros.push(query_start.elapsed().as_micros() as u64);
            let labels = completions.iter().map(|item| item.label.clone()).collect::<BTreeSet<_>>();
            score_method_completion_expectation(expectation, &labels, &mut score);
        }
    }

    Ok(score)
}

fn score_method_completion_expectation(
    expectation: &MethodCompletionProviderExpectation,
    labels: &BTreeSet<String>,
    score: &mut MethodCompletionProviderScore,
) {
    let expects_fallback =
        expectation.expected_fallback || expectation.expected_receiver_package.is_none();

    if expects_fallback {
        score.fallback_expected_count += 1;
        if expectation.expected_absent.iter().any(|label| labels.contains(label)) {
            score.false_receiver_count += 1;
        } else {
            score.fallback_correct_count += 1;
        }
    } else {
        score.receiver_expected_count += 1;
        if expectation.expected_present.iter().any(|label| labels.contains(label)) {
            score.receiver_hit_count += 1;
        }
    }

    for label in &expectation.expected_present {
        score.relevance_assertion_count += 1;
        if labels.contains(label) {
            score.relevance_assertion_correct_count += 1;
        }
    }
    for label in &expectation.expected_absent {
        score.relevance_assertion_count += 1;
        if !labels.contains(label) {
            score.relevance_assertion_correct_count += 1;
        }
    }

    if expectation.import_visibility {
        score.import_visibility_expected_count += 1;
        let present_labels_match =
            expectation.expected_present.iter().all(|label| labels.contains(label));
        let absent_labels_match =
            expectation.expected_absent.iter().all(|label| !labels.contains(label));
        if present_labels_match && absent_labels_match {
            score.import_visibility_correct_count += 1;
        }
    }
}

fn score_diagnostic_provider_expectations(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<DiagnosticProviderScore> {
    let mut score = DiagnosticProviderScore::default();

    for fixture in &manifest.fixtures {
        if fixture.provider_expectations.diagnostics.is_empty() {
            continue;
        }

        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "diagnostic provider fixture source")?;
        let provider_source = provider_diagnostic_source(source)?;
        let index_source = provider_diagnostic_index_source(source)?;

        let index = WorkspaceIndex::new();
        let source_path_text = source_path.to_string_lossy();
        index.index_file_str(&source_path_text, &index_source).map_err(|err| {
            eyre!("indexing parser accuracy diagnostic fixture {}: {err}", source_path.display())
        })?;

        let mut parser = Parser::new(&provider_source);
        let output = parser.parse_with_recovery();
        let ast = Arc::new(output.ast);
        let provider = DiagnosticsProvider::new(&ast, provider_source.clone());
        let diagnostics = index
            .with_semantic_queries_for_uri(&source_path_text, |file_id, semantic_queries| {
                provider.get_diagnostics_with_path_and_semantics(
                    &ast,
                    &output.diagnostics,
                    &provider_source,
                    None,
                    &[],
                    Some(&source_path),
                    file_id,
                    &semantic_queries,
                )
            })
            .ok_or_else(|| {
                eyre!(
                    "missing semantic queries for parser accuracy diagnostic fixture {}",
                    source_path.display()
                )
            })?;

        for expectation in &fixture.provider_expectations.diagnostics {
            score_diagnostic_expectation(expectation, &diagnostics, &mut score);
        }
    }

    Ok(score)
}

fn score_diagnostic_expectation(
    expectation: &DiagnosticProviderExpectation,
    diagnostics: &[Diagnostic],
    score: &mut DiagnosticProviderScore,
) {
    let matched = diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some(expectation.expected_code.as_str())
            && diagnostic.message.contains(&expectation.message_contains)
    });

    if expectation.expected_present {
        score.undefined_expected_present_count += 1;
        if !matched {
            score.undefined_false_negative_count += 1;
        }
    } else if expectation.dynamic_boundary {
        score.dynamic_boundary_expected_absent_count += 1;
        if matched {
            score.dynamic_boundary_false_positive_count += 1;
        }
    } else {
        score.undefined_expected_absent_count += 1;
        if matched {
            score.undefined_false_positive_count += 1;
        }
    }
}

fn score_navigation_provider_expectations(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<NavigationProviderScore> {
    let mut score = NavigationProviderScore::default();

    for fixture in &manifest.fixtures {
        if fixture.provider_expectations.navigation.is_empty() {
            continue;
        }

        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "navigation provider fixture source")?;
        let provider_source = provider_navigation_source(source)?;
        let index_source = provider_navigation_index_source(source)?;

        let index = WorkspaceIndex::new();
        let source_path_text = source_path.to_string_lossy();
        index.index_file_str(&source_path_text, &index_source).map_err(|err| {
            eyre!("indexing parser accuracy navigation fixture {}: {err}", source_path.display())
        })?;
        let shard = index.file_fact_shard(&source_path_text).ok_or_else(|| {
            eyre!(
                "missing semantic fact shard for parser accuracy navigation fixture {}",
                source_path.display()
            )
        })?;
        let document_symbol_spans = navigation_document_symbol_spans(
            &provider_source,
            &index.file_symbols(&source_path_text),
        );
        let anchors_by_id =
            shard.anchors.iter().map(|anchor| (anchor.id, anchor)).collect::<BTreeMap<_, _>>();

        for expectation in &fixture.provider_expectations.navigation {
            score_navigation_document_symbols(expectation, &document_symbol_spans, &mut score);
            score_navigation_goto_definition(
                expectation,
                source,
                &index_source,
                &index,
                &source_path_text,
                &anchors_by_id,
                &mut score,
            )?;
            score_navigation_references(
                expectation,
                &index_source,
                &index,
                &source_path_text,
                &shard,
                &anchors_by_id,
                &mut score,
            )?;
            score_navigation_hover(expectation, source, &index, &source_path_text, &mut score)?;
            score_navigation_rename_safe_edit(
                expectation,
                &index,
                &source_path_text,
                &shard,
                &mut score,
            )?;
            score_navigation_safe_delete_blocker(
                expectation,
                &index,
                &source_path_text,
                &shard,
                &mut score,
            )?;
        }
    }

    Ok(score)
}

fn navigation_document_symbol_spans(
    source: &str,
    symbols: &[perl_workspace::workspace::workspace_index::WorkspaceSymbol],
) -> BTreeSet<String> {
    symbols
        .iter()
        .filter_map(|symbol| range_span_text(source, &symbol.range))
        .filter(|span| !span.trim().is_empty())
        .collect()
}

fn score_navigation_document_symbols(
    expectation: &NavigationProviderExpectation,
    document_symbol_spans: &BTreeSet<String>,
    score: &mut NavigationProviderScore,
) {
    if expectation.expected_document_symbols.is_empty() {
        return;
    }

    let expected_spans =
        expectation.expected_document_symbols.iter().cloned().collect::<BTreeSet<_>>();

    score.document_symbol_expected_count += expected_spans.len() as u64;
    score.document_symbol_returned_count += document_symbol_spans.len() as u64;

    for expected_span in &expected_spans {
        if document_symbol_spans.contains(expected_span) {
            score.document_symbol_span_exact_count += 1;
        }
    }
}

fn score_navigation_goto_definition(
    expectation: &NavigationProviderExpectation,
    source: &str,
    index_source: &str,
    index: &WorkspaceIndex,
    source_path_text: &str,
    anchors_by_id: &BTreeMap<AnchorId, &perl_semantic_facts::AnchorFact>,
    score: &mut NavigationProviderScore,
) -> Result<()> {
    let Some(expected_span) = expectation.expected_definition_span.as_ref() else {
        return Ok(());
    };
    score.goto_definition_expected_count += 1;

    let cursor_offset = navigation_cursor_offset(source, expectation)?;
    let actual_spans = index
        .with_semantic_queries_for_uri(source_path_text, |file_id, semantic_queries| {
            let context = QueryContext::new(file_id, None, cursor_offset);
            let query_start = Instant::now();
            let outcome =
                goto_definition_cutover(index, &semantic_queries, &expectation.symbol, &context);
            score.definition_query_micros.push(query_start.elapsed().as_micros() as u64);
            definition_result_spans(index_source, &outcome.result, anchors_by_id)
        })
        .ok_or_else(|| eyre!("missing semantic queries for navigation fixture"))?;

    if actual_spans.is_empty() {
        return Ok(());
    }

    score.goto_definition_hit_count += 1;
    if actual_spans.contains(expected_span) {
        score.goto_definition_span_exact_count += 1;
    } else {
        score.goto_definition_false_target_count += 1;
    }
    Ok(())
}

fn score_navigation_references(
    expectation: &NavigationProviderExpectation,
    index_source: &str,
    index: &WorkspaceIndex,
    source_path_text: &str,
    shard: &FileFactShard,
    anchors_by_id: &BTreeMap<AnchorId, &perl_semantic_facts::AnchorFact>,
    score: &mut NavigationProviderScore,
) -> Result<()> {
    if expectation.expected_references.is_empty() && expectation.unexpected_references.is_empty() {
        return Ok(());
    }

    let entity_id = resolve_navigation_entity_id(shard, &expectation.symbol)
        .with_context(|| format!("resolving navigation entity for {}", expectation.id))?;
    let actual_spans = index
        .with_semantic_queries_for_uri(source_path_text, |_file_id, semantic_queries| {
            let query_start = Instant::now();
            let outcome =
                find_references_cutover(index, &semantic_queries, &expectation.symbol, entity_id);
            score.reference_query_micros.push(query_start.elapsed().as_micros() as u64);
            reference_result_spans(index_source, &outcome.result, anchors_by_id)
        })
        .ok_or_else(|| eyre!("missing semantic queries for navigation fixture"))?;

    let expected_spans = expectation.expected_references.iter().cloned().collect::<BTreeSet<_>>();
    let unexpected_spans =
        expectation.unexpected_references.iter().cloned().collect::<BTreeSet<_>>();
    let true_positive_count = actual_spans.intersection(&expected_spans).count() as u64;
    let unexpected_hit_count = actual_spans.intersection(&unexpected_spans).count() as u64;
    let extra_count = actual_spans
        .difference(&expected_spans)
        .filter(|span| !unexpected_spans.contains(*span))
        .count() as u64;

    score.references_expected_count += expected_spans.len() as u64;
    score.references_hit_count += true_positive_count;
    score.references_returned_count += actual_spans.len() as u64;
    score.references_absent_assertion_count += unexpected_spans.len() as u64;
    score.references_false_positive_count += unexpected_hit_count + extra_count;
    Ok(())
}

fn score_navigation_hover(
    expectation: &NavigationProviderExpectation,
    source: &str,
    index: &WorkspaceIndex,
    source_path_text: &str,
    score: &mut NavigationProviderScore,
) -> Result<()> {
    if expectation.hover_contains.is_empty() {
        return Ok(());
    }
    score.hover_expected_count += 1;

    let cursor_offset = navigation_cursor_offset(source, expectation)?;
    let hover_symbol = expectation
        .cursor_symbol
        .as_deref()
        .unwrap_or_else(|| bare_symbol_name(&expectation.symbol));
    let markdown = index
        .with_semantic_queries_for_uri(source_path_text, |file_id, semantic_queries| {
            let byte_offset = cursor_offset?;
            let outcome =
                hover_cutover(None, &semantic_queries, hover_symbol, file_id, byte_offset, None);
            hover_result_markdown(&outcome.result)
        })
        .ok_or_else(|| eyre!("missing semantic queries for navigation fixture"))?;

    if let Some(markdown) = markdown
        && expectation.hover_contains.iter().all(|expected| markdown.contains(expected))
    {
        score.hover_origin_correct_count += 1;
    }
    Ok(())
}

fn score_navigation_rename_safe_edit(
    expectation: &NavigationProviderExpectation,
    index: &WorkspaceIndex,
    source_path_text: &str,
    shard: &FileFactShard,
    score: &mut NavigationProviderScore,
) -> Result<()> {
    let Some(expected_safe_edit) = expectation.expected_rename_safe_edit else {
        return Ok(());
    };
    let new_name = expectation
        .rename_new_name
        .as_deref()
        .ok_or_else(|| eyre!("rename expectation {} is missing rename_new_name", expectation.id))?;

    score.rename_safe_edit_expected_count += 1;

    let entity_id = resolve_navigation_entity_id(shard, &expectation.symbol)
        .with_context(|| format!("resolving rename entity for {}", expectation.id))?;
    let bare_old_name = bare_symbol_name(&expectation.symbol);
    let outcome = index
        .with_semantic_queries_for_uri(source_path_text, |_file_id, semantic_queries| {
            rename_cutover(true, &semantic_queries, entity_id, new_name)
        })
        .ok_or_else(|| eyre!("missing semantic queries for navigation rename fixture"))?;

    let allowed = matches!(&outcome.result, RenameCutoverResult::Allowed { .. });
    let edits: &[perl_semantic_facts::PlannedEdit] = match &outcome.result {
        RenameCutoverResult::Allowed { edits } => edits.as_slice(),
        RenameCutoverResult::Blocked { .. } => &[],
    };
    let edit_count_matches = match expectation.expected_rename_edit_count {
        Some(expected_count) => edits.len() as u64 == expected_count,
        None => true,
    };
    let actual_safe_edit = allowed
        && !edits.is_empty()
        && edit_count_matches
        && edits.iter().all(|edit| edit.old_text == bare_old_name && edit.new_text == new_name);

    if actual_safe_edit == expected_safe_edit {
        score.rename_safe_edit_correct_count += 1;
    }
    Ok(())
}

fn score_navigation_safe_delete_blocker(
    expectation: &NavigationProviderExpectation,
    index: &WorkspaceIndex,
    source_path_text: &str,
    shard: &FileFactShard,
    score: &mut NavigationProviderScore,
) -> Result<()> {
    let Some(expected_blocked) = expectation.expected_safe_delete_blocked else {
        return Ok(());
    };

    score.safe_delete_blocker_expected_count += 1;

    let entity_id = resolve_navigation_entity_id(shard, &expectation.symbol)
        .with_context(|| format!("resolving safe-delete entity for {}", expectation.id))?;
    let outcome = index
        .with_semantic_queries_for_uri(source_path_text, |_file_id, semantic_queries| {
            safe_delete_cutover(true, &semantic_queries, entity_id, &expectation.symbol)
        })
        .ok_or_else(|| eyre!("missing semantic queries for navigation safe-delete fixture"))?;

    let actual_blocker_count = match &outcome.result {
        SafeDeleteCutoverResult::Allowed => 0,
        SafeDeleteCutoverResult::Blocked { blockers } => blockers.len() as u64,
    };
    let actual_blocked = actual_blocker_count > 0;
    let blocker_count_matches = match expectation.expected_safe_delete_blocker_count {
        Some(expected_count) => actual_blocker_count == expected_count,
        None => true,
    };

    if actual_blocked == expected_blocked && blocker_count_matches {
        score.safe_delete_blocker_correct_count += 1;
    }
    Ok(())
}

fn definition_result_spans(
    source: &str,
    result: &DefinitionCutoverResult,
    anchors_by_id: &BTreeMap<AnchorId, &perl_semantic_facts::AnchorFact>,
) -> BTreeSet<String> {
    match result {
        DefinitionCutoverResult::Exact(candidate) => anchors_by_id
            .get(&candidate.anchor_id)
            .map(|anchor| BTreeSet::from([anchor_text(source, anchor)]))
            .unwrap_or_default(),
        DefinitionCutoverResult::Ambiguous(candidates) => candidates
            .iter()
            .filter_map(|candidate| anchors_by_id.get(&candidate.anchor_id))
            .map(|anchor| anchor_text(source, anchor))
            .collect(),
        DefinitionCutoverResult::LegacyFallback(location) => location
            .as_ref()
            .and_then(|location| range_span_text(source, &location.range))
            .map(|span| BTreeSet::from([span]))
            .unwrap_or_default(),
    }
}

fn reference_result_spans(
    source: &str,
    result: &ReferencesCutoverResult,
    anchors_by_id: &BTreeMap<AnchorId, &perl_semantic_facts::AnchorFact>,
) -> BTreeSet<String> {
    match result {
        ReferencesCutoverResult::Exact(occurrences)
        | ReferencesCutoverResult::Ambiguous(occurrences) => occurrences
            .iter()
            .filter_map(|occurrence| anchors_by_id.get(&occurrence.anchor_id))
            .map(|anchor| anchor_text(source, anchor))
            .collect(),
        ReferencesCutoverResult::LegacyFallback(locations) => locations
            .iter()
            .filter_map(|location| range_span_text(source, &location.range))
            .collect(),
    }
}

fn hover_result_markdown(result: &HoverCutoverResult) -> Option<String> {
    match result {
        HoverCutoverResult::Exact(explanation)
        | HoverCutoverResult::Ambiguous(explanation)
        | HoverCutoverResult::DynamicBoundary(explanation) => Some(explanation.markdown.clone()),
        HoverCutoverResult::LegacyFallback(markdown) => markdown.clone(),
    }
}

fn navigation_cursor_offset(
    source: &str,
    expectation: &NavigationProviderExpectation,
) -> Result<Option<u32>> {
    let Some(marker) = expectation.cursor_marker.as_deref() else {
        return Ok(None);
    };
    let cursor_symbol = expectation
        .cursor_symbol
        .as_deref()
        .unwrap_or_else(|| bare_symbol_name(&expectation.symbol));
    let offset = locate_navigation_cursor_marker(source, marker, cursor_symbol)
        .with_context(|| format!("locating navigation cursor marker for {}", expectation.id))?;
    Ok(Some(
        u32::try_from(offset).with_context(|| {
            format!("navigation cursor offset for {} exceeds u32", expectation.id)
        })?,
    ))
}

fn locate_navigation_cursor_marker(source: &str, marker: &str, symbol: &str) -> Result<usize> {
    let marker_offset =
        source.find(marker).ok_or_else(|| eyre!("cursor marker '{marker}' was not found"))?;
    let line_start = source[..marker_offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let comment_offset =
        source[line_start..marker_offset]
            .rfind('#')
            .map(|idx| line_start + idx)
            .ok_or_else(|| eyre!("cursor marker '{marker}' is not inside a line comment"))?;
    let line_prefix = &source[line_start..comment_offset];
    let symbol_offset = line_prefix
        .rfind(symbol)
        .ok_or_else(|| eyre!("cursor symbol '{symbol}' was not found before marker '{marker}'"))?;
    Ok(line_start + symbol_offset)
}

fn resolve_navigation_entity_id(shard: &FileFactShard, symbol: &str) -> Result<EntityId> {
    if let Some(entity) = shard.entities.iter().find(|entity| entity.canonical_name == symbol) {
        return Ok(entity.id);
    }

    let suffix = format!("::{symbol}");
    let mut candidates = shard
        .entities
        .iter()
        .filter(|entity| entity.canonical_name.ends_with(&suffix))
        .map(|entity| entity.id);
    let Some(first) = candidates.next() else {
        return Err(eyre!("symbol '{symbol}' was not found in navigation fact shard"));
    };
    if candidates.next().is_some() {
        return Err(eyre!("symbol '{symbol}' is ambiguous in navigation fact shard"));
    }
    Ok(first)
}

fn bare_symbol_name(symbol: &str) -> &str {
    match symbol.rsplit_once("::") {
        Some((_, bare)) => bare,
        None => symbol,
    }
}

fn range_span_text(source: &str, range: &Range) -> Option<String> {
    source.get(range.start.byte..range.end.byte).map(ToString::to_string)
}

fn provider_navigation_source(source: &str) -> Result<String> {
    let masked_support = mask_provider_index_support_blocks(source)?;
    mask_cursor_marker_comments(&masked_support)
}

fn provider_navigation_index_source(source: &str) -> Result<String> {
    mask_cursor_marker_comments(source)
}

fn provider_diagnostic_source(source: &str) -> Result<String> {
    let masked_support = mask_provider_index_support_blocks(source)?;
    mask_cursor_marker_comments(&masked_support)
}

fn provider_diagnostic_index_source(source: &str) -> Result<String> {
    mask_cursor_marker_comments(source)
}

fn provider_completion_source(source: &str) -> Result<String> {
    let masked_support = mask_provider_index_support_blocks(source)?;
    mask_cursor_marker_comments(&masked_support)
}

fn provider_completion_index_source(source: &str) -> Result<String> {
    let support_source = source_from_provider_index_support_blocks(source)?;
    mask_cursor_marker_comments(&support_source)
}

fn locate_cursor_marker(source: &str, marker: &str) -> Result<usize> {
    let marker_offset =
        source.find(marker).ok_or_else(|| eyre!("cursor marker '{marker}' was not found"))?;
    let line_start = source[..marker_offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let comment_offset =
        source[line_start..marker_offset]
            .rfind('#')
            .map(|idx| line_start + idx)
            .ok_or_else(|| eyre!("cursor marker '{marker}' is not inside a line comment"))?;

    let mut cursor = comment_offset;
    let bytes = source.as_bytes();
    while cursor > line_start && matches!(bytes[cursor - 1], b' ' | b'\t') {
        cursor -= 1;
    }
    Ok(cursor)
}

fn mask_cursor_marker_comments(source: &str) -> Result<String> {
    let mut ranges = Vec::new();
    let mut search_start = 0usize;
    while let Some(relative_marker) = source[search_start..].find("cursor:") {
        let marker_offset = search_start + relative_marker;
        let line_start = source[..marker_offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let Some(comment_offset) = source[line_start..marker_offset].rfind('#') else {
            search_start = marker_offset + "cursor:".len();
            continue;
        };
        let comment_offset = line_start + comment_offset;
        let line_end = source[marker_offset..]
            .find('\n')
            .map(|idx| marker_offset + idx)
            .unwrap_or(source.len());
        ranges.push(comment_offset..line_end);
        search_start = line_end;
    }
    mask_ranges_preserving_newlines(source, &ranges)
}

fn mask_provider_index_support_blocks(source: &str) -> Result<String> {
    mask_ranges_preserving_newlines(source, &provider_index_support_ranges(source)?)
}

fn source_from_provider_index_support_blocks(source: &str) -> Result<String> {
    let ranges = provider_index_support_ranges(source)?;
    if ranges.is_empty() {
        return Ok(source.to_string());
    }

    let mut bytes = source.as_bytes().to_vec();
    for byte in &mut bytes {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
    for range in ranges {
        bytes[range.clone()].copy_from_slice(&source.as_bytes()[range]);
    }
    String::from_utf8(bytes).context("provider index support source must remain utf-8")
}

fn provider_index_support_ranges(source: &str) -> Result<Vec<std::ops::Range<usize>>> {
    const START: &str = "# provider-index-support:start";
    const END: &str = "# provider-index-support:end";

    let mut ranges = Vec::new();
    let mut search_start = 0usize;
    while let Some(relative_start) = source[search_start..].find(START) {
        let start = search_start + relative_start;
        let after_start = start + START.len();
        let relative_end = source[after_start..]
            .find(END)
            .ok_or_else(|| eyre!("provider index support block is missing end marker"))?;
        let end_marker = after_start + relative_end;
        let end =
            source[end_marker..].find('\n').map(|idx| end_marker + idx + 1).unwrap_or(source.len());
        ranges.push(start..end);
        search_start = end;
    }
    Ok(ranges)
}

fn mask_ranges_preserving_newlines(
    source: &str,
    ranges: &[std::ops::Range<usize>],
) -> Result<String> {
    let mut bytes = source.as_bytes().to_vec();
    for range in ranges {
        if range.end > bytes.len() {
            bail!("mask range extends beyond source length");
        }
        for byte in &mut bytes[range.clone()] {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(bytes).context("masked provider source must remain utf-8")
}

fn score_manifest_scale_cost(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<ScaleCostScore> {
    let mut score = ScaleCostScore::default();
    for fixture in &manifest.fixtures {
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "scale fixture source")?;
        score.fixture_count += 1;
        score.file_bytes += source.len() as u64;
        score.source_lines += line_starts(source).len() as u64;
        score.max_brace_depth = score.max_brace_depth.max(max_brace_depth(source));
        score.export_count += source.matches("@EXPORT").count() as u64;
        score.export_count += source.matches("%EXPORT_TAGS").count() as u64;

        let lex_start = Instant::now();
        let tokens = collect_parser_tokens(source)?;
        score.lex_ms.push(lex_start.elapsed().as_secs_f64() * 1000.0);
        score.token_count += tokens.len() as u64;

        let parse_start = Instant::now();
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        score.parse_ms.push(parse_start.elapsed().as_secs_f64() * 1000.0);

        let ast_start = Instant::now();
        collect_scale_from_node(&output.ast, source, 0, &mut score);
        score.ast_projection_ms.push(ast_start.elapsed().as_secs_f64() * 1000.0);

        let semantic_start = Instant::now();
        let (predictions, workspace_insert_ms) =
            extract_symbol_predictions_with_insert_timing(&source_path, source)?;
        score.semantic_extraction_ms.push(semantic_start.elapsed().as_secs_f64() * 1000.0);
        score.workspace_insert_ms.push(workspace_insert_ms);
        score.symbol_count += (predictions.entities.len()
            + predictions.occurrences.len()
            + predictions.edges.len()) as u64;

        let actual_by_line = extract_line_tags(source);
        score.dynamic_boundary_count +=
            actual_by_line.values().filter(|tags| tags.contains(&LineTag::DynamicBoundary)).count()
                as u64;
    }
    Ok(score)
}

fn collect_parser_tokens(source: &str) -> Result<Vec<String>> {
    let mut stream = TokenStream::new(source);
    let mut tokens = Vec::new();
    loop {
        let token =
            stream.next().map_err(|err| eyre!("tokenizing parser accuracy fixture: {err}"))?;
        if token.kind == TokenKind::Eof {
            break;
        }
        tokens.push(format!("{:?}:{}", token.kind, token.text));
    }
    Ok(tokens)
}

fn collect_scale_from_node(node: &Node, source: &str, depth: u64, score: &mut ScaleCostScore) {
    score.ast_node_count += 1;
    score.max_nesting_depth = score.max_nesting_depth.max(depth);
    match &node.kind {
        NodeKind::Package { .. } => score.package_count += 1,
        NodeKind::Subroutine { .. } | NodeKind::Method { .. } => score.sub_count += 1,
        NodeKind::Use { .. } => score.import_count += 1,
        NodeKind::FunctionCall { name, .. } if name == "require" => score.import_count += 1,
        NodeKind::Regex { .. }
        | NodeKind::Match { .. }
        | NodeKind::Substitution { .. }
        | NodeKind::Transliteration { .. } => {
            score.quote_like_count += 1;
            score.max_regex_length = score.max_regex_length.max(node_span_len(source, node));
        }
        NodeKind::Heredoc { .. } => {
            score.max_heredoc_body_bytes =
                score.max_heredoc_body_bytes.max(node_span_len(source, node));
        }
        _ => {}
    }
    node.for_each_child(|child| collect_scale_from_node(child, source, depth + 1, score));
}

fn node_span_len(source: &str, node: &Node) -> u64 {
    if node.location.end <= source.len()
        && node.location.start <= node.location.end
        && source.is_char_boundary(node.location.start)
        && source.is_char_boundary(node.location.end)
    {
        (node.location.end - node.location.start) as u64
    } else {
        0
    }
}

fn max_brace_depth(source: &str) -> u64 {
    let mut current = 0_u64;
    let mut max_depth = 0_u64;
    for ch in source.chars() {
        match ch {
            '{' => {
                current += 1;
                max_depth = max_depth.max(current);
            }
            '}' => current = current.saturating_sub(1),
            _ => {}
        }
    }
    max_depth
}

fn score_manifest_determinism(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<DeterminismScore> {
    let mut score = DeterminismScore::default();
    for fixture in &manifest.fixtures {
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "determinism fixture source")?;
        score.fixture_count += 1;

        let first_tokens = collect_parser_tokens(source)?;
        let second_tokens = collect_parser_tokens(source)?;
        if stable_hash(&first_tokens) == stable_hash(&second_tokens) {
            score.token_stream_stable_count += 1;
        }

        let first_parse = parse_determinism_hashes(source);
        let second_parse = parse_determinism_hashes(source);
        if first_parse.parse_hash == second_parse.parse_hash {
            score.parse_hash_stable_count += 1;
            score.repeated_parse_stable_count += 1;
        }
        if first_parse.ast_hash == second_parse.ast_hash {
            score.ast_hash_stable_count += 1;
        }
        if first_parse.diagnostic_hash == second_parse.diagnostic_hash {
            score.diagnostic_hash_stable_count += 1;
        }

        let first_fact_hash = semantic_fact_hash(&source_path, source)?;
        let second_fact_hash = semantic_fact_hash(&source_path, source)?;
        if first_fact_hash == second_fact_hash {
            score.semantic_fact_hash_stable_count += 1;
        }

        if let Some(comment_variant) = comment_invariance_variant(source) {
            score.comment_invariance_sample_count += 1;
            if parser_accuracy_projection_signature(source)
                == parser_accuracy_projection_signature(&comment_variant)
            {
                score.comment_invariance_stable_count += 1;
            }
        }
    }
    Ok(score)
}

fn comment_invariance_variant(source: &str) -> Option<String> {
    if source.contains("<<") || source.contains("__DATA__") || source.contains("__END__") {
        return None;
    }

    let mut variant = String::with_capacity(source.len() + 43);
    variant.push_str(source);
    if !source.ends_with('\n') {
        variant.push('\n');
    }
    variant.push_str("# parser accuracy comment invariance probe\n");
    Some(variant)
}

fn parser_accuracy_projection_signature(source: &str) -> u64 {
    let line_tags = extract_line_tags(source);
    let ast_predictions = extract_ast_predictions(source);
    let diagnostic_hash = parse_determinism_hashes(source).diagnostic_hash;
    stable_hash(&format!("{line_tags:?}\n{ast_predictions:?}\n{diagnostic_hash}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParseDeterminismHashes {
    parse_hash: u64,
    ast_hash: u64,
    diagnostic_hash: u64,
}

fn parse_determinism_hashes(source: &str) -> ParseDeterminismHashes {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let ast_debug = format!("{:?}", output.ast);
    let diagnostics_debug = format!("{:?}", output.diagnostics);
    ParseDeterminismHashes {
        parse_hash: stable_hash(&(ast_debug.as_str(), diagnostics_debug.as_str())),
        ast_hash: stable_hash(&ast_debug),
        diagnostic_hash: stable_hash(&diagnostics_debug),
    }
}

fn semantic_fact_hash(source_path: &Path, source: &str) -> Result<u64> {
    let index = WorkspaceIndex::new();
    let source_path_text = source_path.to_string_lossy();
    index.index_file_str(&source_path_text, source).map_err(|err| {
        eyre!("indexing parser accuracy determinism fixture {}: {err}", source_path.display())
    })?;
    let shard = index.file_fact_shard(&source_path_text).ok_or_else(|| {
        eyre!(
            "missing canonical fact shard for parser accuracy determinism fixture {}",
            source_path.display()
        )
    })?;
    Ok(stable_hash(&format!("{:?}", shard)))
}

fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn audit_gold_drift(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<GoldDrift> {
    let mut drift = GoldDrift::default();
    let mut symbol_ids = BTreeSet::new();

    let current_expectations = gold_expectation_signatures(manifest);
    if let Some(baseline) = read_gold_baseline(root)? {
        drift.added_expectation_count =
            current_expectations.difference(&baseline.expectation_signatures).count() as u64;
        drift.added_expectation_sample_count = current_expectations.len() as u64;
        let current_line_expectations = line_gold_signatures(&current_expectations);
        let baseline_line_expectations = line_gold_signatures(&baseline.expectation_signatures);
        drift.changed_line_count = current_line_expectations
            .symmetric_difference(&baseline_line_expectations)
            .count() as u64;
        drift.changed_line_sample_count =
            current_line_expectations.union(&baseline_line_expectations).count() as u64;
    }

    for fixture in &manifest.fixtures {
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "gold fixture source")?;
        drift.span_error_count += count_span_expectation_errors(source, &fixture.span_expectations);
        drift.duplicate_symbol_id_count +=
            count_duplicate_symbol_ids(&fixture.symbol_expectations, &mut symbol_ids);
        drift.missing_resolves_to_target_count +=
            count_missing_edge_targets(&fixture.symbol_expectations);
    }

    Ok(drift)
}

fn gold_expectation_signatures(manifest: &ParserAccuracyManifest) -> BTreeSet<String> {
    let mut signatures = BTreeSet::new();
    for fixture in &manifest.fixtures {
        collect_fixture_gold_signatures(fixture, &mut signatures);
    }
    signatures
}

fn collect_fixture_gold_signatures(fixture: &FixtureMetadata, signatures: &mut BTreeSet<String>) {
    let fixture_id = fixture.id.as_str();
    for expectation in &fixture.line_expectations {
        for tag in &expectation.expected_tags {
            signatures.insert(format!("{fixture_id}::line::{}::{tag:?}", expectation.line));
        }
    }
    for expectation in &fixture.ast_expectations {
        signatures.insert(format!("{fixture_id}::ast::{}", expectation.id));
    }
    for expectation in &fixture.symbol_expectations.entities {
        signatures.insert(format!("{fixture_id}::symbol_entity::{}", expectation.id));
    }
    for expectation in &fixture.symbol_expectations.occurrences {
        signatures.insert(format!("{fixture_id}::symbol_occurrence::{}", expectation.id));
    }
    for expectation in &fixture.symbol_expectations.edges {
        signatures.insert(format!("{fixture_id}::symbol_edge::{}", expectation.id));
    }
    for expectation in &fixture.symbol_safety_regions {
        signatures.insert(format!(
            "{fixture_id}::symbol_safety::{:?}::{}::{}",
            expectation.kind, expectation.line, expectation.span_text
        ));
    }
    for expectation in &fixture.recovery_expectations {
        signatures.insert(format!("{fixture_id}::recovery::{}", expectation.id));
        for line_expectation in &expectation.post_error_line_expectations {
            for tag in &line_expectation.expected_tags {
                signatures.insert(format!(
                    "{fixture_id}::recovery::{}::post_line::{}::{tag:?}",
                    expectation.id, line_expectation.line
                ));
            }
        }
        for span in &expectation.post_error_symbol_spans {
            signatures
                .insert(format!("{fixture_id}::recovery::{}::post_symbol::{span}", expectation.id));
        }
    }
    for expectation in &fixture.incremental_expectations {
        signatures.insert(format!("{fixture_id}::incremental::{}", expectation.id));
    }
    for expectation in &fixture.span_expectations {
        signatures.insert(format!("{fixture_id}::span::{}", expectation.id));
    }
    for expectation in &fixture.provider_expectations.method_completion {
        signatures.insert(format!("{fixture_id}::provider_method_completion::{}", expectation.id));
    }
    for expectation in &fixture.provider_expectations.diagnostics {
        signatures.insert(format!("{fixture_id}::provider_diagnostic::{}", expectation.id));
    }
    for expectation in &fixture.provider_expectations.navigation {
        signatures.insert(format!("{fixture_id}::provider_navigation::{}", expectation.id));
    }
}

fn line_gold_signatures(signatures: &BTreeSet<String>) -> BTreeSet<String> {
    signatures
        .iter()
        .filter(|signature| signature.contains("::line::") || signature.contains("::post_line::"))
        .cloned()
        .collect()
}

fn count_span_expectation_errors(source: &str, expectations: &[SpanExpectation]) -> u64 {
    let mut error_count = 0;
    for expectation in expectations {
        if expectation.byte_end < expectation.byte_start
            || expectation.byte_end > source.len()
            || !source.is_char_boundary(expectation.byte_start)
            || !source.is_char_boundary(expectation.byte_end)
        {
            error_count += 1;
            continue;
        }
        let Some(actual) = source.get(expectation.byte_start..expectation.byte_end) else {
            error_count += 1;
            continue;
        };
        if actual != expectation.span_text {
            error_count += 1;
        }
    }
    error_count
}

fn count_duplicate_symbol_ids(
    expectations: &SymbolExpectations,
    seen: &mut BTreeSet<String>,
) -> u64 {
    let mut duplicate_count = 0;
    for id in expectations
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .chain(expectations.occurrences.iter().map(|occurrence| occurrence.id.as_str()))
        .chain(expectations.edges.iter().map(|edge| edge.id.as_str()))
    {
        if !seen.insert(id.to_string()) {
            duplicate_count += 1;
        }
    }
    duplicate_count
}

fn count_missing_edge_targets(expectations: &SymbolExpectations) -> u64 {
    let entity_names = expectations
        .entities
        .iter()
        .map(|entity| entity.canonical_name.as_str())
        .collect::<BTreeSet<_>>();
    expectations
        .edges
        .iter()
        .map(|edge| {
            u64::from(!entity_names.contains(edge.from.as_str()))
                + u64::from(!entity_names.contains(edge.to.as_str()))
        })
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActualSpanCoordinates {
    byte_start: usize,
    byte_end: usize,
    line_start: u64,
    line_end: u64,
    utf16_start: SpanPositionExpectation,
    utf16_end: SpanPositionExpectation,
}

fn score_span_expectation(source: &str, expectation: &SpanExpectation, score: &mut SpanScore) {
    score.expectation_count += 1;

    let has_crlf = source.contains("\r\n");
    let has_unicode = !expectation.span_text.is_ascii();
    let has_tab = expectation.span_text.contains('\t');
    if has_crlf {
        score.crlf_sample_count += 1;
    }
    if has_unicode {
        score.unicode_sample_count += 1;
    }
    if has_tab {
        score.tab_sample_count += 1;
    }

    if expectation.byte_end < expectation.byte_start {
        score.invalid_count += 1;
        score.inverted_count += 1;
        return;
    }
    if expectation.byte_end > source.len() {
        score.invalid_count += 1;
        score.out_of_bounds_count += 1;
        return;
    }
    if !source.is_char_boundary(expectation.byte_start)
        || !source.is_char_boundary(expectation.byte_end)
    {
        score.invalid_count += 1;
        score.non_char_boundary_count += 1;
        return;
    }

    let actual = match resolve_actual_span_coordinates(source, expectation) {
        Some(actual) => actual,
        None => {
            score.invalid_count += 1;
            return;
        }
    };

    let byte_exact =
        actual.byte_start == expectation.byte_start && actual.byte_end == expectation.byte_end;
    let line_exact =
        actual.line_start == expectation.line_start && actual.line_end == expectation.line_end;
    let utf16_exact =
        actual.utf16_start == expectation.utf16_start && actual.utf16_end == expectation.utf16_end;

    if byte_exact {
        score.byte_exact_count += 1;
    }
    if line_exact {
        score.line_exact_count += 1;
    }
    if utf16_exact {
        score.utf16_exact_count += 1;
    }
    if span_is_near(expectation, &actual) {
        score.near_count += 1;
    }
    if has_crlf && (!line_exact || !utf16_exact) {
        score.crlf_position_error_count += 1;
    }
    if has_unicode && !utf16_exact {
        score.unicode_position_error_count += 1;
    }
    if has_tab && !utf16_exact {
        score.tab_column_mismatch_count += 1;
    }
}

fn resolve_actual_span_coordinates(
    source: &str,
    expectation: &SpanExpectation,
) -> Option<ActualSpanCoordinates> {
    let (byte_start, byte_end) = if expectation.span_text.is_empty() {
        (expectation.byte_start, expectation.byte_end)
    } else {
        let occurrence = expectation.occurrence.unwrap_or(1);
        let byte_start = find_text_occurrence(source, &expectation.span_text, occurrence)?;
        (byte_start, byte_start + expectation.span_text.len())
    };

    if byte_end > source.len()
        || byte_end < byte_start
        || !source.is_char_boundary(byte_start)
        || !source.is_char_boundary(byte_end)
    {
        return None;
    }

    let line_starts = line_starts(source);
    let mapper = PositionMapper::new(source);
    let start = mapper.byte_to_lsp_pos(byte_start);
    let end = mapper.byte_to_lsp_pos(byte_end);

    Some(ActualSpanCoordinates {
        byte_start,
        byte_end,
        line_start: line_for_offset(&line_starts, byte_start),
        line_end: line_for_offset(&line_starts, byte_end),
        utf16_start: SpanPositionExpectation { line: start.line, character: start.character },
        utf16_end: SpanPositionExpectation { line: end.line, character: end.character },
    })
}

fn span_is_near(expectation: &SpanExpectation, actual: &ActualSpanCoordinates) -> bool {
    expectation.byte_start.abs_diff(actual.byte_start) <= 2
        && expectation.byte_end.abs_diff(actual.byte_end) <= 2
}

fn score_manifest_ast(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<AstScore> {
    let mut score = AstScore::default();
    for fixture in &manifest.fixtures {
        if fixture.ast_expectations.is_empty() {
            continue;
        }
        let source_path = root.join(&fixture.source_path);
        let source = source_cache.read(&source_path, "fixture source")?;
        let expected_lines = fixture
            .ast_expectations
            .iter()
            .map(|expectation| expectation.line)
            .collect::<BTreeSet<_>>();
        let predictions = extract_ast_predictions(source)
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

fn expected_ast_delimiter_pair(expectation: &AstExpectation) -> Option<AstDelimiterPair> {
    let pair = match expectation.kind.as_str() {
        "Subroutine" | "Format" => AstDelimiterPair { open: '{', close: '}' },
        "FunctionCall" | "MethodCall" => AstDelimiterPair { open: '(', close: ')' },
        _ => return None,
    };
    span_has_delimiter_pair(&expectation.span_text, pair).then_some(pair)
}

fn span_has_delimiter_pair(span_text: &str, pair: AstDelimiterPair) -> bool {
    match (span_text.find(pair.open), span_text.rfind(pair.close)) {
        (Some(open), Some(close)) => open < close,
        _ => false,
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
        let expected_delimiter_pair = expected_ast_delimiter_pair(expectation);
        let prediction_index = best_ast_prediction_index(expectation, predictions, &matched);

        let Some(prediction_index) = prediction_index else {
            if expected_delimiter_pair.is_some() {
                score.delimiter_pairing_expected_count += 1;
            }
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
        if let Some(pair) = expected_delimiter_pair {
            score.delimiter_pairing_expected_count += 1;
            if span_has_delimiter_pair(&prediction.span_text, pair) {
                score.delimiter_pairing_correct_count += 1;
            }
        }
    }

    score.node_kind_false_positive_count += predictions
        .iter()
        .enumerate()
        .filter(|(index, prediction)| !matched.contains(index) && prediction.kind != "Error")
        .count() as u64;
}

fn best_ast_prediction_index(
    expectation: &AstExpectation,
    predictions: &[AstPrediction],
    matched: &BTreeSet<usize>,
) -> Option<usize> {
    let mut best = None;
    for (index, prediction) in predictions.iter().enumerate() {
        if matched.contains(&index) {
            continue;
        }
        if prediction.kind != expectation.kind || prediction.line != expectation.line {
            continue;
        }

        let score = ast_prediction_match_score(expectation, prediction);
        if best.is_none_or(|(_, best_score)| score > best_score) {
            best = Some((index, score));
        }
    }
    best.map(|(index, _)| index)
}

fn ast_prediction_match_score(expectation: &AstExpectation, prediction: &AstPrediction) -> u8 {
    let mut score = 0;
    if prediction.span_text == expectation.span_text {
        score += 8;
    }
    if let Some(parent_kind) = &expectation.parent_kind
        && prediction.parent_kind.as_ref() == Some(parent_kind)
    {
        score += 4;
    }
    if let Some(depth) = expectation.depth
        && prediction.depth == depth
    {
        score += 2;
    }
    if let Some(operator) = &expectation.operator
        && prediction.operator.as_ref() == Some(operator)
    {
        score += 2;
    }
    if let Some(parent_operator) = &expectation.parent_operator
        && prediction.parent_operator.as_ref() == Some(parent_operator)
    {
        score += 1;
    }
    score
}

fn score_manifest_symbols(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    source_cache: &mut MetricSourceCache,
) -> Result<SymbolScore> {
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
        let source = source_cache.read(&source_path, "fixture source")?;
        let predictions = extract_symbol_predictions(&source_path, source)?;
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
    let (predictions, _) = extract_symbol_predictions_with_insert_timing(source_path, source)?;
    Ok(predictions)
}

fn extract_symbol_predictions_with_insert_timing(
    source_path: &Path,
    source: &str,
) -> Result<(SymbolPredictions, f64)> {
    let index = WorkspaceIndex::new();
    let source_path_text = source_path.to_string_lossy();
    let insert_start = Instant::now();
    index.index_file_str(&source_path_text, source).map_err(|err| {
        eyre!("indexing parser accuracy fixture {}: {err}", source_path.display())
    })?;
    let workspace_insert_ms = insert_start.elapsed().as_secs_f64() * 1000.0;
    let shard = index.file_fact_shard(&source_path_text).ok_or_else(|| {
        eyre!("missing canonical fact shard for parser accuracy fixture {}", source_path.display())
    })?;
    Ok((symbol_predictions_from_shard(source, &shard), workspace_insert_ms))
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
    score_proof_sets(
        &expected_entities,
        &predictions.entities,
        count_false_positives,
        &mut score.proof_score,
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
    score_proof_sets(
        &expected_occurrences,
        &predictions.occurrences,
        count_false_positives,
        &mut score.proof_score,
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
    score_proof_sets(
        &expected_edges,
        &predictions.edges,
        count_false_positives,
        &mut score.proof_score,
    );
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

fn score_proof_sets<T: Ord + ProofShape>(
    expected: &BTreeSet<T>,
    predicted: &BTreeSet<T>,
    count_false_positives: bool,
    proof_score: &mut ProofScore,
) {
    if !count_false_positives {
        return;
    }

    for prediction in predicted {
        for bucket in proof_buckets(prediction.provenance(), prediction.confidence()) {
            *proof_score.predicted_by_bucket.entry(bucket).or_default() += 1;
        }
    }

    for true_positive in expected.intersection(predicted) {
        for bucket in proof_buckets(true_positive.provenance(), true_positive.confidence()) {
            *proof_score.true_positive_by_bucket.entry(bucket).or_default() += 1;
        }
    }

    proof_score.high_confidence_false_positive_count += predicted
        .difference(expected)
        .filter(|false_positive| false_positive.confidence() == "High")
        .count() as u64;
}

fn proof_buckets(provenance: &str, confidence: &str) -> Vec<ProofBucket> {
    let mut buckets = Vec::new();
    if provenance == "ExactAst" {
        buckets.push(ProofBucket::Exact);
    }
    match confidence {
        "High" => buckets.push(ProofBucket::High),
        "Medium" => buckets.push(ProofBucket::Medium),
        "Low" => buckets.push(ProofBucket::Low),
        _ => {}
    }
    if provenance.contains("Heuristic")
        || provenance.contains("Fallback")
        || provenance == "FrameworkSynthesis"
    {
        buckets.push(ProofBucket::Heuristic);
    }
    if provenance == "DynamicBoundary" {
        buckets.push(ProofBucket::Dynamic);
    }
    buckets
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

fn recovery_metrics(score: &RecoveryScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.expectation_count == 0 {
        return vec![insufficient(
            "recovery_first_error_line_accuracy",
            "recovery gold labels are not available",
        )];
    }

    let region_precision_denominator =
        score.error_region_true_positive_count + score.error_region_false_positive_count;
    let region_recall_denominator =
        score.error_region_true_positive_count + score.error_region_false_negative_count;
    let post_line_precision_denominator = score.post_error_line_score.true_positive_count
        + score.post_error_line_score.false_positive_count;
    let post_line_recall_denominator = score.post_error_line_score.true_positive_count
        + score.post_error_line_score.false_negative_count;

    vec![
        measured_rate(
            "recovery_first_error_line_accuracy",
            score.first_error_line_correct_count,
            score.expectation_count,
            cadence,
        ),
        optional_measured_rate(
            "recovery_error_region_precision",
            ratio(score.error_region_true_positive_count, region_precision_denominator),
            region_precision_denominator,
            "no predicted recovery error-region lines are available",
            cadence,
        ),
        optional_measured_rate(
            "recovery_error_region_recall",
            ratio(score.error_region_true_positive_count, region_recall_denominator),
            region_recall_denominator,
            "no expected recovery error-region lines are available",
            cadence,
        ),
        optional_measured_value(
            "recovery_spillover_mean_lines",
            mean_u64(&score.spillover_lines),
            score.spillover_lines.len() as u64,
            "no recovery spillover samples are available",
            cadence,
        ),
        optional_measured_value(
            "recovery_spillover_p95_lines",
            p95_u64(&score.spillover_lines),
            score.spillover_lines.len() as u64,
            "no recovery spillover samples are available",
            cadence,
        ),
        optional_measured_value(
            "recovery_spillover_max_lines",
            score.spillover_lines.iter().copied().max().map(|value| value as f64),
            score.spillover_lines.len() as u64,
            "no recovery spillover samples are available",
            cadence,
        ),
        optional_measured_count(
            "recovery_salvaged_lines_after_error",
            score.post_error_line_score.exact_match_count,
            score.post_error_line_score.line_count,
            "no post-error line labels are available",
            cadence,
        ),
        optional_measured_count(
            "recovery_salvaged_symbols_after_error",
            score.post_error_symbol_found_count,
            score.post_error_symbol_expected_count,
            "no post-error symbol labels are available",
            cadence,
        ),
        optional_measured_rate(
            "recovery_post_error_symbol_recall",
            ratio(score.post_error_symbol_found_count, score.post_error_symbol_expected_count),
            score.post_error_symbol_expected_count,
            "no post-error symbol labels are available",
            cadence,
        ),
        optional_measured_rate(
            "recovery_post_error_line_f1",
            f1_from_counts(
                score.post_error_line_score.true_positive_count,
                score.post_error_line_score.false_positive_count,
                score.post_error_line_score.false_negative_count,
            ),
            post_line_recall_denominator.max(post_line_precision_denominator),
            "post-error line precision or recall denominator is unavailable",
            cadence,
        ),
    ]
}

fn incremental_metrics(score: &IncrementalScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.expectation_count == 0 {
        return vec![insufficient(
            "incremental_full_parse_equivalence_rate",
            "incremental equivalence gold labels are not available",
        )];
    }

    let checkpoint_sample_count = score.checkpoint_hit_count + score.checkpoint_miss_count;

    vec![
        measured_rate(
            "incremental_full_parse_equivalence_rate",
            score.full_parse_equivalent_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate(
            "incremental_edit_apply_equivalence_rate",
            score.edit_apply_equivalent_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate(
            "incremental_no_panic_rate",
            score.no_panic_count,
            score.expectation_count,
            cadence,
        ),
        measured_count(
            "incremental_no_progress_count",
            score.no_progress_count,
            score.expectation_count,
            cadence,
        ),
        measured_count(
            "incremental_timeout_count",
            score.timeout_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate(
            "incremental_full_reparse_fallback_rate",
            score.fallback_count,
            score.expectation_count,
            cadence,
        ),
        optional_measured_rate(
            "incremental_checkpoint_hit_rate",
            ratio(score.checkpoint_hit_count, checkpoint_sample_count),
            checkpoint_sample_count,
            "no incremental checkpoint probes are available",
            cadence,
        ),
        optional_measured_rate(
            "incremental_checkpoint_miss_rate",
            ratio(score.checkpoint_miss_count, checkpoint_sample_count),
            checkpoint_sample_count,
            "no incremental checkpoint probes are available",
            cadence,
        ),
        optional_measured_value(
            "incremental_reparse_byte_ratio_p95",
            p95_f64(&score.reparse_byte_ratios),
            score.reparse_byte_ratios.len() as u64,
            "no incremental reparse byte samples are available",
            cadence,
        ),
        insufficient(
            "incremental_reused_token_ratio",
            "incremental reparse result does not report token reuse yet",
        ),
        optional_measured_value(
            "incremental_reused_node_ratio",
            mean_f64(&score.reused_node_ratios),
            score.reused_node_ratios.len() as u64,
            "no incremental node reuse samples are available",
            cadence,
        ),
        optional_measured_rate(
            "incremental_changed_range_accuracy",
            ratio(score.changed_range_correct_count, score.changed_range_sample_count),
            score.changed_range_sample_count,
            "no incremental changed-range samples are available",
            cadence,
        ),
    ]
}

fn span_metrics(score: &SpanScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.expectation_count == 0 {
        return vec![insufficient("byte_span_exact_rate", "span gold labels are not available")];
    }

    vec![
        measured_rate(
            "byte_span_exact_rate",
            score.byte_exact_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate(
            "line_span_exact_rate",
            score.line_exact_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate(
            "utf16_range_exact_rate",
            score.utf16_exact_count,
            score.expectation_count,
            cadence,
        ),
        measured_rate("span_near_rate", score.near_count, score.expectation_count, cadence),
        measured_count("span_invalid_count", score.invalid_count, score.expectation_count, cadence),
        measured_count(
            "span_out_of_bounds_count",
            score.out_of_bounds_count,
            score.expectation_count,
            cadence,
        ),
        measured_count(
            "span_inverted_count",
            score.inverted_count,
            score.expectation_count,
            cadence,
        ),
        measured_count(
            "span_non_char_boundary_count",
            score.non_char_boundary_count,
            score.expectation_count,
            cadence,
        ),
        optional_measured_count(
            "crlf_position_error_count",
            score.crlf_position_error_count,
            score.crlf_sample_count,
            "no CRLF span samples are available",
            cadence,
        ),
        optional_measured_count(
            "unicode_position_error_count",
            score.unicode_position_error_count,
            score.unicode_sample_count,
            "no Unicode span samples are available",
            cadence,
        ),
        optional_measured_count(
            "tab_column_mismatch_count",
            score.tab_column_mismatch_count,
            score.tab_sample_count,
            "no tab span samples are available",
            cadence,
        ),
    ]
}

fn confidence_metrics(score: &SymbolScore, cadence: Cadence) -> Vec<MetricRow> {
    let mut rows = Vec::new();
    for bucket in [
        ProofBucket::Exact,
        ProofBucket::High,
        ProofBucket::Medium,
        ProofBucket::Low,
        ProofBucket::Heuristic,
        ProofBucket::Dynamic,
    ] {
        let true_positive_count =
            *score.proof_score.true_positive_by_bucket.get(&bucket).unwrap_or(&0);
        let predicted_count = *score.proof_score.predicted_by_bucket.get(&bucket).unwrap_or(&0);
        rows.push(optional_measured_rate(
            bucket.precision_metric(),
            ratio(true_positive_count, predicted_count),
            predicted_count,
            bucket.insufficient_reason(),
            cadence,
        ));
    }

    let high_confidence_predictions =
        *score.proof_score.predicted_by_bucket.get(&ProofBucket::High).unwrap_or(&0);
    rows.push(optional_measured_rate(
        "confidence_calibration_error",
        ratio(score.proof_score.high_confidence_false_positive_count, high_confidence_predictions),
        high_confidence_predictions,
        "no high-confidence fact predictions are available in fully labeled fixtures",
        cadence,
    ));
    rows
}

fn unsupported_metrics(score: &UnsupportedScore, cadence: Cadence) -> Vec<MetricRow> {
    vec![
        optional_measured_count(
            "unsupported_construct_detected_count",
            score.detected_count,
            score.line_labeled_construct_count,
            "no unsupported-construct line labels are available",
            cadence,
        ),
        optional_measured_count(
            "unsupported_construct_missed_count",
            score.line_labeled_construct_count.saturating_sub(score.detected_count),
            score.line_labeled_construct_count,
            "no unsupported-construct line labels are available",
            cadence,
        ),
        optional_measured_count(
            "unsupported_construct_family_count",
            score.family_count,
            score.manifest_construct_count,
            "no unsupported constructs are declared in the manifest",
            cadence,
        ),
        optional_measured_count(
            "unsupported_construct_false_exact_count",
            score.false_exact_count,
            score.false_exact_sample_count,
            "no conservative unsupported symbol labels are available",
            cadence,
        ),
        optional_measured_count(
            "unsupported_but_salvaged_count",
            score.salvaged_count,
            score.false_exact_sample_count,
            "no conservative unsupported symbol labels are available",
            cadence,
        ),
    ]
}

fn provider_impact_metrics(
    method_completion_score: &MethodCompletionProviderScore,
    diagnostic_score: &DiagnosticProviderScore,
    navigation_score: &NavigationProviderScore,
    cadence: Cadence,
) -> Vec<MetricRow> {
    const PROVIDER_METRICS: &[&str] = &[];

    let diagnostic_false_positive_count = diagnostic_score
        .dynamic_boundary_false_positive_count
        .saturating_add(diagnostic_score.undefined_false_positive_count);
    let diagnostic_expected_absent_count = diagnostic_score
        .dynamic_boundary_expected_absent_count
        .saturating_add(diagnostic_score.undefined_expected_absent_count);

    let mut rows = vec![
        optional_measured_rate(
            "method_completion_receiver_hit_rate",
            ratio(
                method_completion_score.receiver_hit_count,
                method_completion_score.receiver_expected_count,
            ),
            method_completion_score.receiver_expected_count,
            "no method-completion receiver expectations are available",
            cadence,
        ),
        optional_measured_count(
            "method_completion_false_receiver_count",
            method_completion_score.false_receiver_count,
            method_completion_score.fallback_expected_count,
            "no method-completion fallback expectations are available",
            cadence,
        ),
        optional_measured_count(
            "method_completion_dynamic_receiver_fallback_count",
            method_completion_score.fallback_correct_count,
            method_completion_score.fallback_expected_count,
            "no method-completion fallback expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "method_completion_visible_symbol_relevance",
            ratio(
                method_completion_score.relevance_assertion_correct_count,
                method_completion_score.relevance_assertion_count,
            ),
            method_completion_score.relevance_assertion_count,
            "no method-completion visible-symbol assertions are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_completion_visible_symbol_relevance",
            ratio(
                method_completion_score.relevance_assertion_correct_count,
                method_completion_score.relevance_assertion_count,
            ),
            method_completion_score.relevance_assertion_count,
            "no provider completion visible-symbol assertions are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_completion_import_visibility_accuracy",
            ratio(
                method_completion_score.import_visibility_correct_count,
                method_completion_score.import_visibility_expected_count,
            ),
            method_completion_score.import_visibility_expected_count,
            "no provider completion import-visibility expectations are available",
            cadence,
        ),
        optional_measured_count(
            "diagnostic_dynamic_boundary_false_positive_count",
            diagnostic_score.dynamic_boundary_false_positive_count,
            diagnostic_score.dynamic_boundary_expected_absent_count,
            "no diagnostic dynamic-boundary expectations are available",
            cadence,
        ),
        optional_measured_count(
            "diagnostic_undefined_symbol_false_positive_count",
            diagnostic_score.undefined_false_positive_count,
            diagnostic_score.undefined_expected_absent_count,
            "no diagnostic undefined-symbol false-positive expectations are available",
            cadence,
        ),
        optional_measured_count(
            "diagnostic_undefined_symbol_false_negative_count",
            diagnostic_score.undefined_false_negative_count,
            diagnostic_score.undefined_expected_present_count,
            "no diagnostic undefined-symbol false-negative expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_diagnostic_false_negative_rate",
            ratio(
                diagnostic_score.undefined_false_negative_count,
                diagnostic_score.undefined_expected_present_count,
            ),
            diagnostic_score.undefined_expected_present_count,
            "no provider diagnostic false-negative expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_diagnostic_false_positive_rate",
            ratio(diagnostic_false_positive_count, diagnostic_expected_absent_count),
            diagnostic_expected_absent_count,
            "no provider diagnostic false-positive expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "document_symbol_span_exact_rate",
            ratio(
                navigation_score.document_symbol_span_exact_count,
                navigation_score.document_symbol_expected_count,
            ),
            navigation_score.document_symbol_expected_count,
            "no navigation document-symbol expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_document_symbol_recall",
            ratio(
                navigation_score.document_symbol_span_exact_count,
                navigation_score.document_symbol_expected_count,
            ),
            navigation_score.document_symbol_expected_count,
            "no provider document-symbol recall expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_document_symbol_precision",
            ratio(
                navigation_score.document_symbol_span_exact_count,
                navigation_score.document_symbol_returned_count,
            ),
            navigation_score.document_symbol_returned_count,
            "no provider document-symbol precision expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "goto_definition_hit_rate",
            ratio(
                navigation_score.goto_definition_hit_count,
                navigation_score.goto_definition_expected_count,
            ),
            navigation_score.goto_definition_expected_count,
            "no navigation goto-definition expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_goto_definition_hit_rate",
            ratio(
                navigation_score.goto_definition_hit_count,
                navigation_score.goto_definition_expected_count,
            ),
            navigation_score.goto_definition_expected_count,
            "no provider goto-definition expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "goto_definition_span_exact_rate",
            ratio(
                navigation_score.goto_definition_span_exact_count,
                navigation_score.goto_definition_expected_count,
            ),
            navigation_score.goto_definition_expected_count,
            "no navigation goto-definition span expectations are available",
            cadence,
        ),
        optional_measured_count(
            "goto_definition_false_target_count",
            navigation_score.goto_definition_false_target_count,
            navigation_score.goto_definition_expected_count,
            "no navigation goto-definition expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "references_precision",
            ratio(
                navigation_score.references_hit_count,
                navigation_score.references_returned_count,
            ),
            navigation_score.references_returned_count,
            "no navigation reference results are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_references_precision",
            ratio(
                navigation_score.references_hit_count,
                navigation_score.references_returned_count,
            ),
            navigation_score.references_returned_count,
            "no provider reference results are available",
            cadence,
        ),
        optional_measured_rate(
            "references_recall",
            ratio(
                navigation_score.references_hit_count,
                navigation_score.references_expected_count,
            ),
            navigation_score.references_expected_count,
            "no navigation reference expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_references_recall",
            ratio(
                navigation_score.references_hit_count,
                navigation_score.references_expected_count,
            ),
            navigation_score.references_expected_count,
            "no provider reference expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_rename_safe_edit_accuracy",
            ratio(
                navigation_score.rename_safe_edit_correct_count,
                navigation_score.rename_safe_edit_expected_count,
            ),
            navigation_score.rename_safe_edit_expected_count,
            "no provider rename safe-edit expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_safe_delete_blocker_accuracy",
            ratio(
                navigation_score.safe_delete_blocker_correct_count,
                navigation_score.safe_delete_blocker_expected_count,
            ),
            navigation_score.safe_delete_blocker_expected_count,
            "no provider safe-delete blocker expectations are available",
            cadence,
        ),
        optional_measured_count(
            "references_false_positive_count",
            navigation_score.references_false_positive_count,
            navigation_score
                .references_expected_count
                .saturating_add(navigation_score.references_absent_assertion_count),
            "no navigation reference expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "hover_origin_accuracy",
            ratio(
                navigation_score.hover_origin_correct_count,
                navigation_score.hover_expected_count,
            ),
            navigation_score.hover_expected_count,
            "no navigation hover expectations are available",
            cadence,
        ),
        optional_measured_rate(
            "provider_hover_symbol_origin_accuracy",
            ratio(
                navigation_score.hover_origin_correct_count,
                navigation_score.hover_expected_count,
            ),
            navigation_score.hover_expected_count,
            "no provider hover symbol-origin expectations are available",
            cadence,
        ),
    ];

    rows.extend(
        PROVIDER_METRICS
            .iter()
            .map(|metric| insufficient(metric, "provider gold fixtures are not wired yet"))
            .collect::<Vec<_>>(),
    );
    rows
}

fn scale_metrics(score: &ScaleCostScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.fixture_count == 0 {
        return vec![insufficient(
            "scale_file_bytes",
            "parser accuracy fixtures are not available",
        )];
    }

    vec![
        measured_count("scale_file_bytes", score.file_bytes, score.fixture_count, cadence),
        measured_count("scale_line_count", score.source_lines, score.fixture_count, cadence),
        measured_count("scale_token_count", score.token_count, score.fixture_count, cadence),
        measured_count("scale_ast_node_count", score.ast_node_count, score.fixture_count, cadence),
        measured_count("scale_symbol_count", score.symbol_count, score.fixture_count, cadence),
        measured_count("scale_import_count", score.import_count, score.fixture_count, cadence),
        measured_count("scale_export_count", score.export_count, score.fixture_count, cadence),
        measured_count("scale_sub_count", score.sub_count, score.fixture_count, cadence),
        measured_count("scale_package_count", score.package_count, score.fixture_count, cadence),
        measured_count(
            "scale_max_nesting_depth",
            score.max_nesting_depth,
            score.fixture_count,
            cadence,
        ),
        measured_count(
            "scale_max_brace_depth",
            score.max_brace_depth,
            score.fixture_count,
            cadence,
        ),
        measured_count(
            "scale_max_regex_length",
            score.max_regex_length,
            score.fixture_count,
            cadence,
        ),
        measured_count(
            "scale_max_heredoc_body_bytes",
            score.max_heredoc_body_bytes,
            score.fixture_count,
            cadence,
        ),
        measured_count(
            "scale_quote_like_count",
            score.quote_like_count,
            score.fixture_count,
            cadence,
        ),
        measured_count(
            "scale_dynamic_boundary_count",
            score.dynamic_boundary_count,
            score.fixture_count,
            cadence,
        ),
    ]
}

fn cost_metrics(
    score: &ScaleCostScore,
    recovery_score: &RecoveryScore,
    method_completion_score: &MethodCompletionProviderScore,
    navigation_score: &NavigationProviderScore,
    cadence: Cadence,
) -> Vec<MetricRow> {
    vec![
        optional_measured_value(
            "lex_ms_p95",
            p95_f64(&score.lex_ms),
            score.fixture_count,
            "lexer timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "parse_ms_p95",
            p95_f64(&score.parse_ms),
            score.fixture_count,
            "parser timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "ast_projection_ms_p95",
            p95_f64(&score.ast_projection_ms),
            score.fixture_count,
            "AST projection timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "recovery_ms_p95",
            p95_micros_as_ms(&recovery_score.recovery_parse_micros),
            recovery_score.recovery_parse_micros.len() as u64,
            "recovery timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "semantic_extraction_ms_p95",
            p95_f64(&score.semantic_extraction_ms),
            score.fixture_count,
            "semantic extraction timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "workspace_insert_ms_p95",
            p95_f64(&score.workspace_insert_ms),
            score.workspace_insert_ms.len() as u64,
            "workspace insert timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "definition_query_ms_p95",
            p95_micros_as_ms(&navigation_score.definition_query_micros),
            navigation_score.definition_query_micros.len() as u64,
            "provider definition query timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "reference_query_ms_p95",
            p95_micros_as_ms(&navigation_score.reference_query_micros),
            navigation_score.reference_query_micros.len() as u64,
            "provider reference query timing samples are not available",
            cadence,
        ),
        optional_measured_value(
            "completion_query_ms_p95",
            p95_micros_as_ms(&method_completion_score.completion_query_micros),
            method_completion_score.completion_query_micros.len() as u64,
            "provider completion query timing samples are not available",
            cadence,
        ),
        insufficient("peak_rss_mb", "memory telemetry is not wired yet"),
        insufficient("allocated_bytes", "allocation telemetry is not wired yet"),
        insufficient("allocation_count", "allocation telemetry is not wired yet"),
    ]
}

fn cache_reuse_metrics(score: &IncrementalScore, cadence: Cadence) -> Vec<MetricRow> {
    let checkpoint_total = score.checkpoint_hit_count + score.checkpoint_miss_count;
    let content_hash_total = score.content_hash_hit_count + score.content_hash_miss_count;
    let semantic_fact_cache_total =
        score.semantic_fact_cache_hit_count + score.semantic_fact_cache_miss_count;
    vec![
        optional_measured_rate(
            "lexer_checkpoint_reuse_rate",
            ratio(score.checkpoint_hit_count, checkpoint_total),
            checkpoint_total,
            "lexer checkpoint telemetry is not available",
            cadence,
        ),
        optional_measured_rate(
            "parser_checkpoint_reuse_rate",
            ratio(score.checkpoint_hit_count, checkpoint_total),
            checkpoint_total,
            "parser checkpoint telemetry is not available",
            cadence,
        ),
        optional_measured_rate(
            "semantic_fact_cache_hit_rate",
            ratio(score.semantic_fact_cache_hit_count, semantic_fact_cache_total),
            semantic_fact_cache_total,
            "semantic fact cache telemetry is not wired yet",
            cadence,
        ),
        optional_measured_rate(
            "workspace_shard_reuse_rate",
            ratio(
                score.workspace_shard_reuse_count,
                score.workspace_shard_replacement_attempt_count,
            ),
            score.workspace_shard_replacement_attempt_count,
            "workspace shard reuse telemetry is not wired yet",
            cadence,
        ),
        optional_measured_rate(
            "unchanged_file_skip_rate",
            ratio(score.unchanged_file_skip_count, score.unchanged_file_index_attempt_count),
            score.unchanged_file_index_attempt_count,
            "unchanged-file skip telemetry is not wired yet",
            cadence,
        ),
        optional_measured_rate(
            "content_hash_hit_rate",
            ratio(score.content_hash_hit_count, content_hash_total),
            content_hash_total,
            "content hash telemetry is not available",
            cadence,
        ),
        optional_measured_count(
            "fast_path_attempt_count",
            score.expectation_count,
            score.expectation_count,
            "incremental fast-path fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "fast_path_success_count",
            score.full_parse_equivalent_count,
            score.expectation_count,
            "incremental fast-path fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "fast_path_fallback_count",
            score.fallback_count,
            score.expectation_count,
            "incremental fast-path fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "fast_path_wrong_result_count",
            score.expectation_count.saturating_sub(score.full_parse_equivalent_count),
            score.expectation_count,
            "incremental fast-path fixtures are not available",
            cadence,
        ),
    ]
}

fn determinism_metrics(score: &DeterminismScore, cadence: Cadence) -> Vec<MetricRow> {
    if score.fixture_count == 0 {
        return vec![insufficient(
            "parse_hash_stability_rate",
            "determinism fixtures are not available",
        )];
    }

    vec![
        measured_rate(
            "parse_hash_stability_rate",
            score.parse_hash_stable_count,
            score.fixture_count,
            cadence,
        ),
        measured_rate(
            "token_stream_hash_stability_rate",
            score.token_stream_stable_count,
            score.fixture_count,
            cadence,
        ),
        measured_rate(
            "ast_hash_stability_rate",
            score.ast_hash_stable_count,
            score.fixture_count,
            cadence,
        ),
        measured_rate(
            "semantic_fact_hash_stability_rate",
            score.semantic_fact_hash_stable_count,
            score.fixture_count,
            cadence,
        ),
        measured_rate(
            "diagnostic_hash_stability_rate",
            score.diagnostic_hash_stable_count,
            score.fixture_count,
            cadence,
        ),
        measured_rate(
            "repeated_parse_determinism_rate",
            score.repeated_parse_stable_count,
            score.fixture_count,
            cadence,
        ),
        insufficient(
            "whitespace_invariance_rate",
            "metamorphic whitespace fixtures are not wired yet",
        ),
        optional_measured_rate(
            "comment_invariance_rate",
            ratio(score.comment_invariance_stable_count, score.comment_invariance_sample_count),
            score.comment_invariance_sample_count,
            "metamorphic comment fixtures are not wired yet",
            cadence,
        ),
        insufficient(
            "newline_style_invariance_rate",
            "metamorphic newline fixtures are not wired yet",
        ),
    ]
}

fn gold_drift_metrics(drift: &GoldDrift, fixture_count: u64, cadence: Cadence) -> Vec<MetricRow> {
    vec![
        optional_measured_count(
            "gold_schema_errors",
            drift.schema_error_count,
            fixture_count,
            "gold fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "gold_span_errors",
            drift.span_error_count,
            fixture_count,
            "gold fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "gold_duplicate_symbol_ids",
            drift.duplicate_symbol_id_count,
            fixture_count,
            "gold fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "gold_missing_resolves_to_targets",
            drift.missing_resolves_to_target_count,
            fixture_count,
            "gold fixtures are not available",
            cadence,
        ),
        optional_measured_count(
            "gold_changed_line_count",
            drift.changed_line_count,
            drift.changed_line_sample_count,
            "gold drift baseline is not wired yet",
            cadence,
        ),
        insufficient("gold_changed_symbol_count", "gold drift baseline is not wired yet"),
        insufficient("gold_removed_expectation_count", "gold drift baseline is not wired yet"),
        optional_measured_count(
            "gold_added_expectation_count",
            drift.added_expectation_count,
            drift.added_expectation_sample_count,
            "gold drift baseline is not wired yet",
            cadence,
        ),
        insufficient(
            "gold_dynamic_expectation_change_count",
            "gold drift baseline is not wired yet",
        ),
        insufficient(
            "gold_weakening_explanation_required_count",
            "gold weakening explanation checks require a baseline diff in CI",
        ),
    ]
}

fn sync_runtime_metric_rows(artifact: &mut ParserAccuracyArtifact, cadence: Cadence) {
    let runtime = &artifact.metric_runtime;
    artifact.metrics.extend([
        measured_value("metric_runtime_ms", runtime.runtime_ms, 1, cadence),
        measured_count("metric_timeout_count", runtime.timeout_count, 1, cadence),
        measured_count("metric_flake_count", runtime.flake_count, 1, cadence),
        measured_count("metric_artifact_size_bytes", runtime.artifact_size_bytes, 1, cadence),
        measured_count(
            "metric_ci_runner_failure_count",
            runtime.ci_runner_failure_count,
            1,
            cadence,
        ),
        measured_count("metric_orphan_process_count", runtime.orphan_process_count, 1, cadence),
        optional_measured_value(
            "metric_cache_hit_rate",
            runtime.cache_hit_rate,
            runtime.cache_sample_count,
            "metric cache telemetry is not wired yet",
            cadence,
        ),
    ]);
}

fn sync_allocation_metric_rows(artifact: &mut ParserAccuracyArtifact, cadence: Cadence) {
    if let Some(peak_rss_mb) = artifact.metric_runtime.peak_rss_mb {
        if let Some(row) = artifact.metrics.iter_mut().find(|row| row.name() == "peak_rss_mb") {
            *row = measured_value("peak_rss_mb", peak_rss_mb, 1, cadence);
        }
    }
    if let Some(allocated_bytes) = artifact.metric_runtime.allocated_bytes {
        if let Some(row) = artifact.metrics.iter_mut().find(|row| row.name() == "allocated_bytes") {
            *row = measured_count("allocated_bytes", allocated_bytes, 1, cadence);
        }
    }
    if let Some(allocation_count) = artifact.metric_runtime.allocation_count {
        if let Some(row) = artifact.metrics.iter_mut().find(|row| row.name() == "allocation_count")
        {
            *row = measured_count("allocation_count", allocation_count, 1, cadence);
        }
    }
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
    measured_value(metric, value as f64, sample_count, cadence)
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

fn optional_measured_value(
    metric: &str,
    value: Option<f64>,
    sample_count: u64,
    insufficient_reason: &str,
    cadence: Cadence,
) -> MetricRow {
    match value {
        Some(value) if sample_count > 0 => measured_value(metric, value, sample_count, cadence),
        _ => insufficient(metric, insufficient_reason),
    }
}

fn measured_rate(metric: &str, numerator: u64, denominator: u64, cadence: Cadence) -> MetricRow {
    let value = ratio(numerator, denominator).unwrap_or(0.0);
    measured_value(metric, value, denominator, cadence)
}

fn measured_value(metric: &str, value: f64, sample_count: u64, cadence: Cadence) -> MetricRow {
    MetricRow::Measured {
        metric: metric.to_string(),
        value,
        previous: None,
        delta: None,
        floor: None,
        threshold: None,
        sample_count,
        direction: Direction::Neutral,
        confidence: Confidence::High,
        cadence,
        macro_value: None,
        micro_value: None,
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
        Some(value) if sample_count > 0 => measured_value(metric, value, sample_count, cadence),
        _ => insufficient(metric, insufficient_reason),
    }
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 { None } else { Some(numerator as f64 / denominator as f64) }
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

fn f1_from_counts(true_positive: u64, false_positive: u64, false_negative: u64) -> Option<f64> {
    let denominator = (2 * true_positive) + false_positive + false_negative;
    ratio(2 * true_positive, denominator)
}

fn mean_u64(values: &[u64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<u64>() as f64 / values.len() as f64)
}

fn p95_u64(values: &[u64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[index] as f64)
}

fn mean_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn p95_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[index])
}

fn p95_micros_as_ms(values: &[u64]) -> Option<f64> {
    p95_u64(values).map(|micros| micros / 1000.0)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReceiptFile {
    pub name: &'static str,
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct FailurePacketStatusReceipt {
    schema_version: u32,
    commit: String,
    cadence: Cadence,
    generated_by: &'static str,
    failure_packet_count: u64,
    failure_packets: Vec<FailurePacketStatusEntry>,
}

#[derive(Debug, Serialize)]
struct FailurePacketStatusEntry {
    id: String,
    failure_kind: String,
    likely_layer: String,
    fixture_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u64>,
    expected: Vec<String>,
    actual: Vec<String>,
    actual_nearest: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_next_fix: Option<String>,
    suggested_next_pr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_observed_commit: Option<String>,
}

#[derive(Debug, Serialize)]
struct FixtureInventoryStatusReceipt {
    schema_version: u32,
    commit: String,
    generated_by: &'static str,
    fixture_count: u64,
    family_count: u64,
    scored_lines: u64,
    scored_symbols: u64,
    fixtures: Vec<FixtureInventoryStatusEntry>,
}

#[derive(Debug, Serialize)]
struct FixtureInventoryStatusEntry {
    id: String,
    family: String,
    source_path: String,
    label_mode: LabelMode,
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
    provider_expectation_counts: ProviderExpectationCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FailureWorklistRow {
    family: String,
    count: u64,
    likely_layer: String,
    first_fixture: String,
    suggested_pr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeasurementGapRow {
    metric: String,
    reason: String,
    suggested_pr: String,
}

#[derive(Debug, Serialize)]
struct ProviderExpectationCounts {
    method_completion: u64,
    diagnostics: u64,
    navigation: u64,
}

pub fn status_receipt_files_from_target(root: &Path) -> Result<Vec<StatusReceiptFile>> {
    let manifest_path = root.join(DEFAULT_MANIFEST);
    let manifest = read_manifest(root, &manifest_path)?;
    let artifact_path = root.join(DEFAULT_OUTPUT);
    let raw = fs::read_to_string(&artifact_path)
        .with_context(|| format!("reading parser accuracy artifact {}", artifact_path.display()))?;
    let artifact: ParserAccuracyArtifact = serde_json::from_str(&raw)
        .with_context(|| format!("parsing parser accuracy artifact {}", artifact_path.display()))?;
    validate_artifact_contract(&artifact)?;
    status_receipt_files(root, &manifest, &artifact)
}

pub fn status_receipt_equivalent_ignoring_commit(existing: &str, generated: &str) -> bool {
    let Ok(mut existing_value) = serde_json::from_str::<serde_json::Value>(existing) else {
        return existing == generated;
    };
    let Ok(mut generated_value) = serde_json::from_str::<serde_json::Value>(generated) else {
        return existing == generated;
    };
    if let (Some(existing_object), Some(generated_object)) =
        (existing_value.as_object_mut(), generated_value.as_object_mut())
    {
        existing_object.remove("commit");
        generated_object.remove("commit");
    }
    existing_value == generated_value
}

fn write_status_receipts(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    artifact: &ParserAccuracyArtifact,
) -> Result<()> {
    for receipt in status_receipt_files(root, manifest, artifact)? {
        let parent = receipt
            .path
            .parent()
            .ok_or_else(|| eyre!("parser accuracy status receipt path has no parent"))?;
        fs::create_dir_all(parent).with_context(|| {
            format!("creating parser accuracy status receipt dir {}", parent.display())
        })?;
        fs::write(&receipt.path, receipt.content)
            .with_context(|| format!("writing {}", receipt.path.display()))?;
        println!("parser accuracy status receipt written: {}", receipt.path.display());
    }
    Ok(())
}

fn status_receipt_files(
    root: &Path,
    manifest: &ParserAccuracyManifest,
    artifact: &ParserAccuracyArtifact,
) -> Result<Vec<StatusReceiptFile>> {
    Ok(vec![
        StatusReceiptFile {
            name: FAILURE_PACKET_STATUS_RECEIPT,
            path: root.join(FAILURE_PACKET_STATUS_RECEIPT),
            content: render_failure_packet_status_receipt(artifact)?,
        },
        StatusReceiptFile {
            name: FIXTURE_INVENTORY_STATUS_RECEIPT,
            path: root.join(FIXTURE_INVENTORY_STATUS_RECEIPT),
            content: render_fixture_inventory_status_receipt(manifest, artifact)?,
        },
        StatusReceiptFile {
            name: FAILURE_WORKLIST_STATUS_RECEIPT,
            path: root.join(FAILURE_WORKLIST_STATUS_RECEIPT),
            content: render_failure_worklist_status_receipt(artifact),
        },
        StatusReceiptFile {
            name: NEXT_POINTER_STATUS_RECEIPT,
            path: root.join(NEXT_POINTER_STATUS_RECEIPT),
            content: render_next_pointer_status_receipt(artifact),
        },
    ])
}

fn render_failure_packet_status_receipt(artifact: &ParserAccuracyArtifact) -> Result<String> {
    let receipt = FailurePacketStatusReceipt {
        schema_version: 1,
        commit: artifact.commit.clone(),
        cadence: artifact.cadence,
        generated_by: "cargo xtask metrics parser-accuracy --export-status-receipts",
        failure_packet_count: artifact.failure_packets.len() as u64,
        failure_packets: artifact.failure_packets.iter().map(failure_packet_status_entry).collect(),
    };
    render_json_with_newline(&receipt)
}

fn failure_packet_status_entry(packet: &FailurePacket) -> FailurePacketStatusEntry {
    FailurePacketStatusEntry {
        id: failure_packet_status_id(packet),
        failure_kind: packet.failure_kind.clone(),
        likely_layer: packet.likely_layer.clone(),
        fixture_id: packet.fixture_id.clone(),
        family: packet.family.clone(),
        metric: packet.metric.clone(),
        line: packet.line,
        expected: packet.expected.clone(),
        actual: packet.actual.clone(),
        actual_nearest: packet.nearest_predictions.clone(),
        source_excerpt: packet.source_excerpt.clone(),
        details: packet.details.clone(),
        suggested_next_fix: packet.suggested_next_fix.clone(),
        suggested_next_pr: suggested_next_pr_for_failure_packet(packet),
        first_observed_commit: None,
    }
}

fn failure_packet_status_id(packet: &FailurePacket) -> String {
    let identity = format!(
        "{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}",
        packet.failure_kind,
        packet.likely_layer,
        packet.fixture_id,
        packet.family.as_deref().unwrap_or(""),
        packet.metric,
        packet.line,
        packet.expected,
        packet.source_excerpt
    );
    format!("packet-{:016x}", stable_hash(&identity))
}

fn suggested_next_pr_for_failure_packet(packet: &FailurePacket) -> String {
    match packet.likely_layer.as_str() {
        "parser" => "fix(parser-core): resolve parser projection failure packet".to_string(),
        "ast_projection" => {
            "feat(parser-accuracy): tighten AST projection fixture expectations".to_string()
        }
        "semantic_fact_extraction" => {
            "fix(semantic): resolve parser-accuracy semantic fact packet".to_string()
        }
        layer => format!("fix(parser-accuracy): resolve {layer} failure packet"),
    }
}

fn render_fixture_inventory_status_receipt(
    manifest: &ParserAccuracyManifest,
    artifact: &ParserAccuracyArtifact,
) -> Result<String> {
    let receipt = FixtureInventoryStatusReceipt {
        schema_version: 1,
        commit: artifact.commit.clone(),
        generated_by: "cargo xtask metrics parser-accuracy --export-status-receipts",
        fixture_count: artifact.denominator.fixture_count,
        family_count: artifact.denominator.fixture_family_count,
        scored_lines: artifact.denominator.scored_line_count,
        scored_symbols: artifact.denominator.scored_symbol_count,
        fixtures: manifest.fixtures.iter().map(fixture_inventory_status_entry).collect(),
    };
    render_json_with_newline(&receipt)
}

fn render_failure_worklist_status_receipt(artifact: &ParserAccuracyArtifact) -> String {
    let rows = failure_worklist_rows(artifact);
    let mut output = String::new();
    output.push_str("# Parser-accuracy failure worklist\n\n");
    output.push_str(&format!(
        "Source: `target/metrics/parser_accuracy.json` ({} failure packets)\n\n",
        artifact.failure_packets.len()
    ));
    output.push_str("| Family | Count | Likely layer | First fixture | Suggested PR |\n");
    output.push_str("|---|---:|---|---|---|\n");

    if rows.is_empty() {
        output.push_str("| none | 0 | n/a | n/a | n/a |\n");
    } else {
        for row in rows {
            output.push_str(&format!(
                "| {} | {} | {} | `{}` | `{}` |\n",
                row.family, row.count, row.likely_layer, row.first_fixture, row.suggested_pr
            ));
        }
    }

    output
}

fn render_next_pointer_status_receipt(artifact: &ParserAccuracyArtifact) -> String {
    let rows = failure_worklist_rows(artifact);
    let mut output = String::new();
    output.push_str("# Parser Accuracy Next\n\n");
    output.push_str("Source: `target/metrics/parser_accuracy.json`\n\n");
    output.push_str(&format!(
        "Denominator: {} fixtures / {} families; {} scored lines; {} scored symbols.\n\n",
        artifact.denominator.fixture_count,
        artifact.denominator.fixture_family_count,
        artifact.denominator.scored_line_count,
        artifact.denominator.scored_symbol_count
    ));
    output.push_str(&format!("Failure packets: {} active.\n\n", artifact.failure_packets.len()));

    if let Some(row) = rows.first() {
        output.push_str("| Field | Value |\n");
        output.push_str("|---|---|\n");
        output.push_str(&format!("| Pointer | `{}` |\n", row.family));
        output.push_str(&format!("| Packet count | {} |\n", row.count));
        output.push_str(&format!("| Likely layer | `{}` |\n", row.likely_layer));
        output.push_str(&format!("| First fixture | `{}` |\n", row.first_fixture));
        output.push_str(&format!("| Suggested PR | `{}` |\n", row.suggested_pr));
        output.push_str(
            "\nUse this pointer only after open measurement/tracking PRs are settled. If the pointed lane has already landed, regenerate this file and take the next row.\n",
        );
    } else {
        output.push_str("Pointer: no active failure packets.\n\n");
        output.push_str("## Next Measurement Gaps\n\n");
        output.push_str("| Metric | Reason | Suggested PR |\n");
        output.push_str("|---|---|---|\n");

        let gaps = next_measurement_gap_rows(artifact);
        if gaps.is_empty() {
            output.push_str("| none | n/a | n/a |\n");
        } else {
            for gap in gaps {
                output.push_str(&format!(
                    "| `{}` | {} | `{}` |\n",
                    markdown_table_cell(&gap.metric),
                    markdown_table_cell(&gap.reason),
                    markdown_table_cell(&gap.suggested_pr)
                ));
            }
        }
        output.push_str(
            "\nUse this section only when there are no active failure packets. Keep each measurement gap in its own PR and regenerate this file after a lane lands.\n",
        );
    }

    output
}

fn next_measurement_gap_rows(artifact: &ParserAccuracyArtifact) -> Vec<MeasurementGapRow> {
    const MAX_NEXT_GAPS: usize = 5;

    let mut rows: Vec<_> = artifact
        .metrics
        .iter()
        .filter_map(|row| match row {
            MetricRow::InsufficientData { metric, reason, sample_count, .. }
                if *sample_count == 0 =>
            {
                Some(MeasurementGapRow {
                    metric: metric.clone(),
                    reason: reason.clone(),
                    suggested_pr: suggested_next_pr_for_measurement_gap(metric),
                })
            }
            MetricRow::Measured { .. } | MetricRow::InsufficientData { .. } => None,
        })
        .collect();

    rows.sort_by(|left, right| {
        measurement_gap_priority(&left.metric, &left.reason)
            .cmp(&measurement_gap_priority(&right.metric, &right.reason))
            .then_with(|| left.metric.cmp(&right.metric))
    });
    rows.truncate(MAX_NEXT_GAPS);
    rows
}

fn measurement_gap_priority(metric: &str, reason: &str) -> u8 {
    if metric.starts_with("provider_") {
        0
    } else if reason.contains("timing") || metric.ends_with("_ms_p95") {
        1
    } else if reason.contains("telemetry") {
        2
    } else {
        3
    }
}

fn suggested_next_pr_for_measurement_gap(metric: &str) -> String {
    if metric.starts_with("provider_") {
        format!("test(parser-accuracy): wire provider gold fixture for {metric}")
    } else if metric.ends_with("_ms_p95") || metric.ends_with("_query_ms_p95") {
        format!("feat(metrics): wire parser-accuracy timing for {metric}")
    } else {
        format!("feat(metrics): wire parser-accuracy measurement for {metric}")
    }
}

fn markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn failure_worklist_rows(artifact: &ParserAccuracyArtifact) -> Vec<FailureWorklistRow> {
    let mut rows: BTreeMap<String, FailureWorklistRow> = BTreeMap::new();

    for packet in &artifact.failure_packets {
        let family = packet.failure_kind.clone();
        let suggested_pr = suggested_next_pr_for_failure_packet(packet);
        rows.entry(family.clone())
            .and_modify(|row| {
                row.count += 1;
            })
            .or_insert_with(|| FailureWorklistRow {
                family,
                count: 1,
                likely_layer: packet.likely_layer.clone(),
                first_fixture: packet.fixture_id.clone(),
                suggested_pr,
            });
    }

    let mut rows: Vec<_> = rows.into_values().collect();
    rows.sort_by(|left, right| {
        right.count.cmp(&left.count).then_with(|| left.family.cmp(&right.family))
    });
    rows
}

fn fixture_inventory_status_entry(fixture: &FixtureMetadata) -> FixtureInventoryStatusEntry {
    FixtureInventoryStatusEntry {
        id: fixture.id.clone(),
        family: fixture.family.clone(),
        source_path: fixture.source_path.clone(),
        label_mode: fixture.label_mode,
        scored_lines: fixture.scored_lines,
        scored_symbols: fixture.scored_symbols,
        fully_labeled_regions: fixture.fully_labeled_regions,
        partial_labeled_regions: fixture.partial_labeled_regions,
        unknown_regions: fixture.unknown_regions,
        negative_regions: fixture.negative_regions,
        dynamic_boundaries: fixture.dynamic_boundaries,
        unsupported_constructs: fixture.unsupported_constructs,
        real_project_file: fixture.real_project_file,
        generated: fixture.generated,
        provider_expectation_counts: ProviderExpectationCounts {
            method_completion: fixture.provider_expectations.method_completion.len() as u64,
            diagnostics: fixture.provider_expectations.diagnostics.len() as u64,
            navigation: fixture.provider_expectations.navigation.len() as u64,
        },
    }
}

fn render_json_with_newline(value: &impl Serialize) -> Result<String> {
    let mut rendered = serde_json::to_string_pretty(value)?;
    rendered.push('\n');
    Ok(rendered)
}

fn write_ratchet_receipt(root: &Path, artifact: &ParserAccuracyArtifact) -> Result<()> {
    let path = root.join(DEFAULT_RATCHET_RECEIPT);
    let parent =
        path.parent().ok_or_else(|| eyre!("parser accuracy ratchet receipt path has no parent"))?;
    fs::create_dir_all(parent).with_context(|| {
        format!("creating parser accuracy ratchet receipt dir {}", parent.display())
    })?;
    let receipt = ratchet_receipt_for_artifact(artifact);
    let json = serde_json::to_string_pretty(&receipt)
        .context("serializing parser accuracy ratchet receipt")?;
    fs::write(&path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))?;
    println!("parser accuracy ratchet receipt written: {}", path.display());
    Ok(())
}

fn ratchet_receipt_for_artifact(artifact: &ParserAccuracyArtifact) -> MetricReceipt {
    let floor_metrics = SAFETY_FLOOR_METRICS
        .iter()
        .map(|(metric, _floor)| ((*metric).to_string(), measured_metric_value(artifact, metric)))
        .collect();
    let improvement_metrics = DEFERRED_PRECISION_RECALL_CANDIDATES
        .iter()
        .map(|metric| ((*metric).to_string(), measured_metric_value(artifact, metric)))
        .collect();

    MetricReceipt {
        subsystem: artifact.subsystem.to_string(),
        generated_at: artifact.generated_at.clone(),
        commit: artifact.commit.clone(),
        floor_metrics,
        improvement_metrics,
    }
}

fn measured_metric_value(artifact: &ParserAccuracyArtifact, name: &str) -> Option<f64> {
    artifact.metrics.iter().find_map(|row| match row {
        MetricRow::Measured { metric, value, .. } if metric == name => Some(*value),
        _ => None,
    })
}

fn apply_safety_floor_metadata(metrics: &mut [MetricRow]) {
    for row in metrics {
        let MetricRow::Measured {
            metric, value, previous, delta, floor, threshold, direction, ..
        } = row
        else {
            continue;
        };

        let Some((_, floor_value)) =
            SAFETY_FLOOR_METRICS.iter().find(|(candidate, _)| candidate == metric)
        else {
            continue;
        };

        *previous = Some(*floor_value);
        *delta = Some(*value - *floor_value);
        *floor = Some(*floor_value);
        *threshold = Some(*floor_value);
        *direction = Direction::Down;
    }
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

    fn write_method_completion_provider_fixture(root: &Path) -> Result<()> {
        fs::write(
            root.join("method_completion_provider.pl"),
            r#"# provider-index-support:start
package Accuracy::Provider::Foo;
sub own_method { 1 }
sub shared_name { 1 }

package Accuracy::Provider::Bar;
sub unrelated_method { 1 }

package Accuracy::Provider::Imported;
sub imported_method { 1 }
# provider-index-support:end

package Accuracy::Provider::Foo;

sub self_case {
    my $self = shift;
    $self-> # cursor:self
}

package Accuracy::Provider::UseCases;
use Accuracy::Provider::Imported;

sub dynamic_bless_case {
    my $class = "Accuracy::Provider::Foo";
    my $x = bless {}, $class;
    $x-> # cursor:dynamic_bless
}

sub imported_unknown_case {
    my $imported_unknown = get_object();
    $imported_unknown-> # cursor:imported_unknown
}

1;
"#,
        )?;
        Ok(())
    }

    fn write_diagnostic_provider_fixture(root: &Path) -> Result<()> {
        fs::write(
            root.join("diagnostic_provider.pl"),
            r#"# provider-index-support:start
package Accuracy::Diagnostics::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(imported_known);
sub imported_known { 1 }
# provider-index-support:end

package Accuracy::Diagnostics::UseCases;
use strict;
use warnings;
use Accuracy::Diagnostics::Exporter qw(imported_known);

sub ordinary_undefined_variable {
    print $ordinary_missing;
}

sub ordinary_undefined_bareword {
    print truly_missing_symbol;
}

sub eval_string_boundary {
    eval "sub eval_generated_symbol { 1 }";
    print eval_generated_symbol;
}

sub dynamic_require_boundary {
    my $module = "Accuracy::Diagnostics::Dynamic";
    require $module;
    $module->import(qw(dynamic_imported_symbol generated_accessor));
    print dynamic_imported_symbol;
    print generated_accessor;
}

our $AUTOLOAD;
sub AUTOLOAD {
    our $AUTOLOAD;
    return $AUTOLOAD;
}

sub autoload_boundary {
    print $AUTOLOAD;
}

sub known_imported_symbol {
    print imported_known;
}

our $package_local_symbol;

sub package_local_symbol_case {
    print $package_local_symbol;
}

1;
"#,
        )?;
        Ok(())
    }

    fn write_navigation_provider_fixture(root: &Path) -> Result<()> {
        fs::write(
            root.join("navigation_provider.pl"),
            r#"# provider-index-support:start
package Accuracy::Navigation::Exporter;
use Exporter 'import';
our @EXPORT_OK = qw(imported_nav);
sub imported_nav { 1 }

package Accuracy::Navigation::Parent;
sub inherited_method { 1 }

package Accuracy::Navigation::Generated;
sub generated_accessor { 1 }
# provider-index-support:end

package Accuracy::Navigation::UseCases;
use strict;
use warnings;
use parent 'Accuracy::Navigation::Parent';
use Accuracy::Navigation::Exporter qw(imported_nav);

sub own_sub { 1 }
sub own_method { 1 }

sub bare_call_case {
    own_sub(); # cursor:bare_call
}

sub qualified_call_case {
    Accuracy::Navigation::UseCases::own_sub(); # cursor:qualified_call
}

sub imported_symbol_case {
    imported_nav(); # cursor:imported_nav
}

sub method_receiver_case {
    my $self = shift;
    $self->inherited_method(); # cursor:inherited_method
}

sub generated_accessor_case {
    Accuracy::Navigation::Generated::generated_accessor(); # cursor:generated_accessor
}

sub dynamic_boundary_case {
    my $name = "own_sub";
    no strict 'refs';
    &$name(); # cursor:dynamic_ref
}

1;
"#,
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
                    recovery_expectations: vec![],
                    incremental_expectations: vec![],
                    span_expectations: vec![],
                    provider_expectations: ProviderExpectations::default(),
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
                    recovery_expectations: vec![],
                    incremental_expectations: vec![],
                    span_expectations: vec![],
                    provider_expectations: ProviderExpectations::default(),
                },
            ],
        }
    }

    fn method_completion_provider_manifest() -> ParserAccuracyManifest {
        ParserAccuracyManifest {
            schema_version: 1,
            fixtures: vec![FixtureMetadata {
                id: "method_completion_provider".to_string(),
                family: "provider_method_completion".to_string(),
                label_mode: LabelMode::Partial,
                source_path: "method_completion_provider.pl".to_string(),
                scored_lines: 0,
                scored_symbols: 0,
                fully_labeled_regions: 0,
                partial_labeled_regions: 1,
                unknown_regions: 0,
                negative_regions: 0,
                dynamic_boundaries: 1,
                unsupported_constructs: 0,
                real_project_file: false,
                generated: false,
                line_expectations: vec![],
                ast_expectations: vec![],
                symbol_expectations: SymbolExpectations::default(),
                symbol_safety_regions: vec![],
                recovery_expectations: vec![],
                incremental_expectations: vec![],
                span_expectations: vec![],
                provider_expectations: ProviderExpectations {
                    method_completion: vec![
                        MethodCompletionProviderExpectation {
                            id: "self_receiver_methods".to_string(),
                            cursor_marker: "cursor:self".to_string(),
                            expected_receiver_package: Some("Accuracy::Provider::Foo".to_string()),
                            expected_present: vec![
                                "own_method".to_string(),
                                "shared_name".to_string(),
                            ],
                            expected_absent: vec!["unrelated_method".to_string()],
                            expected_fallback: false,
                            import_visibility: false,
                        },
                        MethodCompletionProviderExpectation {
                            id: "dynamic_bless_does_not_infer_exact_receiver".to_string(),
                            cursor_marker: "cursor:dynamic_bless".to_string(),
                            expected_receiver_package: None,
                            expected_present: vec![],
                            expected_absent: vec![
                                "own_method".to_string(),
                                "shared_name".to_string(),
                                "unrelated_method".to_string(),
                                "imported_method".to_string(),
                            ],
                            expected_fallback: true,
                            import_visibility: false,
                        },
                        MethodCompletionProviderExpectation {
                            id: "imported_unknown_receiver_uses_bounded_import_fallback"
                                .to_string(),
                            cursor_marker: "cursor:imported_unknown".to_string(),
                            expected_receiver_package: None,
                            expected_present: vec!["imported_method".to_string()],
                            expected_absent: vec!["unrelated_method".to_string()],
                            expected_fallback: true,
                            import_visibility: true,
                        },
                    ],
                    diagnostics: vec![],
                    navigation: vec![],
                },
            }],
        }
    }

    fn diagnostic_provider_manifest() -> ParserAccuracyManifest {
        ParserAccuracyManifest {
            schema_version: 1,
            fixtures: vec![FixtureMetadata {
                id: "diagnostic_provider".to_string(),
                family: "provider_diagnostics".to_string(),
                label_mode: LabelMode::Partial,
                source_path: "diagnostic_provider.pl".to_string(),
                scored_lines: 0,
                scored_symbols: 0,
                fully_labeled_regions: 0,
                partial_labeled_regions: 1,
                unknown_regions: 0,
                negative_regions: 0,
                dynamic_boundaries: 4,
                unsupported_constructs: 0,
                real_project_file: false,
                generated: false,
                line_expectations: vec![],
                ast_expectations: vec![],
                symbol_expectations: SymbolExpectations::default(),
                symbol_safety_regions: vec![],
                recovery_expectations: vec![],
                incremental_expectations: vec![],
                span_expectations: vec![],
                provider_expectations: ProviderExpectations {
                    method_completion: vec![],
                    diagnostics: vec![
                        DiagnosticProviderExpectation {
                            id: "eval_generated_symbol_suppressed".to_string(),
                            expected_code: "PL109".to_string(),
                            message_contains: "eval_generated_symbol".to_string(),
                            expected_present: false,
                            dynamic_boundary: true,
                        },
                        DiagnosticProviderExpectation {
                            id: "dynamic_imported_symbol_suppressed".to_string(),
                            expected_code: "PL109".to_string(),
                            message_contains: "dynamic_imported_symbol".to_string(),
                            expected_present: false,
                            dynamic_boundary: true,
                        },
                        DiagnosticProviderExpectation {
                            id: "generated_accessor_suppressed".to_string(),
                            expected_code: "PL109".to_string(),
                            message_contains: "generated_accessor".to_string(),
                            expected_present: false,
                            dynamic_boundary: true,
                        },
                        DiagnosticProviderExpectation {
                            id: "autoload_symbol_suppressed".to_string(),
                            expected_code: "PL103".to_string(),
                            message_contains: "$AUTOLOAD".to_string(),
                            expected_present: false,
                            dynamic_boundary: true,
                        },
                        DiagnosticProviderExpectation {
                            id: "known_imported_symbol_not_diagnosed".to_string(),
                            expected_code: "PL109".to_string(),
                            message_contains: "imported_known".to_string(),
                            expected_present: false,
                            dynamic_boundary: false,
                        },
                        DiagnosticProviderExpectation {
                            id: "package_local_symbol_not_diagnosed".to_string(),
                            expected_code: "PL103".to_string(),
                            message_contains: "$package_local_symbol".to_string(),
                            expected_present: false,
                            dynamic_boundary: false,
                        },
                        DiagnosticProviderExpectation {
                            id: "ordinary_undefined_variable_diagnosed".to_string(),
                            expected_code: "PL103".to_string(),
                            message_contains: "$ordinary_missing".to_string(),
                            expected_present: true,
                            dynamic_boundary: false,
                        },
                        DiagnosticProviderExpectation {
                            id: "ordinary_undefined_bareword_diagnosed".to_string(),
                            expected_code: "PL109".to_string(),
                            message_contains: "truly_missing_symbol".to_string(),
                            expected_present: true,
                            dynamic_boundary: false,
                        },
                    ],
                    navigation: vec![],
                },
            }],
        }
    }

    fn navigation_provider_manifest() -> ParserAccuracyManifest {
        ParserAccuracyManifest {
            schema_version: 1,
            fixtures: vec![FixtureMetadata {
                id: "navigation_provider".to_string(),
                family: "provider_navigation".to_string(),
                label_mode: LabelMode::Partial,
                source_path: "navigation_provider.pl".to_string(),
                scored_lines: 0,
                scored_symbols: 0,
                fully_labeled_regions: 0,
                partial_labeled_regions: 1,
                unknown_regions: 0,
                negative_regions: 0,
                dynamic_boundaries: 1,
                unsupported_constructs: 0,
                real_project_file: false,
                generated: false,
                line_expectations: vec![],
                ast_expectations: vec![],
                symbol_expectations: SymbolExpectations::default(),
                symbol_safety_regions: vec![],
                recovery_expectations: vec![],
                incremental_expectations: vec![],
                span_expectations: vec![],
                provider_expectations: ProviderExpectations {
                    method_completion: vec![],
                    diagnostics: vec![],
                    navigation: vec![
                        NavigationProviderExpectation {
                            id: "document_symbol_spans".to_string(),
                            symbol: "own_sub".to_string(),
                            cursor_marker: None,
                            cursor_symbol: None,
                            expected_document_symbols: vec![
                                "$name".to_string(),
                                "$self".to_string(),
                                "package Accuracy::Navigation::UseCases".to_string(),
                                "sub bare_call_case {\n    own_sub();                   \n}"
                                    .to_string(),
                                "sub dynamic_boundary_case {\n    my $name = \"own_sub\";\n    no strict 'refs';\n    &$name();                     \n}"
                                    .to_string(),
                                "sub generated_accessor_case {\n    Accuracy::Navigation::Generated::generated_accessor();                            \n}"
                                    .to_string(),
                                "sub imported_symbol_case {\n    imported_nav();                      \n}"
                                    .to_string(),
                                "sub method_receiver_case {\n    my $self = shift;\n    $self->inherited_method();                          \n}"
                                    .to_string(),
                                "sub own_method { 1 }".to_string(),
                                "sub own_sub { 1 }".to_string(),
                                "sub qualified_call_case {\n    Accuracy::Navigation::UseCases::own_sub();                        \n}"
                                    .to_string(),
                            ],
                            expected_definition_span: None,
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "bare_call_goto".to_string(),
                            symbol: "own_sub".to_string(),
                            cursor_marker: Some("cursor:bare_call".to_string()),
                            cursor_symbol: Some("own_sub".to_string()),
                            expected_document_symbols: vec![],
                            expected_definition_span: Some("own_sub".to_string()),
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "qualified_call_goto".to_string(),
                            symbol: "Accuracy::Navigation::UseCases::own_sub".to_string(),
                            cursor_marker: Some("cursor:qualified_call".to_string()),
                            cursor_symbol: Some("own_sub".to_string()),
                            expected_document_symbols: vec![],
                            expected_definition_span: Some("own_sub".to_string()),
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![
                                "Accuracy::Navigation::UseCases::own_sub".to_string(),
                                "Subroutine".to_string(),
                            ],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "imported_symbol_goto_and_hover".to_string(),
                            symbol: "imported_nav".to_string(),
                            cursor_marker: Some("cursor:imported_nav".to_string()),
                            cursor_symbol: Some("imported_nav".to_string()),
                            expected_document_symbols: vec![],
                            expected_definition_span: Some("imported_nav".to_string()),
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "own_sub_references".to_string(),
                            symbol: "Accuracy::Navigation::UseCases::own_sub".to_string(),
                            cursor_marker: None,
                            cursor_symbol: None,
                            expected_document_symbols: vec![],
                            expected_definition_span: None,
                            expected_references: vec![
                                "Accuracy::Navigation::UseCases::own_sub()".to_string(),
                            ],
                            unexpected_references: vec!["&$name()".to_string()],
                            hover_contains: vec![],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "own_sub_rename_safe_edits".to_string(),
                            symbol: "Accuracy::Navigation::UseCases::own_sub".to_string(),
                            cursor_marker: None,
                            cursor_symbol: None,
                            expected_document_symbols: vec![],
                            expected_definition_span: None,
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![],
                            rename_new_name: Some("renamed_sub".to_string()),
                            expected_rename_safe_edit: Some(true),
                            expected_rename_edit_count: Some(3),
                            expected_safe_delete_blocked: None,
                            expected_safe_delete_blocker_count: None,
                        },
                        NavigationProviderExpectation {
                            id: "own_sub_safe_delete_blocked".to_string(),
                            symbol: "Accuracy::Navigation::UseCases::own_sub".to_string(),
                            cursor_marker: None,
                            cursor_symbol: None,
                            expected_document_symbols: vec![],
                            expected_definition_span: None,
                            expected_references: vec![],
                            unexpected_references: vec![],
                            hover_contains: vec![],
                            rename_new_name: None,
                            expected_rename_safe_edit: None,
                            expected_rename_edit_count: None,
                            expected_safe_delete_blocked: Some(true),
                            expected_safe_delete_blocker_count: Some(1),
                        },
                    ],
                },
            }],
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
    fn metric_source_cache_reports_reused_fixture_source_hits() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let source_path = tmp.path().join("fixture.pl");
        fs::write(&source_path, "package Cache;\n1;\n")?;
        let mut source_cache = MetricSourceCache::default();

        assert_eq!(source_cache.hit_rate(), None);
        assert_eq!(source_cache.read(&source_path, "test fixture source")?, "package Cache;\n1;\n");
        assert_eq!(source_cache.hit_rate(), Some(0.0));
        assert_eq!(source_cache.read(&source_path, "test fixture source")?, "package Cache;\n1;\n");
        assert_eq!(source_cache.hit_rate(), Some(0.5));

        Ok(())
    }

    #[test]
    fn measured_memory_prefers_positive_rss_delta_and_falls_back_to_allocator_peak() {
        assert_eq!(measured_memory_mb(Some(10.0), Some(13.5), 1.0), Some(3.5));
        assert_eq!(measured_memory_mb(Some(13.5), Some(10.0), 2.25), Some(2.25));
        assert_eq!(measured_memory_mb(None, Some(10.0), 0.0), None);
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
    fn method_completion_provider_scorer_measures_receiver_and_fallback() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_method_completion_provider_fixture(tmp.path())?;
        let mut source_cache = MetricSourceCache::default();

        let score = score_method_completion_provider_expectations(
            tmp.path(),
            &method_completion_provider_manifest(),
            &mut source_cache,
        )?;

        assert_eq!(score.receiver_expected_count, 1);
        assert_eq!(score.receiver_hit_count, 1);
        assert_eq!(score.fallback_expected_count, 2);
        assert_eq!(score.fallback_correct_count, 2);
        assert_eq!(score.false_receiver_count, 0);
        assert_eq!(score.relevance_assertion_count, 9);
        assert_eq!(score.relevance_assertion_correct_count, 9);
        assert_eq!(score.import_visibility_expected_count, 1);
        assert_eq!(score.import_visibility_correct_count, 1);
        assert_eq!(score.completion_query_micros.len(), 3);

        let metrics = provider_impact_metrics(
            &score,
            &DiagnosticProviderScore::default(),
            &NavigationProviderScore::default(),
            Cadence::Pr,
        );
        let false_receiver = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "method_completion_false_receiver_count"
                )
            })
            .ok_or_else(|| eyre!("method completion false receiver row should be measured"))?;
        assert!(matches!(
            false_receiver,
            MetricRow::Measured { value, sample_count: 2, .. }
                if (*value - 0.0).abs() < f64::EPSILON
        ));
        let visible_symbol_relevance = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_completion_visible_symbol_relevance"
                )
            })
            .ok_or_else(|| eyre!("provider completion visible-symbol row should be measured"))?;
        assert!(matches!(
            visible_symbol_relevance,
            MetricRow::Measured { value, sample_count: 9, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let import_visibility = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_completion_import_visibility_accuracy"
                )
            })
            .ok_or_else(|| eyre!("provider completion import visibility row should be measured"))?;
        assert!(matches!(
            import_visibility,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let cost_metrics = cost_metrics(
            &ScaleCostScore::default(),
            &RecoveryScore::default(),
            &score,
            &NavigationProviderScore::default(),
            Cadence::Pr,
        );
        let completion_query_ms = cost_metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. } if metric == "completion_query_ms_p95"
                )
            })
            .ok_or_else(|| eyre!("completion query timing row should be measured"))?;
        assert!(matches!(
            completion_query_ms,
            MetricRow::Measured { value, sample_count: 3, .. } if *value >= 0.0
        ));
        Ok(())
    }

    #[test]
    fn diagnostic_provider_scorer_measures_false_positive_and_negative_rows() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_diagnostic_provider_fixture(tmp.path())?;
        let mut source_cache = MetricSourceCache::default();

        let score = score_diagnostic_provider_expectations(
            tmp.path(),
            &diagnostic_provider_manifest(),
            &mut source_cache,
        )?;

        assert_eq!(score.dynamic_boundary_expected_absent_count, 4);
        assert_eq!(score.dynamic_boundary_false_positive_count, 0);
        assert_eq!(score.undefined_expected_absent_count, 2);
        assert_eq!(score.undefined_false_positive_count, 0);
        assert_eq!(score.undefined_expected_present_count, 2);
        assert_eq!(score.undefined_false_negative_count, 0);

        let metrics = provider_impact_metrics(
            &MethodCompletionProviderScore::default(),
            &score,
            &NavigationProviderScore::default(),
            Cadence::Pr,
        );
        let dynamic_false_positive = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "diagnostic_dynamic_boundary_false_positive_count"
                )
            })
            .ok_or_else(|| eyre!("diagnostic dynamic-boundary false-positive row should exist"))?;
        assert!(matches!(
            dynamic_false_positive,
            MetricRow::Measured { value, sample_count: 4, .. }
                if (*value - 0.0).abs() < f64::EPSILON
        ));
        let false_negative_rate = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_diagnostic_false_negative_rate"
                )
            })
            .ok_or_else(|| eyre!("provider diagnostic false-negative row should be measured"))?;
        assert!(matches!(
            false_negative_rate,
            MetricRow::Measured { value, sample_count: 2, .. }
                if (*value - 0.0).abs() < f64::EPSILON
        ));
        let false_positive_rate = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_diagnostic_false_positive_rate"
                )
            })
            .ok_or_else(|| eyre!("provider diagnostic false-positive row should be measured"))?;
        assert!(matches!(
            false_positive_rate,
            MetricRow::Measured { value, sample_count: 6, .. }
                if (*value - 0.0).abs() < f64::EPSILON
        ));
        Ok(())
    }

    #[test]
    fn navigation_provider_scorer_measures_provider_rows() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_navigation_provider_fixture(tmp.path())?;
        let mut source_cache = MetricSourceCache::default();

        let score = score_navigation_provider_expectations(
            tmp.path(),
            &navigation_provider_manifest(),
            &mut source_cache,
        )?;

        assert_eq!(score.document_symbol_expected_count, 11);
        assert_eq!(score.document_symbol_returned_count, 11);
        assert_eq!(score.document_symbol_span_exact_count, 11);
        assert_eq!(score.goto_definition_expected_count, 3);
        assert_eq!(score.goto_definition_hit_count, 3);
        assert_eq!(score.goto_definition_span_exact_count, 3);
        assert_eq!(score.goto_definition_false_target_count, 0);
        assert_eq!(score.definition_query_micros.len(), 3);
        assert_eq!(score.references_expected_count, 1);
        assert_eq!(score.references_hit_count, 1);
        assert_eq!(score.references_returned_count, 1);
        assert_eq!(score.references_false_positive_count, 0);
        assert_eq!(score.reference_query_micros.len(), 1);
        assert_eq!(score.hover_expected_count, 1);
        assert_eq!(score.hover_origin_correct_count, 1);
        assert_eq!(score.rename_safe_edit_expected_count, 1);
        assert_eq!(score.rename_safe_edit_correct_count, 0);
        assert_eq!(score.safe_delete_blocker_expected_count, 1);
        assert_eq!(score.safe_delete_blocker_correct_count, 1);

        let metrics = provider_impact_metrics(
            &MethodCompletionProviderScore::default(),
            &DiagnosticProviderScore::default(),
            &score,
            Cadence::Pr,
        );
        let goto_hit_rate = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. } if metric == "goto_definition_hit_rate"
                )
            })
            .ok_or_else(|| eyre!("navigation goto-definition row should exist"))?;
        assert!(matches!(
            goto_hit_rate,
            MetricRow::Measured { value, sample_count: 3, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let document_symbol_precision = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_document_symbol_precision"
                )
            })
            .ok_or_else(|| eyre!("provider document-symbol precision row should exist"))?;
        assert!(matches!(
            document_symbol_precision,
            MetricRow::Measured { value, sample_count: 11, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let document_symbol_recall = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_document_symbol_recall"
                )
            })
            .ok_or_else(|| eyre!("provider document-symbol recall row should exist"))?;
        assert!(matches!(
            document_symbol_recall,
            MetricRow::Measured { value, sample_count: 11, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let goto_hit_rate = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_goto_definition_hit_rate"
                )
            })
            .ok_or_else(|| eyre!("provider goto-definition hit-rate row should exist"))?;
        assert!(matches!(
            goto_hit_rate,
            MetricRow::Measured { value, sample_count: 3, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let hover_origin_accuracy = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_hover_symbol_origin_accuracy"
                )
            })
            .ok_or_else(|| eyre!("provider hover symbol-origin row should exist"))?;
        assert!(matches!(
            hover_origin_accuracy,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let references_precision = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_references_precision"
                )
            })
            .ok_or_else(|| eyre!("provider references precision row should exist"))?;
        assert!(matches!(
            references_precision,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let references_recall = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_references_recall"
                )
            })
            .ok_or_else(|| eyre!("provider references recall row should exist"))?;
        assert!(matches!(
            references_recall,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let rename_safe_edit_accuracy = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_rename_safe_edit_accuracy"
                )
            })
            .ok_or_else(|| eyre!("provider rename safe-edit row should exist"))?;
        assert!(matches!(
            rename_safe_edit_accuracy,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 0.0).abs() < f64::EPSILON
        ));
        let safe_delete_blocker_accuracy = metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. }
                        if metric == "provider_safe_delete_blocker_accuracy"
                )
            })
            .ok_or_else(|| eyre!("provider safe-delete blocker row should exist"))?;
        assert!(matches!(
            safe_delete_blocker_accuracy,
            MetricRow::Measured { value, sample_count: 1, .. }
                if (*value - 1.0).abs() < f64::EPSILON
        ));
        let cost_metrics = cost_metrics(
            &ScaleCostScore::default(),
            &RecoveryScore::default(),
            &MethodCompletionProviderScore::default(),
            &score,
            Cadence::Pr,
        );
        let definition_query_ms = cost_metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. } if metric == "definition_query_ms_p95"
                )
            })
            .ok_or_else(|| eyre!("definition query timing row should be measured"))?;
        assert!(matches!(
            definition_query_ms,
            MetricRow::Measured { value, sample_count: 3, .. } if *value >= 0.0
        ));
        let reference_query_ms = cost_metrics
            .iter()
            .find(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured { metric, .. } if metric == "reference_query_ms_p95"
                )
            })
            .ok_or_else(|| eyre!("reference query timing row should be measured"))?;
        assert!(matches!(
            reference_query_ms,
            MetricRow::Measured { value, sample_count: 1, .. } if *value >= 0.0
        ));
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
    fn line_tags_count_eval_as_function_like_call() -> Result<()> {
        let actual_by_line =
            extract_line_tags("package Accuracy::Eval;\n\nmy $code = '1';\neval $code;\n");
        let line_tags = actual_by_line
            .get(&4)
            .ok_or_else(|| eyre!("expected line 4 tags for eval expression"))?;

        assert!(line_tags.contains(&LineTag::FunctionCall));
        Ok(())
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
    fn ast_scorer_counts_delimiter_pairing_from_gold_span() {
        let expectations = vec![AstExpectation {
            id: "subroutine".to_string(),
            kind: "Subroutine".to_string(),
            line: 3,
            span_text: "sub answer { 42 }".to_string(),
            parent_kind: Some("Program".to_string()),
            depth: Some(1),
            operator: None,
            parent_operator: None,
        }];
        let predictions = vec![AstPrediction {
            kind: "Subroutine".to_string(),
            line: 3,
            span_text: "sub answer { 42 }".to_string(),
            parent_kind: Some("Program".to_string()),
            depth: 1,
            operator: None,
            parent_operator: None,
        }];
        let mut score = AstScore::default();

        score_ast_expectations(&expectations, &predictions, &mut score);

        assert_eq!(score.delimiter_pairing_expected_count, 1);
        assert_eq!(score.delimiter_pairing_correct_count, 1);
        let metrics = ast_metrics(&score, Cadence::Pr);
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "ast_delimiter_pairing_accuracy"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn ast_scorer_prefers_best_shape_when_kind_and_line_are_ambiguous() -> Result<()> {
        let expectations = vec![AstExpectation {
            id: "operator_multiplication".to_string(),
            kind: "Binary".to_string(),
            line: 3,
            span_text: "2 * 3".to_string(),
            parent_kind: Some("Binary".to_string()),
            depth: Some(3),
            operator: Some("*".to_string()),
            parent_operator: Some("+".to_string()),
        }];
        let predictions = vec![
            AstPrediction {
                kind: "Binary".to_string(),
                line: 3,
                span_text: "1 + 2 * 3".to_string(),
                parent_kind: Some("VariableDeclaration".to_string()),
                depth: 2,
                operator: Some("+".to_string()),
                parent_operator: None,
            },
            AstPrediction {
                kind: "Binary".to_string(),
                line: 3,
                span_text: "2 * 3".to_string(),
                parent_kind: Some("Binary".to_string()),
                depth: 3,
                operator: Some("*".to_string()),
                parent_operator: Some("+".to_string()),
            },
        ];
        let mut score = AstScore::default();

        score_ast_expectations(&expectations, &predictions, &mut score);

        assert_eq!(score.node_kind_true_positive_count, 1);
        assert_eq!(score.span_exact_count, 1);
        assert_eq!(score.parent_child_correct_count, 1);
        assert_eq!(score.tree_depth_correct_count, 1);
        assert_eq!(score.operator_precedence_correct_count, 1);
        Ok(())
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
    fn safety_floor_metadata_marks_only_zero_false_precision_candidates() {
        let mut metrics = vec![
            measured_count("dynamic_false_precision_count", 0, 1, Cadence::Pr),
            measured_count("fast_path_wrong_result_count", 0, 1, Cadence::Pr),
            measured_count("line_construct_f1", 1, 1, Cadence::Pr),
        ];

        apply_safety_floor_metadata(&mut metrics);

        for name in ["dynamic_false_precision_count", "fast_path_wrong_result_count"] {
            assert!(metrics.iter().any(|metric| {
                matches!(
                    metric,
                    MetricRow::Measured {
                        metric,
                        value,
                        previous: Some(0.0),
                        delta: Some(0.0),
                        floor: Some(0.0),
                        threshold: Some(0.0),
                        direction: Direction::Down,
                        sample_count: 1,
                        ..
                    } if metric == name && (*value - 0.0).abs() < f64::EPSILON
                )
            }));
        }
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured {
                    metric,
                    floor: None,
                    threshold: None,
                    direction: Direction::Neutral,
                    ..
                } if metric == "line_construct_f1"
            )
        }));
    }

    #[test]
    fn ratchet_receipt_keeps_precision_recall_metrics_out_of_floor_map() {
        let artifact = ParserAccuracyArtifact {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            generated_at: "2026-05-03T00:00:00Z".to_string(),
            commit: "test".to_string(),
            cadence: Cadence::Pr,
            denominator: Denominator::default(),
            families: Vec::new(),
            metrics: vec![
                measured_count("dynamic_false_precision_count", 0, 1, Cadence::Pr),
                measured_count("fast_path_wrong_result_count", 0, 1, Cadence::Pr),
                measured_value("line_construct_f1", 0.875, 8, Cadence::Pr),
                measured_value("symbol_decl_precision", 0.9, 10, Cadence::Pr),
            ],
            failure_packets: Vec::new(),
            gold_drift: GoldDrift::default(),
            metric_runtime: MetricRuntime::default(),
        };

        let receipt = ratchet_receipt_for_artifact(&artifact);

        assert_eq!(receipt.floor_metrics.len(), 2);
        assert_eq!(receipt.floor_metrics.get("dynamic_false_precision_count"), Some(&Some(0.0)));
        assert_eq!(receipt.floor_metrics.get("fast_path_wrong_result_count"), Some(&Some(0.0)));
        assert!(
            !receipt.floor_metrics.contains_key("line_construct_f1"),
            "precision/recall rows must stay out of hard floors until sample counts stabilize"
        );
        assert_eq!(receipt.improvement_metrics.get("line_construct_f1"), Some(&Some(0.875)));
        assert_eq!(receipt.improvement_metrics.get("symbol_decl_precision"), Some(&Some(0.9)));
    }

    #[test]
    fn status_receipts_export_failure_packets_inventory_worklist_and_pointer() -> Result<()> {
        let manifest = fixture_manifest();
        let artifact = ParserAccuracyArtifact {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            generated_at: "2026-05-05T00:00:00Z".to_string(),
            commit: "test-commit".to_string(),
            cadence: Cadence::Pr,
            denominator: compute_denominator(&manifest),
            families: summarize_families(&manifest),
            metrics: vec![measured_count("dynamic_false_precision_count", 0, 1, Cadence::Pr)],
            failure_packets: vec![FailurePacket {
                failure_kind: "missing_symbol_reference".to_string(),
                likely_layer: "semantic_fact_extraction".to_string(),
                fixture_id: "qualified_refs".to_string(),
                family: Some("qualified_references".to_string()),
                metric: Some("symbol_ref_f1".to_string()),
                line: Some(7),
                expected: vec!["Accuracy::Refs::target".to_string()],
                actual: vec![],
                nearest_predictions: vec!["Accuracy::Refs::nearby".to_string()],
                source_excerpt: Some("Accuracy::Refs::target();".to_string()),
                details: None,
                suggested_next_fix: Some(
                    "Inspect reference extraction before changing gold.".to_string(),
                ),
            }],
            gold_drift: GoldDrift::default(),
            metric_runtime: MetricRuntime::default(),
        };

        let failure_receipt = render_failure_packet_status_receipt(&artifact)?;
        assert!(failure_receipt.contains("\"failure_packet_count\": 1"));
        assert!(failure_receipt.contains("\"actual_nearest\""));
        assert!(failure_receipt.contains("\"suggested_next_pr\""));

        let inventory_receipt = render_fixture_inventory_status_receipt(&manifest, &artifact)?;
        assert!(inventory_receipt.contains("\"fixture_count\": 2"));
        assert!(inventory_receipt.contains("\"provider_expectation_counts\""));

        let worklist = render_failure_worklist_status_receipt(&artifact);
        assert!(worklist.contains("| missing_symbol_reference | 1 | semantic_fact_extraction |"));
        assert!(worklist.contains("fix(semantic): resolve parser-accuracy semantic fact packet"));

        let pointer = render_next_pointer_status_receipt(&artifact);
        assert!(pointer.contains("| Pointer | `missing_symbol_reference` |"));
        assert!(pointer.contains("| First fixture | `qualified_refs` |"));

        let files = status_receipt_files(Path::new("."), &manifest, &artifact)?;
        assert!(
            files.iter().any(|file| file.name == FAILURE_WORKLIST_STATUS_RECEIPT),
            "status export must include parser_accuracy_failure_worklist.md"
        );
        assert!(
            files.iter().any(|file| file.name == NEXT_POINTER_STATUS_RECEIPT),
            "status export must include parser_accuracy_next.md"
        );
        Ok(())
    }

    #[test]
    fn next_pointer_lists_measurement_gaps_when_failure_packets_are_empty() {
        let manifest = fixture_manifest();
        let artifact = ParserAccuracyArtifact {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            generated_at: "2026-05-05T00:00:00Z".to_string(),
            commit: "test-commit".to_string(),
            cadence: Cadence::Pr,
            denominator: compute_denominator(&manifest),
            families: summarize_families(&manifest),
            metrics: vec![
                insufficient("gold_changed_line_count", "gold drift baseline is not wired yet"),
                insufficient("completion_query_ms_p95", "provider query timing is not wired yet"),
                insufficient(
                    "provider_goto_definition_hit_rate",
                    "provider gold fixtures are not wired yet",
                ),
            ],
            failure_packets: Vec::new(),
            gold_drift: GoldDrift::default(),
            metric_runtime: MetricRuntime::default(),
        };

        let pointer = render_next_pointer_status_receipt(&artifact);

        assert!(pointer.contains("Pointer: no active failure packets."));
        assert!(pointer.contains("## Next Measurement Gaps"));
        assert!(pointer.contains("provider_goto_definition_hit_rate"));
        assert!(pointer.contains(
            "test(parser-accuracy): wire provider gold fixture for provider_goto_definition_hit_rate"
        ));
        assert!(pointer.contains("completion_query_ms_p95"));
        assert!(matches!(
            (
                pointer.find("provider_goto_definition_hit_rate"),
                pointer.find("completion_query_ms_p95")
            ),
            (Some(provider_index), Some(timing_index)) if provider_index < timing_index
        ));
    }

    #[test]
    fn status_receipt_stale_check_ignores_commit_only() {
        let existing = r#"{"schema_version":1,"commit":"old","fixture_count":2}"#;
        let generated = r#"{"schema_version":1,"commit":"new","fixture_count":2}"#;
        let stale = r#"{"schema_version":1,"commit":"new","fixture_count":3}"#;

        assert!(status_receipt_equivalent_ignoring_commit(existing, generated));
        assert!(!status_receipt_equivalent_ignoring_commit(existing, stale));
    }

    #[test]
    fn recovery_scorer_counts_local_spillover_and_salvaged_symbols() {
        let expectation = RecoveryExpectation {
            id: "recover_before_sub".to_string(),
            first_error_line: 3,
            error_region: LineRange { start: 3, end: 3 },
            recovery_line: 5,
            post_error_line_expectations: vec![LineExpectation {
                line: 5,
                expected_tags: tags(&[LineTag::SubDecl]),
            }],
            post_error_symbol_spans: vec!["after_recovery".to_string()],
        };
        let prediction = RecoveryPrediction {
            first_error_line: Some(3),
            error_region_lines: [3].into_iter().collect(),
            actual_by_line: [(5, tags(&[LineTag::SubDecl]))].into_iter().collect(),
            symbol_spans: [SymbolSpanLocation { line: 5, span_text: "after_recovery".to_string() }]
                .into_iter()
                .collect(),
        };
        let mut score = RecoveryScore::default();

        score_recovery_expectation(&expectation, &prediction, &mut score);

        assert_eq!(score.first_error_line_correct_count, 1);
        assert_eq!(score.error_region_true_positive_count, 1);
        assert_eq!(score.spillover_lines, vec![0]);
        assert_eq!(score.post_error_line_score.exact_match_count, 1);
        assert_eq!(score.post_error_symbol_found_count, 1);
    }

    #[test]
    fn recovery_metrics_emit_measured_containment_rows() {
        let score = RecoveryScore {
            expectation_count: 1,
            first_error_line_correct_count: 1,
            error_region_true_positive_count: 1,
            spillover_lines: vec![0, 2],
            recovery_parse_micros: vec![1_000, 2_000],
            post_error_line_score: LineScore {
                line_count: 1,
                true_positive_count: 1,
                exact_match_count: 1,
                ..LineScore::default()
            },
            post_error_symbol_expected_count: 1,
            post_error_symbol_found_count: 1,
            ..RecoveryScore::default()
        };

        let metrics = recovery_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "recovery_spillover_p95_lines"
                        && (*value - 2.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "recovery_post_error_symbol_recall"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        let cost_metrics = cost_metrics(
            &ScaleCostScore::default(),
            &score,
            &MethodCompletionProviderScore::default(),
            &NavigationProviderScore::default(),
            Cadence::Pr,
        );
        assert!(cost_metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "recovery_ms_p95"
                        && (*value - 2.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn incremental_scorer_compares_full_parse_and_apply_path() {
        let source =
            "package Accuracy::IncrementalSmallEdit;\n\nmy $value = 1;\n\nsub value { $value }\n";
        let expectation = IncrementalExpectation {
            id: "small_value_edit_matches_full_parse".to_string(),
            edits: vec![IncrementalEditExpectation {
                old_text: "my $value = 1;".to_string(),
                new_text: "my $value = 2;".to_string(),
                occurrence: None,
            }],
        };
        let mut score = IncrementalScore::default();

        score_incremental_expectation(source, &expectation, &mut score);

        assert_eq!(score.expectation_count, 1);
        assert_eq!(score.no_panic_count, 1);
        assert_eq!(score.edit_apply_equivalent_count, 1);
        assert_eq!(score.full_parse_equivalent_count, 1);
        assert_eq!(score.changed_range_correct_count, 1);
        assert_eq!(score.checkpoint_hit_count, 1);
        assert_eq!(score.checkpoint_miss_count, 0);
        assert_eq!(score.reparse_byte_ratios.len(), 1);
        assert_eq!(score.reused_node_ratios.len(), 1);
    }

    #[test]
    fn incremental_metrics_emit_equivalence_rows_and_token_reuse_gap() {
        let score = IncrementalScore {
            expectation_count: 1,
            full_parse_equivalent_count: 1,
            edit_apply_equivalent_count: 1,
            no_panic_count: 1,
            checkpoint_hit_count: 1,
            reparse_byte_ratios: vec![0.25],
            reused_node_ratios: vec![0.75],
            changed_range_sample_count: 1,
            changed_range_correct_count: 1,
            ..IncrementalScore::default()
        };

        let metrics = incremental_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "incremental_full_parse_equivalence_rate"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "incremental_reused_token_ratio"
            )
        }));
    }

    #[test]
    fn span_scorer_counts_utf16_unicode_crlf_and_tab_coordinates() {
        let source = "package Span;\r\nmy $emoji = \"😀\";\n\treturn \"café\";\r\n";
        let expectation = SpanExpectation {
            id: "emoji_string".to_string(),
            span_text: "\"😀\"".to_string(),
            occurrence: None,
            byte_start: 27,
            byte_end: 33,
            line_start: 2,
            line_end: 2,
            utf16_start: SpanPositionExpectation { line: 1, character: 12 },
            utf16_end: SpanPositionExpectation { line: 1, character: 16 },
        };
        let tab_expectation = SpanExpectation {
            id: "tabbed_return".to_string(),
            span_text: "\treturn \"café\";".to_string(),
            occurrence: None,
            byte_start: 35,
            byte_end: 51,
            line_start: 3,
            line_end: 3,
            utf16_start: SpanPositionExpectation { line: 2, character: 0 },
            utf16_end: SpanPositionExpectation { line: 2, character: 15 },
        };
        let mut score = SpanScore::default();

        score_span_expectation(source, &expectation, &mut score);
        score_span_expectation(source, &tab_expectation, &mut score);

        assert_eq!(score.expectation_count, 2);
        assert_eq!(score.byte_exact_count, 2);
        assert_eq!(score.line_exact_count, 2);
        assert_eq!(score.utf16_exact_count, 2);
        assert_eq!(score.crlf_sample_count, 2);
        assert_eq!(score.unicode_sample_count, 2);
        assert_eq!(score.tab_sample_count, 1);
        assert_eq!(score.unicode_position_error_count, 0);
        assert_eq!(score.tab_column_mismatch_count, 0);
    }

    #[test]
    fn span_metrics_emit_coordinate_rows() {
        let score = SpanScore {
            expectation_count: 2,
            byte_exact_count: 2,
            line_exact_count: 2,
            utf16_exact_count: 2,
            near_count: 2,
            crlf_sample_count: 1,
            unicode_sample_count: 1,
            tab_sample_count: 1,
            ..SpanScore::default()
        };

        let metrics = span_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "utf16_range_exact_rate"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "unicode_position_error_count"
                        && (*value - 0.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn confidence_metrics_emit_precision_and_calibration_rows() {
        let mut proof_score = ProofScore::default();
        proof_score.true_positive_by_bucket.insert(ProofBucket::Exact, 1);
        proof_score.predicted_by_bucket.insert(ProofBucket::Exact, 2);
        proof_score.true_positive_by_bucket.insert(ProofBucket::High, 1);
        proof_score.predicted_by_bucket.insert(ProofBucket::High, 2);
        proof_score.predicted_by_bucket.insert(ProofBucket::Medium, 1);
        proof_score.high_confidence_false_positive_count = 1;
        let score = SymbolScore { proof_score, ..SymbolScore::default() };

        let metrics = confidence_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "exact_fact_precision"
                        && (*value - 0.5).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "confidence_calibration_error"
                        && (*value - 0.5).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "low_confidence_precision"
            )
        }));
    }

    #[test]
    fn unsupported_metrics_emit_construct_rows() {
        let score = UnsupportedScore {
            manifest_construct_count: 2,
            family_count: 2,
            line_labeled_construct_count: 1,
            detected_count: 1,
            salvaged_count: 1,
            false_exact_count: 0,
            false_exact_sample_count: 1,
        };

        let metrics = unsupported_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "unsupported_construct_detected_count"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "unsupported_construct_family_count"
                        && (*value - 2.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "unsupported_construct_false_exact_count"
                        && (*value - 0.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn provider_impact_metrics_remain_insufficient_until_gold_exists() {
        let metrics = provider_impact_metrics(
            &MethodCompletionProviderScore::default(),
            &DiagnosticProviderScore::default(),
            &NavigationProviderScore::default(),
            Cadence::Pr,
        );

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "provider_goto_definition_hit_rate"
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "provider_diagnostic_false_negative_rate"
            )
        }));
    }

    #[test]
    fn scale_and_cost_metrics_emit_shape_rows_and_timing_rows() {
        let score = ScaleCostScore {
            fixture_count: 2,
            file_bytes: 120,
            source_lines: 10,
            token_count: 30,
            ast_node_count: 20,
            symbol_count: 6,
            import_count: 2,
            export_count: 1,
            sub_count: 3,
            package_count: 2,
            max_nesting_depth: 4,
            max_brace_depth: 3,
            max_regex_length: 12,
            max_heredoc_body_bytes: 40,
            quote_like_count: 2,
            dynamic_boundary_count: 1,
            lex_ms: vec![0.1, 0.2],
            parse_ms: vec![0.3, 0.4],
            ast_projection_ms: vec![0.01, 0.02],
            semantic_extraction_ms: vec![0.5, 0.7],
            workspace_insert_ms: vec![0.8, 1.2],
        };

        let mut metrics = scale_metrics(&score, Cadence::Pr);
        metrics.extend(cost_metrics(
            &score,
            &RecoveryScore::default(),
            &MethodCompletionProviderScore::default(),
            &NavigationProviderScore::default(),
            Cadence::Pr,
        ));

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "scale_token_count"
                        && (*value - 30.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "parse_ms_p95"
                        && (*value - 0.4).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "workspace_insert_ms_p95"
                        && (*value - 1.2).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "peak_rss_mb"
            )
        }));
    }

    #[test]
    fn cache_reuse_metrics_emit_fast_path_rows() {
        let score = IncrementalScore {
            expectation_count: 2,
            full_parse_equivalent_count: 1,
            fallback_count: 1,
            checkpoint_hit_count: 3,
            checkpoint_miss_count: 1,
            content_hash_hit_count: 3,
            content_hash_miss_count: 1,
            semantic_fact_cache_hit_count: 2,
            semantic_fact_cache_miss_count: 2,
            workspace_shard_reuse_count: 3,
            workspace_shard_replacement_attempt_count: 4,
            unchanged_file_skip_count: 1,
            unchanged_file_index_attempt_count: 4,
            ..IncrementalScore::default()
        };

        let metrics = cache_reuse_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "lexer_checkpoint_reuse_rate"
                        && (*value - 0.75).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "parser_checkpoint_reuse_rate"
                        && (*value - 0.75).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "content_hash_hit_rate"
                        && (*value - 0.75).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "semantic_fact_cache_hit_rate"
                        && (*value - 0.5).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "workspace_shard_reuse_rate"
                        && (*value - 0.75).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "unchanged_file_skip_rate"
                        && (*value - 0.25).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "fast_path_wrong_result_count"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn determinism_metrics_emit_hash_stability_rows() {
        let score = DeterminismScore {
            fixture_count: 2,
            token_stream_stable_count: 2,
            parse_hash_stable_count: 2,
            ast_hash_stable_count: 1,
            semantic_fact_hash_stable_count: 2,
            diagnostic_hash_stable_count: 2,
            repeated_parse_stable_count: 2,
            comment_invariance_stable_count: 1,
            comment_invariance_sample_count: 2,
        };

        let metrics = determinism_metrics(&score, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "ast_hash_stability_rate"
                        && (*value - 0.5).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "whitespace_invariance_rate"
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 2, .. }
                    if metric == "comment_invariance_rate"
                        && (*value - 0.5).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn runtime_metric_rows_are_synced_after_artifact_size_settles() {
        let mut artifact = ParserAccuracyArtifact {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            generated_at: "2026-05-02T00:00:00Z".to_string(),
            commit: "test".to_string(),
            cadence: Cadence::Pr,
            denominator: Denominator::default(),
            families: Vec::new(),
            metrics: Vec::new(),
            failure_packets: Vec::new(),
            gold_drift: GoldDrift::default(),
            metric_runtime: MetricRuntime {
                runtime_ms: 12.5,
                artifact_size_bytes: 900,
                cache_hit_rate: Some(0.25),
                cache_sample_count: 4,
                ..MetricRuntime::default()
            },
        };

        sync_runtime_metric_rows(&mut artifact, Cadence::Pr);

        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "metric_runtime_ms"
                        && (*value - 12.5).abs() < f64::EPSILON
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "metric_artifact_size_bytes"
                        && (*value - 900.0).abs() < f64::EPSILON
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "metric_cache_hit_rate"
                        && (*value - 0.25).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn allocation_rows_are_replaced_after_runtime_measurement() {
        let mut artifact = ParserAccuracyArtifact {
            schema_version: 1,
            subsystem: "parser_accuracy".to_string(),
            generated_at: "2026-05-02T00:00:00Z".to_string(),
            commit: "test".to_string(),
            cadence: Cadence::Pr,
            denominator: Denominator::default(),
            families: Vec::new(),
            metrics: vec![
                insufficient("peak_rss_mb", "memory telemetry is not wired yet"),
                insufficient("allocated_bytes", "allocation telemetry is not wired yet"),
                insufficient("allocation_count", "allocation telemetry is not wired yet"),
            ],
            failure_packets: Vec::new(),
            gold_drift: GoldDrift::default(),
            metric_runtime: MetricRuntime {
                peak_rss_mb: Some(1.5),
                allocated_bytes: Some(1234),
                allocation_count: Some(56),
                ..MetricRuntime::default()
            },
        };

        sync_allocation_metric_rows(&mut artifact, Cadence::Pr);

        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "peak_rss_mb"
                        && (*value - 1.5).abs() < f64::EPSILON
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "allocated_bytes"
                        && (*value - 1234.0).abs() < f64::EPSILON
            )
        }));
        assert!(artifact.metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 1, .. }
                    if metric == "allocation_count"
                        && (*value - 56.0).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn gold_drift_audit_counts_span_duplicate_and_missing_edge_errors() {
        let span_expectations = vec![
            SpanExpectation {
                id: "good".to_string(),
                span_text: "Alpha".to_string(),
                occurrence: None,
                byte_start: 0,
                byte_end: 5,
                line_start: 1,
                line_end: 1,
                utf16_start: SpanPositionExpectation { line: 0, character: 0 },
                utf16_end: SpanPositionExpectation { line: 0, character: 5 },
            },
            SpanExpectation {
                id: "bad_text".to_string(),
                span_text: "Beta".to_string(),
                occurrence: None,
                byte_start: 0,
                byte_end: 5,
                line_start: 1,
                line_end: 1,
                utf16_start: SpanPositionExpectation { line: 0, character: 0 },
                utf16_end: SpanPositionExpectation { line: 0, character: 4 },
            },
        ];
        let expectations = SymbolExpectations {
            entities: vec![SymbolEntityExpectation {
                id: "dup".to_string(),
                kind: "Package".to_string(),
                canonical_name: "Alpha".to_string(),
                span_text: "Alpha".to_string(),
                package: None,
                scope: None,
                provenance: "ExactAst".to_string(),
                confidence: "High".to_string(),
            }],
            occurrences: vec![SymbolOccurrenceExpectation {
                id: "dup".to_string(),
                kind: "Reference".to_string(),
                canonical_name: Some("Alpha::missing".to_string()),
                span_text: "missing".to_string(),
                package: None,
                scope: None,
                provenance: "ExactAst".to_string(),
                confidence: "High".to_string(),
            }],
            edges: vec![SymbolEdgeExpectation {
                id: "edge".to_string(),
                kind: "Defines".to_string(),
                from: "Alpha".to_string(),
                to: "Alpha::missing".to_string(),
                provenance: "ExactAst".to_string(),
                confidence: "High".to_string(),
            }],
        };
        let mut seen = BTreeSet::new();

        assert_eq!(count_span_expectation_errors("Alpha", &span_expectations), 1);
        assert_eq!(count_duplicate_symbol_ids(&expectations, &mut seen), 1);
        assert_eq!(count_missing_edge_targets(&expectations), 1);
    }

    #[test]
    fn gold_drift_audit_counts_added_expectations_from_baseline() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_fixture_sources(tmp.path())?;
        let manifest = fixture_manifest();
        let mut baseline_signatures = gold_expectation_signatures(&manifest);
        baseline_signatures.remove("package_basic::line::3::SubDecl");
        let baseline_dir = tmp.path().join(".ci/metrics/baselines");
        fs::create_dir_all(&baseline_dir)?;
        fs::write(
            baseline_dir.join("parser_accuracy_gold.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "expectation_signatures": baseline_signatures,
            }))?,
        )?;
        let mut source_cache = MetricSourceCache::default();

        let drift = audit_gold_drift(tmp.path(), &manifest, &mut source_cache)?;

        assert_eq!(drift.added_expectation_count, 1);
        assert_eq!(
            drift.added_expectation_sample_count,
            gold_expectation_signatures(&manifest).len() as u64
        );
        assert_eq!(drift.changed_line_count, 1);
        assert_eq!(
            drift.changed_line_sample_count,
            line_gold_signatures(&gold_expectation_signatures(&manifest)).len() as u64
        );
        Ok(())
    }

    #[test]
    fn gold_drift_metrics_emit_validation_and_baseline_rows() {
        let drift = GoldDrift {
            span_error_count: 1,
            duplicate_symbol_id_count: 2,
            missing_resolves_to_target_count: 3,
            changed_line_count: 1,
            changed_line_sample_count: 4,
            added_expectation_count: 1,
            added_expectation_sample_count: 4,
            ..GoldDrift::default()
        };

        let metrics = gold_drift_metrics(&drift, 4, Cadence::Pr);

        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "gold_span_errors"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "gold_changed_line_count"
                        && (*value - 1.0).abs() < f64::EPSILON
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::InsufficientData { metric, sample_count: 0, .. }
                    if metric == "gold_removed_expectation_count"
            )
        }));
        assert!(metrics.iter().any(|metric| {
            matches!(
                metric,
                MetricRow::Measured { metric, value, sample_count: 4, .. }
                    if metric == "gold_added_expectation_count"
                        && (*value - 1.0).abs() < f64::EPSILON
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

    #[test]
    fn artifact_failure_packets_include_actionable_context() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_fixture_sources(tmp.path())?;
        let artifact = build_artifact(tmp.path(), &fixture_manifest(), Cadence::Pr)?;

        let packet = artifact
            .failure_packets
            .iter()
            .find(|packet| packet.metric.as_deref() == Some("ast_node_kind_f1"))
            .ok_or_else(|| eyre!("expected at least one AST failure packet"))?;

        assert_eq!(packet.likely_layer, "ast_projection");
        assert_eq!(packet.family.as_deref(), Some("packages"));
        assert_eq!(packet.line, Some(1));
        assert!(!packet.expected.is_empty());
        assert!(!packet.actual.is_empty());
        assert!(!packet.nearest_predictions.is_empty());
        assert!(packet.source_excerpt.as_deref().is_some_and(|line| line.contains("package")));
        assert!(packet.suggested_next_fix.is_some());
        Ok(())
    }
}

//! Parser accuracy artifact types and status-row formatting.
//!
//! Holds the serde-deserializable structs for the JSON artifact produced by
//! `cargo xtask metrics parser-accuracy --json` plus the helper functions that
//! render those structs into status-doc rows.

use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParserAccuracyArtifactSummary {
    pub(super) schema_version: u32,
    pub(super) subsystem: String,
    pub(super) generated_at: String,
    pub(super) commit: String,
    pub(super) cadence: String,
    pub(super) denominator: ParserAccuracyDenominator,
    pub(super) families: Vec<ParserAccuracyFamilySummary>,
    pub(super) metrics: Vec<ParserAccuracyMetricSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParserAccuracyDenominator {
    pub(super) fixture_count: u64,
    pub(super) fixture_family_count: u64,
    pub(super) scored_line_count: u64,
    pub(super) scored_symbol_count: u64,
    pub(super) fully_labeled_region_count: u64,
    pub(super) partial_labeled_region_count: u64,
    pub(super) unknown_region_count: u64,
    pub(super) negative_region_count: u64,
    pub(super) dynamic_boundary_case_count: u64,
    pub(super) unsupported_construct_case_count: u64,
    pub(super) real_project_file_count: u64,
    pub(super) generated_fixture_count: u64,
    pub(super) hand_labeled_fixture_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ParserAccuracyFamilySummary {
    pub(super) family: String,
    pub(super) fixture_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum ParserAccuracyMetricSummary {
    Measured { metric: String, value: f64, sample_count: u64 },
    InsufficientData { metric: String, reason: String, sample_count: u64 },
}

pub(super) fn read_parser_accuracy_artifact(root: &Path) -> Option<ParserAccuracyArtifactSummary> {
    let path = root.join("target/metrics/parser_accuracy.json");
    let raw = fs::read_to_string(path).ok()?;
    let artifact: ParserAccuracyArtifactSummary = serde_json::from_str(&raw).ok()?;
    if artifact.schema_version != 1 || artifact.subsystem != "parser_accuracy" {
        return None;
    }
    Some(artifact)
}

pub(super) fn parser_accuracy_rows(artifact: Option<&ParserAccuracyArtifactSummary>) -> String {
    const ARTIFACT_PATH: &str = "`target/metrics/parser_accuracy.json`";
    const SPEC_PATH: &str = "`.kiro/specs/parser-accuracy-observability`";
    const SCHEMA_PATH: &str = "`.ci/schemas/parser-accuracy.schema.json`";

    let Some(artifact) = artifact else {
        return format!(
            "| **Accuracy denominator** | insufficient_data | Generate with `cargo xtask metrics parser-accuracy --json`; missing artifact is not treated as zero | {ARTIFACT_PATH}; {SPEC_PATH} |\n\
             | **Accuracy scorers** | insufficient_data | line/AST/symbol scoring rows wait for real denominators and validated artifact input | {SCHEMA_PATH} |"
        );
    };

    let d = &artifact.denominator;
    let family_summary = parser_accuracy_family_summary(&artifact.families);
    let metric_summary = parser_accuracy_metric_summary(&artifact.metrics);
    format!(
        "| **Accuracy denominator** | {} fixtures / {} families | {} scored lines, {} scored symbols, {} fully labeled, {} partial, {} unknown, {} negative, {} dynamic boundaries, {} unsupported, {} real-project, {} generated, {} hand-labeled; cadence `{}`, commit `{}`, generated `{}` | {ARTIFACT_PATH}; {SPEC_PATH} |\n\
         | **Accuracy families** | {} | fixture family inventory from parser accuracy manifest | {ARTIFACT_PATH} |\n\
         | **Accuracy scorers** | {} | missing accuracy rows stay `insufficient_data`; they are not rendered as zero or pass | {SCHEMA_PATH} |",
        d.fixture_count,
        d.fixture_family_count,
        d.scored_line_count,
        d.scored_symbol_count,
        d.fully_labeled_region_count,
        d.partial_labeled_region_count,
        d.unknown_region_count,
        d.negative_region_count,
        d.dynamic_boundary_case_count,
        d.unsupported_construct_case_count,
        d.real_project_file_count,
        d.generated_fixture_count,
        d.hand_labeled_fixture_count,
        artifact.cadence,
        artifact.commit,
        artifact.generated_at,
        family_summary,
        metric_summary,
    )
}

fn parser_accuracy_family_summary(families: &[ParserAccuracyFamilySummary]) -> String {
    if families.is_empty() {
        return "insufficient_data".to_string();
    }

    let rendered = families
        .iter()
        .take(6)
        .map(|family| format!("{} ({})", family.family, family.fixture_count))
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = families.len().saturating_sub(6);
    if hidden == 0 { rendered } else { format!("{rendered}, +{hidden} more") }
}

fn parser_accuracy_metric_summary(metrics: &[ParserAccuracyMetricSummary]) -> String {
    let mut measured = Vec::new();
    let mut insufficient = Vec::new();

    for metric in metrics {
        match metric {
            ParserAccuracyMetricSummary::Measured { metric, value, sample_count } => {
                measured.push(format!("{metric}={value:.1} (n={sample_count})"));
            }
            ParserAccuracyMetricSummary::InsufficientData { metric, reason, sample_count } => {
                insufficient
                    .push(format!("{metric}: insufficient_data ({reason}; n={sample_count})"));
            }
        }
    }

    if measured.is_empty() && insufficient.is_empty() {
        return "insufficient_data".to_string();
    }

    let mut parts = Vec::new();
    if !measured.is_empty() {
        parts.push(format!("measured {}", measured.join(", ")));
    }
    if !insufficient.is_empty() {
        parts.push(insufficient.join(", "));
    }
    parts.join("; ")
}

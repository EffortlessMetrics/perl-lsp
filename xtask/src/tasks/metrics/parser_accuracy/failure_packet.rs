use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use color_eyre::eyre::{Context, Result};

use super::*;

const FAILURE_PACKET_LIMIT: usize = 50;

pub(super) fn collect_failure_packets(
    root: &Path,
    manifest: &ParserAccuracyManifest,
) -> Result<Vec<FailurePacket>> {
    let mut packets = Vec::new();
    for fixture in &manifest.fixtures {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            break;
        }

        let source_path = root.join(&fixture.source_path);
        let source = fs::read_to_string(&source_path).with_context(|| {
            format!("reading parser accuracy fixture source {}", source_path.display())
        })?;

        collect_line_failure_packets(fixture, &source, &mut packets);
        collect_ast_failure_packets(fixture, &source, &mut packets);
        collect_symbol_failure_packets(&source_path, fixture, &source, &mut packets)?;
    }
    Ok(packets)
}

fn collect_line_failure_packets(
    fixture: &FixtureMetadata,
    source: &str,
    packets: &mut Vec<FailurePacket>,
) {
    if fixture.line_expectations.is_empty() {
        return;
    }

    let actual_by_line = extract_line_tags(source);
    for expectation in &fixture.line_expectations {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            return;
        }

        let actual = actual_by_line.get(&expectation.line).cloned().unwrap_or_default();
        if actual == expectation.expected_tags {
            continue;
        }

        push_failure_packet(
            packets,
            FailurePacket {
                failure_kind: "line_tag_mismatch".to_string(),
                likely_layer: "parser".to_string(),
                fixture_id: fixture.id.clone(),
                family: Some(fixture.family.clone()),
                metric: Some("line_construct_f1".to_string()),
                line: Some(expectation.line),
                expected: line_tag_labels(&expectation.expected_tags),
                actual: line_tag_labels(&actual),
                nearest_predictions: line_tag_labels(&actual),
                source_excerpt: source_line_excerpt(source, expectation.line),
                details: Some("expected line construct tags did not match parser projection".to_string()),
                suggested_next_fix: Some(
                    "check whether the parser projection missed this construct or the gold line label is too narrow"
                        .to_string(),
                ),
            },
        );
    }
}

fn collect_ast_failure_packets(
    fixture: &FixtureMetadata,
    source: &str,
    packets: &mut Vec<FailurePacket>,
) {
    if fixture.ast_expectations.is_empty() {
        return;
    }

    let expected_lines = fixture
        .ast_expectations
        .iter()
        .map(|expectation| expectation.line)
        .collect::<BTreeSet<_>>();
    let predictions = extract_ast_predictions(source)
        .into_iter()
        .filter(|prediction| expected_lines.contains(&prediction.line))
        .collect::<Vec<_>>();
    let mut matched = BTreeSet::new();

    for expectation in &fixture.ast_expectations {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            return;
        }

        let predictions_on_line = predictions
            .iter()
            .filter(|prediction| prediction.line == expectation.line)
            .collect::<Vec<_>>();
        let prediction_index = best_ast_prediction_index(expectation, &predictions, &matched);
        let Some(prediction_index) = prediction_index else {
            push_failure_packet(
                packets,
                FailurePacket {
                    failure_kind: "missing_ast_node".to_string(),
                    likely_layer: "ast_projection".to_string(),
                    fixture_id: fixture.id.clone(),
                    family: Some(fixture.family.clone()),
                    metric: Some("ast_node_kind_f1".to_string()),
                    line: Some(expectation.line),
                    expected: vec![ast_expectation_label(expectation)],
                    actual: predictions_on_line
                        .iter()
                        .map(|prediction| ast_prediction_label(prediction))
                        .collect(),
                    nearest_predictions: predictions_on_line
                        .iter()
                        .take(5)
                        .map(|prediction| ast_prediction_label(prediction))
                        .collect(),
                    source_excerpt: source_line_excerpt(source, expectation.line),
                    details: Some(format!(
                        "expected AST node `{}` was not projected",
                        expectation.kind
                    )),
                    suggested_next_fix: Some(
                        "inspect parser node construction and AST projection for this fixture line"
                            .to_string(),
                    ),
                },
            );
            continue;
        };

        matched.insert(prediction_index);
        let prediction = &predictions[prediction_index];
        let parent_matches = expectation
            .parent_kind
            .as_ref()
            .is_none_or(|parent_kind| prediction.parent_kind.as_ref() == Some(parent_kind));
        let span_matches = prediction.span_text == expectation.span_text;
        if !parent_matches || !span_matches {
            push_failure_packet(
                packets,
                FailurePacket {
                    failure_kind: "ast_shape_mismatch".to_string(),
                    likely_layer: "ast_projection".to_string(),
                    fixture_id: fixture.id.clone(),
                    family: Some(fixture.family.clone()),
                    metric: Some("ast_node_kind_f1".to_string()),
                    line: Some(expectation.line),
                    expected: vec![ast_expectation_label(expectation)],
                    actual: vec![ast_prediction_label(prediction)],
                    nearest_predictions: predictions_on_line
                        .iter()
                        .take(5)
                        .map(|prediction| ast_prediction_label(prediction))
                        .collect(),
                    source_excerpt: source_line_excerpt(source, expectation.line),
                    details: Some(
                        "AST node kind matched but span or parent shape differed".to_string(),
                    ),
                    suggested_next_fix: Some(
                        "compare the projected AST span and parent edge with the gold expectation"
                            .to_string(),
                    ),
                },
            );
        }
    }
}

fn collect_symbol_failure_packets(
    source_path: &Path,
    fixture: &FixtureMetadata,
    source: &str,
    packets: &mut Vec<FailurePacket>,
) -> Result<()> {
    if fixture.symbol_expectations.entities.is_empty()
        && fixture.symbol_expectations.occurrences.is_empty()
        && fixture.symbol_expectations.edges.is_empty()
    {
        return Ok(());
    }

    let predictions = extract_symbol_predictions(source_path, source)?;

    let expected_entities = fixture
        .symbol_expectations
        .entities
        .iter()
        .map(entity_key_from_expectation)
        .collect::<BTreeSet<_>>();
    for missing in expected_entities.difference(&predictions.entities) {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            return Ok(());
        }
        push_failure_packet(
            packets,
            symbol_failure_packet(
                fixture,
                "missing_symbol_declaration",
                "symbol_decl_f1",
                symbol_entity_label(missing),
                predictions.entities.iter().take(5).map(symbol_entity_label).collect(),
                source_excerpt_for_span(source, &missing.span_text),
            ),
        );
    }

    let expected_occurrences = fixture
        .symbol_expectations
        .occurrences
        .iter()
        .map(occurrence_key_from_expectation)
        .collect::<BTreeSet<_>>();
    for missing in expected_occurrences.difference(&predictions.occurrences) {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            return Ok(());
        }
        push_failure_packet(
            packets,
            symbol_failure_packet(
                fixture,
                "missing_symbol_reference",
                "symbol_ref_f1",
                symbol_occurrence_label(missing),
                predictions.occurrences.iter().take(5).map(symbol_occurrence_label).collect(),
                source_excerpt_for_span(source, &missing.span_text),
            ),
        );
    }

    let expected_edges = fixture
        .symbol_expectations
        .edges
        .iter()
        .map(edge_key_from_expectation)
        .collect::<BTreeSet<_>>();
    for missing in expected_edges.difference(&predictions.edges) {
        if packets.len() >= FAILURE_PACKET_LIMIT {
            return Ok(());
        }
        push_failure_packet(
            packets,
            symbol_failure_packet(
                fixture,
                "missing_symbol_edge",
                "symbol_edge_f1",
                symbol_edge_label(missing),
                predictions.edges.iter().take(5).map(symbol_edge_label).collect(),
                None,
            ),
        );
    }

    Ok(())
}

fn symbol_failure_packet(
    fixture: &FixtureMetadata,
    failure_kind: &str,
    metric: &str,
    expected: String,
    nearest_predictions: Vec<String>,
    source_excerpt: Option<String>,
) -> FailurePacket {
    FailurePacket {
        failure_kind: failure_kind.to_string(),
        likely_layer: "semantic_fact_extraction".to_string(),
        fixture_id: fixture.id.clone(),
        family: Some(fixture.family.clone()),
        metric: Some(metric.to_string()),
        line: None,
        expected: vec![expected],
        actual: Vec::new(),
        nearest_predictions,
        source_excerpt,
        details: Some(
            "gold symbol expectation was not present in canonical fact predictions".to_string(),
        ),
        suggested_next_fix: Some(
            "inspect fact extraction for this fixture before changing the gold expectation"
                .to_string(),
        ),
    }
}

fn push_failure_packet(packets: &mut Vec<FailurePacket>, packet: FailurePacket) {
    if packets.len() < FAILURE_PACKET_LIMIT {
        packets.push(packet);
    }
}

fn source_line_excerpt(source: &str, line: u64) -> Option<String> {
    source.lines().nth(line.saturating_sub(1) as usize).map(|line| line.trim().to_string())
}

fn source_excerpt_for_span(source: &str, span_text: &str) -> Option<String> {
    source.lines().find(|line| line.contains(span_text)).map(|line| line.trim().to_string())
}

fn line_tag_labels(tags: &BTreeSet<LineTag>) -> Vec<String> {
    tags.iter().map(|tag| line_tag_label(*tag).to_string()).collect()
}

fn line_tag_label(tag: LineTag) -> &'static str {
    match tag {
        LineTag::PackageDecl => "package_decl",
        LineTag::SubDecl => "sub_decl",
        LineTag::MethodDecl => "method_decl",
        LineTag::VariableDecl => "variable_decl",
        LineTag::Import => "import",
        LineTag::Export => "export",
        LineTag::FunctionCall => "function_call",
        LineTag::MethodCall => "method_call",
        LineTag::Regex => "regex",
        LineTag::QuoteLike => "quote_like",
        LineTag::HeredocOpener => "heredoc_opener",
        LineTag::HeredocBody => "heredoc_body",
        LineTag::HeredocTerminator => "heredoc_terminator",
        LineTag::Pod => "pod",
        LineTag::FormatDecl => "format_decl",
        LineTag::GivenWhen => "given_when",
        LineTag::DoWhile => "do_while",
        LineTag::UntilLoop => "until_loop",
        LineTag::DynamicBoundary => "dynamic_boundary",
        LineTag::ParseError => "parse_error",
        LineTag::RecoveryRegion => "recovery_region",
        LineTag::UnsupportedConstruct => "unsupported_construct",
    }
}

fn ast_expectation_label(expectation: &AstExpectation) -> String {
    format!(
        "{} line={} span={:?} parent={:?}",
        expectation.kind, expectation.line, expectation.span_text, expectation.parent_kind
    )
}

fn ast_prediction_label(prediction: &AstPrediction) -> String {
    format!(
        "{} line={} span={:?} parent={:?}",
        prediction.kind, prediction.line, prediction.span_text, prediction.parent_kind
    )
}

fn symbol_entity_label(entity: &SymbolEntityKey) -> String {
    format!("{} {} span={:?}", entity.kind, entity.canonical_name, entity.span_text)
}

fn symbol_occurrence_label(occurrence: &SymbolOccurrenceKey) -> String {
    format!("{} {:?} span={:?}", occurrence.kind, occurrence.canonical_name, occurrence.span_text)
}

fn symbol_edge_label(edge: &SymbolEdgeKey) -> String {
    format!("{} {} -> {}", edge.kind, edge.from, edge.to)
}

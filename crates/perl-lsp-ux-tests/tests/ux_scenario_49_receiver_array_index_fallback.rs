//! Scenario 49 - Receiver array-index fallback receipt.
//!
//! This receipt records array-index receiver boundaries for completion. The
//! semantic substrate can classify static array indexes, but completion must
//! not treat array-index receiver facts as exact source-backed receiver
//! evidence without a later promotion receipt. Dynamic array indexes remain
//! fallback-only.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_49_receiver_array_index_fallback.rs";
const DB_PATH: &str = "lib/RealReceiver/DB.pm";
const STATIC_ARRAY_PROBE_PATH: &str = "script/static-array-index-receiver.pl";
const DYNAMIC_ARRAY_PROBE_PATH: &str = "script/dynamic-array-index-receiver.pl";

const DB_PM: &str = r#"package RealReceiver::DB;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub connect {
    return 1;
}

sub disconnect {
    return 1;
}

1;
"#;

const STATIC_ARRAY_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my @items = (RealReceiver::DB->new);
$items[0]->
"#;

const DYNAMIC_ARRAY_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my @items = (RealReceiver::DB->new);
my $i = 0;
$items[$i]->
"#;

#[derive(Debug)]
struct ArrayReceiverProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver_marker: &'static str,
    expected_label: &'static str,
    expected_detail: &'static str,
    forbidden_details: &'static [&'static str],
    dynamic_boundary: bool,
}

#[derive(Debug, Serialize)]
struct ArrayReceiverReport {
    name: &'static str,
    file: &'static str,
    receiver_fact_class: &'static str,
    candidate_count: usize,
    expected_label_present: bool,
    expected_label_detail: Option<String>,
    expected_sort_text: Option<String>,
    source_backed: bool,
    confidence: &'static str,
    fresh: bool,
    fallback_state: &'static str,
    fallback_or_labeled_boundary: bool,
    dynamic_boundary: bool,
    blocked_reason: Option<String>,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(DB_PATH, DB_PM)
            .with_file(STATIC_ARRAY_PROBE_PATH, STATIC_ARRAY_PROBE)
            .with_file(DYNAMIC_ARRAY_PROBE_PATH, DYNAMIC_ARRAY_PROBE),
    )
}

fn position_after(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset =
        source.find(needle).with_context(|| format!("missing `{needle}`"))? + needle.len();
    position_from_byte_offset(source, byte_offset)
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn completion_label(item: &Value) -> Option<&str> {
    item.get("label")
        .and_then(Value::as_str)
        .or_else(|| item.get("insertText").and_then(Value::as_str))
        .or_else(|| item.get("filterText").and_then(Value::as_str))
}

fn completion_text(item: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["label", "insertText", "filterText", "detail"] {
        if let Some(value) = item.get(key).and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    if let Some(documentation) = item.get("documentation") {
        if let Some(value) = documentation.as_str() {
            parts.push(value.to_string());
        } else if let Some(value) = documentation.get("value").and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    parts.join("\n")
}

fn completion_sort_text(item: &Value) -> Option<String> {
    item.get("sortText").and_then(Value::as_str).map(str::to_string)
}

fn item_has_completion_shape(item: &Value) -> bool {
    item.get("label").and_then(Value::as_str).is_some()
        || item.get("insertText").and_then(Value::as_str).is_some()
        || item.get("filterText").and_then(Value::as_str).is_some()
}

fn probe_array_receiver(
    harness: &UxHarness,
    probe: &ArrayReceiverProbe,
) -> Result<ArrayReceiverReport> {
    let (line, character) = position_after(probe.source, probe.receiver_marker)?;
    let items = harness.completion(probe.file, line, character)?;
    for item in &items {
        anyhow::ensure!(
            item_has_completion_shape(item),
            "completion item for probe {} must include label, insertText, or filterText: {item:?}",
            probe.name
        );
    }

    let expected_item =
        items.iter().find(|item| completion_label(item) == Some(probe.expected_label));
    let expected_label_detail = expected_item.map(completion_text);
    let expected_sort_text = expected_item.and_then(completion_sort_text);
    let expected_label_present = expected_item.is_some();
    let expected_detail = expected_label_detail.as_deref().unwrap_or_default();
    anyhow::ensure!(
        expected_detail.contains(probe.expected_detail),
        "probe {} must preserve fallback detail `{}`; got {expected_detail:?}",
        probe.name,
        probe.expected_detail
    );
    let sort_text = expected_sort_text.as_deref().with_context(|| {
        format!("probe {} fallback completion must include sortText", probe.name)
    })?;
    anyhow::ensure!(
        sort_text.starts_with("6_"),
        "probe {} fallback completion must remain tier 6; got {sort_text:?}",
        probe.name
    );

    let forbidden_hit = items.iter().find_map(|item| {
        let text = completion_text(item);
        probe
            .forbidden_details
            .iter()
            .find(|forbidden| text.contains(**forbidden))
            .map(|forbidden| (*forbidden).to_string())
    });
    anyhow::ensure!(
        forbidden_hit.is_none(),
        "probe {} exposed forbidden receiver detail {:?}",
        probe.name,
        forbidden_hit
    );

    let source_backed = expected_detail.contains("receiver: source-backed");
    let medium_confidence = expected_detail.contains("medium confidence");
    let has_receiver_detail = expected_detail.contains("receiver:");
    let fallback_detail = expected_detail.contains("low confidence")
        || expected_detail.contains("fallback")
        || expected_detail.contains("unknown");
    let blocked_reason = if expected_label_present {
        None
    } else {
        Some("expected_label_absent_or_blocked".to_string())
    };
    let fallback_state = if blocked_reason.is_some() {
        "blocked"
    } else if medium_confidence {
        "medium_confidence_labeled"
    } else if fallback_detail {
        "low_confidence_labeled"
    } else if !has_receiver_detail {
        "legacy_workspace_candidate_without_receiver_evidence"
    } else {
        "none"
    };
    let fallback_or_labeled_boundary = fallback_state != "none";
    let confidence = if medium_confidence {
        "medium"
    } else if fallback_detail {
        "low"
    } else {
        "unknown"
    };

    Ok(ArrayReceiverReport {
        name: probe.name,
        file: probe.file,
        receiver_fact_class: probe.name,
        candidate_count: items.len(),
        expected_label_present,
        expected_label_detail,
        expected_sort_text,
        source_backed,
        confidence,
        fresh: true,
        fallback_state,
        fallback_or_labeled_boundary,
        dynamic_boundary: probe.dynamic_boundary,
        blocked_reason,
    })
}

fn report_by_name<'a>(
    reports: &'a [ArrayReceiverReport],
    name: &str,
) -> Result<&'a ArrayReceiverReport> {
    reports
        .iter()
        .find(|report| report.name == name)
        .with_context(|| format!("missing array receiver report `{name}`"))
}

fn array_receiver_probes() -> Vec<ArrayReceiverProbe> {
    vec![
        ArrayReceiverProbe {
            name: "static_array_index_receiver",
            file: STATIC_ARRAY_PROBE_PATH,
            source: STATIC_ARRAY_PROBE,
            receiver_marker: "$items[0]->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: constructor assignment",
                "receiver: type engine",
                "receiver: literal bless",
                "receiver: static package",
                "receiver: self",
                "receiver: this",
            ],
            dynamic_boundary: false,
        },
        ArrayReceiverProbe {
            name: "dynamic_array_index_receiver",
            file: DYNAMIC_ARRAY_PROBE_PATH,
            source: DYNAMIC_ARRAY_PROBE,
            receiver_marker: "$items[$i]->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: constructor assignment",
                "receiver: type engine",
                "receiver: literal bless",
                "receiver: static package",
                "receiver: self",
                "receiver: this",
            ],
            dynamic_boundary: true,
        },
    ]
}

#[test]
fn scenario_49_receiver_array_index_fallback_receipt() {
    run_ux_scenario(
        "receiver_array_index_fallback",
        SCENARIO_FILE,
        "scenario_49_receiver_array_index_fallback_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for path in [DB_PATH, STATIC_ARRAY_PROBE_PATH, DYNAMIC_ARRAY_PROBE_PATH] {
                let source = match path {
                    DB_PATH => DB_PM,
                    STATIC_ARRAY_PROBE_PATH => STATIC_ARRAY_PROBE,
                    DYNAMIC_ARRAY_PROBE_PATH => DYNAMIC_ARRAY_PROBE,
                    _ => unreachable!("all paths are covered"),
                };
                harness.open_file(path, source)?;
            }
            std::thread::sleep(Duration::from_millis(500));

            let probes = array_receiver_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_array_receiver(&harness, probe)?;
                if report.expected_label_present {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let exact_source_backed_count =
                reports.iter().filter(|report| report.source_backed).count();
            let fallback_or_labeled_boundary_count =
                reports.iter().filter(|report| report.fallback_or_labeled_boundary).count();
            let receipt = json!({
                "schema_version": 1,
                "receipt": "receiver_array_index_fallback",
                "workspace_fixture": "RealReceiver static/dynamic array-index CPAN-style workspace",
                "claim_boundary": "receipt-only receiver fallback proof; no completion behavior change, support-tier promotion, array-index receiver promotion, dynamic-boundary promotion, parser/corpus bucket movement, release-lineage sync, or source-repo development continuation",
                "probe_count": reports.len(),
                "exact_source_backed_count": exact_source_backed_count,
                "fallback_or_labeled_boundary_count": fallback_or_labeled_boundary_count,
                "reports": reports,
            });
            eprintln!(
                "receiver_array_index_fallback_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all array-index receiver probes produced reports",
                reports.len() == probes.len(),
            )?;
            for probe_name in ["static_array_index_receiver", "dynamic_array_index_receiver"] {
                let report = report_by_name(&reports, probe_name)?;
                recorder.check(
                    &format!("{probe_name} stayed tier-6 low-confidence fallback"),
                    report.expected_label_present
                        && report.confidence == "low"
                        && report
                            .expected_sort_text
                            .as_deref()
                            .is_some_and(|sort| sort.starts_with("6_"))
                        && report.fallback_state == "low_confidence_labeled"
                        && !report.source_backed
                        && report.fallback_or_labeled_boundary,
                )?;
            }
            let dynamic_report = report_by_name(&reports, "dynamic_array_index_receiver")?;
            recorder.check(
                "dynamic array-index receiver used low-confidence fallback without exact receiver evidence",
                dynamic_report.dynamic_boundary
                    && !dynamic_report.source_backed
                    && dynamic_report.fallback_state == "low_confidence_labeled",
            )?;
            recorder.check(
                "no array-index receiver probe produced exact source-backed completion evidence",
                exact_source_backed_count == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

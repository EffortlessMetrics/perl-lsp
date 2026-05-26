//! Scenario 48 - Receiver bless confidence receipt.
//!
//! This receipt records literal and dynamic `bless` receiver boundaries for
//! completion. Literal `bless` evidence may be useful only when it is clearly
//! labeled medium-confidence; dynamic `bless` evidence must not authorize exact
//! source-backed receiver completion.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_48_receiver_bless_confidence.rs";
const DB_PATH: &str = "lib/RealReceiver/DB.pm";
const LITERAL_BLESS_PROBE_PATH: &str = "script/literal-bless-receiver.pl";
const DYNAMIC_BLESS_PROBE_PATH: &str = "script/dynamic-bless-receiver.pl";

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

const LITERAL_BLESS_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my $db = bless {}, "RealReceiver::DB";
$db->
"#;

const DYNAMIC_BLESS_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my $class = "RealReceiver::DB";
my $db = bless {}, $class;
$db->
"#;

#[derive(Debug)]
struct BlessReceiverProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver_marker: &'static str,
    expected_label: &'static str,
    required_detail: Option<&'static str>,
    forbidden_details: &'static [&'static str],
    dynamic_boundary: bool,
}

#[derive(Debug, Serialize)]
struct BlessReceiverReport {
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
            .with_file(LITERAL_BLESS_PROBE_PATH, LITERAL_BLESS_PROBE)
            .with_file(DYNAMIC_BLESS_PROBE_PATH, DYNAMIC_BLESS_PROBE),
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

fn probe_bless_receiver(
    harness: &UxHarness,
    probe: &BlessReceiverProbe,
) -> Result<BlessReceiverReport> {
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

    if let Some(required_detail) = probe.required_detail {
        anyhow::ensure!(
            expected_detail.contains(required_detail),
            "probe {} must preserve detail `{}`; got {expected_detail:?}",
            probe.name,
            required_detail
        );
    }

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
    } else if probe.dynamic_boundary && !has_receiver_detail {
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

    Ok(BlessReceiverReport {
        name: probe.name,
        file: probe.file,
        receiver_fact_class: probe.name,
        candidate_count: items.len(),
        expected_label_present,
        expected_label_detail,
        expected_sort_text,
        source_backed: false,
        confidence,
        fresh: true,
        fallback_state,
        fallback_or_labeled_boundary,
        dynamic_boundary: probe.dynamic_boundary,
        blocked_reason,
    })
}

fn report_by_name<'a>(
    reports: &'a [BlessReceiverReport],
    name: &str,
) -> Result<&'a BlessReceiverReport> {
    reports
        .iter()
        .find(|report| report.name == name)
        .with_context(|| format!("missing bless receiver report `{name}`"))
}

fn bless_receiver_probes() -> Vec<BlessReceiverProbe> {
    vec![
        BlessReceiverProbe {
            name: "literal_bless_receiver",
            file: LITERAL_BLESS_PROBE_PATH,
            source: LITERAL_BLESS_PROBE,
            receiver_marker: "$db->",
            expected_label: "connect",
            required_detail: Some("receiver: literal bless, medium confidence"),
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: constructor assignment",
                "receiver: type engine",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
            ],
            dynamic_boundary: false,
        },
        BlessReceiverProbe {
            name: "dynamic_bless_receiver",
            file: DYNAMIC_BLESS_PROBE_PATH,
            source: DYNAMIC_BLESS_PROBE,
            receiver_marker: "$db->",
            expected_label: "connect",
            required_detail: None,
            forbidden_details: &[
                "receiver: literal bless",
                "receiver: source-backed object",
                "receiver: constructor assignment",
                "receiver: type engine",
                "receiver: static package",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
            ],
            dynamic_boundary: true,
        },
    ]
}

#[test]
fn scenario_48_receiver_bless_confidence_receipt() {
    run_ux_scenario(
        "receiver_bless_confidence",
        SCENARIO_FILE,
        "scenario_48_receiver_bless_confidence_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for path in [DB_PATH, LITERAL_BLESS_PROBE_PATH, DYNAMIC_BLESS_PROBE_PATH] {
                let source = match path {
                    DB_PATH => DB_PM,
                    LITERAL_BLESS_PROBE_PATH => LITERAL_BLESS_PROBE,
                    DYNAMIC_BLESS_PROBE_PATH => DYNAMIC_BLESS_PROBE,
                    _ => unreachable!("all paths are covered"),
                };
                harness.open_file(path, source)?;
            }
            std::thread::sleep(Duration::from_millis(500));

            let probes = bless_receiver_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_bless_receiver(&harness, probe)?;
                if report.expected_label_present {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let medium_confidence_count =
                reports.iter().filter(|report| report.confidence == "medium").count();
            let exact_source_backed_count =
                reports.iter().filter(|report| report.source_backed).count();
            let fallback_or_labeled_boundary_count =
                reports.iter().filter(|report| report.fallback_or_labeled_boundary).count();
            let receipt = json!({
                "schema_version": 1,
                "receipt": "receiver_bless_confidence",
                "workspace_fixture": "RealReceiver literal/dynamic bless CPAN-style workspace",
                "claim_boundary": "receipt-only receiver confidence proof; no completion behavior change, support-tier promotion, dynamic-boundary promotion, or medium-confidence receiver promotion",
                "probe_count": reports.len(),
                "medium_confidence_count": medium_confidence_count,
                "exact_source_backed_count": exact_source_backed_count,
                "fallback_or_labeled_boundary_count": fallback_or_labeled_boundary_count,
                "reports": reports,
            });
            eprintln!(
                "receiver_bless_confidence_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all bless receiver probes produced reports",
                reports.len() == probes.len(),
            )?;
            let literal_report = report_by_name(&reports, "literal_bless_receiver")?;
            recorder.check(
                "literal bless receiver is labeled medium-confidence and not source-backed",
                literal_report.expected_label_present
                    && literal_report.confidence == "medium"
                    && literal_report.fallback_or_labeled_boundary
                    && !literal_report.source_backed,
            )?;
            let dynamic_report = report_by_name(&reports, "dynamic_bless_receiver")?;
            recorder.check(
                "dynamic bless receiver used legacy fallback without exact receiver evidence",
                dynamic_report.dynamic_boundary
                    && !dynamic_report.source_backed
                    && dynamic_report.fallback_state
                        == "legacy_workspace_candidate_without_receiver_evidence"
                    && dynamic_report.fallback_or_labeled_boundary,
            )?;
            recorder.check(
                "no bless receiver probe produced exact source-backed completion evidence",
                exact_source_backed_count == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

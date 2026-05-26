//! Scenario 50 - Receiver self/this quality receipt.
//!
//! This receipt records current-package `$self->` and `$this->` receiver
//! completion over a small multi-file workspace. Current-package local methods
//! remain ordinary local method candidates; inherited workspace methods may
//! carry exact self/this receiver detail. This receipt records that boundary
//! without broadening receiver completion behavior.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_50_receiver_self_this_quality.rs";
const PARENT_PATH: &str = "lib/RealReceiver/SelfParent.pm";
const SELF_CHILD_PATH: &str = "lib/RealReceiver/SelfChild.pm";
const THIS_CHILD_PATH: &str = "lib/RealReceiver/ThisChild.pm";

const PARENT_PM: &str = r#"package RealReceiver::SelfParent;
use strict;
use warnings;

sub inherited_ping {
    return 1;
}

sub shadowed_ping {
    return 'parent';
}

1;
"#;

const SELF_CHILD_PM: &str = r#"package RealReceiver::SelfChild;
use strict;
use warnings;
use parent 'RealReceiver::SelfParent';

sub child_ping {
    return 1;
}

sub shadowed_ping {
    return 'child';
}

sub run {
    my ($self) = @_;
    $self->
}

1;
"#;

const THIS_CHILD_PM: &str = r#"package RealReceiver::ThisChild;
use strict;
use warnings;
use parent 'RealReceiver::SelfParent';

sub this_ping {
    return 1;
}

sub run {
    my $this = shift;
    $this->
}

1;
"#;

#[derive(Debug)]
struct SelfThisReceiverProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver_marker: &'static str,
    expected_label: &'static str,
    expected_detail: &'static str,
    expected_sort_prefix: &'static str,
    receiver_detail_expected: bool,
    forbidden_details: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct SelfThisReceiverReport {
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
    self_this_receiver_labeled: bool,
    local_method_boundary: bool,
    inherited_boundary: bool,
    blocked_reason: Option<String>,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(PARENT_PATH, PARENT_PM)
            .with_file(SELF_CHILD_PATH, SELF_CHILD_PM)
            .with_file(THIS_CHILD_PATH, THIS_CHILD_PM),
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

fn probe_self_this_receiver(
    harness: &UxHarness,
    probe: &SelfThisReceiverProbe,
) -> Result<SelfThisReceiverReport> {
    let (line, character) = position_after(probe.source, probe.receiver_marker)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.completion(probe.file, line, character)?;
        for item in &items {
            anyhow::ensure!(
                item_has_completion_shape(item),
                "completion item for probe {} must include label, insertText, or filterText: {item:?}",
                probe.name
            );
        }
        let expected_matched = items
            .iter()
            .find(|item| completion_label(item) == Some(probe.expected_label))
            .map(completion_text)
            .as_deref()
            .is_some_and(|detail| detail.contains(probe.expected_detail));
        if expected_matched || Instant::now() >= deadline {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let expected_item =
        items.iter().find(|item| completion_label(item) == Some(probe.expected_label));
    let expected_label_detail = expected_item.map(completion_text);
    let expected_sort_text = expected_item.and_then(completion_sort_text);
    let expected_label_present = expected_item.is_some();
    let expected_detail = expected_label_detail.as_deref().unwrap_or_default();
    anyhow::ensure!(
        expected_detail.contains(probe.expected_detail),
        "probe {} must expose expected detail `{}`; got {expected_detail:?}",
        probe.name,
        probe.expected_detail
    );
    let sort_text = expected_sort_text
        .as_deref()
        .with_context(|| format!("probe {} exact completion must include sortText", probe.name))?;
    anyhow::ensure!(
        sort_text.starts_with(probe.expected_sort_prefix),
        "probe {} sortText must start with {}; got {sort_text:?}",
        probe.name,
        probe.expected_sort_prefix
    );

    let forbidden_hit = expected_label_detail.as_deref().and_then(|text| {
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

    let self_this_receiver_labeled = expected_detail.contains("receiver: self/this");
    anyhow::ensure!(
        self_this_receiver_labeled == probe.receiver_detail_expected,
        "probe {} receiver detail expectation mismatch; expected {}, got detail {expected_detail:?}",
        probe.name,
        probe.receiver_detail_expected
    );
    let local_method_boundary =
        !probe.receiver_detail_expected && expected_detail.ends_with("\nmethod");
    let inherited_boundary = expected_detail.contains("inherited method from");
    let fallback_state =
        if expected_label_present { "none" } else { "expected_label_absent_or_blocked" };

    Ok(SelfThisReceiverReport {
        name: probe.name,
        file: probe.file,
        receiver_fact_class: "self_this_current_package",
        candidate_count: items.len(),
        expected_label_present,
        expected_label_detail,
        expected_sort_text,
        source_backed: self_this_receiver_labeled,
        confidence: if self_this_receiver_labeled { "high" } else { "local_syntax" },
        fresh: true,
        fallback_state,
        self_this_receiver_labeled,
        local_method_boundary,
        inherited_boundary,
        blocked_reason: if expected_label_present {
            None
        } else {
            Some("expected_label_absent_or_blocked".to_string())
        },
    })
}

fn report_by_name<'a>(
    reports: &'a [SelfThisReceiverReport],
    name: &str,
) -> Result<&'a SelfThisReceiverReport> {
    reports
        .iter()
        .find(|report| report.name == name)
        .with_context(|| format!("missing self/this receiver report `{name}`"))
}

fn self_this_receiver_probes() -> Vec<SelfThisReceiverProbe> {
    vec![
        SelfThisReceiverProbe {
            name: "self_receiver_own_method",
            file: SELF_CHILD_PATH,
            source: SELF_CHILD_PM,
            receiver_marker: "$self->",
            expected_label: "child_ping",
            expected_detail: "method",
            expected_sort_prefix: "1_",
            receiver_detail_expected: false,
            forbidden_details: &[
                "receiver: self/this",
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
                "receiver: static package",
                "receiver: unknown, low confidence",
            ],
        },
        SelfThisReceiverProbe {
            name: "self_receiver_inherited_method",
            file: SELF_CHILD_PATH,
            source: SELF_CHILD_PM,
            receiver_marker: "$self->",
            expected_label: "inherited_ping",
            expected_detail: "inherited method from RealReceiver::SelfParent",
            expected_sort_prefix: "4_",
            receiver_detail_expected: true,
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
                "receiver: static package",
                "receiver: unknown, low confidence",
            ],
        },
        SelfThisReceiverProbe {
            name: "self_receiver_shadow_prefers_nearest_method",
            file: SELF_CHILD_PATH,
            source: SELF_CHILD_PM,
            receiver_marker: "$self->",
            expected_label: "shadowed_ping",
            expected_detail: "method",
            expected_sort_prefix: "1_",
            receiver_detail_expected: false,
            forbidden_details: &[
                "receiver: self/this",
                "inherited method from RealReceiver::SelfParent",
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
                "receiver: static package",
                "receiver: unknown, low confidence",
            ],
        },
        SelfThisReceiverProbe {
            name: "this_receiver_own_method",
            file: THIS_CHILD_PATH,
            source: THIS_CHILD_PM,
            receiver_marker: "$this->",
            expected_label: "this_ping",
            expected_detail: "method",
            expected_sort_prefix: "1_",
            receiver_detail_expected: false,
            forbidden_details: &[
                "receiver: self/this",
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
                "receiver: static package",
                "receiver: unknown, low confidence",
            ],
        },
        SelfThisReceiverProbe {
            name: "this_receiver_inherited_method",
            file: THIS_CHILD_PATH,
            source: THIS_CHILD_PM,
            receiver_marker: "$this->",
            expected_label: "inherited_ping",
            expected_detail: "inherited method from RealReceiver::SelfParent",
            expected_sort_prefix: "4_",
            receiver_detail_expected: true,
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
                "receiver: static package",
                "receiver: unknown, low confidence",
            ],
        },
    ]
}

#[test]
fn scenario_50_receiver_self_this_quality_receipt() {
    run_ux_scenario(
        "receiver_self_this_quality",
        SCENARIO_FILE,
        "scenario_50_receiver_self_this_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for path in [PARENT_PATH, SELF_CHILD_PATH, THIS_CHILD_PATH] {
                let source = match path {
                    PARENT_PATH => PARENT_PM,
                    SELF_CHILD_PATH => SELF_CHILD_PM,
                    THIS_CHILD_PATH => THIS_CHILD_PM,
                    _ => unreachable!("all paths are covered"),
                };
                harness.open_file(path, source)?;
            }
            std::thread::sleep(Duration::from_millis(500));

            let probes = self_this_receiver_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_self_this_receiver(&harness, probe)?;
                if report.expected_label_present {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let self_this_labeled_count =
                reports.iter().filter(|report| report.self_this_receiver_labeled).count();
            let local_method_boundary_count =
                reports.iter().filter(|report| report.local_method_boundary).count();
            let inherited_boundary_count =
                reports.iter().filter(|report| report.inherited_boundary).count();
            let fallback_or_blocked_count = reports
                .iter()
                .filter(|report| report.fallback_state != "none" || report.blocked_reason.is_some())
                .count();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "receiver_self_this_quality",
                "workspace_fixture": "RealReceiver current-package self/this multi-file workspace",
                "claim_boundary": "receipt-only receiver quality proof for $self/$this current-package method completion; no completion behavior change, support-tier promotion, broader receiver promotion, parser/corpus bucket movement, release-lineage sync, or source-repo development continuation",
                "probe_count": reports.len(),
                "self_this_labeled_count": self_this_labeled_count,
                "local_method_boundary_count": local_method_boundary_count,
                "inherited_boundary_count": inherited_boundary_count,
                "fallback_or_blocked_count": fallback_or_blocked_count,
                "reports": reports,
            });
            eprintln!(
                "receiver_self_this_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all self/this receiver probes produced reports",
                reports.len() == probes.len(),
            )?;
            for probe_name in [
                "self_receiver_own_method",
                "self_receiver_shadow_prefers_nearest_method",
                "this_receiver_own_method",
            ] {
                let report = report_by_name(&reports, probe_name)?;
                recorder.check(
                    &format!("{probe_name} stayed an ordinary local method candidate"),
                    report.expected_label_present
                        && !report.source_backed
                        && report.confidence == "local_syntax"
                        && report.local_method_boundary
                        && report.fallback_state == "none"
                        && report.blocked_reason.is_none()
                        && report
                            .expected_label_detail
                            .as_deref()
                            .is_some_and(|detail| detail.ends_with("\nmethod")),
                )?;
            }
            for probe_name in ["self_receiver_inherited_method", "this_receiver_inherited_method"] {
                let report = report_by_name(&reports, probe_name)?;
                recorder.check(
                    &format!("{probe_name} acted with exact high-confidence self/this detail"),
                    report.expected_label_present
                        && report.source_backed
                        && report.confidence == "high"
                        && report.self_this_receiver_labeled
                        && report.fallback_state == "none"
                        && report.blocked_reason.is_none()
                        && report
                            .expected_label_detail
                            .as_deref()
                            .is_some_and(|detail| detail.contains("receiver: self/this")),
                )?;
            }
            recorder.check(
                "self/this receiver detail is limited to inherited workspace methods",
                self_this_labeled_count == 2,
            )?;
            recorder.check(
                "self/this current-package methods remain local syntax candidates",
                local_method_boundary_count == 3,
            )?;
            recorder.check(
                "self/this inherited method probes kept inherited boundary label",
                inherited_boundary_count == 2,
            )?;
            recorder.check(
                "self/this probes did not fall back or block",
                fallback_or_blocked_count == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

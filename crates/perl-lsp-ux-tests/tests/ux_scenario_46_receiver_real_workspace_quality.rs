//! Scenario 46 - Receiver real-workspace quality receipt.
//!
//! This receipt exercises receiver-aware completion over a small multi-file
//! CPAN-style workspace. It records which receiver facts acted, fell back, or
//! stayed blocked without changing completion behavior.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_46_receiver_real_workspace_quality.rs";
const APP_PATH: &str = "lib/RealReceiver/App.pm";
const DB_PATH: &str = "lib/RealReceiver/DB.pm";
const MAILER_PATH: &str = "lib/RealReceiver/Mailer.pm";
const STATIC_PROBE_PATH: &str = "script/static-receiver.pl";
const HASHREF_PROBE_PATH: &str = "script/hashref-receiver.pl";
const DYNAMIC_PROBE_PATH: &str = "script/dynamic-receiver.pl";
const UNKNOWN_PROBE_PATH: &str = "script/unknown-receiver.pl";

const APP_PM: &str = r#"package RealReceiver::App;
use strict;
use warnings;
use RealReceiver::DB;
use RealReceiver::Mailer;

sub new {
    my ($class) = @_;
    return bless {
        db => RealReceiver::DB->new,
        mailer => RealReceiver::Mailer->new,
    }, $class;
}

sub run {
    return 1;
}

sub service_name {
    return 'real';
}

1;
"#;

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

const MAILER_PM: &str = r#"package RealReceiver::Mailer;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub send {
    return 1;
}

1;
"#;

const STATIC_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::App;

my $app = RealReceiver::App->new;
$app->
"#;

const HASHREF_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my $services = { db => RealReceiver::DB->new };
$services->{db}->
"#;

const DYNAMIC_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my $slot = 'db';
my $services = { db => RealReceiver::DB->new };
$services->{$slot}->
"#;

const UNKNOWN_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;

my ($receiver) = @_;
$receiver->
"#;

#[derive(Debug)]
struct ReceiverProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver_marker: &'static str,
    expected_label: &'static str,
    expected_receiver_detail: Option<&'static str>,
    forbidden_receiver_details: &'static [&'static str],
    fallback_allowed: bool,
}

#[derive(Debug, Serialize)]
struct ReceiverProbeReport {
    name: &'static str,
    file: &'static str,
    receiver_fact_class: &'static str,
    candidate_count: usize,
    expected_label_present: bool,
    expected_label_detail: Option<String>,
    source_backed: bool,
    fresh: bool,
    fallback_used: bool,
    blocked_reason: Option<String>,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(APP_PATH, APP_PM)
            .with_file(DB_PATH, DB_PM)
            .with_file(MAILER_PATH, MAILER_PM)
            .with_file(STATIC_PROBE_PATH, STATIC_PROBE)
            .with_file(HASHREF_PROBE_PATH, HASHREF_PROBE)
            .with_file(DYNAMIC_PROBE_PATH, DYNAMIC_PROBE)
            .with_file(UNKNOWN_PROBE_PATH, UNKNOWN_PROBE),
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

fn item_has_completion_shape(item: &Value) -> bool {
    item.get("label").and_then(Value::as_str).is_some()
        || item.get("insertText").and_then(Value::as_str).is_some()
        || item.get("filterText").and_then(Value::as_str).is_some()
}

fn probe_receiver_completion(
    harness: &UxHarness,
    probe: &ReceiverProbe,
) -> Result<ReceiverProbeReport> {
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
    let expected_label_present = expected_item.is_some();
    let expected_detail_matched =
        match (probe.expected_receiver_detail, expected_label_detail.as_deref()) {
            (Some(expected), Some(detail)) => detail.contains(expected),
            (Some(_), None) => false,
            (None, _) => true,
        };

    let forbidden_hit = items.iter().find_map(|item| {
        let text = completion_text(item);
        probe
            .forbidden_receiver_details
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

    let fallback_used = expected_label_detail.as_deref().is_some_and(|detail| {
        detail.contains("fallback")
            || detail.contains("low confidence")
            || detail.contains("unknown")
    }) || !expected_detail_matched;

    Ok(ReceiverProbeReport {
        name: probe.name,
        file: probe.file,
        receiver_fact_class: probe.name,
        candidate_count: items.len(),
        expected_label_present,
        expected_label_detail,
        source_backed: !probe.fallback_allowed && expected_detail_matched,
        fresh: true,
        fallback_used,
        blocked_reason: if expected_label_present {
            None
        } else {
            Some("expected_label_absent_or_blocked".to_string())
        },
    })
}

fn report_by_name<'a>(
    reports: &'a [ReceiverProbeReport],
    name: &str,
) -> Result<&'a ReceiverProbeReport> {
    reports
        .iter()
        .find(|report| report.name == name)
        .with_context(|| format!("missing receiver probe report `{name}`"))
}

fn receiver_probes() -> Vec<ReceiverProbe> {
    vec![
        ReceiverProbe {
            name: "constructor_assignment_receiver",
            file: STATIC_PROBE_PATH,
            source: STATIC_PROBE,
            receiver_marker: "$app->",
            expected_label: "run",
            expected_receiver_detail: Some("receiver: source-backed object"),
            forbidden_receiver_details: &[],
            fallback_allowed: false,
        },
        ReceiverProbe {
            name: "hashref_slot_receiver",
            file: HASHREF_PROBE_PATH,
            source: HASHREF_PROBE,
            receiver_marker: "$services->{db}->",
            expected_label: "connect",
            expected_receiver_detail: None,
            forbidden_receiver_details: &["receiver: source-backed hashref slot"],
            fallback_allowed: true,
        },
        ReceiverProbe {
            name: "dynamic_hash_key_receiver",
            file: DYNAMIC_PROBE_PATH,
            source: DYNAMIC_PROBE,
            receiver_marker: "$services->{$slot}->",
            expected_label: "connect",
            expected_receiver_detail: None,
            forbidden_receiver_details: &[
                "receiver: source-backed hash slot",
                "receiver: source-backed hashref slot",
            ],
            fallback_allowed: true,
        },
        ReceiverProbe {
            name: "unknown_receiver",
            file: UNKNOWN_PROBE_PATH,
            source: UNKNOWN_PROBE,
            receiver_marker: "$receiver->",
            expected_label: "connect",
            expected_receiver_detail: None,
            forbidden_receiver_details: &[
                "receiver: source-backed object",
                "receiver: source-backed hash slot",
                "receiver: source-backed hashref slot",
            ],
            fallback_allowed: true,
        },
    ]
}

#[test]
fn scenario_46_receiver_real_workspace_quality_receipt() {
    run_ux_scenario(
        "receiver_real_workspace_quality",
        SCENARIO_FILE,
        "scenario_46_receiver_real_workspace_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for path in [
                APP_PATH,
                DB_PATH,
                MAILER_PATH,
                STATIC_PROBE_PATH,
                HASHREF_PROBE_PATH,
                DYNAMIC_PROBE_PATH,
                UNKNOWN_PROBE_PATH,
            ] {
                let source = match path {
                    APP_PATH => APP_PM,
                    DB_PATH => DB_PM,
                    MAILER_PATH => MAILER_PM,
                    STATIC_PROBE_PATH => STATIC_PROBE,
                    HASHREF_PROBE_PATH => HASHREF_PROBE,
                    DYNAMIC_PROBE_PATH => DYNAMIC_PROBE,
                    UNKNOWN_PROBE_PATH => UNKNOWN_PROBE,
                    _ => unreachable!("all paths are covered"),
                };
                harness.open_file(path, source)?;
            }
            std::thread::sleep(Duration::from_millis(500));

            let probes = receiver_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_receiver_completion(&harness, probe)?;
                if report.expected_label_present {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let exact_source_backed_count =
                reports.iter().filter(|report| report.source_backed).count();
            let fallback_or_blocked_count = reports
                .iter()
                .filter(|report| report.fallback_used || report.blocked_reason.is_some())
                .count();
            let missing_expected_labels = reports
                .iter()
                .filter(|report| !report.expected_label_present && !report.fallback_used)
                .map(|report| report.name)
                .collect::<Vec<_>>();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "receiver_real_workspace_quality",
                "workspace_fixture": "RealReceiver multi-file CPAN-style workspace",
                "claim_boundary": "receipt-only receiver quality proof; no completion behavior change, support-tier promotion, or generated/dynamic promotion",
                "probe_count": reports.len(),
                "exact_source_backed_count": exact_source_backed_count,
                "fallback_or_blocked_count": fallback_or_blocked_count,
                "missing_expected_labels": missing_expected_labels,
                "reports": reports,
            });
            eprintln!(
                "receiver_real_workspace_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("all receiver probes produced reports", reports.len() == probes.len())?;
            let constructor_report = report_by_name(&reports, "constructor_assignment_receiver")?;
            recorder.check(
                "constructor assignment receiver acted with source-backed detail",
                constructor_report.expected_label_present
                    && constructor_report.source_backed
                    && !constructor_report.fallback_used
                    && constructor_report.blocked_reason.is_none(),
            )?;
            for fallback_probe in
                ["hashref_slot_receiver", "dynamic_hash_key_receiver", "unknown_receiver"]
            {
                let report = report_by_name(&reports, fallback_probe)?;
                recorder.check(
                    &format!("{fallback_probe} preserved fallback or blocker state"),
                    !report.source_backed
                        && (report.fallback_used || report.blocked_reason.is_some()),
                )?;
            }
            recorder.check(
                "hashref, dynamic, and unknown receivers preserved fallback or blocker state",
                fallback_or_blocked_count >= 3,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

//! Scenario 47 - Receiver method/accessor fallback receipt.
//!
//! This receipt records project-shaped receiver forms that currently remain
//! fallback-only for completion. Medium-confidence accessor-return and
//! method-return facts, including local accessor-chain returns, must not
//! authorize exact receiver-scoped completion.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_47_receiver_method_accessor_fallback.rs";
const DB_PATH: &str = "lib/RealReceiver/DB.pm";
const CONTAINER_PATH: &str = "lib/RealReceiver/Container.pm";
const ACCESSOR_SERVICE_PATH: &str = "lib/RealReceiver/AccessorService.pm";
const METHOD_SERVICE_PATH: &str = "lib/RealReceiver/MethodService.pm";
const LOCAL_CHAIN_SERVICE_PATH: &str = "lib/RealReceiver/LocalAccessorChainService.pm";
const ASSIGNED_CHAIN_SERVICE_PATH: &str = "lib/RealReceiver/AssignedAccessorChainService.pm";
const ACCESSOR_PROBE_PATH: &str = "script/accessor-return-receiver.pl";
const METHOD_PROBE_PATH: &str = "script/method-return-receiver.pl";
const LOCAL_CHAIN_PROBE_PATH: &str = "script/local-accessor-chain-receiver.pl";
const ASSIGNED_CHAIN_PROBE_PATH: &str = "script/assigned-accessor-chain-receiver.pl";

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

const ACCESSOR_SERVICE_PM: &str = r#"package RealReceiver::AccessorService;
use strict;
use warnings;
use Moo;
use RealReceiver::DB;

has db => (is => 'ro', isa => 'RealReceiver::DB');

sub new {
    my ($class) = @_;
    return bless { db => RealReceiver::DB->new }, $class;
}

1;
"#;

const CONTAINER_PM: &str = r#"package RealReceiver::Container;
use strict;
use warnings;
use Moo;
use RealReceiver::DB;

has db => (is => 'ro', isa => 'RealReceiver::DB');

sub new {
    my ($class) = @_;
    return bless { db => RealReceiver::DB->new }, $class;
}

1;
"#;

const METHOD_SERVICE_PM: &str = r#"package RealReceiver::MethodService;
use strict;
use warnings;
use RealReceiver::DB;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub db {
    return RealReceiver::DB->new;
}

1;
"#;

const LOCAL_CHAIN_SERVICE_PM: &str = r#"package RealReceiver::LocalAccessorChainService;
use strict;
use warnings;
use RealReceiver::Container;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub db {
    my $db = RealReceiver::Container->new->db;
    return $db;
}

1;
"#;

const ASSIGNED_CHAIN_SERVICE_PM: &str = r#"package RealReceiver::AssignedAccessorChainService;
use strict;
use warnings;
use RealReceiver::Container;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub db {
    my $db;
    $db = RealReceiver::Container->new->db;
    return $db;
}

1;
"#;

const ACCESSOR_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;
use RealReceiver::AccessorService;

my $service = RealReceiver::AccessorService->new;
$service->db->
"#;

const METHOD_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;
use RealReceiver::MethodService;

my $service = RealReceiver::MethodService->new;
$service->db->
"#;

const LOCAL_CHAIN_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;
use RealReceiver::LocalAccessorChainService;

my $service = RealReceiver::LocalAccessorChainService->new;
$service->db->
"#;

const ASSIGNED_CHAIN_PROBE: &str = r#"use strict;
use warnings;
use lib 'lib';
use RealReceiver::DB;
use RealReceiver::AssignedAccessorChainService;

my $service = RealReceiver::AssignedAccessorChainService->new;
$service->db->
"#;

#[derive(Debug)]
struct ReceiverFallbackProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver_marker: &'static str,
    expected_label: &'static str,
    expected_detail: &'static str,
    forbidden_details: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct ReceiverFallbackReport {
    name: &'static str,
    file: &'static str,
    receiver_fact_class: &'static str,
    candidate_count: usize,
    expected_label_present: bool,
    expected_label_detail: Option<String>,
    expected_sort_text: Option<String>,
    source_backed: bool,
    fresh: bool,
    fallback_used: bool,
    blocked_reason: Option<String>,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_file(DB_PATH, DB_PM)
            .with_file(CONTAINER_PATH, CONTAINER_PM)
            .with_file(ACCESSOR_SERVICE_PATH, ACCESSOR_SERVICE_PM)
            .with_file(METHOD_SERVICE_PATH, METHOD_SERVICE_PM)
            .with_file(LOCAL_CHAIN_SERVICE_PATH, LOCAL_CHAIN_SERVICE_PM)
            .with_file(ASSIGNED_CHAIN_SERVICE_PATH, ASSIGNED_CHAIN_SERVICE_PM)
            .with_file(ACCESSOR_PROBE_PATH, ACCESSOR_PROBE)
            .with_file(METHOD_PROBE_PATH, METHOD_PROBE)
            .with_file(LOCAL_CHAIN_PROBE_PATH, LOCAL_CHAIN_PROBE)
            .with_file(ASSIGNED_CHAIN_PROBE_PATH, ASSIGNED_CHAIN_PROBE),
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

fn probe_receiver_completion(
    harness: &UxHarness,
    probe: &ReceiverFallbackProbe,
) -> Result<ReceiverFallbackReport> {
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

    let detail = expected_label_detail.as_deref().unwrap_or_default();
    anyhow::ensure!(
        detail.contains(probe.expected_detail),
        "probe {} must preserve fallback detail `{}`; got {detail:?}",
        probe.name,
        probe.expected_detail
    );
    if let Some(sort_text) = expected_sort_text.as_deref() {
        anyhow::ensure!(
            sort_text.starts_with("6_"),
            "probe {} fallback completion must remain tier 6; got {sort_text:?}",
            probe.name
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

    Ok(ReceiverFallbackReport {
        name: probe.name,
        file: probe.file,
        receiver_fact_class: probe.name,
        candidate_count: items.len(),
        expected_label_present,
        expected_label_detail,
        expected_sort_text,
        source_backed: false,
        fresh: true,
        fallback_used: true,
        blocked_reason: if expected_label_present {
            None
        } else {
            Some("expected_label_absent_or_blocked".to_string())
        },
    })
}

fn report_by_name<'a>(
    reports: &'a [ReceiverFallbackReport],
    name: &str,
) -> Result<&'a ReceiverFallbackReport> {
    reports
        .iter()
        .find(|report| report.name == name)
        .with_context(|| format!("missing receiver fallback report `{name}`"))
}

fn receiver_probes() -> Vec<ReceiverFallbackProbe> {
    vec![
        ReceiverFallbackProbe {
            name: "accessor_return_receiver",
            file: ACCESSOR_PROBE_PATH,
            source: ACCESSOR_PROBE,
            receiver_marker: "$service->db->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
            ],
        },
        ReceiverFallbackProbe {
            name: "method_return_receiver",
            file: METHOD_PROBE_PATH,
            source: METHOD_PROBE,
            receiver_marker: "$service->db->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
            ],
        },
        ReceiverFallbackProbe {
            name: "local_accessor_chain_method_return_receiver",
            file: LOCAL_CHAIN_PROBE_PATH,
            source: LOCAL_CHAIN_PROBE,
            receiver_marker: "$service->db->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
            ],
        },
        ReceiverFallbackProbe {
            name: "assigned_accessor_chain_method_return_receiver",
            file: ASSIGNED_CHAIN_PROBE_PATH,
            source: ASSIGNED_CHAIN_PROBE,
            receiver_marker: "$service->db->",
            expected_label: "connect",
            expected_detail: "receiver: unknown, low confidence",
            forbidden_details: &[
                "receiver: source-backed object",
                "receiver: hash slot",
                "receiver: source-backed hashref slot",
                "receiver: literal bless",
                "receiver: type engine",
            ],
        },
    ]
}

#[test]
fn scenario_47_receiver_method_accessor_fallback_receipt() {
    run_ux_scenario(
        "receiver_method_accessor_fallback",
        SCENARIO_FILE,
        "scenario_47_receiver_method_accessor_fallback_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            for path in [
                DB_PATH,
                CONTAINER_PATH,
                ACCESSOR_SERVICE_PATH,
                METHOD_SERVICE_PATH,
                LOCAL_CHAIN_SERVICE_PATH,
                ASSIGNED_CHAIN_SERVICE_PATH,
                ACCESSOR_PROBE_PATH,
                METHOD_PROBE_PATH,
                LOCAL_CHAIN_PROBE_PATH,
                ASSIGNED_CHAIN_PROBE_PATH,
            ] {
                let source = match path {
                    DB_PATH => DB_PM,
                    CONTAINER_PATH => CONTAINER_PM,
                    ACCESSOR_SERVICE_PATH => ACCESSOR_SERVICE_PM,
                    METHOD_SERVICE_PATH => METHOD_SERVICE_PM,
                    LOCAL_CHAIN_SERVICE_PATH => LOCAL_CHAIN_SERVICE_PM,
                    ASSIGNED_CHAIN_SERVICE_PATH => ASSIGNED_CHAIN_SERVICE_PM,
                    ACCESSOR_PROBE_PATH => ACCESSOR_PROBE,
                    METHOD_PROBE_PATH => METHOD_PROBE,
                    LOCAL_CHAIN_PROBE_PATH => LOCAL_CHAIN_PROBE,
                    ASSIGNED_CHAIN_PROBE_PATH => ASSIGNED_CHAIN_PROBE,
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

            let fallback_count = reports.iter().filter(|report| report.fallback_used).count();
            let exact_source_backed_count =
                reports.iter().filter(|report| report.source_backed).count();
            let receipt = json!({
                "schema_version": 1,
                "receipt": "receiver_method_accessor_fallback",
                "workspace_fixture": "RealReceiver accessor/method/local-accessor-chain receiver CPAN-style workspace",
                "claim_boundary": "receipt-only receiver fallback proof; no completion behavior change, support-tier promotion, local accessor-chain receiver promotion, or medium-confidence receiver promotion",
                "probe_count": reports.len(),
                "exact_source_backed_count": exact_source_backed_count,
                "fallback_count": fallback_count,
                "reports": reports,
            });
            eprintln!(
                "receiver_method_accessor_fallback_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("all receiver probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "accessor and method receiver probes stayed fallback-only",
                fallback_count == probes.len() && exact_source_backed_count == 0,
            )?;
            for fallback_probe in [
                "accessor_return_receiver",
                "method_return_receiver",
                "local_accessor_chain_method_return_receiver",
                "assigned_accessor_chain_method_return_receiver",
            ] {
                let report = report_by_name(&reports, fallback_probe)?;
                recorder.check(
                    &format!("{fallback_probe} preserved low-confidence fallback detail"),
                    report.expected_label_present
                        && report.fallback_used
                        && !report.source_backed
                        && report.expected_label_detail.as_deref().is_some_and(|detail| {
                            detail.contains("receiver: unknown, low confidence")
                        })
                        && report
                            .expected_sort_text
                            .as_deref()
                            .is_some_and(|sort_text| sort_text.starts_with("6_")),
                )?;
            }

            harness.assert_no_crash();
            Ok(())
        },
    );
}

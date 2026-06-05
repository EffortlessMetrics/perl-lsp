//! Scenario 62 - project-shaped test assertion inline-completion quality receipt.
//!
//! This receipt exercises deterministic test-aware inline completion over a
//! small CPAN-shaped workspace. It proves `.t` files with project imports still
//! receive framework-aware assertion ghost text, and that visible lexical return
//! or generic test snippets do not leak into assertion slots.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available,
    missing_binary_skip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_62_project_test_assertion_inline_completion_quality.rs";
const APP_PATH: &str = "lib/My/App.pm";
const TEST_MORE_PATH: &str = "t/project-test-more.t";
const TEST2_PATH: &str = "t/project-test2.t";

const APP_PM: &str = r#"package My::App;
use strict;
use warnings;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub total { 42 }
sub is_ready { 1 }

1;
"#;

const TEST_MORE_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Test::More;
use My::App;

my $app = My::App->new;
my $got = $app->total;
my $expected = 42;

"#;

const TEST2_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Test2::V0;
use My::App;

my $app = My::App->new;
my $result = $app->is_ready;
my $expected = 1;

"#;

#[derive(Debug, Serialize)]
struct ProjectTestAssertionReport {
    name: &'static str,
    file: &'static str,
    framework: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ProjectTestAssertionProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    framework: &'static str,
    expected: &'static str,
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(APP_PATH, APP_PM)
        .with_file(TEST_MORE_PATH, TEST_MORE_SOURCE)
        .with_file(TEST2_PATH, TEST2_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn project_test_assertion_probes() -> Vec<ProjectTestAssertionProbe> {
    vec![
        ProjectTestAssertionProbe {
            name: "test_more_project_assertion",
            file: TEST_MORE_PATH,
            source: TEST_MORE_SOURCE,
            framework: "Test::More",
            expected: "is($got, $expected, 'test description');",
            forbidden: &[
                "done_testing();",
                "return $got;",
                "return $expected;",
                "ok($result, 'test description');",
                "My::App;",
            ],
        },
        ProjectTestAssertionProbe {
            name: "test2_project_assertion",
            file: TEST2_PATH,
            source: TEST2_SOURCE,
            framework: "Test2::V0",
            expected: "is($result, $expected, 'test description');",
            forbidden: &[
                "done_testing();",
                "return $result;",
                "return $expected;",
                "ok($result, 'test description');",
                "My::App;",
            ],
        },
    ]
}

fn cursor_at_end(source: &str) -> Result<(u32, u32)> {
    position_from_byte_offset(source, source.len())
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn inline_insert_text(item: &Value) -> Option<String> {
    item.get("insertText").and_then(Value::as_str).map(str::to_string)
}

fn item_has_inline_shape(item: &Value) -> bool {
    item.get("insertText").and_then(Value::as_str).is_some()
}

fn inline_registration_seen(events: &[LspEvent]) -> bool {
    events.iter().any(|event| {
        let LspEvent::Other { method, params } = event else {
            return false;
        };
        method == "client/registerCapability"
            && params.get("registrations").and_then(Value::as_array).into_iter().flatten().any(
                |registration| {
                    registration.get("method").and_then(Value::as_str)
                        == Some("textDocument/inlineCompletion")
                        && registration.get("id").and_then(Value::as_str)
                            == Some("perl-inlineCompletion")
                },
            )
    })
}

fn wait_for_inline_registration(harness: &UxHarness) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if inline_registration_seen(&harness.client.peek_events()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn probe_project_test_assertion(
    harness: &UxHarness,
    probe: &ProjectTestAssertionProbe,
) -> Result<ProjectTestAssertionReport> {
    let (line, character) = cursor_at_end(probe.source)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.inline_completion_with_trigger_kind(probe.file, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item_has_inline_shape(item),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if insert_texts.iter().any(|insert_text| insert_text == probe.expected)
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let expected_present = insert_texts.iter().any(|actual| actual == probe.expected);
    let forbidden_insert_texts = probe
        .forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect::<Vec<_>>();

    Ok(ProjectTestAssertionReport {
        name: probe.name,
        file: probe.file,
        framework: probe.framework,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_text: probe.expected,
        expected_present,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_62_project_test_assertion_inline_completion_quality_receipt() {
    run_ux_scenario(
        "project_test_assertion_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_62_project_test_assertion_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(APP_PATH, APP_PM)?;
            harness.open_file(TEST_MORE_PATH, TEST_MORE_SOURCE)?;
            harness.open_file(TEST2_PATH, TEST2_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = project_test_assertion_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_project_test_assertion(&harness, probe)?;
                if report.expected_present && report.forbidden_insert_texts.is_empty() {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let missing_expected = reports
                .iter()
                .filter(|report| !report.expected_present)
                .map(|report| report.name)
                .collect::<Vec<_>>();
            let forbidden_hits = reports
                .iter()
                .filter(|report| !report.forbidden_insert_texts.is_empty())
                .map(|report| report.name)
                .collect::<Vec<_>>();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "project_test_assertion_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped lib/My/App.pm plus Test::More and Test2 project tests",
                "claim_boundary": "project-shaped stdio inline-completion test assertion receipt only; no provider behavior change, source mirror, release action, next-edit runtime, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "project_test_assertion_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "project Test::More inline completion used visible $got/$expected lexicals",
                reports.iter().any(|report| {
                    report.name == "test_more_project_assertion"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project Test2 inline completion used visible $result lexical",
                reports.iter().any(|report| {
                    report.name == "test2_project_assertion"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project test assertion inline completion avoided return noise",
                reports.iter().all(|report| {
                    !report.forbidden_insert_texts.iter().any(|text| text.starts_with("return "))
                }),
            )?;
            recorder.check(
                "project test assertion inline completion avoided premature done_testing",
                reports
                    .iter()
                    .all(|report| !report.forbidden_insert_texts.contains(&"done_testing();")),
            )?;
            recorder.check(
                "project test assertion inline completion avoided module-import ghost text",
                reports.iter().all(|report| !report.forbidden_insert_texts.contains(&"My::App;")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

//! Scenario 63 - project-shaped control-flow inline-completion quality receipt.
//!
//! This receipt exercises deterministic loop and guard inline completion over a
//! small CPAN-shaped workspace. It proves project imports and nearby lexicals do
//! not cause control-flow ghost text to fall back to placeholders or unrelated
//! values.

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

const SCENARIO_FILE: &str = "ux_scenario_63_project_control_flow_inline_completion_quality.rs";
const APP_PATH: &str = "lib/My/App.pm";
const LOOP_PATH: &str = "script/project-loop.pl";
const RETURN_GUARD_PATH: &str = "script/project-return-guard.pl";
const NEXT_GUARD_PATH: &str = "script/project-next-guard.pl";

const APP_PM: &str = r#"package My::App;
use strict;
use warnings;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub fetch_users { return (); }
sub is_ready { return 1; }
sub should_skip { return 0; }
sub total { return 42; }

1;
"#;

const LOOP_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use My::App;

my @users = My::App::fetch_users();
for "#;

const RETURN_GUARD_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use My::App;

sub process_project {
    my $app = My::App->new;
    my $result = $app->total;
    my $is_ready = $app->is_ready;
    return unless "#;

const NEXT_GUARD_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use My::App;

sub active_users {
    my @users = My::App::fetch_users();
    for my $user (@users) {
        my $should_skip = My::App::should_skip($user);
        next if "#;

#[derive(Debug, Serialize)]
struct ProjectControlFlowReport {
    name: &'static str,
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ProjectControlFlowProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    expected: &'static str,
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(APP_PATH, APP_PM)
        .with_file(LOOP_PATH, LOOP_SOURCE)
        .with_file(RETURN_GUARD_PATH, RETURN_GUARD_SOURCE)
        .with_file(NEXT_GUARD_PATH, NEXT_GUARD_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn project_control_flow_probes() -> Vec<ProjectControlFlowProbe> {
    vec![
        ProjectControlFlowProbe {
            name: "project_array_loop_binding",
            file: LOOP_PATH,
            source: LOOP_SOURCE,
            expected: "my $user (@users) {\n    \n}",
            forbidden: &["(@items)", "keys %users_by_id", "My::App;"],
        },
        ProjectControlFlowProbe {
            name: "project_return_guard",
            file: RETURN_GUARD_PATH,
            source: RETURN_GUARD_SOURCE,
            expected: "$is_ready;",
            forbidden: &["$result;", "$condition;", "return $result;", "My::App;"],
        },
        ProjectControlFlowProbe {
            name: "project_next_guard",
            file: NEXT_GUARD_PATH,
            source: NEXT_GUARD_SOURCE,
            expected: "$should_skip;",
            forbidden: &["$user;", "$condition;", "return $user;", "My::App;"],
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

fn probe_project_control_flow(
    harness: &UxHarness,
    probe: &ProjectControlFlowProbe,
) -> Result<ProjectControlFlowReport> {
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

    Ok(ProjectControlFlowReport {
        name: probe.name,
        file: probe.file,
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
fn scenario_63_project_control_flow_inline_completion_quality_receipt() {
    run_ux_scenario(
        "project_control_flow_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_63_project_control_flow_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(APP_PATH, APP_PM)?;
            harness.open_file(LOOP_PATH, LOOP_SOURCE)?;
            harness.open_file(RETURN_GUARD_PATH, RETURN_GUARD_SOURCE)?;
            harness.open_file(NEXT_GUARD_PATH, NEXT_GUARD_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = project_control_flow_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_project_control_flow(&harness, probe)?;
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
                "receipt": "project_control_flow_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped lib/My/App.pm plus project script control-flow sites",
                "claim_boundary": "project-shaped stdio inline-completion control-flow receipt only; no provider behavior change, source mirror, release action, next-edit runtime, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "project_control_flow_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "project loop binding used the visible @users collection",
                reports.iter().any(|report| {
                    report.name == "project_array_loop_binding"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project return guard used the visible boolean lexical",
                reports.iter().any(|report| {
                    report.name == "project_return_guard"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project next guard used the visible skip lexical",
                reports.iter().any(|report| {
                    report.name == "project_next_guard"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project control-flow inline completion avoided placeholder snippets",
                reports.iter().all(|report| {
                    !report
                        .forbidden_insert_texts
                        .iter()
                        .any(|text| text.contains("condition") || text.contains("@items"))
                }),
            )?;
            recorder.check(
                "project control-flow inline completion avoided module-import ghost text",
                reports.iter().all(|report| !report.forbidden_insert_texts.contains(&"My::App;")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

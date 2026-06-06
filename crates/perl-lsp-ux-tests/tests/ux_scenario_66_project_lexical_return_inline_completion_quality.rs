//! Scenario 66 - project-shaped lexical return inline-completion quality receipt.
//!
//! This receipt exercises deterministic visible-lexical inline completion over a
//! small CPAN-shaped workspace. It proves project imports, sibling modules, and
//! test files do not pull blank-line return ghost text away from nearby lexicals.

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

const SCENARIO_FILE: &str = "ux_scenario_66_project_lexical_return_inline_completion_quality.rs";
const APP_PATH: &str = "lib/My/App.pm";
const CALCULATOR_PATH: &str = "lib/My/Project/Calculator.pm";
const CONFIG_PATH: &str = "lib/My/Project/Config.pm";
const TEST_PATH: &str = "t/project-lexical-return.t";

const APP_PM: &str = r#"package My::App;
use strict;
use warnings;
use My::Project::Calculator;
use My::Project::Config;

sub project_total {
    my ($items) = @_;
    my $result = My::Project::Calculator::total($items);
    
}

sub project_name {
    my $name = My::Project::Config::name();
    # keep the local result visible after a comment
    
}

1;
"#;

const CALCULATOR_PM: &str = r#"package My::Project::Calculator;
use strict;
use warnings;

sub total {
    my ($items) = @_;
    return scalar @{$items};
}

1;
"#;

const CONFIG_PM: &str = r#"package My::Project::Config;
use strict;
use warnings;

sub name { return 'project'; }

1;
"#;

const TEST_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Test::More;
use My::App;

my $got = My::App::project_total([1, 2, 3]);
my $expected = 3;
is($got, $expected, 'project total is available');
done_testing;
"#;

#[derive(Debug, Serialize)]
struct ProjectLexicalReturnReport {
    name: &'static str,
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    first_insert_text: Option<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    expected_first: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ProjectLexicalReturnProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    marker: &'static str,
    expected: &'static str,
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(APP_PATH, APP_PM)
        .with_file(CALCULATOR_PATH, CALCULATOR_PM)
        .with_file(CONFIG_PATH, CONFIG_PM)
        .with_file(TEST_PATH, TEST_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn project_lexical_return_probes() -> Vec<ProjectLexicalReturnProbe> {
    vec![
        ProjectLexicalReturnProbe {
            name: "project_total_return_uses_result",
            file: APP_PATH,
            source: APP_PM,
            marker: "    \n}",
            expected: "return $result;",
            forbidden: &[
                "return $items;",
                "return $name;",
                "My::Project::Calculator;",
                "is($got, $expected, 'test description');",
                "done_testing();",
                "new()",
            ],
        },
        ProjectLexicalReturnProbe {
            name: "project_comment_return_uses_name",
            file: APP_PATH,
            source: APP_PM,
            marker: "# keep the local result visible after a comment\n    \n}",
            expected: "return $name;",
            forbidden: &[
                "return $result;",
                "return $items;",
                "My::Project::Config;",
                "is($got, $expected, 'test description');",
                "done_testing();",
                "new()",
            ],
        },
    ]
}

fn cursor_on_blank_line(source: &str, marker: &str) -> Result<(u32, u32)> {
    let byte_offset =
        source.find(marker).with_context(|| format!("missing blank-line marker `{marker}`"))?
            + marker.find("    \n").with_context(|| {
                format!("marker `{marker}` must include an indented blank line")
            })?
            + "    ".len();
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

fn probe_project_lexical_return(
    harness: &UxHarness,
    probe: &ProjectLexicalReturnProbe,
) -> Result<ProjectLexicalReturnReport> {
    let (line, character) = cursor_on_blank_line(probe.source, probe.marker)?;
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
    let first_insert_text = insert_texts.first().cloned();
    let expected_first = first_insert_text.as_deref() == Some(probe.expected);
    let forbidden_insert_texts = probe
        .forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect::<Vec<_>>();

    Ok(ProjectLexicalReturnReport {
        name: probe.name,
        file: probe.file,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        first_insert_text,
        expected_insert_text: probe.expected,
        expected_present,
        expected_first,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_66_project_lexical_return_inline_completion_quality_receipt() {
    run_ux_scenario(
        "project_lexical_return_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_66_project_lexical_return_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(APP_PATH, APP_PM)?;
            harness.open_file(CALCULATOR_PATH, CALCULATOR_PM)?;
            harness.open_file(CONFIG_PATH, CONFIG_PM)?;
            harness.open_file(TEST_PATH, TEST_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = project_lexical_return_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_project_lexical_return(&harness, probe)?;
                if report.expected_first && report.forbidden_insert_texts.is_empty() {
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
                "receipt": "project_lexical_return_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped lib/My/App.pm plus sibling project modules and Test::More project test",
                "claim_boundary": "project-shaped stdio inline-completion lexical return receipt only; no provider behavior change, source mirror, release action, next-edit runtime, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "project_lexical_return_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "project lexical return used the visible $result lexical",
                reports.iter().any(|report| {
                    report.name == "project_total_return_uses_result"
                        && report.expected_first
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project lexical return after a comment used the visible $name lexical",
                reports.iter().any(|report| {
                    report.name == "project_comment_return_uses_name"
                        && report.expected_first
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project lexical return avoided module-import ghost text",
                reports.iter().all(|report| {
                    !report
                        .forbidden_insert_texts
                        .iter()
                        .any(|text| text.starts_with("My::Project::"))
                }),
            )?;
            recorder.check(
                "project lexical return avoided test and constructor snippets",
                reports.iter().all(|report| {
                    !report.forbidden_insert_texts.iter().any(|text| {
                        text.starts_with("is(") || *text == "done_testing();" || *text == "new()"
                    })
                }),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

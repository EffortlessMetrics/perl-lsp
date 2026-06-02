//! Scenario 64 - project-shaped constructor inline-completion quality receipt.
//!
//! This receipt exercises deterministic constructor-style inline completion over
//! a small CPAN-shaped workspace. It proves project imports, sibling modules,
//! and test files do not pull constructor ghost text away from the local shift
//! or signatures idiom.

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

const SCENARIO_FILE: &str = "ux_scenario_64_project_constructor_inline_completion_quality.rs";
const ROLE_PATH: &str = "lib/My/Constructor/Role.pm";
const SHIFT_APP_PATH: &str = "lib/My/Constructor/ShiftApp.pm";
const SIGNATURE_APP_PATH: &str = "lib/My/Constructor/SignatureApp.pm";
const TEST_PATH: &str = "t/constructor-style.t";

const ROLE_PM: &str = r#"package My::Constructor::Role;
use strict;
use warnings;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub is_ready { 1 }

1;
"#;

const SHIFT_APP_PM: &str = r#"package My::Constructor::ShiftApp;
use strict;
use warnings;
use My::Constructor::Role;

sub existing {
    my $self = shift;
    return $self;
}

sub build_role {
    return My::Constructor::Role->new;
}

sub new"#;

const SIGNATURE_APP_PM: &str = r#"package My::Constructor::SignatureApp;
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';
use My::Constructor::Role;

sub existing ($self, %args) {
    return $self;
}

sub build_role ($self) {
    return My::Constructor::Role->new;
}

sub new"#;

const TEST_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Test::More;
use My::Constructor::ShiftApp;
use My::Constructor::SignatureApp;

my $got = My::Constructor::ShiftApp->new;
my $expected = My::Constructor::SignatureApp->new;
is(ref $got, 'My::Constructor::ShiftApp', 'shift constructor');
is(ref $expected, 'My::Constructor::SignatureApp', 'signature constructor');
done_testing;
"#;

const SHIFT_STYLE_EXPECTED: &str =
    " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}";
const SIGNATURE_STYLE_EXPECTED: &str =
    " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}";

#[derive(Debug, Serialize)]
struct ProjectConstructorReport {
    name: &'static str,
    file: &'static str,
    style: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ProjectConstructorProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    style: &'static str,
    expected: &'static str,
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(ROLE_PATH, ROLE_PM)
        .with_file(SHIFT_APP_PATH, SHIFT_APP_PM)
        .with_file(SIGNATURE_APP_PATH, SIGNATURE_APP_PM)
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

fn project_constructor_probes() -> Vec<ProjectConstructorProbe> {
    vec![
        ProjectConstructorProbe {
            name: "project_shift_constructor",
            file: SHIFT_APP_PATH,
            source: SHIFT_APP_PM,
            style: "shift",
            expected: SHIFT_STYLE_EXPECTED,
            forbidden: &[
                SIGNATURE_STYLE_EXPECTED,
                "is($got, $expected, 'test description');",
                "done_testing();",
                "My::Constructor::Role;",
                "$is_ready;",
            ],
        },
        ProjectConstructorProbe {
            name: "project_signature_constructor",
            file: SIGNATURE_APP_PATH,
            source: SIGNATURE_APP_PM,
            style: "signature",
            expected: SIGNATURE_STYLE_EXPECTED,
            forbidden: &[
                SHIFT_STYLE_EXPECTED,
                "is($got, $expected, 'test description');",
                "done_testing();",
                "My::Constructor::Role;",
                "$is_ready;",
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

fn probe_project_constructor(
    harness: &UxHarness,
    probe: &ProjectConstructorProbe,
) -> Result<ProjectConstructorReport> {
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

    Ok(ProjectConstructorReport {
        name: probe.name,
        file: probe.file,
        style: probe.style,
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
fn scenario_64_project_constructor_inline_completion_quality_receipt() {
    run_ux_scenario(
        "project_constructor_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_64_project_constructor_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(ROLE_PATH, ROLE_PM)?;
            harness.open_file(SHIFT_APP_PATH, SHIFT_APP_PM)?;
            harness.open_file(SIGNATURE_APP_PATH, SIGNATURE_APP_PM)?;
            harness.open_file(TEST_PATH, TEST_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = project_constructor_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_project_constructor(&harness, probe)?;
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
                "receipt": "project_constructor_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped lib/My/Constructor modules plus constructor-style project test",
                "claim_boundary": "project-shaped stdio inline-completion constructor receipt only; no provider behavior change, source mirror, release action, next-edit runtime, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "project_constructor_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "project shift-style constructor kept the local shift/bless idiom",
                reports.iter().any(|report| {
                    report.name == "project_shift_constructor"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project signature-style constructor kept the local signature/bless idiom",
                reports.iter().any(|report| {
                    report.name == "project_signature_constructor"
                        && report.expected_present
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project constructor inline completion avoided test assertion ghost text",
                reports.iter().all(|report| {
                    !report
                        .forbidden_insert_texts
                        .iter()
                        .any(|text| text.starts_with("is(") || *text == "done_testing();")
                }),
            )?;
            recorder.check(
                "project constructor inline completion avoided module-import ghost text",
                reports.iter().all(|report| {
                    !report.forbidden_insert_texts.contains(&"My::Constructor::Role;")
                }),
            )?;
            recorder.check(
                "project constructor inline completion avoided visible guard snippets",
                reports.iter().all(|report| !report.forbidden_insert_texts.contains(&"$is_ready;")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

//! Scenario 65 - project-shaped DBI receiver inline-completion quality receipt.
//!
//! This receipt exercises deterministic DBI receiver inline completion over a
//! small CPAN-shaped workspace. It proves project imports, sibling modules, and
//! test files do not pull `$dbh->` or `$sth->` ghost text away from DBI-specific
//! handle methods.

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

const SCENARIO_FILE: &str = "ux_scenario_65_project_dbi_receiver_inline_completion_quality.rs";
const SCHEMA_PATH: &str = "lib/My/Db/Schema.pm";
const HANDLE_PATH: &str = "lib/My/Db/Handle.pm";
const STATEMENT_PATH: &str = "lib/My/Db/Statement.pm";
const TEST_PATH: &str = "t/dbi-receiver.t";

const SCHEMA_PM: &str = r#"package My::Db::Schema;
use strict;
use warnings;

sub table_name { 'users' }
sub is_ready { 1 }

1;
"#;

const HANDLE_PM: &str = r#"package My::Db::Handle;
use strict;
use warnings;
use DBI;
use My::Db::Schema;

sub database_handle {
    my ($dsn, $sql) = @_;
    my $table_name = My::Db::Schema::table_name();
    my $dbh = DBI->connect($dsn);
    $dbh->"#;

const STATEMENT_PM: &str = r#"package My::Db::Statement;
use strict;
use warnings;
use DBI;
use My::Db::Schema;

sub statement_handle {
    my ($dbh, $sql) = @_;
    my $is_ready = My::Db::Schema::is_ready();
    my $sth = $dbh->prepare($sql);
    $sth->f"#;

const TEST_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Test::More;
use My::Db::Handle;
use My::Db::Statement;

my $got = My::Db::Schema::table_name();
my $expected = 'users';
is($got, $expected, 'project schema is available');
done_testing;
"#;

#[derive(Debug, Serialize)]
struct ProjectDbiReceiverReport {
    name: &'static str,
    file: &'static str,
    receiver: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_texts: Vec<&'static str>,
    missing_expected_insert_texts: Vec<&'static str>,
    forbidden_insert_texts: Vec<&'static str>,
}

#[derive(Debug)]
struct ProjectDbiReceiverProbe {
    name: &'static str,
    file: &'static str,
    source: &'static str,
    receiver: &'static str,
    expected: &'static [&'static str],
    forbidden: &'static [&'static str],
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(SCHEMA_PATH, SCHEMA_PM)
        .with_file(HANDLE_PATH, HANDLE_PM)
        .with_file(STATEMENT_PATH, STATEMENT_PM)
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

fn project_dbi_receiver_probes() -> Vec<ProjectDbiReceiverProbe> {
    vec![
        ProjectDbiReceiverProbe {
            name: "project_database_handle_receiver",
            file: HANDLE_PATH,
            source: HANDLE_PM,
            receiver: "$dbh",
            expected: &["prepare()", "do()", "disconnect()"],
            forbidden: &[
                "new()",
                "fetchrow_hashref()",
                "finish()",
                "My::Db::Schema;",
                "is($got, $expected, 'test description');",
                "$is_ready;",
                "$table_name;",
            ],
        },
        ProjectDbiReceiverProbe {
            name: "project_statement_handle_receiver",
            file: STATEMENT_PATH,
            source: STATEMENT_PM,
            receiver: "$sth",
            expected: &["fetchrow_hashref()", "fetchrow_array()", "finish()"],
            forbidden: &[
                "new()",
                "prepare()",
                "disconnect()",
                "My::Db::Schema;",
                "is($got, $expected, 'test description');",
                "$is_ready;",
                "$table_name;",
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

fn probe_project_dbi_receiver(
    harness: &UxHarness,
    probe: &ProjectDbiReceiverProbe,
) -> Result<ProjectDbiReceiverReport> {
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
        if missing_expected_insert_texts(&insert_texts, probe.expected).is_empty()
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let missing_expected_insert_texts =
        missing_expected_insert_texts(&insert_texts, probe.expected);
    let forbidden_insert_texts = present_forbidden_insert_texts(&insert_texts, probe.forbidden);

    Ok(ProjectDbiReceiverReport {
        name: probe.name,
        file: probe.file,
        receiver: probe.receiver,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_texts: probe.expected.to_vec(),
        missing_expected_insert_texts,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

fn missing_expected_insert_texts<'a>(
    insert_texts: &[String],
    expected: &'a [&str],
) -> Vec<&'a str> {
    expected
        .iter()
        .copied()
        .filter(|expected| !insert_texts.iter().any(|actual| actual == expected))
        .collect()
}

fn present_forbidden_insert_texts<'a>(
    insert_texts: &[String],
    forbidden: &'a [&str],
) -> Vec<&'a str> {
    forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect()
}

#[test]
fn scenario_65_project_dbi_receiver_inline_completion_quality_receipt() {
    run_ux_scenario(
        "project_dbi_receiver_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_65_project_dbi_receiver_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(SCHEMA_PATH, SCHEMA_PM)?;
            harness.open_file(HANDLE_PATH, HANDLE_PM)?;
            harness.open_file(STATEMENT_PATH, STATEMENT_PM)?;
            harness.open_file(TEST_PATH, TEST_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            let probes = project_dbi_receiver_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = probe_project_dbi_receiver(&harness, probe)?;
                if report.missing_expected_insert_texts.is_empty()
                    && report.forbidden_insert_texts.is_empty()
                {
                    recorder.mark_first_useful_result(probe.name);
                }
                reports.push(report);
            }

            let missing_expected = reports
                .iter()
                .filter(|report| !report.missing_expected_insert_texts.is_empty())
                .map(|report| report.name)
                .collect::<Vec<_>>();
            let forbidden_hits = reports
                .iter()
                .filter(|report| !report.forbidden_insert_texts.is_empty())
                .map(|report| report.name)
                .collect::<Vec<_>>();

            let receipt = json!({
                "schema_version": 1,
                "receipt": "project_dbi_receiver_inline_completion_quality",
                "workspace_fixture": "CPAN-shaped lib/My/Db modules plus DBI receiver project test",
                "claim_boundary": "project-shaped stdio inline-completion DBI receiver receipt only; no provider behavior change, source mirror, release action, next-edit runtime, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "missing_expected": missing_expected,
                "forbidden_hits": forbidden_hits,
                "reports": reports,
            });
            eprintln!(
                "project_dbi_receiver_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "project database handle receiver returned DBI handle methods",
                reports.iter().any(|report| {
                    report.name == "project_database_handle_receiver"
                        && report.missing_expected_insert_texts.is_empty()
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project statement handle receiver returned DBI statement methods",
                reports.iter().any(|report| {
                    report.name == "project_statement_handle_receiver"
                        && report.missing_expected_insert_texts.is_empty()
                        && report.forbidden_insert_texts.is_empty()
                }),
            )?;
            recorder.check(
                "project DBI receiver inline completion avoided generic constructor guesses",
                reports.iter().all(|report| !report.forbidden_insert_texts.contains(&"new()")),
            )?;
            recorder.check(
                "project DBI receiver inline completion avoided module-import ghost text",
                reports
                    .iter()
                    .all(|report| !report.forbidden_insert_texts.contains(&"My::Db::Schema;")),
            )?;
            recorder.check(
                "project DBI receiver inline completion avoided test assertion and visible lexical ghost text",
                reports.iter().all(|report| {
                    !report
                        .forbidden_insert_texts
                        .iter()
                        .any(|text| text.starts_with("is(") || *text == "$is_ready;" || *text == "$table_name;")
                }),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

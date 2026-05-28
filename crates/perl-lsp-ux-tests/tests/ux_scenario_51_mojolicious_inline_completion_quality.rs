//! Scenario 51 - Mojolicious inline-completion quality receipt.
//!
//! This receipt exercises inline completion over the committed Mojolicious
//! skeleton workspace. It records whether invoked module ghost text uses
//! reachable project modules and whether automatic ghost text stays silent in
//! a hard reject zone.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    LspEvent, ProjectFixtureFile, UxCiTier, UxComponent, UxHarness, binary_available,
    fixture_scenario_config, load_mojolicious_fixture_files, missing_binary_skip,
    open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_51_mojolicious_inline_completion_quality.rs";
const MODULE_IMPORT_PROBE_PATH: &str = "script/inline-mojolicious-import.pl";
const HARD_ZONE_PROBE_PATH: &str = "script/inline-mojolicious-comment.pl";
const MODULE_MARKER: &str = "use Mojolicious::";
const HARD_ZONE_MARKER: &str = "Mojolicious::";

const MODULE_IMPORT_PROBE_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Mojolicious::
"#;

const HARD_ZONE_PROBE_SOURCE: &str = r#"# use Mojolicious::
"#;

const EXPECTED_MODULE_INSERTS: &[&str] =
    &["Mojolicious::Commands;", "Mojolicious::Controller;", "Mojolicious::Renderer;"];

const FORBIDDEN_MODULE_INSERTS: &[&str] =
    &["strict;", "warnings;", "feature ':5.36';", "Mojo::Base;"];

#[derive(Debug, Serialize)]
struct InlineModuleProbeReport {
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_texts: Vec<&'static str>,
    missing_expected_insert_texts: Vec<&'static str>,
    forbidden_insert_texts: Vec<&'static str>,
    expected_range: InlineRangeExpectation,
    range_reports: Vec<InlineRangeReport>,
    range_violation_insert_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InlineRangeExpectation {
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
    replaces: &'static str,
}

#[derive(Debug, Serialize)]
struct InlineRangeReport {
    insert_text: String,
    range: Option<Value>,
    single_line: bool,
    replaces_typed_prefix: bool,
}

#[derive(Debug, Serialize)]
struct InlineSelectedCompletionReport {
    file: &'static str,
    selected_text: &'static str,
    selected_range: InlineRangeExpectation,
    accepted_candidate_count: usize,
    accepted_insert_texts: Vec<String>,
    accepted_ranges_match_selection: bool,
    conflicting_selected_text: &'static str,
    conflicting_candidate_count: usize,
    conflicting_insert_texts: Vec<String>,
    conflict_suppressed: bool,
}

#[derive(Debug, Serialize)]
struct InlineSilenceProbeReport {
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    stayed_silent: bool,
    insert_texts: Vec<String>,
}

fn create_harness(fixture_files: &[ProjectFixtureFile]) -> Result<UxHarness> {
    let mut config = fixture_scenario_config(fixture_files)
        .with_file(MODULE_IMPORT_PROBE_PATH, MODULE_IMPORT_PROBE_SOURCE)
        .with_file(HARD_ZONE_PROBE_PATH, HARD_ZONE_PROBE_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
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

fn probe_module_inline_completion(harness: &UxHarness) -> Result<InlineModuleProbeReport> {
    let (line, character) = position_after(MODULE_IMPORT_PROBE_SOURCE, MODULE_MARKER)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.inline_completion_with_trigger_kind(
            MODULE_IMPORT_PROBE_PATH,
            line,
            character,
            1,
        )?;
        for item in &items {
            anyhow::ensure!(
                item_has_inline_shape(item),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if EXPECTED_MODULE_INSERTS
            .iter()
            .all(|expected| insert_texts.iter().any(|actual| actual == expected))
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let missing_expected_insert_texts = EXPECTED_MODULE_INSERTS
        .iter()
        .copied()
        .filter(|expected| !insert_texts.iter().any(|actual| actual == expected))
        .collect::<Vec<_>>();
    let forbidden_insert_texts = FORBIDDEN_MODULE_INSERTS
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect::<Vec<_>>();
    let expected_range = expected_module_range()?;
    let range_reports = EXPECTED_MODULE_INSERTS
        .iter()
        .filter_map(|expected| {
            items
                .iter()
                .find(|item| item.get("insertText").and_then(Value::as_str) == Some(*expected))
                .map(|item| range_report_for_item(item, &expected_range))
        })
        .collect::<Vec<_>>();
    let range_violation_insert_texts = range_reports
        .iter()
        .filter(|report| !report.replaces_typed_prefix)
        .map(|report| report.insert_text.clone())
        .collect::<Vec<_>>();

    Ok(InlineModuleProbeReport {
        file: MODULE_IMPORT_PROBE_PATH,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_texts: EXPECTED_MODULE_INSERTS.to_vec(),
        missing_expected_insert_texts,
        forbidden_insert_texts,
        expected_range,
        range_reports,
        range_violation_insert_texts,
    })
}

fn expected_module_range() -> Result<InlineRangeExpectation> {
    let marker_start = MODULE_IMPORT_PROBE_SOURCE
        .find(MODULE_MARKER)
        .with_context(|| format!("missing `{MODULE_MARKER}`"))?;
    let start_byte = marker_start + "use ".len();
    let end_byte = marker_start + MODULE_MARKER.len();
    let (start_line, start_character) =
        position_from_byte_offset(MODULE_IMPORT_PROBE_SOURCE, start_byte)?;
    let (end_line, end_character) =
        position_from_byte_offset(MODULE_IMPORT_PROBE_SOURCE, end_byte)?;

    Ok(InlineRangeExpectation {
        start_line,
        start_character,
        end_line,
        end_character,
        replaces: "Mojolicious::",
    })
}

fn range_report_for_item(item: &Value, expected: &InlineRangeExpectation) -> InlineRangeReport {
    let insert_text =
        item.get("insertText").and_then(Value::as_str).unwrap_or_default().to_string();
    let range = item.get("range").cloned();
    let tuple = range_tuple(item);
    let single_line = tuple.is_some_and(|(start_line, _, end_line, _)| start_line == end_line);
    let replaces_typed_prefix =
        tuple.is_some_and(|(start_line, start_character, end_line, end_character)| {
            start_line == expected.start_line
                && start_character == expected.start_character
                && end_line == expected.end_line
                && end_character == expected.end_character
        });

    InlineRangeReport { insert_text, range, single_line, replaces_typed_prefix }
}

fn range_tuple(item: &Value) -> Option<(u32, u32, u32, u32)> {
    let range = item.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    Some((
        u32::try_from(start.get("line")?.as_u64()?).ok()?,
        u32::try_from(start.get("character")?.as_u64()?).ok()?,
        u32::try_from(end.get("line")?.as_u64()?).ok()?,
        u32::try_from(end.get("character")?.as_u64()?).ok()?,
    ))
}

fn probe_hard_zone_inline_completion(harness: &UxHarness) -> Result<InlineSilenceProbeReport> {
    let (line, character) = position_after(HARD_ZONE_PROBE_SOURCE, HARD_ZONE_MARKER)?;
    let items =
        harness.inline_completion_with_trigger_kind(HARD_ZONE_PROBE_PATH, line, character, 2)?;
    for item in &items {
        anyhow::ensure!(
            item_has_inline_shape(item),
            "inline item must include insertText: {item:?}"
        );
    }
    let insert_texts = insert_texts_for(&items);

    Ok(InlineSilenceProbeReport {
        file: HARD_ZONE_PROBE_PATH,
        trigger_kind: 2,
        candidate_count: items.len(),
        stayed_silent: items.is_empty(),
        insert_texts,
    })
}

fn probe_selected_completion_info(harness: &UxHarness) -> Result<InlineSelectedCompletionReport> {
    let (line, character) = position_after(MODULE_IMPORT_PROBE_SOURCE, MODULE_MARKER)?;
    let selected_range = expected_module_range()?;
    let selected_text = "Mojolicious::Commands";
    let accepted_items = harness.inline_completion_with_context(
        MODULE_IMPORT_PROBE_PATH,
        line,
        character,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": range_json(&selected_range),
                "text": selected_text,
            }
        }),
    )?;
    for item in &accepted_items {
        anyhow::ensure!(
            item_has_inline_shape(item),
            "inline item must include insertText: {item:?}"
        );
    }
    let accepted_insert_texts = insert_texts_for(&accepted_items);
    let accepted_ranges_match_selection = accepted_items
        .iter()
        .all(|item| range_report_for_item(item, &selected_range).replaces_typed_prefix);

    let conflicting_selected_text = "Mojo::Base";
    let conflicting_items = harness.inline_completion_with_context(
        MODULE_IMPORT_PROBE_PATH,
        line,
        character,
        json!({
            "triggerKind": 1,
            "selectedCompletionInfo": {
                "range": range_json(&selected_range),
                "text": conflicting_selected_text,
            }
        }),
    )?;
    for item in &conflicting_items {
        anyhow::ensure!(
            item_has_inline_shape(item),
            "inline item must include insertText: {item:?}"
        );
    }
    let conflicting_insert_texts = insert_texts_for(&conflicting_items);

    Ok(InlineSelectedCompletionReport {
        file: MODULE_IMPORT_PROBE_PATH,
        selected_text,
        selected_range,
        accepted_candidate_count: accepted_items.len(),
        accepted_insert_texts,
        accepted_ranges_match_selection,
        conflicting_selected_text,
        conflicting_candidate_count: conflicting_items.len(),
        conflicting_insert_texts,
        conflict_suppressed: conflicting_items.is_empty(),
    })
}

fn range_json(range: &InlineRangeExpectation) -> Value {
    json!({
        "start": {
            "line": range.start_line,
            "character": range.start_character,
        },
        "end": {
            "line": range.end_line,
            "character": range.end_character,
        }
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_51_mojolicious_inline_completion_quality_receipt() {
    run_ux_scenario(
        "mojolicious_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_51_mojolicious_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;
            let fixture_file_count = fixture_files.len();
            let harness = create_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            harness.open_file(MODULE_IMPORT_PROBE_PATH, MODULE_IMPORT_PROBE_SOURCE)?;
            harness.open_file(HARD_ZONE_PROBE_PATH, HARD_ZONE_PROBE_SOURCE)?;
            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            recorder.mark_request_start("module_import_inline_completion");
            let module_report = probe_module_inline_completion(&harness)?;
            if module_report.missing_expected_insert_texts.is_empty() {
                recorder.mark_first_useful_result("module_import_inline_completion");
            }

            recorder.mark_request_start("selected_completion_info_alignment");
            let selected_completion_report = probe_selected_completion_info(&harness)?;
            if selected_completion_report
                .accepted_insert_texts
                .iter()
                .any(|insert_text| insert_text == "Mojolicious::Commands;")
                && selected_completion_report.accepted_ranges_match_selection
                && selected_completion_report.conflict_suppressed
            {
                recorder.mark_first_useful_result("selected_completion_info_alignment");
            }

            recorder.mark_request_start("hard_zone_inline_completion");
            let hard_zone_report = probe_hard_zone_inline_completion(&harness)?;
            if hard_zone_report.stayed_silent {
                recorder.mark_first_useful_result("hard_zone_inline_completion");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "mojolicious_inline_completion_quality",
                "workspace_fixture": "mojolicious_skeleton",
                "claim_boundary": "real-workspace inline-completion quality receipt only; no provider behavior change, support-tier promotion, source mirror, release action, or AI behavior",
                "fixture_file_count": fixture_file_count,
                "dynamic_registration_seen": dynamic_registration_seen,
                "module_probe": module_report,
                "selected_completion_probe": selected_completion_report,
                "hard_zone_probe": hard_zone_report,
            });
            eprintln!(
                "mojolicious_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "invoked module inline completion returned candidates",
                module_report.candidate_count > 0,
            )?;
            recorder.check(
                "invoked module inline completion used reachable Mojolicious modules",
                module_report.missing_expected_insert_texts.is_empty(),
            )?;
            recorder.check(
                "invoked module inline completion avoided unrelated/generic inserts",
                module_report.forbidden_insert_texts.is_empty(),
            )?;
            recorder.check(
                "invoked module inline completion replaced the typed module prefix",
                module_report.range_violation_insert_texts.is_empty(),
            )?;
            recorder.check(
                "selectedCompletionInfo returned only the selected extending module",
                selected_completion_report
                    .accepted_insert_texts
                    .iter()
                    .any(|insert_text| insert_text == "Mojolicious::Commands;")
                    && selected_completion_report.accepted_candidate_count == 1,
            )?;
            recorder.check(
                "selectedCompletionInfo preserved the selected completion range",
                selected_completion_report.accepted_ranges_match_selection,
            )?;
            recorder.check(
                "conflicting selectedCompletionInfo suppressed ghost text",
                selected_completion_report.conflict_suppressed,
            )?;
            recorder.check(
                "automatic inline completion stayed silent in line comment",
                hard_zone_report.stayed_silent,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

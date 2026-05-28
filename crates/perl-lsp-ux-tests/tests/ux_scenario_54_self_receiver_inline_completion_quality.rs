//! Scenario 54 - self receiver inline-completion quality receipt.
//!
//! This receipt exercises deterministic current-package receiver inline
//! completion through a real stdio LSP process. It records whether `$self->`
//! ghost text uses methods from the package already on screen instead of
//! unrelated package methods or generic guesses.

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

const SCENARIO_FILE: &str = "ux_scenario_54_self_receiver_inline_completion_quality.rs";
const SELF_RECEIVER_PATH: &str = "lib/Inline/SelfReceiver.pm";
const SELF_RECEIVER_MARKER: &str = "$self->";

const SELF_RECEIVER_SOURCE: &str = r#"package Other;
sub external {}

package Inline::SelfReceiver;
use strict;
use warnings;

sub save {}
sub display_name {}

sub caller {
    my $self = shift;
    $self->
}
"#;

const EXPECTED_METHOD_INSERTS: &[&str] = &["save()", "display_name()"];
const FORBIDDEN_METHOD_INSERTS: &[&str] = &["external()", "new()"];

#[derive(Debug, Serialize)]
struct SelfReceiverProbeReport {
    file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_texts: Vec<&'static str>,
    missing_expected_insert_texts: Vec<&'static str>,
    forbidden_insert_texts: Vec<&'static str>,
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default().with_file(SELF_RECEIVER_PATH, SELF_RECEIVER_SOURCE);
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

fn probe_self_receiver_inline_completion(harness: &UxHarness) -> Result<SelfReceiverProbeReport> {
    let (line, character) = position_after(SELF_RECEIVER_SOURCE, SELF_RECEIVER_MARKER)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items =
            harness.inline_completion_with_trigger_kind(SELF_RECEIVER_PATH, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item_has_inline_shape(item),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if EXPECTED_METHOD_INSERTS
            .iter()
            .all(|expected| insert_texts.iter().any(|actual| actual == expected))
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let missing_expected_insert_texts = EXPECTED_METHOD_INSERTS
        .iter()
        .copied()
        .filter(|expected| !insert_texts.iter().any(|actual| actual == expected))
        .collect::<Vec<_>>();
    let forbidden_insert_texts = FORBIDDEN_METHOD_INSERTS
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect::<Vec<_>>();

    Ok(SelfReceiverProbeReport {
        file: SELF_RECEIVER_PATH,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_texts: EXPECTED_METHOD_INSERTS.to_vec(),
        missing_expected_insert_texts,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_54_self_receiver_inline_completion_quality_receipt() {
    run_ux_scenario(
        "self_receiver_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_54_self_receiver_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(SELF_RECEIVER_PATH, SELF_RECEIVER_SOURCE)?;
            std::thread::sleep(Duration::from_millis(250));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            recorder.mark_request_start("self_receiver_inline_completion");
            let receiver_report = probe_self_receiver_inline_completion(&harness)?;
            if receiver_report.missing_expected_insert_texts.is_empty() {
                recorder.mark_first_useful_result("self_receiver_inline_completion");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "self_receiver_inline_completion_quality",
                "claim_boundary": "stdio inline-completion self-receiver quality receipt only; no provider behavior change, support-tier promotion, source mirror, release action, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "receiver_probe": receiver_report,
            });
            eprintln!(
                "self_receiver_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "$self receiver inline completion returned candidates",
                receiver_report.candidate_count > 0,
            )?;
            recorder.check(
                "$self receiver inline completion used current-package methods",
                receiver_report.missing_expected_insert_texts.is_empty(),
            )?;
            recorder.check(
                "$self receiver inline completion avoided unrelated/generic methods",
                receiver_report.forbidden_insert_texts.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

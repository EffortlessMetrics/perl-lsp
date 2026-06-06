//! Scenario 52 - Perl test inline-completion quality receipt.
//!
//! This receipt exercises deterministic test-aware inline completion over a
//! real stdio LSP process. It records whether invoked ghost text follows the
//! active test framework and visible lexical names instead of falling back to
//! generic snippets.

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

const SCENARIO_FILE: &str = "ux_scenario_52_test_inline_completion_quality.rs";
const TEST_MORE_PATH: &str = "t/inline-test-more.t";
const TEST2_PATH: &str = "t/inline-test2.t";

const TEST_MORE_SOURCE: &str = r#"use strict;
use warnings;
use Test::More;

my $got = compute_total();
my $expected = 42;

"#;

const TEST2_SOURCE: &str = r#"use strict;
use warnings;
use Test2::V0;

my $result = compute_total();

"#;

#[derive(Debug, Serialize)]
struct InlineTestProbeReport {
    file: &'static str,
    framework: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
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

fn probe_test_inline_completion(
    harness: &UxHarness,
    file: &'static str,
    source: &str,
    framework: &'static str,
    expected_insert_text: &'static str,
    forbidden: &[&'static str],
) -> Result<InlineTestProbeReport> {
    let (line, character) = cursor_at_end(source)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.inline_completion_with_trigger_kind(file, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item_has_inline_shape(item),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if insert_texts.iter().any(|insert_text| insert_text == expected_insert_text)
            || Instant::now() >= deadline
        {
            break items;
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let insert_texts = insert_texts_for(&items);
    let forbidden_insert_texts = forbidden
        .iter()
        .copied()
        .filter(|forbidden_text| insert_texts.iter().any(|actual| actual == forbidden_text))
        .collect::<Vec<_>>();
    let expected_present = insert_texts.iter().any(|actual| actual == expected_insert_text);

    Ok(InlineTestProbeReport {
        file,
        framework,
        trigger_kind: 1,
        candidate_count: items.len(),
        insert_texts,
        expected_insert_text,
        expected_present,
        forbidden_insert_texts,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_52_test_inline_completion_quality_receipt() {
    run_ux_scenario(
        "test_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_52_test_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(TEST_MORE_PATH, TEST_MORE_SOURCE)?;
            harness.open_file(TEST2_PATH, TEST2_SOURCE)?;
            std::thread::sleep(Duration::from_millis(250));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            recorder.mark_request_start("test_more_inline_completion");
            let test_more_report = probe_test_inline_completion(
                &harness,
                TEST_MORE_PATH,
                TEST_MORE_SOURCE,
                "Test::More",
                "is($got, $expected, 'test description');",
                &["done_testing();", "ok($result, 'test description');"],
            )?;
            if test_more_report.expected_present {
                recorder.mark_first_useful_result("test_more_inline_completion");
            }

            recorder.mark_request_start("test2_inline_completion");
            let test2_report = probe_test_inline_completion(
                &harness,
                TEST2_PATH,
                TEST2_SOURCE,
                "Test2::V0",
                "ok($result, 'test description');",
                &["done_testing();", "is($got, $expected, 'test description');"],
            )?;
            if test2_report.expected_present {
                recorder.mark_first_useful_result("test2_inline_completion");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "test_inline_completion_quality",
                "claim_boundary": "stdio inline-completion test-source quality receipt only; no provider behavior change, support-tier promotion, source mirror, release action, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "test_more_probe": test_more_report,
                "test2_probe": test2_report,
            });
            eprintln!(
                "test_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "Test::More inline completion used visible $got/$expected lexicals",
                test_more_report.expected_present,
            )?;
            recorder.check(
                "Test::More inline completion avoided unrelated test snippets",
                test_more_report.forbidden_insert_texts.is_empty(),
            )?;
            recorder.check(
                "Test2 inline completion used visible $result lexical",
                test2_report.expected_present,
            )?;
            recorder.check(
                "Test2 inline completion avoided unrelated Test::More snippets",
                test2_report.forbidden_insert_texts.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

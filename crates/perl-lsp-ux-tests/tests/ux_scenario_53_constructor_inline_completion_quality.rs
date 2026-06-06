//! Scenario 53 - constructor inline-completion quality receipt.
//!
//! This receipt exercises deterministic constructor-style inline completion
//! through a real stdio LSP process. It records whether invoked ghost text
//! follows nearby constructor idioms instead of mixing shift, @_ and signature
//! styles.

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

const SCENARIO_FILE: &str = "ux_scenario_53_constructor_inline_completion_quality.rs";
const SHIFT_STYLE_PATH: &str = "lib/Inline/ShiftStyle.pm";
const SIGNATURE_STYLE_PATH: &str = "lib/Inline/SignatureStyle.pm";

const SHIFT_STYLE_SOURCE: &str = r#"package Inline::ShiftStyle;
use strict;
use warnings;

sub existing {
    my $self = shift;
    return $self;
}

sub new"#;

const SIGNATURE_STYLE_SOURCE: &str = r#"package Inline::SignatureStyle;
use strict;
use warnings;
use feature 'signatures';
no warnings 'experimental::signatures';

sub existing ($self, %args) {
    return $self;
}

sub new"#;

const SHIFT_STYLE_EXPECTED: &str =
    " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}";
const SIGNATURE_STYLE_EXPECTED: &str =
    " ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}";

#[derive(Debug, Serialize)]
struct ConstructorProbeReport {
    file: &'static str,
    style: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    insert_texts: Vec<String>,
    expected_insert_text: &'static str,
    expected_present: bool,
    forbidden_insert_texts: Vec<&'static str>,
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(SHIFT_STYLE_PATH, SHIFT_STYLE_SOURCE)
        .with_file(SIGNATURE_STYLE_PATH, SIGNATURE_STYLE_SOURCE);
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

fn probe_constructor_inline_completion(
    harness: &UxHarness,
    file: &'static str,
    source: &str,
    style: &'static str,
    expected_insert_text: &'static str,
    forbidden: &[&'static str],
) -> Result<ConstructorProbeReport> {
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

    Ok(ConstructorProbeReport {
        file,
        style,
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
fn scenario_53_constructor_inline_completion_quality_receipt() {
    run_ux_scenario(
        "constructor_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_53_constructor_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(SHIFT_STYLE_PATH, SHIFT_STYLE_SOURCE)?;
            harness.open_file(SIGNATURE_STYLE_PATH, SIGNATURE_STYLE_SOURCE)?;
            std::thread::sleep(Duration::from_millis(250));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            recorder.mark_request_start("shift_style_constructor_inline_completion");
            let shift_style_report = probe_constructor_inline_completion(
                &harness,
                SHIFT_STYLE_PATH,
                SHIFT_STYLE_SOURCE,
                "shift",
                SHIFT_STYLE_EXPECTED,
                &[SIGNATURE_STYLE_EXPECTED],
            )?;
            if shift_style_report.expected_present {
                recorder.mark_first_useful_result("shift_style_constructor_inline_completion");
            }

            recorder.mark_request_start("signature_style_constructor_inline_completion");
            let signature_style_report = probe_constructor_inline_completion(
                &harness,
                SIGNATURE_STYLE_PATH,
                SIGNATURE_STYLE_SOURCE,
                "signature",
                SIGNATURE_STYLE_EXPECTED,
                &[
                    " {\n    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;\n}",
                ],
            )?;
            if signature_style_report.expected_present {
                recorder.mark_first_useful_result("signature_style_constructor_inline_completion");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "constructor_inline_completion_quality",
                "claim_boundary": "stdio inline-completion constructor-style quality receipt only; no provider behavior change, support-tier promotion, source mirror, release action, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "shift_style_probe": shift_style_report,
                "signature_style_probe": signature_style_report,
            });
            eprintln!(
                "constructor_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "shift-style constructor inline completion used shift idiom",
                shift_style_report.expected_present,
            )?;
            recorder.check(
                "shift-style constructor inline completion avoided signature idiom",
                shift_style_report.forbidden_insert_texts.is_empty(),
            )?;
            recorder.check(
                "signature-style constructor inline completion used signature idiom",
                signature_style_report.expected_present,
            )?;
            recorder.check(
                "signature-style constructor inline completion avoided shift idiom",
                signature_style_report.forbidden_insert_texts.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

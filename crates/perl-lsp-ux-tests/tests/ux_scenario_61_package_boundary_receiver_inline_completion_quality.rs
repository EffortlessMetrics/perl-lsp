//! Scenario 61 - package-boundary receiver inline-completion quality receipt.
//!
//! This receipt exercises `$self->` inline completion in a project-shaped
//! workspace with another package nearby. It proves current-package methods stay
//! preferred and unrelated package methods or generic constructor guesses do not
//! leak into ghost text.

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

const SCENARIO_FILE: &str = "ux_scenario_61_package_boundary_receiver_inline_completion_quality.rs";
const MODEL_PATH: &str = "lib/Inline/PackageBoundary/Model.pm";
const OTHER_PATH: &str = "lib/Inline/PackageBoundary/Other.pm";
const SELF_RECEIVER_MARKER: &str = "$self->";

const MODEL_SOURCE: &str = r#"package Inline::PackageBoundary::Model;
use strict;
use warnings;

sub save {}
sub display_name {}
sub hydrate {}

sub caller {
    my $self = shift;
    $self->
}

1;
"#;

const OTHER_SOURCE: &str = r#"package Inline::PackageBoundary::Other;
use strict;
use warnings;

sub external {}
sub archive {}
sub new {}

1;
"#;

const EXPECTED_METHOD_INSERTS: &[&str] = &["save()", "display_name()", "hydrate()"];
const FORBIDDEN_METHOD_INSERTS: &[&str] = &["external()", "archive()", "new()"];

#[derive(Debug, Serialize)]
struct PackageBoundaryReceiverReport {
    file: &'static str,
    sibling_file: &'static str,
    trigger_kind: u8,
    candidate_count: usize,
    first_insert_text: Option<String>,
    insert_texts: Vec<String>,
    expected_insert_texts: Vec<&'static str>,
    missing_expected_insert_texts: Vec<&'static str>,
    forbidden_insert_texts: Vec<&'static str>,
    current_package_methods_preferred: bool,
}

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .env("PERL_LSP_WORKSPACE", "1")
        .with_file(MODEL_PATH, MODEL_SOURCE)
        .with_file(OTHER_PATH, OTHER_SOURCE);
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

fn probe_package_boundary_receiver(harness: &UxHarness) -> Result<PackageBoundaryReceiverReport> {
    let (line, character) = position_after(MODEL_SOURCE, SELF_RECEIVER_MARKER)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let items = loop {
        let items = harness.inline_completion_with_trigger_kind(MODEL_PATH, line, character, 1)?;
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
    let first_insert_text = insert_texts.first().cloned();
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
    let current_package_methods_preferred = first_insert_text
        .as_deref()
        .is_some_and(|first| EXPECTED_METHOD_INSERTS.iter().any(|expected| expected == &first));

    Ok(PackageBoundaryReceiverReport {
        file: MODEL_PATH,
        sibling_file: OTHER_PATH,
        trigger_kind: 1,
        candidate_count: items.len(),
        first_insert_text,
        insert_texts,
        expected_insert_texts: EXPECTED_METHOD_INSERTS.to_vec(),
        missing_expected_insert_texts,
        forbidden_insert_texts,
        current_package_methods_preferred,
    })
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(inline_insert_text).collect()
}

#[test]
fn scenario_61_package_boundary_receiver_inline_completion_quality_receipt() {
    run_ux_scenario(
        "package_boundary_receiver_inline_completion_quality",
        SCENARIO_FILE,
        "scenario_61_package_boundary_receiver_inline_completion_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(OTHER_PATH, OTHER_SOURCE)?;
            harness.open_file(MODEL_PATH, MODEL_SOURCE)?;
            std::thread::sleep(Duration::from_millis(300));

            recorder.mark_request_start("dynamic_inline_registration");
            let dynamic_registration_seen = wait_for_inline_registration(&harness);
            if dynamic_registration_seen {
                recorder.mark_first_useful_result("dynamic_inline_registration");
            }

            recorder.mark_request_start("package_boundary_receiver_inline_completion");
            let receiver_report = probe_package_boundary_receiver(&harness)?;
            if receiver_report.missing_expected_insert_texts.is_empty()
                && receiver_report.forbidden_insert_texts.is_empty()
                && receiver_report.current_package_methods_preferred
            {
                recorder.mark_first_useful_result("package_boundary_receiver_inline_completion");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "package_boundary_receiver_inline_completion_quality",
                "workspace_fixture": "Inline::PackageBoundary multi-file receiver workspace",
                "claim_boundary": "stdio inline-completion package-boundary receiver receipt only; no provider behavior change, source mirror, release action, next-edit, or AI behavior",
                "dynamic_registration_seen": dynamic_registration_seen,
                "receiver_probe": receiver_report,
            });
            eprintln!(
                "package_boundary_receiver_inline_completion_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("dynamic inline registration was observed", dynamic_registration_seen)?;
            recorder.check(
                "package-boundary receiver inline completion returned candidates",
                receiver_report.candidate_count > 0,
            )?;
            recorder.check(
                "package-boundary receiver inline completion used current-package methods",
                receiver_report.missing_expected_insert_texts.is_empty(),
            )?;
            recorder.check(
                "package-boundary receiver inline completion preferred current-package methods",
                receiver_report.current_package_methods_preferred,
            )?;
            recorder.check(
                "package-boundary receiver inline completion avoided unrelated/generic methods",
                receiver_report.forbidden_insert_texts.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

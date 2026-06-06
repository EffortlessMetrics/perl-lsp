//! Scenario 57 - loop-binding inline-completion quality proof.
//!
//! This test exercises deterministic visible-collection inline completion
//! through a real stdio LSP process. It verifies that invoked ghost text after
//! `for ` uses arrays and hash keys already visible in the file instead of
//! falling back to placeholder loop snippets.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const ARRAY_LOOP_PATH: &str = "lib/Inline/LoopArray.pl";
const HASH_LOOP_PATH: &str = "lib/Inline/LoopHash.pl";
const ARRAY_PREFERENCE_PATH: &str = "lib/Inline/LoopArrayPreference.pl";
const STATUS_LOOP_PATH: &str = "lib/Inline/LoopStatus.pl";

const ARRAY_LOOP_SOURCE: &str = r#"use strict;
use warnings;

my @users = fetch_users();
for "#;

const HASH_LOOP_SOURCE: &str = r#"use strict;
use warnings;

my %users_by_id = load_users();
for "#;

const ARRAY_PREFERENCE_SOURCE: &str = r#"use strict;
use warnings;

my %users_by_id = load_users();
my @users = values %users_by_id;
for "#;

const STATUS_LOOP_SOURCE: &str = r#"use strict;
use warnings;

my @status = fetch_status();
for "#;

const PLACEHOLDER_INSERT: &str = "(@items)";

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(ARRAY_LOOP_PATH, ARRAY_LOOP_SOURCE)
        .with_file(HASH_LOOP_PATH, HASH_LOOP_SOURCE)
        .with_file(ARRAY_PREFERENCE_PATH, ARRAY_PREFERENCE_SOURCE)
        .with_file(STATUS_LOOP_PATH, STATUS_LOOP_SOURCE);
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

fn wait_for_loop_binding(
    harness: &UxHarness,
    file: &'static str,
    source: &str,
    expected_insert: &str,
) -> Result<Vec<String>> {
    let (line, character) = cursor_at_end(source)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let items = harness.inline_completion_with_trigger_kind(file, line, character, 1)?;
        for item in &items {
            anyhow::ensure!(
                item.get("insertText").and_then(Value::as_str).is_some(),
                "inline item must include insertText: {item:?}"
            );
        }
        let insert_texts = insert_texts_for(&items);
        if insert_texts.iter().any(|insert_text| insert_text == expected_insert)
            || Instant::now() >= deadline
        {
            return Ok(insert_texts);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get("insertText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn assert_excludes_fragment(insert_texts: &[String], forbidden: &str, label: &str) {
    assert!(
        insert_texts.iter().all(|actual| !actual.contains(forbidden)),
        "{label} included forbidden fragment `{forbidden}`; actual: {insert_texts:?}"
    );
}

#[test]
fn scenario_57_loop_binding_inline_completion_quality_stdio() -> Result<()> {
    if !binary_available() {
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file(ARRAY_LOOP_PATH, ARRAY_LOOP_SOURCE)?;
    harness.open_file(HASH_LOOP_PATH, HASH_LOOP_SOURCE)?;
    harness.open_file(ARRAY_PREFERENCE_PATH, ARRAY_PREFERENCE_SOURCE)?;
    harness.open_file(STATUS_LOOP_PATH, STATUS_LOOP_SOURCE)?;
    std::thread::sleep(Duration::from_millis(250));

    assert!(
        wait_for_inline_registration(&harness),
        "dynamic inline-completion registration was not observed"
    );

    let array_insert_texts = wait_for_loop_binding(
        &harness,
        ARRAY_LOOP_PATH,
        ARRAY_LOOP_SOURCE,
        "my $user (@users) {\n    \n}",
    )?;
    assert!(
        array_insert_texts.iter().any(|actual| actual == "my $user (@users) {\n    \n}"),
        "array loop binding did not include visible @users insert; actual: {array_insert_texts:?}"
    );
    assert_excludes_fragment(&array_insert_texts, PLACEHOLDER_INSERT, "array loop binding");

    let hash_insert_texts = wait_for_loop_binding(
        &harness,
        HASH_LOOP_PATH,
        HASH_LOOP_SOURCE,
        "my $id (keys %users_by_id) {\n    \n}",
    )?;
    assert!(
        hash_insert_texts.iter().any(|actual| actual == "my $id (keys %users_by_id) {\n    \n}"),
        "hash loop binding did not include visible %users_by_id key insert; actual: {hash_insert_texts:?}"
    );
    assert_excludes_fragment(&hash_insert_texts, PLACEHOLDER_INSERT, "hash loop binding");

    let array_preference_insert_texts = wait_for_loop_binding(
        &harness,
        ARRAY_PREFERENCE_PATH,
        ARRAY_PREFERENCE_SOURCE,
        "my $user (@users) {\n    \n}",
    )?;
    assert!(
        array_preference_insert_texts.iter().any(|actual| actual == "my $user (@users) {\n    \n}"),
        "array preference loop binding did not keep @users preferred; actual: {array_preference_insert_texts:?}"
    );
    assert_excludes_fragment(
        &array_preference_insert_texts,
        "keys %users_by_id",
        "array preference loop binding",
    );
    assert_excludes_fragment(
        &array_preference_insert_texts,
        PLACEHOLDER_INSERT,
        "array preference loop binding",
    );

    let status_insert_texts = wait_for_loop_binding(
        &harness,
        STATUS_LOOP_PATH,
        STATUS_LOOP_SOURCE,
        "my $item (@status) {\n    \n}",
    )?;
    assert!(
        status_insert_texts.iter().any(|actual| actual == "my $item (@status) {\n    \n}"),
        "status loop binding did not avoid unsafe singular trimming; actual: {status_insert_texts:?}"
    );
    assert_excludes_fragment(&status_insert_texts, "$statu", "status loop binding");
    assert_excludes_fragment(&status_insert_texts, PLACEHOLDER_INSERT, "status loop binding");

    harness.assert_no_crash();
    Ok(())
}

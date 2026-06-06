//! Scenario 58 - guard-condition inline-completion quality proof.
//!
//! This test exercises deterministic visible-scalar inline completion through
//! a real stdio LSP process. It verifies that guard clauses such as
//! `return unless ` and `next if ` use scalar facts already visible in the file
//! instead of falling back to placeholder conditions or unrelated result values.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const RETURN_GUARD_PATH: &str = "lib/Inline/ReturnGuard.pl";
const NEXT_GUARD_PATH: &str = "lib/Inline/NextGuard.pl";

const RETURN_GUARD_SOURCE: &str = r#"use strict;
use warnings;

sub handle_user {
    my $result = load_user();
    my $is_valid = validate_user($result);
    return unless "#;

const NEXT_GUARD_SOURCE: &str = r#"use strict;
use warnings;

sub active_users {
    my @users = fetch_users();
    for my $user (@users) {
        my $should_skip = should_skip_user($user);
        next if "#;

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
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

fn wait_for_guard_condition(
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

#[test]
fn scenario_58_guard_condition_inline_completion_quality_stdio() -> Result<()> {
    if !binary_available() {
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file(RETURN_GUARD_PATH, RETURN_GUARD_SOURCE)?;
    harness.open_file(NEXT_GUARD_PATH, NEXT_GUARD_SOURCE)?;
    std::thread::sleep(Duration::from_millis(250));

    assert!(
        wait_for_inline_registration(&harness),
        "dynamic inline-completion registration was not observed"
    );

    let return_guard_insert_texts =
        wait_for_guard_condition(&harness, RETURN_GUARD_PATH, RETURN_GUARD_SOURCE, "$is_valid;")?;
    assert!(
        return_guard_insert_texts.iter().any(|actual| actual == "$is_valid;"),
        "return guard did not include visible boolean lexical; actual: {return_guard_insert_texts:?}"
    );
    assert!(
        !return_guard_insert_texts.iter().any(|actual| actual == "$result;"),
        "return guard included forbidden generic result insert; actual: {return_guard_insert_texts:?}"
    );

    let next_guard_insert_texts =
        wait_for_guard_condition(&harness, NEXT_GUARD_PATH, NEXT_GUARD_SOURCE, "$should_skip;")?;
    assert!(
        next_guard_insert_texts.iter().any(|actual| actual == "$should_skip;"),
        "next guard did not include visible skip lexical; actual: {next_guard_insert_texts:?}"
    );
    assert!(
        !next_guard_insert_texts.iter().any(|actual| actual == "$user;"),
        "next guard included forbidden loop item insert; actual: {next_guard_insert_texts:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

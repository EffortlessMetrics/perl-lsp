//! Scenario 55 - DBI receiver inline-completion quality proof.
//!
//! This test exercises deterministic DBI receiver inline completion through a
//! real stdio LSP process. It records whether `$dbh->` and `$sth->f` ghost text
//! uses DBI handle methods instead of generic constructor guesses.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const DBI_HANDLE_PATH: &str = "lib/Inline/DbiHandle.pl";
const DBI_STATEMENT_PATH: &str = "lib/Inline/DbiStatement.pl";

const DBI_HANDLE_SOURCE: &str = r#"use strict;
use warnings;
use DBI;

my $dbh = DBI->connect($dsn);
$dbh->
"#;

const DBI_STATEMENT_SOURCE: &str = r#"use strict;
use warnings;
use DBI;

my $dbh = DBI->connect($dsn);
my $sth = $dbh->prepare($sql);
$sth->f
"#;

const DBI_HANDLE_LINE: u32 = 5;
const DBI_HANDLE_CHARACTER: u32 = 6;
const DBI_STATEMENT_LINE: u32 = 6;
const DBI_STATEMENT_CHARACTER: u32 = 7;
const EXPECTED_HANDLE_INSERTS: &[&str] = &["prepare()", "do()", "disconnect()"];
const EXPECTED_STATEMENT_INSERTS: &[&str] = &["fetchrow_hashref()", "fetchrow_array()", "finish()"];
const FORBIDDEN_INSERTS: &[&str] = &["new()"];

fn create_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig::default()
        .with_file(DBI_HANDLE_PATH, DBI_HANDLE_SOURCE)
        .with_file(DBI_STATEMENT_PATH, DBI_STATEMENT_SOURCE);
    config.client_capability_overrides = json!({
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    });

    UxHarness::new(config)
}

fn insert_texts_for(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get("insertText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn missing_expected<'a>(insert_texts: &[String], expected: &'a [&str]) -> Vec<&'a str> {
    expected
        .iter()
        .copied()
        .filter(|expected| !insert_texts.iter().any(|actual| actual == expected))
        .collect()
}

fn present_forbidden<'a>(insert_texts: &[String], forbidden: &'a [&str]) -> Vec<&'a str> {
    forbidden
        .iter()
        .copied()
        .filter(|forbidden| insert_texts.iter().any(|actual| actual == forbidden))
        .collect()
}

fn wait_for_expected_inserts(
    harness: &UxHarness,
    file: &str,
    line: u32,
    character: u32,
    expected: &[&str],
) -> Result<Vec<String>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let insert_texts = insert_texts_for(
            &harness.inline_completion_with_trigger_kind(file, line, character, 1)?,
        );
        if missing_expected(&insert_texts, expected).is_empty() || Instant::now() >= deadline {
            return Ok(insert_texts);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn scenario_55_dbi_receiver_inline_completion_quality_stdio() -> Result<()> {
    if !binary_available() {
        return Ok(());
    }

    let harness = create_harness()?;
    harness.open_file(DBI_HANDLE_PATH, DBI_HANDLE_SOURCE)?;
    harness.open_file(DBI_STATEMENT_PATH, DBI_STATEMENT_SOURCE)?;
    std::thread::sleep(Duration::from_millis(250));

    let handle_insert_texts = wait_for_expected_inserts(
        &harness,
        DBI_HANDLE_PATH,
        DBI_HANDLE_LINE,
        DBI_HANDLE_CHARACTER,
        EXPECTED_HANDLE_INSERTS,
    )?;
    let handle_missing = missing_expected(&handle_insert_texts, EXPECTED_HANDLE_INSERTS);
    let handle_forbidden = present_forbidden(&handle_insert_texts, FORBIDDEN_INSERTS);

    let statement_insert_texts = wait_for_expected_inserts(
        &harness,
        DBI_STATEMENT_PATH,
        DBI_STATEMENT_LINE,
        DBI_STATEMENT_CHARACTER,
        EXPECTED_STATEMENT_INSERTS,
    )?;
    let statement_missing = missing_expected(&statement_insert_texts, EXPECTED_STATEMENT_INSERTS);
    let statement_forbidden = present_forbidden(&statement_insert_texts, FORBIDDEN_INSERTS);

    assert!(
        !handle_insert_texts.is_empty(),
        "DBI database handle inline completion returned no candidates"
    );
    assert!(
        handle_missing.is_empty(),
        "DBI database handle inline completion missed expected methods: {handle_missing:?}; actual: {handle_insert_texts:?}"
    );
    assert!(
        handle_forbidden.is_empty(),
        "DBI database handle inline completion returned forbidden methods: {handle_forbidden:?}"
    );
    assert!(
        !statement_insert_texts.is_empty(),
        "DBI statement handle inline completion returned no candidates"
    );
    assert!(
        statement_missing.is_empty(),
        "DBI statement handle inline completion missed expected methods: {statement_missing:?}; actual: {statement_insert_texts:?}"
    );
    assert!(
        statement_forbidden.is_empty(),
        "DBI statement handle inline completion returned forbidden methods: {statement_forbidden:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

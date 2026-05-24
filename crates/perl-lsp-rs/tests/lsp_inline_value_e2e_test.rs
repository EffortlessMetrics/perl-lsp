//! End-to-end content tests for `textDocument/inlineValue`.
//!
//! The existing 3.17 test only asserts the response is `null | array`. These
//! tests drive the JSON-RPC handler end-to-end and verify the *content* of
//! returned inline values: variable detection across sigils (`$`, `@`, `%`),
//! UTF-16 range correctness, and `stoppedLocation` scoping.
//!
//! Contract being locked in (see runtime/language/misc.rs::handle_inline_value
//! and runtime/language/misc/inline_values.rs):
//! - Each item is `InlineValueVariableLookup` with `range`, `variableName`,
//!   `caseSensitiveLookup`.
//! - `variableName` includes the sigil (e.g. "$x", "@arr", "%h").
//! - `stoppedLocation.end.line` truncates the effective range when present.

mod support;

use perl_tdd_support::must_some;
use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request_inline_values(
    harness: &mut LspHarness,
    uri: &str,
    start_line: u32,
    end_line: u32,
    stopped_end_line: Option<u32>,
) -> Result<Vec<Value>, String> {
    let mut params = json!({
        "textDocument": { "uri": uri },
        "range": {
            "start": { "line": start_line, "character": 0 },
            "end":   { "line": end_line,   "character": 0 }
        },
        "context": {
            "frameId": 1
        }
    });
    if let Some(stop_line) = stopped_end_line {
        params["context"]["stoppedLocation"] = json!({
            "start": { "line": stop_line, "character": 0 },
            "end":   { "line": stop_line, "character": 0 }
        });
    }

    let response = harness.request("textDocument/inlineValue", params)?;
    if response.is_null() {
        return Ok(Vec::new());
    }
    let arr = response.as_array().ok_or_else(|| format!("expected array, got {response}"))?.clone();
    Ok(arr)
}

fn variable_names(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|i| i.get("variableName").and_then(Value::as_str)).collect()
}

fn assert_inline_value_shape(item: &Value) {
    let range = must_some(item.get("range"));
    let start = must_some(range.get("start"));
    let end = must_some(range.get("end"));
    assert!(start.get("line").is_some_and(Value::is_number), "missing start.line: {item}");
    assert!(
        start.get("character").is_some_and(Value::is_number),
        "missing start.character: {item}"
    );
    assert!(end.get("line").is_some_and(Value::is_number), "missing end.line: {item}");
    assert!(end.get("character").is_some_and(Value::is_number), "missing end.character: {item}");

    let var_name = must_some(item.get("variableName").and_then(Value::as_str));
    let first = must_some(var_name.chars().next());
    assert!(
        matches!(first, '$' | '@' | '%'),
        "variableName must start with a Perl sigil, got {var_name:?}"
    );

    assert_eq!(
        item.get("caseSensitiveLookup").and_then(Value::as_bool),
        Some(true),
        "Perl is case-sensitive - caseSensitiveLookup must be true: {item}"
    );
}

#[test]
fn inline_value_detects_scalar_variables() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_scalar.pl";
    harness.open(uri, "my $count = 5;\nprint $count;\n")?;

    let items = request_inline_values(&mut harness, uri, 0, 1, None)?;
    assert!(!items.is_empty(), "expected scalar variables to be detected");
    for item in &items {
        assert_inline_value_shape(item);
    }

    let names = variable_names(&items);
    let scalar_hits = names.iter().filter(|n| **n == "$count").count();
    assert!(scalar_hits >= 2, "expected both $count occurrences, got names={names:?}");

    Ok(())
}

#[test]
fn inline_value_detects_array_variables() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_array.pl";
    harness.open(uri, "my @items = (1, 2, 3);\nprint scalar(@items);\n")?;

    let items = request_inline_values(&mut harness, uri, 0, 1, None)?;
    for item in &items {
        assert_inline_value_shape(item);
    }

    let names = variable_names(&items);
    assert!(
        names.contains(&"@items"),
        "expected @items to be reported with @ sigil, got names={names:?}"
    );

    Ok(())
}

#[test]
fn inline_value_detects_hash_variables() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_hash.pl";
    harness.open(uri, "my %config = (key => 'val');\nprint keys %config;\n")?;

    let items = request_inline_values(&mut harness, uri, 0, 1, None)?;
    for item in &items {
        assert_inline_value_shape(item);
    }

    let names = variable_names(&items);
    assert!(
        names.contains(&"%config"),
        "expected %config to be reported with % sigil, got names={names:?}"
    );

    Ok(())
}

#[test]
fn inline_value_detects_multiple_variables_on_same_line() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_multi.pl";
    harness.open(uri, "my ($a, $b, $c) = (1, 2, 3);\n")?;

    let items = request_inline_values(&mut harness, uri, 0, 0, None)?;
    for item in &items {
        assert_inline_value_shape(item);
    }

    let names = variable_names(&items);
    for v in ["$a", "$b", "$c"] {
        assert!(names.contains(&v), "expected {v} on multi-variable line, got names={names:?}");
    }

    Ok(())
}

#[test]
fn inline_value_stopped_location_truncates_effective_range() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_stop.pl";
    harness.open(
        uri,
        "my $first = 1;\n\
         my $second = 2;\n\
         my $third = 3;\n",
    )?;

    // Request spans lines 0..=2, but stoppedLocation caps effective range at line 0.
    let items = request_inline_values(&mut harness, uri, 0, 2, Some(0))?;
    for item in &items {
        assert_inline_value_shape(item);
    }

    let names = variable_names(&items);
    assert!(names.contains(&"$first"), "expected $first within stopped line, got names={names:?}");
    assert!(
        !names.contains(&"$second"),
        "expected $second to be excluded by stoppedLocation cap, got names={names:?}"
    );
    assert!(
        !names.contains(&"$third"),
        "expected $third to be excluded by stoppedLocation cap, got names={names:?}"
    );

    Ok(())
}

#[test]
fn inline_value_ranges_align_with_variable_columns() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///inline_value_range.pl";
    let line = "my $value = 99;";
    harness.open(uri, &format!("{line}\n"))?;

    let items = request_inline_values(&mut harness, uri, 0, 0, None)?;
    let value_item = must_some(
        items.iter().find(|i| i.get("variableName").and_then(Value::as_str) == Some("$value")),
    );

    let range = must_some(value_item.get("range"));
    let start = must_some(range.get("start"));
    let end = must_some(range.get("end"));
    let start_line = must_some(start.get("line").and_then(Value::as_u64));
    let start_char = must_some(start.get("character").and_then(Value::as_u64));
    let end_line = must_some(end.get("line").and_then(Value::as_u64));
    let end_char = must_some(end.get("character").and_then(Value::as_u64));

    assert_eq!(start_line, 0, "$value is on line 0");
    assert_eq!(end_line, 0, "$value range ends on line 0");

    // "my $value = 99;" - `$value` spans columns 3..9 (sigil included, 6 UTF-16 units).
    let expected_start = line.find('$').map(|i| i as u64).unwrap_or(0);
    assert_eq!(start_char, expected_start, "$value start column mismatch");
    assert_eq!(end_char, expected_start + 6, "$value end column should cover sigil+name");

    Ok(())
}

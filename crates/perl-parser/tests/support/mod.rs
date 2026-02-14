//! Shared test utilities for LSP integration tests
//!
//! Provides robust assertion helpers and utilities for testing LSP functionality

#![allow(clippy::collapsible_if)]

pub mod client_caps;
pub mod env_guard;
pub mod parser_error_helpers;

#[cfg(feature = "incremental")]
pub mod incremental_test_utils;

use serde_json::Value;
use std::time::{Duration, Instant};
use perl_tdd_support::{must, must_some};

// ===================== Constants =====================

/// Default timeout for async operations
#[allow(dead_code)]
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1500);

/// Default polling interval
#[allow(dead_code)]
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(50);

// ===================== UTF-16 Helpers (for LSP) =====================

/// Convert byte offset to UTF-16 column position (for LSP)
#[allow(dead_code)]
pub fn utf16_col(s: &str, byte_off: usize) -> u32 {
    let prefix = &s[..byte_off.min(s.len())];
    prefix.encode_utf16().count() as u32
}

/// Calculate line and UTF-16 column from byte offset in source
#[allow(dead_code)]
pub fn position_at(source: &str, offset: usize) -> (u32, u32) {
    let lines: Vec<&str> = source.lines().collect();
    let mut bytes_so_far = 0;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if bytes_so_far + line_len > offset {
            let col_bytes = offset - bytes_so_far;
            let col_utf16 = utf16_col(line, col_bytes);
            return (line_idx as u32, col_utf16);
        }
        bytes_so_far += line_len;
    }

    // If we're at the end, return the last position
    let last_line_idx = lines.len().saturating_sub(1);
    let last_line = lines.get(last_line_idx).copied().unwrap_or("");
    (last_line_idx as u32, utf16_col(last_line, last_line.len()))
}

// ===================== Text Edit Helpers =====================

/// Apply a list of text edits to a document
/// Edits are applied from end to start to avoid position shifts
#[allow(dead_code)]
pub fn apply_text_edits(text: &str, edits: &[Value]) -> String {
    if edits.is_empty() {
        return text.to_string();
    }

    // Sort edits by start position (reverse order for applying)
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|a, b| {
        let a_start = &a["range"]["start"];
        let b_start = &b["range"]["start"];

        let a_line = a_start["line"].as_u64().unwrap_or(0);
        let b_line = b_start["line"].as_u64().unwrap_or(0);

        match b_line.cmp(&a_line) {
            std::cmp::Ordering::Equal => {
                let a_char = a_start["character"].as_u64().unwrap_or(0);
                let b_char = b_start["character"].as_u64().unwrap_or(0);
                b_char.cmp(&a_char)
            }
            other => other,
        }
    });

    let mut result = text.to_string();
    let lines: Vec<&str> = text.lines().collect();

    for edit in sorted_edits {
        let range = &edit["range"];
        let new_text = edit["newText"].as_str().unwrap_or("");

        let start_line = range["start"]["line"].as_u64().unwrap_or(0) as usize;
        let start_char = range["start"]["character"].as_u64().unwrap_or(0) as usize;
        let end_line = range["end"]["line"].as_u64().unwrap_or(0) as usize;
        let end_char = range["end"]["character"].as_u64().unwrap_or(0) as usize;

        // Convert UTF-16 positions to byte offsets
        let start_offset = line_col_to_offset(&lines, start_line, start_char);
        let end_offset = line_col_to_offset(&lines, end_line, end_char);

        // Apply the edit
        result.replace_range(start_offset..end_offset, new_text);
    }

    result
}

/// Convert line/column (UTF-16) to byte offset
fn line_col_to_offset(lines: &[&str], line: usize, col_utf16: usize) -> usize {
    let mut offset = 0;

    for line_str in lines.iter().take(line) {
        offset += line_str.len() + 1; // +1 for newline
    }

    if line < lines.len() {
        // Convert UTF-16 column to byte offset
        let line_str = lines[line];
        let mut byte_offset = 0;
        let mut utf16_count = 0;

        for ch in line_str.chars() {
            if utf16_count >= col_utf16 {
                break;
            }
            byte_offset += ch.len_utf8();
            utf16_count += ch.len_utf16();
        }

        offset += byte_offset;
    }

    offset
}

// ===================== Extraction Helpers =====================

/// Extract object from optional value with meaningful error
#[allow(dead_code)]
pub fn expect_obj(v: &Option<Value>) -> &serde_json::Map<String, Value> {
    use perl_tdd_support::must_some;
    let val = must_some(v.as_ref());
    let obj = val.as_object();
    assert!(obj.is_some(), "Expected JSON object, got: {:?}", val);
    obj.unwrap_or_else(|| unreachable!())
}

/// Extract array from optional value with meaningful error
#[allow(dead_code)]
pub fn expect_arr(v: &Option<Value>) -> &Vec<Value> {
    use perl_tdd_support::must_some;
    let val = must_some(v.as_ref());
    let arr = val.as_array();
    assert!(arr.is_some(), "Expected JSON array, got: {:?}", val);
    arr.unwrap_or_else(|| unreachable!())
}

// ===================== Assertion Helpers =====================

/// Assert hover response has meaningful text content
#[allow(dead_code)]
pub fn assert_hover_has_text(v: &Option<Value>) {
    if let Some(hover) = v {
        if !hover.is_null() {
            let obj_opt = hover.as_object();
            assert!(obj_opt.is_some(), "Hover should be object, got: {:?}", hover);
            let obj = obj_opt.unwrap_or_else(|| unreachable!());
            assert!(obj.contains_key("contents"), "hover must have contents field");

            let contents = &obj["contents"];
            let has_text = contents.is_string()
                || contents.get("value").and_then(|s| s.as_str()).is_some()
                || contents.get("kind").is_some();
            assert!(has_text, "hover must include text/markdown content");

            // Optional: check range if present
            if let Some(range) = obj.get("range") {
                assert_range_valid(range, "hover range");
            }
        }
    }
}

/// Assert completion response has items with proper structure
#[allow(dead_code)]
pub fn assert_completion_has_items(v: &Option<Value>) {
    if let Some(comp) = v {
        if !comp.is_null() {
            let items = if let Some(arr) = comp.as_array() {
                arr
            } else if let Some(obj) = comp.as_object() {
                let items_opt = obj.get("items").and_then(|v| v.as_array());
                assert!(items_opt.is_some(), "Completion object must have items array, got: {:?}", obj);
                items_opt.unwrap_or_else(|| unreachable!())
            } else {
                unreachable!("Completion response must be array or object with items, got: {:?}", comp);
            };

            assert!(!items.is_empty(), "completion must return at least one item");

            // Validate first item has required fields
            if let Some(first) = items.first() {
                let item_opt = first.as_object();
                assert!(item_opt.is_some(), "Completion item must be object, got: {:?}", first);
                let item = item_opt.unwrap_or_else(|| unreachable!());
                assert!(item.contains_key("label"), "completion item must have label");
            }
        }
    }
}

/// Assert rename has actual edits with validation
#[allow(dead_code)]
pub fn assert_rename_has_edits(v: &Option<Value>) {
    let obj = expect_obj(v);

    // Check for either changes or documentChanges per LSP spec
    let has_changes = obj
        .get("changes")
        .and_then(|c| c.as_object())
        .map(|changes| {
            // Ensure at least one file has edits
            changes
                .values()
                .any(|edits| edits.as_array().map(|arr| !arr.is_empty()).unwrap_or(false))
        })
        .unwrap_or(false);

    let has_doc_changes = obj
        .get("documentChanges")
        .and_then(|dc| dc.as_array())
        .map(|changes| {
            // Ensure at least one document change with edits
            changes.iter().any(|change| {
                change
                    .get("edits")
                    .and_then(|e| e.as_array())
                    .map(|arr| !arr.is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    assert!(has_changes || has_doc_changes, "rename returned no edits");
}

/// Assert references are found with validation
#[allow(dead_code)]
pub fn assert_references_found(v: &Option<Value>) {
    assert_references_found_with_min(v, None);
}

/// Assert references are found with minimum count validation
#[allow(dead_code)]
pub fn assert_references_found_with_min(v: &Option<Value>, min_refs: Option<usize>) {
    if let Some(refs_val) = v {
        if !refs_val.is_null() {
            let refs_opt = refs_val.as_array();
            assert!(refs_opt.is_some(), "References should be array, got: {:?}", refs_val);
            let refs = refs_opt.unwrap_or_else(|| unreachable!());

            if let Some(min) = min_refs {
                assert!(
                    refs.len() >= min,
                    "expected at least {} references, found {}",
                    min,
                    refs.len()
                );
            }

            for reference in refs {
                let ref_obj_opt = reference.as_object();
                assert!(ref_obj_opt.is_some(), "Reference must be object, got: {:?}", reference);
                let ref_obj = ref_obj_opt.unwrap_or_else(|| unreachable!());
                assert!(ref_obj.contains_key("uri"), "reference must have uri");
                assert!(ref_obj.contains_key("range"), "reference must have range");
                assert_range_valid(&ref_obj["range"], "reference range");
            }
        }
    }
}

/// Assert call hierarchy has items with proper structure
#[allow(dead_code)]
pub fn assert_call_hierarchy_items(v: &Option<Value>, expected_name: Option<&str>) {
    if let Some(ch_val) = v {
        if !ch_val.is_null() {
            let items_opt = ch_val.as_array();
            assert!(items_opt.is_some(), "Call hierarchy should be array, got: {:?}", ch_val);
            let items = items_opt.unwrap_or_else(|| unreachable!());

            if !items.is_empty() {
                for item in items {
                    let item_obj_opt = item.as_object();
                    assert!(item_obj_opt.is_some(), "Call hierarchy item must be object, got: {:?}", item);
                    let item_obj = item_obj_opt.unwrap_or_else(|| unreachable!());
                    assert!(item_obj.contains_key("name"), "call hierarchy item must have name");
                    assert!(item_obj.contains_key("uri"), "call hierarchy item must have uri");
                    assert!(item_obj.contains_key("range"), "call hierarchy item must have range");

                    // Either selectionRange or detail should be present
                    let has_selection = item_obj.contains_key("selectionRange");
                    let has_detail = item_obj.contains_key("detail");
                    assert!(
                        has_selection || has_detail,
                        "call hierarchy item must have selectionRange or detail"
                    );
                }

                // Check for expected name if provided
                if let Some(name) = expected_name {
                    let found = items.iter().any(|item| {
                        item.get("name")
                            .and_then(|n| n.as_str())
                            .map(|n| n == name)
                            .unwrap_or(false)
                    });
                    assert!(found, "call hierarchy should contain '{}'", name);
                }
            }
        }
    }
}

/// Assert folding ranges are valid
#[allow(dead_code)]
pub fn assert_folding_ranges_valid(v: &Option<Value>) {
    let ranges = expect_arr(v);
    assert!(!ranges.is_empty(), "should have at least one folding range");

    for range in ranges {
        let obj_opt = range.as_object();
        assert!(obj_opt.is_some(), "Folding range must be object, got: {:?}", range);
        let obj = obj_opt.unwrap_or_else(|| unreachable!());

        let start_opt = obj.get("startLine").and_then(|v| v.as_u64());
        assert!(start_opt.is_some(), "Folding range must have startLine, got: {:?}", obj);
        let start = start_opt.unwrap_or_else(|| unreachable!());

        let end_opt = obj.get("endLine").and_then(|v| v.as_u64());
        assert!(end_opt.is_some(), "Folding range must have endLine, got: {:?}", obj);
        let end = end_opt.unwrap_or_else(|| unreachable!());

        assert!(end > start, "folding range must span multiple lines");

        // Optional: check character positions if present
        if let Some(start_char) = obj.get("startCharacter") {
            assert!(start_char.is_u64(), "startCharacter must be number");
        }
        if let Some(end_char) = obj.get("endCharacter") {
            assert!(end_char.is_u64(), "endCharacter must be number");
        }
    }
}

/// Assert code actions are available with validation
#[allow(dead_code)]
pub fn assert_code_actions_available(v: &Option<Value>) {
    if let Some(actions) = v {
        if !actions.is_null() {
            let arr_opt = actions.as_array();
            assert!(arr_opt.is_some(), "Code actions should be array, got: {:?}", actions);
            let arr = arr_opt.unwrap_or_else(|| unreachable!());

            for action in arr {
                let action_obj_opt = action.as_object();
                assert!(action_obj_opt.is_some(), "Code action must be object, got: {:?}", action);
                let action_obj = action_obj_opt.unwrap_or_else(|| unreachable!());
                assert!(action_obj.contains_key("title"), "code action must have title");

                // Must have either command or edit
                let has_command = action_obj.contains_key("command");
                let has_edit = action_obj.contains_key("edit");
                assert!(has_command || has_edit, "code action must have command or edit");

                // If has kind, validate it's a string
                if let Some(kind) = action_obj.get("kind") {
                    assert!(kind.is_string(), "code action kind must be string");
                }
            }
        }
    }
}

/// Assert workspace symbols have proper structure
#[allow(dead_code)]
pub fn assert_workspace_symbols_valid(v: &Option<Value>, expected_name: Option<&str>) {
    if let Some(symbols) = v {
        if !symbols.is_null() {
            let arr_opt = symbols.as_array();
            assert!(arr_opt.is_some(), "Workspace symbols should be array, got: {:?}", symbols);
            let arr = arr_opt.unwrap_or_else(|| unreachable!());

            if !arr.is_empty() {
                // Validate each symbol
                for symbol in arr {
                    let sym_obj_opt = symbol.as_object();
                    assert!(sym_obj_opt.is_some(), "Workspace symbol must be object, got: {:?}", symbol);
                    let sym_obj = sym_obj_opt.unwrap_or_else(|| unreachable!());
                    assert!(sym_obj.contains_key("name"), "workspace symbol must have name");

                    // Must have either location or containerName
                    let has_location = sym_obj.contains_key("location");
                    let has_container = sym_obj.contains_key("containerName");
                    assert!(
                        has_location || has_container,
                        "workspace symbol must have location or containerName"
                    );

                    if let Some(loc) = sym_obj.get("location") {
                        assert_location_valid(loc, "workspace symbol location");
                    }
                }

                // Check for expected name if provided
                if let Some(name) = expected_name {
                    let found = arr.iter().any(|s| {
                        s.get("name")
                            .and_then(|n| n.as_str())
                            .map(|n| n.contains(name))
                            .unwrap_or(false)
                    });
                    assert!(found, "Should find {}-related symbols", name);
                }
            }
        }
    }
}

// ===================== Validation Helpers =====================

/// Validate a range object
fn assert_range_valid(range: &Value, context: &str) {
    let range_obj_opt = range.as_object();
    assert!(range_obj_opt.is_some(), "{} must be object, got: {:?}", context, range);
    let range_obj = range_obj_opt.unwrap_or_else(|| unreachable!());

    // Check start position
    let start_val = range_obj.get("start");
    assert!(start_val.is_some(), "{} must have start, got: {:?}", context, range_obj);
    let start = start_val.unwrap_or_else(|| unreachable!());

    let start_obj_opt = start.as_object();
    assert!(start_obj_opt.is_some(), "{} start must be object, got: {:?}", context, start);
    let start_obj = start_obj_opt.unwrap_or_else(|| unreachable!());
    assert!(
        start_obj.get("line").and_then(|v| v.as_u64()).is_some(),
        "{} start must have line number",
        context
    );
    assert!(
        start_obj.get("character").and_then(|v| v.as_u64()).is_some(),
        "{} start must have character",
        context
    );

    // Check end position
    let end_val = range_obj.get("end");
    assert!(end_val.is_some(), "{} must have end, got: {:?}", context, range_obj);
    let end = end_val.unwrap_or_else(|| unreachable!());

    let end_obj_opt = end.as_object();
    assert!(end_obj_opt.is_some(), "{} end must be object, got: {:?}", context, end);
    let end_obj = end_obj_opt.unwrap_or_else(|| unreachable!());
    assert!(
        end_obj.get("line").and_then(|v| v.as_u64()).is_some(),
        "{} end must have line number",
        context
    );
    assert!(
        end_obj.get("character").and_then(|v| v.as_u64()).is_some(),
        "{} end must have character",
        context
    );
}

/// Validate a location object
#[allow(dead_code)]
fn assert_location_valid(location: &Value, context: &str) {
    let loc_obj_opt = location.as_object();
    assert!(loc_obj_opt.is_some(), "{} must be object, got: {:?}", context, location);
    let loc_obj = loc_obj_opt.unwrap_or_else(|| unreachable!());
    assert!(loc_obj.contains_key("uri"), "{} must have uri", context);
    assert!(
        loc_obj.get("uri").and_then(|v| v.as_str()).is_some(),
        "{} uri must be string",
        context
    );

    let range_val = loc_obj.get("range");
    assert!(range_val.is_some(), "{} must have range, got: {:?}", context, loc_obj);
    if let Some(range) = range_val {
        assert_range_valid(range, &format!("{} range", context));
    }
}

// ===================== Async Helpers =====================

// Note: Async version would require tokio, but we don't need it for these tests
// Use wait_for_sync instead

/// Synchronous wait for condition (for non-async tests)
#[allow(dead_code)]
pub fn wait_for_sync<F>(
    mut condition: F,
    timeout: Option<Duration>,
    poll_interval: Option<Duration>,
    description: &str,
) -> bool
where
    F: FnMut() -> bool,
{
    let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
    let poll_interval = poll_interval.unwrap_or(DEFAULT_POLL_INTERVAL);
    let start = Instant::now();

    loop {
        if condition() {
            let elapsed = start.elapsed();
            if elapsed > Duration::from_millis(100) {
                eprintln!("✓ {} completed in {:?}", description, elapsed);
            }
            return true;
        }

        if start.elapsed() >= timeout {
            eprintln!("✗ {} timed out after {:?}", description, timeout);
            return false;
        }

        std::thread::sleep(poll_interval);
    }
}

// ===================== Test Macros =====================

/// Macro for consistent assertion failure messages
#[macro_export]
macro_rules! assert_lsp {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            must(Err::<(), _>(format!("LSP assertion failed: {}", format!($($arg)*))));
        }
    };
}

/// Macro for optional assertions (warnings in dev, failures in CI)
#[macro_export]
macro_rules! assert_lsp_optional {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            if std::env::var("CI").is_ok() {
                must(Err::<(), _>(format!("LSP assertion failed in CI: {}", format!($($arg)*))));
            } else {
                eprintln!("⚠ LSP warning: {}", format!($($arg)*));
            }
        }
    };
}

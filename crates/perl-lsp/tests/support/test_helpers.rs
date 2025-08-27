//! Test helper functions for LSP test assertions
//!
//! This module provides specialized assertion helpers for validating LSP responses.
//! These functions are designed to be used by tests that need deep validation
//! of LSP protocol structures.
//!
//! ## Usage
//!
//! Import specific functions you need:
//! ```rust,ignore
//! use support::test_helpers::assert_hover_has_text;
//! ```
//!
//! Or import all helpers:
//! ```rust,ignore
//! use support::test_helpers::*;
//! ```

#![allow(dead_code)] // Test infrastructure - functions may not be used by all tests

use serde_json::Value;

/// Assert hover response has text content
pub fn assert_hover_has_text(v: &Option<Value>) {
    if let Some(hover) = v {
        if !hover.is_null() {
            let obj = hover.as_object().expect("hover should be object");
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
pub fn assert_completion_has_items(v: &Option<Value>) {
    if let Some(completion) = v {
        if !completion.is_null() {
            let array = completion.as_array().expect("completion should be array");
            assert!(!array.is_empty(), "completion must have at least one item");

            for item in array {
                let obj = item.as_object().expect("completion item should be object");
                assert!(obj.contains_key("label"), "completion item must have label");

                // Optional: check other fields if present
                if let Some(kind) = obj.get("kind") {
                    assert!(kind.is_number(), "completion kind must be number");
                }
                if let Some(detail) = obj.get("detail") {
                    assert!(detail.is_string(), "completion detail must be string");
                }
            }
        }
    }
}

/// Validate LSP range structure
fn assert_range_valid(range: &Value, context: &str) {
    let obj = range.as_object().expect("range should be object");
    assert!(obj.contains_key("start"), "{} must have start", context);
    assert!(obj.contains_key("end"), "{} must have end", context);

    let start = &obj["start"];
    let end = &obj["end"];

    assert_position_valid(start, &format!("{} start", context));
    assert_position_valid(end, &format!("{} end", context));
}

/// Validate LSP position structure
fn assert_position_valid(position: &Value, context: &str) {
    let obj = position.as_object().expect("position should be object");
    assert!(obj.contains_key("line"), "{} must have line", context);
    assert!(obj.contains_key("character"), "{} must have character", context);

    if let Some(line) = obj.get("line") {
        assert!(line.is_number(), "{} line must be number", context);
        let line_num = line.as_u64().expect("line should be u64");
        assert!(line_num < 1000000, "{} line number should be reasonable", context);
    } else {
        panic!("{} must have line", context);
    }

    if let Some(character) = obj.get("character") {
        assert!(character.is_number(), "{} character must be number", context);
        let char_num = character.as_u64().expect("character should be u64");
        assert!(char_num < 10000, "{} character should be reasonable", context);
    } else {
        panic!("{} must have character", context);
    }
}

/// Assert references are found with validation
pub fn assert_references_found(v: &Option<Value>) {
    assert_references_found_with_min(v, None);
}

/// Assert references are found with minimum count validation
pub fn assert_references_found_with_min(v: &Option<Value>, min_refs: Option<usize>) {
    if let Some(refs_val) = v {
        if !refs_val.is_null() {
            let refs = refs_val.as_array().expect("references should be array");

            if let Some(min) = min_refs {
                assert!(
                    refs.len() >= min,
                    "expected at least {} references, found {}",
                    min,
                    refs.len()
                );
            }

            // Validate each reference has required fields
            for reference in refs {
                let ref_obj = reference.as_object().expect("reference must be object");
                assert!(ref_obj.contains_key("uri"), "reference must have uri");
                assert!(ref_obj.contains_key("range"), "reference must have range");
                assert_range_valid(&ref_obj["range"], "reference range");
            }
        }
    }
}

/// Assert call hierarchy has items with proper structure
pub fn assert_call_hierarchy_items(v: &Option<Value>, expected_name: Option<&str>) {
    if let Some(ch_val) = v {
        if !ch_val.is_null() {
            let items = ch_val.as_array().expect("call hierarchy should be array");

            if !items.is_empty() {
                // Validate each item has required fields
                for item in items {
                    let item_obj = item.as_object().expect("call hierarchy item must be object");
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
pub fn assert_folding_ranges_valid(v: &Option<Value>) {
    if let Some(ranges_val) = v {
        if !ranges_val.is_null() {
            let ranges = ranges_val.as_array().expect("folding ranges should be array");
            assert!(!ranges.is_empty(), "should have at least one folding range");

            for range in ranges {
                let obj = range.as_object().expect("folding range must be object");

                let start = obj
                    .get("startLine")
                    .and_then(|v| v.as_u64())
                    .expect("folding range must have startLine");

                let end = obj
                    .get("endLine")
                    .and_then(|v| v.as_u64())
                    .expect("folding range must have endLine");

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
    }
}

/// Assert code actions are available with validation
pub fn assert_code_actions_available(v: &Option<Value>) {
    if let Some(actions) = v {
        if !actions.is_null() {
            let arr = actions.as_array().expect("code actions should be array");

            for action in arr {
                let action_obj = action.as_object().expect("code action must be object");
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

/// Apply a list of text edits to a document
/// Edits are applied from end to start to avoid position shifts
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

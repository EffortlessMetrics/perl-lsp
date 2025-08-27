//! Test helper functions for LSP test assertions

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
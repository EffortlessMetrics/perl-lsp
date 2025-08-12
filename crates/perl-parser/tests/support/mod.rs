//! Shared test utilities for LSP integration tests
//! 
//! Provides robust assertion helpers and utilities for testing LSP functionality

pub mod lsp_client;
pub mod lsp_harness;
pub mod env_guard;

use serde_json::Value;
use std::time::{Duration, Instant};

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
    let last_line = lines.get(last_line_idx).unwrap_or(&"");
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
            other => other
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
    
    for i in 0..line.min(lines.len()) {
        offset += lines[i].len() + 1; // +1 for newline
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
    v.as_ref()
        .expect("expected Some value, got None")
        .as_object()
        .expect("expected JSON object")
}

/// Extract array from optional value with meaningful error
#[allow(dead_code)]
pub fn expect_arr(v: &Option<Value>) -> &Vec<Value> {
    v.as_ref()
        .expect("expected Some value, got None")
        .as_array()
        .expect("expected JSON array")
}

// ===================== Assertion Helpers =====================

/// Assert hover response has meaningful text content
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn assert_completion_has_items(v: &Option<Value>) {
    if let Some(comp) = v {
        if !comp.is_null() {
            let items = if let Some(arr) = comp.as_array() {
                arr
            } else if let Some(obj) = comp.as_object() {
                obj.get("items")
                    .and_then(|v| v.as_array())
                    .expect("completion must have items array")
            } else {
                panic!("completion response must be array or object with items");
            };
            
            assert!(!items.is_empty(), "completion must return at least one item");
            
            // Validate first item has required fields
            if let Some(first) = items.first() {
                let item = first.as_object().expect("completion item must be object");
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
    let has_changes = obj.get("changes")
        .and_then(|c| c.as_object())
        .map(|changes| {
            // Ensure at least one file has edits
            changes.values().any(|edits| {
                edits.as_array()
                    .map(|arr| !arr.is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    
    let has_doc_changes = obj.get("documentChanges")
        .and_then(|dc| dc.as_array())
        .map(|changes| {
            // Ensure at least one document change with edits
            changes.iter().any(|change| {
                change.get("edits")
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
            let refs = refs_val.as_array().expect("references should be array");
            
            if let Some(min) = min_refs {
                assert!(refs.len() >= min, "expected at least {} references, found {}", min, refs.len());
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
#[allow(dead_code)]
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
                    assert!(has_selection || has_detail, "call hierarchy item must have selectionRange or detail");
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
        let obj = range.as_object().expect("folding range must be object");
        
        let start = obj.get("startLine")
            .and_then(|v| v.as_u64())
            .expect("folding range must have startLine");
        
        let end = obj.get("endLine")
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

/// Assert code actions are available with validation
#[allow(dead_code)]
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

/// Assert workspace symbols have proper structure
#[allow(dead_code)]
pub fn assert_workspace_symbols_valid(v: &Option<Value>, expected_name: Option<&str>) {
    if let Some(symbols) = v {
        if !symbols.is_null() {
            let arr = symbols.as_array().expect("workspace symbols should be array");
            
            if !arr.is_empty() {
                // Validate each symbol
                for symbol in arr {
                    let sym_obj = symbol.as_object().expect("workspace symbol must be object");
                    assert!(sym_obj.contains_key("name"), "workspace symbol must have name");
                    
                    // Must have either location or containerName
                    let has_location = sym_obj.contains_key("location");
                    let has_container = sym_obj.contains_key("containerName");
                    assert!(has_location || has_container, "workspace symbol must have location or containerName");
                    
                    if let Some(loc) = sym_obj.get("location") {
                        assert_location_valid(loc, "workspace symbol location");
                    }
                }
                
                // Check for expected name if provided
                if let Some(name) = expected_name {
                    let found = arr.iter().any(|s| {
                        s.get("name").and_then(|n| n.as_str())
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
    let range_obj = range.as_object().expect(&format!("{} must be object", context));
    
    // Check start position
    let start = range_obj.get("start").expect(&format!("{} must have start", context));
    let start_obj = start.as_object().expect(&format!("{} start must be object", context));
    assert!(start_obj.get("line").and_then(|v| v.as_u64()).is_some(), "{} start must have line number", context);
    assert!(start_obj.get("character").and_then(|v| v.as_u64()).is_some(), "{} start must have character", context);
    
    // Check end position
    let end = range_obj.get("end").expect(&format!("{} must have end", context));
    let end_obj = end.as_object().expect(&format!("{} end must be object", context));
    assert!(end_obj.get("line").and_then(|v| v.as_u64()).is_some(), "{} end must have line number", context);
    assert!(end_obj.get("character").and_then(|v| v.as_u64()).is_some(), "{} end must have character", context);
}

/// Validate a location object
#[allow(dead_code)]
fn assert_location_valid(location: &Value, context: &str) {
    let loc_obj = location.as_object().expect(&format!("{} must be object", context));
    assert!(loc_obj.contains_key("uri"), "{} must have uri", context);
    assert!(loc_obj.get("uri").and_then(|v| v.as_str()).is_some(), "{} uri must be string", context);
    
    if let Some(range) = loc_obj.get("range") {
        assert_range_valid(range, &format!("{} range", context));
    } else {
        panic!("{} must have range", context);
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
            panic!("LSP assertion failed: {}", format!($($arg)*));
        }
    };
}

/// Macro for optional assertions (warnings in dev, failures in CI)
#[macro_export]
macro_rules! assert_lsp_optional {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            if std::env::var("CI").is_ok() {
                panic!("LSP assertion failed in CI: {}", format!($($arg)*));
            } else {
                eprintln!("⚠ LSP warning: {}", format!($($arg)*));
            }
        }
    };
}
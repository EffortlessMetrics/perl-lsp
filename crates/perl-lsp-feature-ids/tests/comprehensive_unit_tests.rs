//! Comprehensive unit tests for the `perl-lsp-feature-ids` crate.
//!
//! Validates every public constant's value, naming conventions, prefix
//! consistency, and cross-constant uniqueness.

use perl_lsp_feature_ids::*;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Helper: collect every public feature ID into a slice for batch assertions
// ---------------------------------------------------------------------------

fn all_lsp_ids() -> Vec<(&'static str, &'static str)> {
    vec![
        ("LSP_COMPLETION", LSP_COMPLETION),
        ("LSP_HOVER", LSP_HOVER),
        ("LSP_SIGNATURE_HELP", LSP_SIGNATURE_HELP),
        ("LSP_DEFINITION", LSP_DEFINITION),
        ("LSP_DECLARATION", LSP_DECLARATION),
        ("LSP_EXECUTE_COMMAND", LSP_EXECUTE_COMMAND),
        ("LSP_TYPE_DEFINITION", LSP_TYPE_DEFINITION),
        ("LSP_IMPLEMENTATION", LSP_IMPLEMENTATION),
        ("LSP_REFERENCES", LSP_REFERENCES),
        ("LSP_DOCUMENT_SYMBOL", LSP_DOCUMENT_SYMBOL),
        ("LSP_WORKSPACE_SYMBOL", LSP_WORKSPACE_SYMBOL),
        ("LSP_CODE_ACTION", LSP_CODE_ACTION),
        ("LSP_CODE_LENS", LSP_CODE_LENS),
        ("LSP_FORMATTING", LSP_FORMATTING),
        ("LSP_RANGE_FORMATTING", LSP_RANGE_FORMATTING),
        ("LSP_RANGES_FORMATTING", LSP_RANGES_FORMATTING),
        ("LSP_ON_TYPE_FORMATTING", LSP_ON_TYPE_FORMATTING),
        ("LSP_RENAME", LSP_RENAME),
        ("LSP_DOCUMENT_LINK", LSP_DOCUMENT_LINK),
        ("LSP_FOLDING_RANGE", LSP_FOLDING_RANGE),
        ("LSP_SELECTION_RANGE", LSP_SELECTION_RANGE),
        ("LSP_INLAY_HINT", LSP_INLAY_HINT),
        ("LSP_SEMANTIC_TOKENS", LSP_SEMANTIC_TOKENS),
        ("LSP_TYPE_HIERARCHY", LSP_TYPE_HIERARCHY),
        ("LSP_CALL_HIERARCHY", LSP_CALL_HIERARCHY),
        ("LSP_PULL_DIAGNOSTICS", LSP_PULL_DIAGNOSTICS),
        ("LSP_INLINE_COMPLETION", LSP_INLINE_COMPLETION),
        ("LSP_INLINE_VALUE", LSP_INLINE_VALUE),
        ("LSP_DOCUMENT_COLOR", LSP_DOCUMENT_COLOR),
        ("LSP_COLOR", LSP_COLOR),
        ("LSP_LINKED_EDITING_RANGE", LSP_LINKED_EDITING_RANGE),
        ("LSP_MONIKER", LSP_MONIKER),
        ("LSP_INLINE_VALUES", LSP_INLINE_VALUES),
        ("LSP_NOTEBOOK_DOCUMENT_SYNC", LSP_NOTEBOOK_DOCUMENT_SYNC),
        ("LSP_NOTEBOOK_CELL_EXECUTION", LSP_NOTEBOOK_CELL_EXECUTION),
        ("LSP_PROGRESS", LSP_PROGRESS),
        ("LSP_SHOW_MESSAGE", LSP_SHOW_MESSAGE),
        ("LSP_LOG_MESSAGE", LSP_LOG_MESSAGE),
        ("LSP_WORK_DONE_PROGRESS", LSP_WORK_DONE_PROGRESS),
        ("LSP_TEXT_DOCUMENT_SYNC", LSP_TEXT_DOCUMENT_SYNC),
        ("LSP_DID_SAVE", LSP_DID_SAVE),
        ("LSP_WILL_SAVE", LSP_WILL_SAVE),
        ("LSP_WILL_SAVE_WAIT_UNTIL", LSP_WILL_SAVE_WAIT_UNTIL),
        ("LSP_DOCUMENT_HIGHLIGHT", LSP_DOCUMENT_HIGHLIGHT),
        ("LSP_PREPARE_RENAME", LSP_PREPARE_RENAME),
        ("LSP_COLOR_PRESENTATION", LSP_COLOR_PRESENTATION),
        ("LSP_COMPLETION_ITEM_RESOLVE", LSP_COMPLETION_ITEM_RESOLVE),
        ("LSP_CODE_ACTION_RESOLVE", LSP_CODE_ACTION_RESOLVE),
        ("LSP_CODE_LENS_RESOLVE", LSP_CODE_LENS_RESOLVE),
        ("LSP_DOCUMENT_LINK_RESOLVE", LSP_DOCUMENT_LINK_RESOLVE),
        ("LSP_INLAY_HINT_RESOLVE", LSP_INLAY_HINT_RESOLVE),
        ("LSP_WORKSPACE_SYMBOL_RESOLVE", LSP_WORKSPACE_SYMBOL_RESOLVE),
        ("LSP_CODE_LENS_REFRESH", LSP_CODE_LENS_REFRESH),
        ("LSP_SEMANTIC_TOKENS_REFRESH", LSP_SEMANTIC_TOKENS_REFRESH),
        ("LSP_INLAY_HINT_REFRESH", LSP_INLAY_HINT_REFRESH),
        ("LSP_INLINE_VALUE_REFRESH", LSP_INLINE_VALUE_REFRESH),
        ("LSP_DIAGNOSTIC_REFRESH", LSP_DIAGNOSTIC_REFRESH),
        ("LSP_FOLDING_RANGE_REFRESH", LSP_FOLDING_RANGE_REFRESH),
    ]
}

fn all_dap_ids() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DAP_CORE", DAP_CORE),
        ("DAP_INLINE_VALUES", DAP_INLINE_VALUES),
        ("DAP_BREAKPOINTS_BASIC", DAP_BREAKPOINTS_BASIC),
    ]
}

fn all_ids() -> Vec<(&'static str, &'static str)> {
    let mut ids = all_lsp_ids();
    ids.extend(all_dap_ids());
    ids
}

// ===========================================================================
// 1. Exact value tests — every constant pinned to its expected string
// ===========================================================================

#[test]
fn lsp_core_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_COMPLETION, "lsp.completion"),
        (LSP_HOVER, "lsp.hover"),
        (LSP_SIGNATURE_HELP, "lsp.signature_help"),
        (LSP_DEFINITION, "lsp.definition"),
        (LSP_DECLARATION, "lsp.declaration"),
        (LSP_EXECUTE_COMMAND, "lsp.execute_command"),
        (LSP_TYPE_DEFINITION, "lsp.type_definition"),
        (LSP_IMPLEMENTATION, "lsp.implementation"),
        (LSP_REFERENCES, "lsp.references"),
        (LSP_DOCUMENT_SYMBOL, "lsp.document_symbol"),
        (LSP_WORKSPACE_SYMBOL, "lsp.workspace_symbol"),
        (LSP_CODE_ACTION, "lsp.code_action"),
        (LSP_CODE_LENS, "lsp.code_lens"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_formatting_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_FORMATTING, "lsp.formatting"),
        (LSP_RANGE_FORMATTING, "lsp.range_formatting"),
        (LSP_RANGES_FORMATTING, "lsp.ranges_formatting"),
        (LSP_ON_TYPE_FORMATTING, "lsp.on_type_formatting"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_navigation_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_RENAME, "lsp.rename"),
        (LSP_PREPARE_RENAME, "lsp.prepare_rename"),
        (LSP_DOCUMENT_LINK, "lsp.document_link"),
        (LSP_FOLDING_RANGE, "lsp.folding_range"),
        (LSP_SELECTION_RANGE, "lsp.selection_range"),
        (LSP_DOCUMENT_HIGHLIGHT, "lsp.document_highlight"),
        (LSP_LINKED_EDITING_RANGE, "lsp.linked_editing_range"),
        (LSP_MONIKER, "lsp.moniker"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_semantic_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_INLAY_HINT, "lsp.inlay_hint"),
        (LSP_SEMANTIC_TOKENS, "lsp.semantic_tokens"),
        (LSP_TYPE_HIERARCHY, "lsp.type_hierarchy"),
        (LSP_CALL_HIERARCHY, "lsp.call_hierarchy"),
        (LSP_PULL_DIAGNOSTICS, "lsp.pull_diagnostics"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_inline_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_INLINE_COMPLETION, "lsp.inline_completion"),
        (LSP_INLINE_VALUE, "lsp.inline_value"),
        (LSP_INLINE_VALUES, "lsp.inline_values"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_color_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_DOCUMENT_COLOR, "lsp.document_color"),
        (LSP_COLOR, "lsp.color"),
        (LSP_COLOR_PRESENTATION, "lsp.color_presentation"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_notebook_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_NOTEBOOK_DOCUMENT_SYNC, "lsp.notebook_document_sync"),
        (LSP_NOTEBOOK_CELL_EXECUTION, "lsp.notebook_cell_execution"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_lifecycle_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_PROGRESS, "lsp.progress"),
        (LSP_SHOW_MESSAGE, "lsp.show_message"),
        (LSP_LOG_MESSAGE, "lsp.log_message"),
        (LSP_WORK_DONE_PROGRESS, "lsp.work_done_progress"),
        (LSP_TEXT_DOCUMENT_SYNC, "lsp.text_document_sync"),
        (LSP_DID_SAVE, "lsp.did_save"),
        (LSP_WILL_SAVE, "lsp.will_save"),
        (LSP_WILL_SAVE_WAIT_UNTIL, "lsp.will_save_wait_until"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_resolve_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_COMPLETION_ITEM_RESOLVE, "lsp.completion_item_resolve"),
        (LSP_CODE_ACTION_RESOLVE, "lsp.code_action_resolve"),
        (LSP_CODE_LENS_RESOLVE, "lsp.code_lens_resolve"),
        (LSP_DOCUMENT_LINK_RESOLVE, "lsp.document_link_resolve"),
        (LSP_INLAY_HINT_RESOLVE, "lsp.inlay_hint_resolve"),
        (LSP_WORKSPACE_SYMBOL_RESOLVE, "lsp.workspace_symbol_resolve"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_refresh_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (LSP_CODE_LENS_REFRESH, "lsp.code_lens_refresh"),
        (LSP_SEMANTIC_TOKENS_REFRESH, "lsp.semantic_tokens_refresh"),
        (LSP_INLAY_HINT_REFRESH, "lsp.inlay_hint_refresh"),
        (LSP_INLINE_VALUE_REFRESH, "lsp.inline_value_refresh"),
        (LSP_DIAGNOSTIC_REFRESH, "lsp.diagnostic_refresh"),
        (LSP_FOLDING_RANGE_REFRESH, "lsp.folding_range_refresh"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

#[test]
fn dap_feature_values() -> Result<(), String> {
    let cases: Vec<(&str, &str)> = vec![
        (DAP_CORE, "dap.core"),
        (DAP_INLINE_VALUES, "dap.inline_values"),
        (DAP_BREAKPOINTS_BASIC, "dap.breakpoints.basic"),
    ];
    for (actual, expected) in &cases {
        if *actual != *expected {
            return Err(format!("expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

// ===========================================================================
// 2. Prefix convention tests
// ===========================================================================

#[test]
fn all_lsp_ids_start_with_lsp_dot() -> Result<(), String> {
    for (name, value) in all_lsp_ids() {
        if !value.starts_with("lsp.") {
            return Err(format!("{name} = \"{value}\" does not start with \"lsp.\""));
        }
    }
    Ok(())
}

#[test]
fn all_dap_ids_start_with_dap_dot() -> Result<(), String> {
    for (name, value) in all_dap_ids() {
        if !value.starts_with("dap.") {
            return Err(format!("{name} = \"{value}\" does not start with \"dap.\""));
        }
    }
    Ok(())
}

// ===========================================================================
// 3. Naming convention tests
// ===========================================================================

#[test]
fn all_ids_use_lowercase_with_underscores_or_dots() -> Result<(), String> {
    for (name, value) in all_ids() {
        for ch in value.chars() {
            if !(ch.is_ascii_lowercase() || ch == '_' || ch == '.') {
                return Err(format!(
                    "{name} = \"{value}\" contains invalid character '{ch}'"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn no_id_is_empty() -> Result<(), String> {
    for (name, value) in all_ids() {
        if value.is_empty() {
            return Err(format!("{name} is empty"));
        }
    }
    Ok(())
}

#[test]
fn no_id_has_leading_or_trailing_whitespace() -> Result<(), String> {
    for (name, value) in all_ids() {
        if value != value.trim() {
            return Err(format!(
                "{name} = \"{value}\" has leading/trailing whitespace"
            ));
        }
    }
    Ok(())
}

#[test]
fn no_id_contains_consecutive_dots() -> Result<(), String> {
    for (name, value) in all_ids() {
        if value.contains("..") {
            return Err(format!("{name} = \"{value}\" contains consecutive dots"));
        }
    }
    Ok(())
}

#[test]
fn no_id_ends_with_dot_or_underscore() -> Result<(), String> {
    for (name, value) in all_ids() {
        if value.ends_with('.') || value.ends_with('_') {
            return Err(format!(
                "{name} = \"{value}\" ends with a dot or underscore"
            ));
        }
    }
    Ok(())
}

// ===========================================================================
// 4. Uniqueness tests
// ===========================================================================

#[test]
fn all_feature_ids_are_unique_excluding_known_aliases() -> Result<(), String> {
    // LSP_COLOR is a known legacy alias of LSP_DOCUMENT_COLOR, so we exclude
    // that pair from the uniqueness check. All others must be distinct.
    let mut seen = HashSet::new();
    let alias_values: HashSet<&str> = HashSet::from(["lsp.color"]);

    for (name, value) in all_ids() {
        if alias_values.contains(value) {
            continue;
        }
        if !seen.insert(value) {
            return Err(format!("duplicate feature id: {name} = \"{value}\""));
        }
    }
    Ok(())
}

#[test]
fn lsp_color_is_legacy_alias_of_document_color() -> Result<(), String> {
    // Both exist but must differ — LSP_COLOR is *not* equal to LSP_DOCUMENT_COLOR
    if LSP_COLOR == LSP_DOCUMENT_COLOR {
        return Err("LSP_COLOR should differ from LSP_DOCUMENT_COLOR".into());
    }
    if LSP_COLOR != "lsp.color" {
        return Err(format!("LSP_COLOR unexpected value: {}", LSP_COLOR));
    }
    if LSP_DOCUMENT_COLOR != "lsp.document_color" {
        return Err(format!(
            "LSP_DOCUMENT_COLOR unexpected value: {}",
            LSP_DOCUMENT_COLOR
        ));
    }
    Ok(())
}

// ===========================================================================
// 5. Cardinality / count tests
// ===========================================================================

#[test]
fn expected_lsp_id_count() -> Result<(), String> {
    let count = all_lsp_ids().len();
    if count != 58 {
        return Err(format!("expected 58 LSP ids, got {count}"));
    }
    Ok(())
}

#[test]
fn expected_dap_id_count() -> Result<(), String> {
    let count = all_dap_ids().len();
    if count != 3 {
        return Err(format!("expected 3 DAP ids, got {count}"));
    }
    Ok(())
}

#[test]
fn total_id_count() -> Result<(), String> {
    let count = all_ids().len();
    if count != 61 {
        return Err(format!("expected 61 total ids, got {count}"));
    }
    Ok(())
}

// ===========================================================================
// 6. Resolve / refresh suffix pairing tests
// ===========================================================================

#[test]
fn resolve_ids_end_with_resolve_suffix() -> Result<(), String> {
    let resolve_ids = [
        LSP_COMPLETION_ITEM_RESOLVE,
        LSP_CODE_ACTION_RESOLVE,
        LSP_CODE_LENS_RESOLVE,
        LSP_DOCUMENT_LINK_RESOLVE,
        LSP_INLAY_HINT_RESOLVE,
        LSP_WORKSPACE_SYMBOL_RESOLVE,
    ];
    for id in &resolve_ids {
        if !id.ends_with("_resolve") {
            return Err(format!("\"{id}\" does not end with _resolve"));
        }
    }
    Ok(())
}

#[test]
fn refresh_ids_end_with_refresh_suffix() -> Result<(), String> {
    let refresh_ids = [
        LSP_CODE_LENS_REFRESH,
        LSP_SEMANTIC_TOKENS_REFRESH,
        LSP_INLAY_HINT_REFRESH,
        LSP_INLINE_VALUE_REFRESH,
        LSP_DIAGNOSTIC_REFRESH,
        LSP_FOLDING_RANGE_REFRESH,
    ];
    for id in &refresh_ids {
        if !id.ends_with("_refresh") {
            return Err(format!("\"{id}\" does not end with _refresh"));
        }
    }
    Ok(())
}

// ===========================================================================
// 7. String interoperability tests
// ===========================================================================

#[test]
fn ids_are_valid_for_hashmap_keys() -> Result<(), String> {
    let mut map = std::collections::HashMap::new();
    for (name, value) in all_ids() {
        map.insert(value, name);
    }
    // All 61 values are distinct strings, so the map has 61 entries.
    if map.len() != 61 {
        return Err(format!("expected 61 map entries, got {}", map.len()));
    }
    Ok(())
}

#[test]
fn ids_are_comparable_with_string_slices() -> Result<(), String> {
    let needle: &str = "lsp.completion";
    if LSP_COMPLETION != needle {
        return Err("LSP_COMPLETION should equal \"lsp.completion\"".into());
    }

    let owned: String = String::from("dap.core");
    if DAP_CORE != owned.as_str() {
        return Err("DAP_CORE should equal owned String \"dap.core\"".into());
    }
    Ok(())
}

#[test]
fn ids_can_be_collected_into_set() -> Result<(), String> {
    let set: HashSet<&str> = all_ids().iter().map(|(_, v)| *v).collect();
    if !set.contains(LSP_HOVER) {
        return Err("set should contain LSP_HOVER".into());
    }
    if !set.contains(DAP_CORE) {
        return Err("set should contain DAP_CORE".into());
    }
    Ok(())
}

// ===========================================================================
// 8. Category grouping tests
// ===========================================================================

#[test]
fn formatting_ids_contain_formatting_substring() -> Result<(), String> {
    let formatting_ids = [
        LSP_FORMATTING,
        LSP_RANGE_FORMATTING,
        LSP_RANGES_FORMATTING,
        LSP_ON_TYPE_FORMATTING,
    ];
    for id in &formatting_ids {
        if !id.contains("formatting") {
            return Err(format!("\"{id}\" should contain \"formatting\""));
        }
    }
    Ok(())
}

#[test]
fn hierarchy_ids_contain_hierarchy_substring() -> Result<(), String> {
    let hierarchy_ids = [LSP_TYPE_HIERARCHY, LSP_CALL_HIERARCHY];
    for id in &hierarchy_ids {
        if !id.contains("hierarchy") {
            return Err(format!("\"{id}\" should contain \"hierarchy\""));
        }
    }
    Ok(())
}

#[test]
fn notebook_ids_contain_notebook_substring() -> Result<(), String> {
    let notebook_ids = [LSP_NOTEBOOK_DOCUMENT_SYNC, LSP_NOTEBOOK_CELL_EXECUTION];
    for id in &notebook_ids {
        if !id.contains("notebook") {
            return Err(format!("\"{id}\" should contain \"notebook\""));
        }
    }
    Ok(())
}

// ===========================================================================
// 9. DAP-specific structure tests
// ===========================================================================

#[test]
fn dap_breakpoints_basic_uses_dot_separator() -> Result<(), String> {
    // DAP_BREAKPOINTS_BASIC uses a nested dot namespace (dap.breakpoints.basic)
    let parts: Vec<&str> = DAP_BREAKPOINTS_BASIC.split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "expected 3 dot-separated segments, got {}",
            parts.len()
        ));
    }
    if parts[0] != "dap" || parts[1] != "breakpoints" || parts[2] != "basic" {
        return Err(format!("unexpected segments: {:?}", parts));
    }
    Ok(())
}

#[test]
fn dap_core_has_two_segments() -> Result<(), String> {
    let parts: Vec<&str> = DAP_CORE.split('.').collect();
    if parts.len() != 2 {
        return Err(format!("expected 2 segments, got {}", parts.len()));
    }
    Ok(())
}

// ===========================================================================
// 10. Static lifetime / const correctness tests
// ===========================================================================

#[test]
fn ids_are_static_str() -> Result<(), String> {
    // Ensures these can be used in static contexts
    fn assert_static(_: &'static str) {}
    assert_static(LSP_COMPLETION);
    assert_static(LSP_HOVER);
    assert_static(DAP_CORE);
    assert_static(DAP_BREAKPOINTS_BASIC);
    assert_static(LSP_FOLDING_RANGE_REFRESH);
    Ok(())
}

#[test]
fn ids_are_usable_in_const_context() -> Result<(), String> {
    const ID: &str = LSP_COMPLETION;
    if ID != "lsp.completion" {
        return Err("const context usage failed".into());
    }
    Ok(())
}

// ===========================================================================
// 11. Segment structure tests
// ===========================================================================

#[test]
fn all_lsp_ids_have_at_least_two_dot_segments() -> Result<(), String> {
    for (name, value) in all_lsp_ids() {
        let count = value.split('.').count();
        if count < 2 {
            return Err(format!("{name} = \"{value}\" has only {count} segment(s)"));
        }
    }
    Ok(())
}

#[test]
fn first_segment_matches_protocol_family() -> Result<(), String> {
    for (name, value) in all_lsp_ids() {
        let first = value.split('.').next().unwrap_or("");
        if first != "lsp" {
            return Err(format!(
                "{name}: first segment is \"{first}\", expected \"lsp\""
            ));
        }
    }
    for (name, value) in all_dap_ids() {
        let first = value.split('.').next().unwrap_or("");
        if first != "dap" {
            return Err(format!(
                "{name}: first segment is \"{first}\", expected \"dap\""
            ));
        }
    }
    Ok(())
}

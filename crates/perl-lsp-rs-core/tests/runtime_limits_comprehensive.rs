//! Comprehensive unit tests for perl-lsp-limits
//!
//! Covers: Default values, presets, update_from_value, global accessors,
//! trait implementations, edge cases, and degradation flags.
#![allow(clippy::field_reassign_with_default)]

use std::time::Duration;

use perl_lsp_rs_core::runtime::limits::{
    LspLimits, code_lens_cap, code_lens_resolve_deadline, completion_cap, completion_deadline,
    diagnostics_per_file_cap, document_symbol_cap, inlay_hints_cap, reference_search_deadline,
    references_cap, regex_scan_deadline, semantic_tokens_deadline, workspace_symbol_cap,
};

// =============================================================================
// Default values — every field
// =============================================================================

#[test]
fn default_result_caps() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert_eq!(d.workspace_symbol_cap, 200);
    assert_eq!(d.references_cap, 500);
    assert_eq!(d.completion_cap, 100);
    assert_eq!(d.document_symbol_cap, 500);
    assert_eq!(d.code_lens_cap, 100);
    assert_eq!(d.diagnostics_per_file_cap, 200);
    assert_eq!(d.inlay_hints_cap, 500);
    Ok(())
}

#[test]
fn default_cache_limits() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert_eq!(d.ast_cache_max_entries, 100);
    assert_eq!(d.ast_cache_ttl_secs, 300);
    assert_eq!(d.symbol_cache_max_entries, 1000);
    Ok(())
}

#[test]
fn default_index_limits() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert_eq!(d.max_indexed_files, 10_000);
    assert_eq!(d.max_symbols_per_file, 5_000);
    assert_eq!(d.max_total_symbols, 500_000);
    assert_eq!(d.parse_storm_threshold, 10);
    Ok(())
}

#[test]
fn default_deadlines() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert_eq!(d.workspace_scan_deadline, Duration::from_secs(30));
    assert_eq!(d.file_index_deadline, Duration::from_secs(5));
    assert_eq!(d.reference_search_deadline, Duration::from_secs(2));
    assert_eq!(d.regex_scan_deadline, Duration::from_secs(1));
    assert_eq!(d.fs_operation_deadline, Duration::from_millis(500));
    assert_eq!(d.semantic_tokens_deadline, Duration::from_secs(2));
    assert_eq!(d.code_lens_resolve_deadline, Duration::from_secs(1));
    assert_eq!(d.completion_deadline, Duration::from_millis(500));
    Ok(())
}

#[test]
fn default_degradation_flags() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert!(d.return_partial_on_timeout);
    assert!(d.include_open_docs_when_degraded);
    Ok(())
}

// =============================================================================
// Preset: large_workspace
// =============================================================================

#[test]
fn large_workspace_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let lw = LspLimits::large_workspace();
    assert_eq!(lw.max_indexed_files, 50_000);
    assert_eq!(lw.max_total_symbols, 2_000_000);
    assert_eq!(lw.workspace_scan_deadline, Duration::from_secs(120));
    Ok(())
}

#[test]
fn large_workspace_inherits_other_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let lw = LspLimits::large_workspace();
    let d = LspLimits::default();
    // Fields NOT overridden must equal defaults
    assert_eq!(lw.workspace_symbol_cap, d.workspace_symbol_cap);
    assert_eq!(lw.references_cap, d.references_cap);
    assert_eq!(lw.completion_cap, d.completion_cap);
    assert_eq!(lw.document_symbol_cap, d.document_symbol_cap);
    assert_eq!(lw.code_lens_cap, d.code_lens_cap);
    assert_eq!(lw.diagnostics_per_file_cap, d.diagnostics_per_file_cap);
    assert_eq!(lw.inlay_hints_cap, d.inlay_hints_cap);
    assert_eq!(lw.ast_cache_max_entries, d.ast_cache_max_entries);
    assert_eq!(lw.ast_cache_ttl_secs, d.ast_cache_ttl_secs);
    assert_eq!(lw.symbol_cache_max_entries, d.symbol_cache_max_entries);
    assert_eq!(lw.max_symbols_per_file, d.max_symbols_per_file);
    assert_eq!(lw.parse_storm_threshold, d.parse_storm_threshold);
    assert_eq!(lw.file_index_deadline, d.file_index_deadline);
    assert_eq!(lw.reference_search_deadline, d.reference_search_deadline);
    assert_eq!(lw.regex_scan_deadline, d.regex_scan_deadline);
    assert_eq!(lw.fs_operation_deadline, d.fs_operation_deadline);
    assert_eq!(lw.semantic_tokens_deadline, d.semantic_tokens_deadline);
    assert_eq!(lw.code_lens_resolve_deadline, d.code_lens_resolve_deadline);
    assert_eq!(lw.completion_deadline, d.completion_deadline);
    assert_eq!(lw.return_partial_on_timeout, d.return_partial_on_timeout);
    assert_eq!(lw.include_open_docs_when_degraded, d.include_open_docs_when_degraded);
    Ok(())
}

// =============================================================================
// Preset: constrained
// =============================================================================

#[test]
fn constrained_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let c = LspLimits::constrained();
    assert_eq!(c.ast_cache_max_entries, 50);
    assert_eq!(c.max_indexed_files, 5_000);
    assert_eq!(c.max_total_symbols, 100_000);
    assert_eq!(c.workspace_scan_deadline, Duration::from_secs(15));
    assert_eq!(c.reference_search_deadline, Duration::from_secs(1));
    Ok(())
}

#[test]
fn constrained_inherits_other_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let c = LspLimits::constrained();
    let d = LspLimits::default();
    assert_eq!(c.workspace_symbol_cap, d.workspace_symbol_cap);
    assert_eq!(c.references_cap, d.references_cap);
    assert_eq!(c.completion_cap, d.completion_cap);
    assert_eq!(c.document_symbol_cap, d.document_symbol_cap);
    assert_eq!(c.code_lens_cap, d.code_lens_cap);
    assert_eq!(c.diagnostics_per_file_cap, d.diagnostics_per_file_cap);
    assert_eq!(c.inlay_hints_cap, d.inlay_hints_cap);
    assert_eq!(c.ast_cache_ttl_secs, d.ast_cache_ttl_secs);
    assert_eq!(c.symbol_cache_max_entries, d.symbol_cache_max_entries);
    assert_eq!(c.max_symbols_per_file, d.max_symbols_per_file);
    assert_eq!(c.parse_storm_threshold, d.parse_storm_threshold);
    assert_eq!(c.file_index_deadline, d.file_index_deadline);
    assert_eq!(c.regex_scan_deadline, d.regex_scan_deadline);
    assert_eq!(c.fs_operation_deadline, d.fs_operation_deadline);
    assert_eq!(c.semantic_tokens_deadline, d.semantic_tokens_deadline);
    assert_eq!(c.code_lens_resolve_deadline, d.code_lens_resolve_deadline);
    assert_eq!(c.completion_deadline, d.completion_deadline);
    assert_eq!(c.return_partial_on_timeout, d.return_partial_on_timeout);
    assert_eq!(c.include_open_docs_when_degraded, d.include_open_docs_when_degraded);
    Ok(())
}

// =============================================================================
// update_from_value — each supported key
// =============================================================================

#[test]
fn update_workspace_symbol_cap() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "workspaceSymbolCap": 42 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, 42);
    Ok(())
}

#[test]
fn update_references_cap() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "referencesCap": 999 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.references_cap, 999);
    Ok(())
}

#[test]
fn update_completion_cap() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "completionCap": 50 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.completion_cap, 50);
    Ok(())
}

#[test]
fn update_ast_cache_max_entries() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "astCacheMaxEntries": 250 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.ast_cache_max_entries, 250);
    Ok(())
}

#[test]
fn update_max_indexed_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "maxIndexedFiles": 20_000 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.max_indexed_files, 20_000);
    Ok(())
}

#[test]
fn update_max_total_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "maxTotalSymbols": 1_000_000 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.max_total_symbols, 1_000_000);
    Ok(())
}

#[test]
fn update_workspace_scan_deadline_ms() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "workspaceScanDeadlineMs": 60000 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(60000));
    Ok(())
}

#[test]
fn update_reference_search_deadline_ms() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "referenceSearchDeadlineMs": 5000 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.reference_search_deadline, Duration::from_millis(5000));
    Ok(())
}

#[test]
fn update_multiple_fields_at_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 300,
            "referencesCap": 1000,
            "completionCap": 75,
            "astCacheMaxEntries": 200,
            "maxIndexedFiles": 25000,
            "maxTotalSymbols": 750000,
            "workspaceScanDeadlineMs": 45000,
            "referenceSearchDeadlineMs": 3000
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, 300);
    assert_eq!(limits.references_cap, 1000);
    assert_eq!(limits.completion_cap, 75);
    assert_eq!(limits.ast_cache_max_entries, 200);
    assert_eq!(limits.max_indexed_files, 25_000);
    assert_eq!(limits.max_total_symbols, 750_000);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(45000));
    assert_eq!(limits.reference_search_deadline, Duration::from_millis(3000));
    Ok(())
}

// =============================================================================
// update_from_value — no-op / resilience
// =============================================================================

#[test]
fn update_no_limits_key_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let before = limits.clone();
    let settings = serde_json::json!({ "other": 42 });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, before.workspace_symbol_cap);
    assert_eq!(limits.references_cap, before.references_cap);
    assert_eq!(limits.max_indexed_files, before.max_indexed_files);
    Ok(())
}

#[test]
fn update_empty_limits_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let before = limits.clone();
    let settings = serde_json::json!({ "limits": {} });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, before.workspace_symbol_cap);
    assert_eq!(limits.references_cap, before.references_cap);
    Ok(())
}

#[test]
fn update_empty_object_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let before = limits.clone();
    let settings = serde_json::json!({});
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, before.workspace_symbol_cap);
    Ok(())
}

#[test]
fn update_null_value_is_noop() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let before = limits.clone();
    let settings = serde_json::Value::Null;
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, before.workspace_symbol_cap);
    Ok(())
}

#[test]
fn update_wrong_type_string_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "workspaceSymbolCap": "not a number" } });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, 200);
    Ok(())
}

#[test]
fn update_wrong_type_bool_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "referencesCap": true } });
    limits.update_from_value(&settings);
    assert_eq!(limits.references_cap, 500);
    Ok(())
}

#[test]
fn update_wrong_type_array_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "maxIndexedFiles": [1, 2, 3] } });
    limits.update_from_value(&settings);
    assert_eq!(limits.max_indexed_files, 10_000);
    Ok(())
}

#[test]
fn update_negative_float_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "completionCap": -1.5 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.completion_cap, 100);
    Ok(())
}

#[test]
fn update_unknown_keys_are_silently_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let before = limits.clone();
    let settings = serde_json::json!({
        "limits": {
            "nonExistentKey": 999,
            "anotherBogus": "hello"
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, before.workspace_symbol_cap);
    assert_eq!(limits.references_cap, before.references_cap);
    Ok(())
}

#[test]
fn update_zero_values_are_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 0,
            "maxIndexedFiles": 0,
            "workspaceScanDeadlineMs": 0
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.workspace_symbol_cap, 0);
    assert_eq!(limits.max_indexed_files, 0);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(0));
    Ok(())
}

#[test]
fn update_preserves_unmentioned_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({ "limits": { "workspaceSymbolCap": 42 } });
    limits.update_from_value(&settings);
    // Only workspaceSymbolCap changed; everything else is default
    assert_eq!(limits.workspace_symbol_cap, 42);
    assert_eq!(limits.references_cap, 500);
    assert_eq!(limits.completion_cap, 100);
    assert_eq!(limits.ast_cache_max_entries, 100);
    assert_eq!(limits.max_indexed_files, 10_000);
    assert_eq!(limits.max_total_symbols, 500_000);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_secs(30));
    assert_eq!(limits.reference_search_deadline, Duration::from_secs(2));
    Ok(())
}

// =============================================================================
// update_from_value — applied to presets
// =============================================================================

#[test]
fn update_on_large_workspace_preset() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::large_workspace();
    let settings = serde_json::json!({ "limits": { "maxIndexedFiles": 100_000 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.max_indexed_files, 100_000);
    // Non-overridden preset value is preserved
    assert_eq!(limits.max_total_symbols, 2_000_000);
    Ok(())
}

#[test]
fn update_on_constrained_preset() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::constrained();
    let settings = serde_json::json!({ "limits": { "astCacheMaxEntries": 25 } });
    limits.update_from_value(&settings);
    assert_eq!(limits.ast_cache_max_entries, 25);
    assert_eq!(limits.max_indexed_files, 5_000);
    Ok(())
}

// =============================================================================
// Global accessor functions
// =============================================================================

#[test]
fn global_workspace_symbol_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(workspace_symbol_cap(), 200);
    Ok(())
}

#[test]
fn global_references_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(references_cap(), 500);
    Ok(())
}

#[test]
fn global_completion_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(completion_cap(), 100);
    Ok(())
}

#[test]
fn global_code_lens_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_lens_cap(), 100);
    Ok(())
}

#[test]
fn global_document_symbol_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(document_symbol_cap(), 500);
    Ok(())
}

#[test]
fn global_inlay_hints_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(inlay_hints_cap(), 500);
    Ok(())
}

#[test]
fn global_diagnostics_per_file_cap_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(diagnostics_per_file_cap(), 200);
    Ok(())
}

#[test]
fn global_reference_search_deadline_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(reference_search_deadline(), Duration::from_secs(2));
    Ok(())
}

#[test]
fn global_regex_scan_deadline_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(regex_scan_deadline(), Duration::from_secs(1));
    Ok(())
}

#[test]
fn global_semantic_tokens_deadline_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(semantic_tokens_deadline(), Duration::from_secs(2));
    Ok(())
}

#[test]
fn global_code_lens_resolve_deadline_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_lens_resolve_deadline(), Duration::from_secs(1));
    Ok(())
}

#[test]
fn global_completion_deadline_returns_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(completion_deadline(), Duration::from_millis(500));
    Ok(())
}

// =============================================================================
// Trait implementations
// =============================================================================

#[test]
fn debug_trait_produces_output() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::default();
    let debug_str = format!("{:?}", limits);
    assert!(debug_str.contains("LspLimits"));
    assert!(debug_str.contains("workspace_symbol_cap"));
    assert!(debug_str.contains("200"));
    Ok(())
}

#[test]
fn clone_produces_independent_copy() -> Result<(), Box<dyn std::error::Error>> {
    let original = LspLimits::default();
    let mut cloned = original.clone();
    cloned.workspace_symbol_cap = 999;
    // Original is unchanged
    assert_eq!(original.workspace_symbol_cap, 200);
    assert_eq!(cloned.workspace_symbol_cap, 999);
    Ok(())
}

// =============================================================================
// Deadline semantics
// =============================================================================

#[test]
fn deadlines_are_positive_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert!(d.workspace_scan_deadline > Duration::ZERO);
    assert!(d.file_index_deadline > Duration::ZERO);
    assert!(d.reference_search_deadline > Duration::ZERO);
    assert!(d.regex_scan_deadline > Duration::ZERO);
    assert!(d.fs_operation_deadline > Duration::ZERO);
    assert!(d.semantic_tokens_deadline > Duration::ZERO);
    assert!(d.code_lens_resolve_deadline > Duration::ZERO);
    assert!(d.completion_deadline > Duration::ZERO);
    Ok(())
}

#[test]
fn workspace_scan_is_longest_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    assert!(d.workspace_scan_deadline > d.file_index_deadline);
    assert!(d.workspace_scan_deadline > d.reference_search_deadline);
    assert!(d.workspace_scan_deadline > d.regex_scan_deadline);
    assert!(d.workspace_scan_deadline > d.fs_operation_deadline);
    assert!(d.workspace_scan_deadline > d.semantic_tokens_deadline);
    assert!(d.workspace_scan_deadline > d.code_lens_resolve_deadline);
    assert!(d.workspace_scan_deadline > d.completion_deadline);
    Ok(())
}

#[test]
fn fs_operation_is_shortest_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    // fs_operation and completion both 500ms — they are the shortest
    assert!(d.fs_operation_deadline <= d.regex_scan_deadline);
    assert!(d.fs_operation_deadline <= d.reference_search_deadline);
    assert!(d.fs_operation_deadline <= d.file_index_deadline);
    assert!(d.fs_operation_deadline <= d.workspace_scan_deadline);
    Ok(())
}

// =============================================================================
// Preset relationship invariants
// =============================================================================

#[test]
fn large_workspace_has_more_capacity_than_default() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    let lw = LspLimits::large_workspace();
    assert!(lw.max_indexed_files > d.max_indexed_files);
    assert!(lw.max_total_symbols > d.max_total_symbols);
    assert!(lw.workspace_scan_deadline > d.workspace_scan_deadline);
    Ok(())
}

#[test]
fn constrained_has_less_capacity_than_default() -> Result<(), Box<dyn std::error::Error>> {
    let d = LspLimits::default();
    let c = LspLimits::constrained();
    assert!(c.ast_cache_max_entries < d.ast_cache_max_entries);
    assert!(c.max_indexed_files < d.max_indexed_files);
    assert!(c.max_total_symbols < d.max_total_symbols);
    assert!(c.workspace_scan_deadline < d.workspace_scan_deadline);
    assert!(c.reference_search_deadline < d.reference_search_deadline);
    Ok(())
}

// =============================================================================
// Struct field mutability
// =============================================================================

#[test]
fn all_fields_are_publicly_writable() -> Result<(), Box<dyn std::error::Error>> {
    let mut l = LspLimits::default();
    l.workspace_symbol_cap = 1;
    l.references_cap = 2;
    l.completion_cap = 3;
    l.document_symbol_cap = 4;
    l.code_lens_cap = 5;
    l.diagnostics_per_file_cap = 6;
    l.inlay_hints_cap = 7;
    l.ast_cache_max_entries = 8;
    l.ast_cache_ttl_secs = 9;
    l.symbol_cache_max_entries = 10;
    l.max_indexed_files = 11;
    l.max_symbols_per_file = 12;
    l.max_total_symbols = 13;
    l.parse_storm_threshold = 14;
    l.workspace_scan_deadline = Duration::from_millis(1);
    l.file_index_deadline = Duration::from_millis(2);
    l.reference_search_deadline = Duration::from_millis(3);
    l.regex_scan_deadline = Duration::from_millis(4);
    l.fs_operation_deadline = Duration::from_millis(5);
    l.semantic_tokens_deadline = Duration::from_millis(6);
    l.code_lens_resolve_deadline = Duration::from_millis(7);
    l.completion_deadline = Duration::from_millis(8);
    l.return_partial_on_timeout = false;
    l.include_open_docs_when_degraded = false;

    assert_eq!(l.workspace_symbol_cap, 1);
    assert_eq!(l.references_cap, 2);
    assert_eq!(l.completion_cap, 3);
    assert_eq!(l.document_symbol_cap, 4);
    assert_eq!(l.code_lens_cap, 5);
    assert_eq!(l.diagnostics_per_file_cap, 6);
    assert_eq!(l.inlay_hints_cap, 7);
    assert_eq!(l.ast_cache_max_entries, 8);
    assert_eq!(l.ast_cache_ttl_secs, 9);
    assert_eq!(l.symbol_cache_max_entries, 10);
    assert_eq!(l.max_indexed_files, 11);
    assert_eq!(l.max_symbols_per_file, 12);
    assert_eq!(l.max_total_symbols, 13);
    assert_eq!(l.parse_storm_threshold, 14);
    assert_eq!(l.workspace_scan_deadline, Duration::from_millis(1));
    assert!(!l.return_partial_on_timeout);
    assert!(!l.include_open_docs_when_degraded);
    Ok(())
}

// =============================================================================
// Large JSON value (boundary)
// =============================================================================

#[test]
fn update_with_large_u64_value() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "maxTotalSymbols": 4_294_967_295u64
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.max_total_symbols, 4_294_967_295);
    Ok(())
}

// =============================================================================
// Sequential updates accumulate
// =============================================================================

#[test]
fn sequential_updates_accumulate() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    let s1 = serde_json::json!({ "limits": { "workspaceSymbolCap": 42 } });
    limits.update_from_value(&s1);
    assert_eq!(limits.workspace_symbol_cap, 42);
    assert_eq!(limits.references_cap, 500); // untouched

    let s2 = serde_json::json!({ "limits": { "referencesCap": 999 } });
    limits.update_from_value(&s2);
    assert_eq!(limits.workspace_symbol_cap, 42); // still 42
    assert_eq!(limits.references_cap, 999);
    Ok(())
}

#[test]
fn update_can_overwrite_previous_update() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let s1 = serde_json::json!({ "limits": { "completionCap": 50 } });
    limits.update_from_value(&s1);
    assert_eq!(limits.completion_cap, 50);

    let s2 = serde_json::json!({ "limits": { "completionCap": 75 } });
    limits.update_from_value(&s2);
    assert_eq!(limits.completion_cap, 75);
    Ok(())
}

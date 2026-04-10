//! Extended unit tests for perl-lsp-limits
//!
//! Covers additional edge cases, boundary conditions, stress tests,
//! and comprehensive API coverage beyond comprehensive_unit_tests.rs
#![allow(clippy::field_reassign_with_default)]

use std::time::Duration;

use perl_lsp_limits::{
    LspLimits, code_lens_cap, code_lens_resolve_deadline, completion_cap, completion_deadline,
    diagnostics_per_file_cap, document_symbol_cap, inlay_hints_cap, reference_search_deadline,
    references_cap, regex_scan_deadline, semantic_tokens_deadline, workspace_symbol_cap,
};

// =============================================================================
// Edge Cases: Zero and Very Large Values
// =============================================================================

#[test]
fn zero_result_caps() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.workspace_symbol_cap = 0;
    limits.references_cap = 0;
    limits.completion_cap = 0;
    limits.document_symbol_cap = 0;
    limits.code_lens_cap = 0;
    limits.diagnostics_per_file_cap = 0;
    limits.inlay_hints_cap = 0;

    assert_eq!(limits.workspace_symbol_cap, 0);
    assert_eq!(limits.references_cap, 0);
    assert_eq!(limits.completion_cap, 0);
    assert_eq!(limits.document_symbol_cap, 0);
    assert_eq!(limits.code_lens_cap, 0);
    assert_eq!(limits.diagnostics_per_file_cap, 0);
    assert_eq!(limits.inlay_hints_cap, 0);
    Ok(())
}

#[test]
fn maximum_result_caps() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let max_val = usize::MAX;

    limits.workspace_symbol_cap = max_val;
    limits.references_cap = max_val;
    limits.completion_cap = max_val;
    limits.document_symbol_cap = max_val;
    limits.code_lens_cap = max_val;
    limits.diagnostics_per_file_cap = max_val;
    limits.inlay_hints_cap = max_val;

    assert_eq!(limits.workspace_symbol_cap, max_val);
    assert_eq!(limits.references_cap, max_val);
    assert_eq!(limits.completion_cap, max_val);
    Ok(())
}

#[test]
fn zero_cache_entries() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.ast_cache_max_entries = 0;
    limits.symbol_cache_max_entries = 0;

    assert_eq!(limits.ast_cache_max_entries, 0);
    assert_eq!(limits.symbol_cache_max_entries, 0);
    Ok(())
}

#[test]
fn maximum_cache_entries() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let max_val = usize::MAX;

    limits.ast_cache_max_entries = max_val;
    limits.symbol_cache_max_entries = max_val;

    assert_eq!(limits.ast_cache_max_entries, max_val);
    assert_eq!(limits.symbol_cache_max_entries, max_val);
    Ok(())
}

#[test]
fn zero_index_limits() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.max_indexed_files = 0;
    limits.max_symbols_per_file = 0;
    limits.max_total_symbols = 0;

    assert_eq!(limits.max_indexed_files, 0);
    assert_eq!(limits.max_symbols_per_file, 0);
    assert_eq!(limits.max_total_symbols, 0);
    Ok(())
}

#[test]
fn maximum_index_limits() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let max_val = usize::MAX;

    limits.max_indexed_files = max_val;
    limits.max_symbols_per_file = max_val;
    limits.max_total_symbols = max_val;

    assert_eq!(limits.max_indexed_files, max_val);
    assert_eq!(limits.max_symbols_per_file, max_val);
    assert_eq!(limits.max_total_symbols, max_val);
    Ok(())
}

#[test]
fn zero_parse_storm_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.parse_storm_threshold = 0;

    assert_eq!(limits.parse_storm_threshold, 0);
    Ok(())
}

#[test]
fn maximum_parse_storm_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.parse_storm_threshold = usize::MAX;

    assert_eq!(limits.parse_storm_threshold, usize::MAX);
    Ok(())
}

// =============================================================================
// Edge Cases: Zero and Maximum TTL
// =============================================================================

#[test]
fn zero_ttl() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.ast_cache_ttl_secs = 0;

    assert_eq!(limits.ast_cache_ttl_secs, 0);
    Ok(())
}

#[test]
fn maximum_ttl() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.ast_cache_ttl_secs = u64::MAX;

    assert_eq!(limits.ast_cache_ttl_secs, u64::MAX);
    Ok(())
}

// =============================================================================
// Edge Cases: Zero Deadlines
// =============================================================================

#[test]
fn zero_deadlines() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.workspace_scan_deadline = Duration::from_millis(0);
    limits.file_index_deadline = Duration::from_millis(0);
    limits.reference_search_deadline = Duration::from_millis(0);
    limits.regex_scan_deadline = Duration::from_millis(0);
    limits.fs_operation_deadline = Duration::from_millis(0);
    limits.semantic_tokens_deadline = Duration::from_millis(0);
    limits.code_lens_resolve_deadline = Duration::from_millis(0);
    limits.completion_deadline = Duration::from_millis(0);

    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(0));
    assert_eq!(limits.file_index_deadline, Duration::from_millis(0));
    assert_eq!(limits.reference_search_deadline, Duration::from_millis(0));
    assert_eq!(limits.regex_scan_deadline, Duration::from_millis(0));
    assert_eq!(limits.fs_operation_deadline, Duration::from_millis(0));
    assert_eq!(limits.semantic_tokens_deadline, Duration::from_millis(0));
    assert_eq!(limits.code_lens_resolve_deadline, Duration::from_millis(0));
    assert_eq!(limits.completion_deadline, Duration::from_millis(0));
    Ok(())
}

#[test]
fn maximum_deadlines() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.workspace_scan_deadline = Duration::from_secs(u64::MAX);
    limits.file_index_deadline = Duration::from_secs(u64::MAX);
    limits.reference_search_deadline = Duration::from_secs(u64::MAX);
    limits.regex_scan_deadline = Duration::from_secs(u64::MAX);
    limits.fs_operation_deadline = Duration::from_secs(u64::MAX);
    limits.semantic_tokens_deadline = Duration::from_secs(u64::MAX);
    limits.code_lens_resolve_deadline = Duration::from_secs(u64::MAX);
    limits.completion_deadline = Duration::from_secs(u64::MAX);

    assert_eq!(limits.workspace_scan_deadline, Duration::from_secs(u64::MAX));
    assert_eq!(limits.file_index_deadline, Duration::from_secs(u64::MAX));
    assert_eq!(limits.reference_search_deadline, Duration::from_secs(u64::MAX));
    assert_eq!(limits.regex_scan_deadline, Duration::from_secs(u64::MAX));
    Ok(())
}

#[test]
fn microsecond_deadlines() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.workspace_scan_deadline = Duration::from_micros(1);
    limits.file_index_deadline = Duration::from_micros(100);
    limits.reference_search_deadline = Duration::from_micros(500);

    assert_eq!(limits.workspace_scan_deadline, Duration::from_micros(1));
    assert_eq!(limits.file_index_deadline, Duration::from_micros(100));
    assert_eq!(limits.reference_search_deadline, Duration::from_micros(500));
    Ok(())
}

// =============================================================================
// Large Workspace Preset Edge Cases
// =============================================================================

#[test]
fn large_workspace_retains_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::large_workspace();

    // These should match defaults
    assert_eq!(limits.workspace_symbol_cap, 200);
    assert_eq!(limits.references_cap, 500);
    assert_eq!(limits.completion_cap, 100);
    assert_eq!(limits.document_symbol_cap, 500);
    assert_eq!(limits.code_lens_cap, 100);
    assert_eq!(limits.diagnostics_per_file_cap, 200);
    assert_eq!(limits.inlay_hints_cap, 500);
    Ok(())
}

#[test]
fn large_workspace_cache_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::large_workspace();

    assert_eq!(limits.ast_cache_max_entries, 100);
    assert_eq!(limits.symbol_cache_max_entries, 1000);
    Ok(())
}

#[test]
fn large_workspace_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::large_workspace();

    assert_eq!(limits.max_indexed_files, 50_000);
    assert_eq!(limits.max_total_symbols, 2_000_000);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_secs(120));
    Ok(())
}

// =============================================================================
// Constrained Environment Preset Edge Cases
// =============================================================================

#[test]
fn constrained_result_caps_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::constrained();

    // Result caps should match defaults
    assert_eq!(limits.workspace_symbol_cap, 200);
    assert_eq!(limits.references_cap, 500);
    assert_eq!(limits.completion_cap, 100);
    Ok(())
}

#[test]
fn constrained_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::constrained();

    assert_eq!(limits.ast_cache_max_entries, 50);
    assert_eq!(limits.max_indexed_files, 5_000);
    assert_eq!(limits.max_total_symbols, 100_000);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_secs(15));
    assert_eq!(limits.reference_search_deadline, Duration::from_secs(1));
    Ok(())
}

// =============================================================================
// Update from Value: Boundary Cases
// =============================================================================

#[test]
fn update_with_zero_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 0,
            "referencesCap": 0,
            "completionCap": 0,
            "astCacheMaxEntries": 0,
            "maxIndexedFiles": 0,
            "maxTotalSymbols": 0,
            "workspaceScanDeadlineMs": 0,
            "referenceSearchDeadlineMs": 0
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, 0);
    assert_eq!(limits.references_cap, 0);
    assert_eq!(limits.completion_cap, 0);
    assert_eq!(limits.ast_cache_max_entries, 0);
    assert_eq!(limits.max_indexed_files, 0);
    assert_eq!(limits.max_total_symbols, 0);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(0));
    assert_eq!(limits.reference_search_deadline, Duration::from_millis(0));
    Ok(())
}

#[test]
fn update_with_huge_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let large_num = u64::MAX / 2;
    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": large_num,
            "referencesCap": large_num,
            "completionCap": large_num,
            "astCacheMaxEntries": large_num,
            "maxIndexedFiles": large_num,
            "maxTotalSymbols": large_num,
            "workspaceScanDeadlineMs": large_num,
            "referenceSearchDeadlineMs": large_num
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, large_num as usize);
    assert_eq!(limits.references_cap, large_num as usize);
    assert_eq!(limits.completion_cap, large_num as usize);
    assert_eq!(limits.ast_cache_max_entries, large_num as usize);
    assert_eq!(limits.max_indexed_files, large_num as usize);
    assert_eq!(limits.max_total_symbols, large_num as usize);
    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(large_num));
    assert_eq!(limits.reference_search_deadline, Duration::from_millis(large_num));
    Ok(())
}

#[test]
fn update_with_missing_limits_key() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "something_else": {
            "workspaceSymbolCap": 500
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    Ok(())
}

#[test]
fn update_with_empty_limits() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "limits": {}
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    Ok(())
}

#[test]
fn update_with_null_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": serde_json::Value::Null,
            "referencesCap": serde_json::Value::Null
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    assert_eq!(limits.references_cap, 500);
    Ok(())
}

#[test]
fn update_with_string_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": "not a number",
            "referencesCap": "300"
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    assert_eq!(limits.references_cap, 500);
    Ok(())
}

#[test]
fn update_with_float_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 300.5,
            "referencesCap": 600.9
        }
    });

    limits.update_from_value(&settings);

    // as_u64() on float fails, so defaults should be preserved
    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    assert_eq!(limits.references_cap, 500);
    Ok(())
}

#[test]
fn update_with_negative_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let default_workspace_cap = limits.workspace_symbol_cap;

    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": -100,
            "referencesCap": -500
        }
    });

    limits.update_from_value(&settings);

    // as_u64() on negative values fails (out of range), so defaults preserved
    assert_eq!(limits.workspace_symbol_cap, default_workspace_cap);
    assert_eq!(limits.references_cap, 500);
    Ok(())
}

// =============================================================================
// Update from Value: Partial Updates
// =============================================================================

#[test]
fn update_subset_of_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    let settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 350,
            "maxIndexedFiles": 15000
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_symbol_cap, 350);
    assert_eq!(limits.references_cap, 500); // unchanged
    assert_eq!(limits.max_indexed_files, 15000);
    assert_eq!(limits.max_total_symbols, 500_000); // unchanged
    Ok(())
}

#[test]
fn update_only_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    let settings = serde_json::json!({
        "limits": {
            "workspaceScanDeadlineMs": 5000
        }
    });

    limits.update_from_value(&settings);

    assert_eq!(limits.workspace_scan_deadline, Duration::from_millis(5000));
    assert_eq!(limits.file_index_deadline, Duration::from_secs(5)); // unchanged
    Ok(())
}

// =============================================================================
// Clone and Debug Traits
// =============================================================================

#[test]
fn clone_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let original = LspLimits {
        workspace_symbol_cap: 123,
        references_cap: 456,
        completion_cap: 789,
        document_symbol_cap: 111,
        code_lens_cap: 222,
        diagnostics_per_file_cap: 333,
        inlay_hints_cap: 444,
        ast_cache_max_entries: 555,
        ast_cache_ttl_secs: 666,
        symbol_cache_max_entries: 777,
        max_indexed_files: 888,
        max_symbols_per_file: 999,
        max_total_symbols: 1111,
        parse_storm_threshold: 2222,
        max_file_size_bytes: 3333,
        workspace_scan_deadline: Duration::from_secs(30),
        file_index_deadline: Duration::from_secs(5),
        reference_search_deadline: Duration::from_secs(2),
        regex_scan_deadline: Duration::from_secs(1),
        fs_operation_deadline: Duration::from_millis(500),
        semantic_tokens_deadline: Duration::from_secs(2),
        code_lens_resolve_deadline: Duration::from_secs(1),
        completion_deadline: Duration::from_millis(500),
        return_partial_on_timeout: true,
        include_open_docs_when_degraded: false,
        memory_budget: perl_lsp_limits::MemoryBudget::default(),
    };

    let cloned = original.clone();

    assert_eq!(original.workspace_symbol_cap, cloned.workspace_symbol_cap);
    assert_eq!(original.references_cap, cloned.references_cap);
    assert_eq!(original.completion_cap, cloned.completion_cap);
    assert_eq!(original.max_indexed_files, cloned.max_indexed_files);
    assert_eq!(original.return_partial_on_timeout, cloned.return_partial_on_timeout);
    assert_eq!(original.include_open_docs_when_degraded, cloned.include_open_docs_when_degraded);
    Ok(())
}

#[test]
fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::default();
    let debug_str = format!("{:?}", limits);

    assert!(debug_str.contains("LspLimits"));
    assert!(debug_str.contains("workspace_symbol_cap"));
    assert!(debug_str.contains("references_cap"));
    Ok(())
}

// =============================================================================
// Degradation Flags
// =============================================================================

#[test]
fn degradation_flags_independent() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    limits.return_partial_on_timeout = true;
    limits.include_open_docs_when_degraded = false;
    assert!(limits.return_partial_on_timeout);
    assert!(!limits.include_open_docs_when_degraded);

    limits.return_partial_on_timeout = false;
    limits.include_open_docs_when_degraded = true;
    assert!(!limits.return_partial_on_timeout);
    assert!(limits.include_open_docs_when_degraded);

    Ok(())
}

#[test]
fn both_degradation_flags_true() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.return_partial_on_timeout = true;
    limits.include_open_docs_when_degraded = true;

    assert!(limits.return_partial_on_timeout);
    assert!(limits.include_open_docs_when_degraded);
    Ok(())
}

#[test]
fn both_degradation_flags_false() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    limits.return_partial_on_timeout = false;
    limits.include_open_docs_when_degraded = false;

    assert!(!limits.return_partial_on_timeout);
    assert!(!limits.include_open_docs_when_degraded);
    Ok(())
}

// =============================================================================
// Global Accessor Fallback Values
// =============================================================================

#[test]
fn global_accessors_return_defaults_on_lock_failure() -> Result<(), Box<dyn std::error::Error>> {
    // These global accessors have fallback values if lock fails
    // We test that they return sensible defaults

    let ws_cap = workspace_symbol_cap();
    assert_eq!(ws_cap, 200);

    let ref_cap = references_cap();
    assert_eq!(ref_cap, 500);

    let comp_cap = completion_cap();
    assert_eq!(comp_cap, 100);

    let doc_sym = document_symbol_cap();
    assert_eq!(doc_sym, 500);

    let code_lens = code_lens_cap();
    assert_eq!(code_lens, 100);

    let diag = diagnostics_per_file_cap();
    assert_eq!(diag, 200);

    let inlay = inlay_hints_cap();
    assert_eq!(inlay, 500);

    Ok(())
}

#[test]
fn global_deadline_accessors() -> Result<(), Box<dyn std::error::Error>> {
    let ref_deadline = reference_search_deadline();
    assert_eq!(ref_deadline, Duration::from_secs(2));

    let regex_deadline = regex_scan_deadline();
    assert_eq!(regex_deadline, Duration::from_secs(1));

    let semantic_deadline = semantic_tokens_deadline();
    assert_eq!(semantic_deadline, Duration::from_secs(2));

    let codelens_deadline = code_lens_resolve_deadline();
    assert_eq!(codelens_deadline, Duration::from_secs(1));

    let completion_dl = completion_deadline();
    assert_eq!(completion_dl, Duration::from_millis(500));

    Ok(())
}

// =============================================================================
// Duration Conversions
// =============================================================================

#[test]
fn duration_as_millis() -> Result<(), Box<dyn std::error::Error>> {
    let d = Duration::from_secs(1);
    assert_eq!(d.as_millis(), 1000);

    let d = Duration::from_millis(500);
    assert_eq!(d.as_millis(), 500);

    let d = Duration::from_micros(1);
    assert_eq!(d.as_micros(), 1);

    Ok(())
}

// =============================================================================
// Field Interactions
// =============================================================================

#[test]
fn symbol_constraints_interaction() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    // Set per-file symbol limit higher than total
    limits.max_symbols_per_file = 10_000;
    limits.max_total_symbols = 5_000;

    // Both should be settable independently (no validation in the struct)
    assert_eq!(limits.max_symbols_per_file, 10_000);
    assert_eq!(limits.max_total_symbols, 5_000);

    Ok(())
}

#[test]
fn file_and_symbol_constraints() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    limits.max_indexed_files = 1;
    limits.max_symbols_per_file = 1;
    limits.max_total_symbols = 1;

    assert_eq!(limits.max_indexed_files, 1);
    assert_eq!(limits.max_symbols_per_file, 1);
    assert_eq!(limits.max_total_symbols, 1);

    Ok(())
}

// =============================================================================
// Update from JSON: Complex Structures
// =============================================================================

#[test]
fn update_with_nested_json_preserves_structure() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let complex_settings = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 250,
            "referencesCap": 450
        },
        "other_section": {
            "key": "value",
            "nested": { "deep": "value" }
        }
    });

    limits.update_from_value(&complex_settings);

    assert_eq!(limits.workspace_symbol_cap, 250);
    assert_eq!(limits.references_cap, 450);

    // Other sections should not affect limits
    assert_eq!(limits.max_indexed_files, 10_000);

    Ok(())
}

#[test]
fn update_multiple_times_accumulates() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    let settings1 = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 300
        }
    });
    limits.update_from_value(&settings1);
    assert_eq!(limits.workspace_symbol_cap, 300);

    let settings2 = serde_json::json!({
        "limits": {
            "referencesCap": 600
        }
    });
    limits.update_from_value(&settings2);
    assert_eq!(limits.workspace_symbol_cap, 300); // preserved
    assert_eq!(limits.references_cap, 600); // updated

    Ok(())
}

#[test]
fn update_can_override_previous() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    let settings1 = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 300
        }
    });
    limits.update_from_value(&settings1);
    assert_eq!(limits.workspace_symbol_cap, 300);

    let settings2 = serde_json::json!({
        "limits": {
            "workspaceSymbolCap": 400
        }
    });
    limits.update_from_value(&settings2);
    assert_eq!(limits.workspace_symbol_cap, 400);

    Ok(())
}

// =============================================================================
// TTL and Cache Interaction
// =============================================================================

#[test]
fn cache_ttl_zero_vs_nonzero() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits1 = LspLimits::default();
    limits1.ast_cache_ttl_secs = 0;

    let mut limits2 = LspLimits::default();
    limits2.ast_cache_ttl_secs = 300;

    assert_eq!(limits1.ast_cache_ttl_secs, 0);
    assert_eq!(limits2.ast_cache_ttl_secs, 300);

    Ok(())
}

#[test]
fn cache_size_and_ttl_independent() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();

    limits.ast_cache_max_entries = 50;
    limits.ast_cache_ttl_secs = 100;

    assert_eq!(limits.ast_cache_max_entries, 50);
    assert_eq!(limits.ast_cache_ttl_secs, 100);

    limits.ast_cache_max_entries = 200;
    limits.ast_cache_ttl_secs = 600;

    assert_eq!(limits.ast_cache_max_entries, 200);
    assert_eq!(limits.ast_cache_ttl_secs, 600);

    Ok(())
}

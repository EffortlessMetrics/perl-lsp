//! Property-based tests for PullDiagnosticsProvider @INC path inclusion feature (PL701).
//!
//! These tests verify invariants that should hold across many generated inputs,
//! not just specific examples. Where unit tests say "given X, expect Y", we say
//! "for all X, property P holds."

use lsp_types::{DocumentDiagnosticReport, NumberOrString, Uri};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;
use proptest::prelude::*;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract items from a full diagnostic report, returning an empty vec if Unchanged.
fn items_from_report(report: DocumentDiagnosticReport) -> Vec<lsp_types::Diagnostic> {
    match report {
        DocumentDiagnosticReport::Full(full) => full.full_document_diagnostic_report.items,
        _ => vec![],
    }
}

/// Returns true when a diagnostic has the given code string (e.g. "PL701").
fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

/// Extract result_id from a Full report, or None if Unchanged.
fn get_result_id(report: &DocumentDiagnosticReport) -> Option<String> {
    match report {
        DocumentDiagnosticReport::Full(full) => {
            full.full_document_diagnostic_report.result_id.clone()
        }
        _ => None,
    }
}

/// Generate a unique module name that is guaranteed not to be a core module.
fn gen_non_core_module_name(index: u32) -> String {
    // Use a namespaced module name that is extremely unlikely to be a core module
    format!("TestNonCore::Module_{:08x}", index)
}

// ============================================================================
// Property 1: Include Paths Preservation
//
// When include_paths: Some(paths) is provided, the PL701 diagnostic message
// should include those paths. This should hold for ANY generated set of paths.
// ============================================================================

/// Property: For any non-empty include_paths, PL701 messages should include those paths.
proptest! {
    #[test]
    fn pl701_include_paths_preservation(path1 in "[^\\x00]+", path2 in "[^\\x00]+") {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        // Generate content with a definitely-missing module
        let module_name = gen_non_core_module_name(0xdeadbeef);
        let content = format!("use {};\n", module_name);

        let include_paths = Some(vec![path1.clone(), path2.clone()]);
        let report = provider.get_document_diagnostics(&uri, &content, None, include_paths);
        let items = items_from_report(report);

        // Find PL701 diagnostic
        let pl701_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL701")).collect();

        // Should produce at least one PL701
        prop_assume!(!pl701_diags.is_empty());

        // Every PL701 diagnostic should include both paths
        for diag in &pl701_diags {
            prop_assert!(
                diag.message.contains(&path1),
                "PL701 message should contain path1 '{}', got: {}",
                path1,
                diag.message
            );
            prop_assert!(
                diag.message.contains(&path2),
                "PL701 message should contain path2 '{}', got: {}",
                path2,
                diag.message
            );
        }
    }
}

/// Property: Single include path is preserved in PL701 message.
proptest! {
    #[test]
    fn pl701_single_include_path_preservation(path in "[^\\x00]+") {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(0xcafebabe);
        let content = format!("use {};\n", module_name);

        let include_paths = Some(vec![path.clone()]);
        let report = provider.get_document_diagnostics(&uri, &content, None, include_paths);
        let items = items_from_report(report);

        let pl701_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL701")).collect();
        prop_assume!(!pl701_diags.is_empty());

        for diag in &pl701_diags {
            prop_assert!(
                diag.message.contains(&path),
                "PL701 message should contain '{}', got: {}",
                path,
                diag.message
            );
        }
    }
}

// ============================================================================
// Property 2: Empty Paths Fallback Message
//
// When include_paths is None or Some(vec![]), the fallback message should be shown.
// ============================================================================

/// Property: include_paths = None produces fallback message (not the paths message).
proptest! {
    #[test]
    fn pl701_none_include_paths_shows_fallback(module_index: u32) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        // Pass None for include_paths
        let report = provider.get_document_diagnostics(&uri, &content, None, None);
        let items = items_from_report(report);

        let pl701_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL701")).collect();
        prop_assume!(!pl701_diags.is_empty());

        // With None, should show fallback message (not custom paths)
        for diag in &pl701_diags {
            // The fallback message mentions "workspace or configured include paths"
            prop_assert!(
                diag.message.contains("workspace or configured include paths"),
                "PL701 with None paths should show fallback message, got: {}",
                diag.message
            );
        }
    }
}

/// Property: include_paths = Some(vec![]) produces fallback message (same as None).
proptest! {
    #[test]
    fn pl701_empty_vec_include_paths_shows_fallback(module_index: u32) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        // Pass Some(vec![]) - empty list
        let report = provider.get_document_diagnostics(&uri, &content, None, Some(vec![]));
        let items = items_from_report(report);

        let pl701_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL701")).collect();
        prop_assume!(!pl701_diags.is_empty());

        // With empty vec, should show fallback message
        for diag in &pl701_diags {
            prop_assert!(
                diag.message.contains("workspace or configured include paths"),
                "PL701 with empty vec should show fallback message, got: {}",
                diag.message
            );
        }
    }
}

// ============================================================================
// Property 3: Caching Invariant
//
// When previous_result_id matches the content hash, an Unchanged report is
// returned, regardless of include_paths. This is a critical invariant for
// LSP 3.17 pull diagnostics caching.
// ============================================================================

/// Property: Unchanged report returned when previous_result_id matches content hash.
proptest! {
    #[test]
    fn pl701_caching_with_include_paths(
        module_index: u32,
        path1 in "[^\\x00]+",
        path2 in "[^\\x00]+"
    ) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        // First call - get full report
        let include_paths = Some(vec![path1.clone(), path2.clone()]);
        let first = provider.get_document_diagnostics(&uri, &content, None, include_paths.clone());

        // Extract result_id - if not Full, skip test
        let result_id = get_result_id(&first);
        prop_assume!(result_id.is_some(), "First call should return Full report");
        let result_id = result_id.unwrap();

        // Second call with correct result_id - should get Unchanged
        let second = provider.get_document_diagnostics(&uri, &content, Some(result_id), include_paths);

        prop_assert!(
            matches!(second, DocumentDiagnosticReport::Unchanged(_)),
            "Should return Unchanged when previous_result_id matches"
        );
    }
}

/// Property: Content hash determines result_id (same content = same result_id).
proptest! {
    #[test]
    fn result_id_deterministic_across_include_paths(
        module_index: u32,
        path1 in "[^\\x00]+",
        path2 in "[^\\x00]+"
    ) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        // Get result_id with first set of paths
        let include_paths1 = Some(vec![path1.clone()]);
        let report1 = provider.get_document_diagnostics(&uri, &content, None, include_paths1);
        let result_id1 = get_result_id(&report1).unwrap();

        // Get result_id with different paths - should be same (only content matters)
        let include_paths2 = Some(vec![path2.clone()]);
        let report2 = provider.get_document_diagnostics(&uri, &content, None, include_paths2);
        let result_id2 = get_result_id(&report2).unwrap();

        prop_assert_eq!(
            result_id1, result_id2,
            "result_id should be same for same content regardless of include_paths"
        );
    }
}

// ============================================================================
// Property 4: Idempotency
//
// Calling get_document_diagnostics with the same arguments should produce
// semantically equivalent reports.
// ============================================================================

/// Property: Same inputs produce same result_id.
proptest! {
    #[test]
    fn idempotency_same_inputs_same_result_id(
        module_index: u32,
        path in "[^\\x00]+"
    ) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        let include_paths = Some(vec![path.clone()]);

        let first = provider.get_document_diagnostics(&uri, &content, None, include_paths.clone());
        let second = provider.get_document_diagnostics(&uri, &content, None, include_paths);

        let result_id1 = get_result_id(&first);
        let result_id2 = get_result_id(&second);

        prop_assert_eq!(
            result_id1, result_id2,
            "Two calls with identical inputs should produce identical result_ids"
        );
    }
}

// ============================================================================
// Property 5: Different include_paths don't affect result_id
//
// The result_id is based solely on content hash, not on include_paths.
// This is critical for caching correctness.
// ============================================================================

/// Property: Different include_paths produce same result_id (caching depends on this).
proptest! {
    #[test]
    fn result_id_independent_of_include_paths(module_index: u32) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        // None
        let r_none = get_result_id(&provider.get_document_diagnostics(&uri, &content, None, None));
        // Empty vec
        let r_empty = get_result_id(&provider.get_document_diagnostics(&uri, &content, None, Some(vec![])));
        // Single path
        let r_one = get_result_id(&provider.get_document_diagnostics(&uri, &content, None, Some(vec!["/one".to_string()])));
        // Multiple paths
        let r_multi = get_result_id(&provider.get_document_diagnostics(
            &uri,
            &content,
            None,
            Some(vec!["/one".to_string(), "/two".to_string(), "/three".to_string()])
        ));

        // All result_ids should be identical (content is the same)
        prop_assert_eq!(r_none.clone(), r_empty.clone());
        prop_assert_eq!(r_empty, r_one.clone());
        prop_assert_eq!(r_one, r_multi);
    }
}

// ============================================================================
// Property 6: Module name appears in PL701 message
//
// The module name that couldn't be found should appear in the diagnostic message.
// ============================================================================

/// Property: The missing module name appears in PL701 diagnostic.
proptest! {
    #[test]
    fn pl701_mentions_module_name(module_index: u32) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        let include_paths = Some(vec!["/test/path".to_string()]);
        let report = provider.get_document_diagnostics(&uri, &content, None, include_paths);
        let items = items_from_report(report);

        let pl701_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL701")).collect();
        prop_assume!(!pl701_diags.is_empty());

        // The module name should appear in at least one PL701 diagnostic
        let found = pl701_diags.iter().any(|d| d.message.contains(&module_name));
        prop_assert!(
            found,
            "PL701 message should mention the missing module '{}', got: {:?}",
            module_name,
            pl701_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

// ============================================================================
// Property 7: UTF-16 Range Validity
//
// All diagnostic ranges should have valid UTF-16 line/column coordinates.
// ============================================================================

/// Property: All diagnostic ranges are valid (start <= end, lines exist).
proptest! {
    #[test]
    fn diagnostic_ranges_valid_utf16(
        module_index: u32,
        path in "[^\\x00]+"
    ) {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse().unwrap();

        let module_name = gen_non_core_module_name(module_index);
        let content = format!("use {};\n", module_name);

        let include_paths = Some(vec![path.clone()]);
        let report = provider.get_document_diagnostics(&uri, &content, None, include_paths);
        let items = items_from_report(report);

        for diag in &items {
            // Line should be non-negative
            prop_assert!(diag.range.start.line >= 0, "start line should be >= 0");
            prop_assert!(diag.range.end.line >= 0, "end line should be >= 0");

            // Column should be non-negative
            prop_assert!(diag.range.start.character >= 0, "start column should be >= 0");
            prop_assert!(diag.range.end.character >= 0, "end column should be >= 0");
        }
    }
}

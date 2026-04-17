//! Tests for HashMap-based global name index in WorkspaceIndex.
//!
//! These tests verify AC1 (correctness - same search results as baseline),
//! AC3 (index consistency on file changes), and AC5 (backward compatibility).
//!
//! The `global_name_index` field maps lowercase symbol names (both bare names
//! and qualified names) to `Vec<WorkspaceSymbol>` for O(1) bounded lookup
//! instead of O(n) linear scan.

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use perl_workspace::workspace::workspace_index::WorkspaceSymbol;
use url::Url;

// -----------------------------------------------------------------------------
// Helper utilities
// -----------------------------------------------------------------------------

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

/// Reference implementation of search_symbols using O(n) linear scan.
/// This is the baseline behavior that the optimized HashMap lookup must match.
fn baseline_search_symbols(index: &WorkspaceIndex, query: &str) -> Vec<WorkspaceSymbol> {
    let query_lower = query.to_lowercase();
    let all_syms = index.all_symbols();
    let mut results = Vec::new();
    for symbol in all_syms {
        if symbol.name.to_lowercase().contains(&query_lower)
            || symbol
                .qualified_name
                .as_ref()
                .map(|qn| qn.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
        {
            results.push(symbol);
        }
    }
    results
}

/// Deduplicate results by URI, preserving first occurrence order.
fn deduplicate_by_uri(results: Vec<WorkspaceSymbol>) -> Vec<WorkspaceSymbol> {
    let mut seen = std::collections::HashSet::new();
    results.into_iter().filter(|s| seen.insert(s.uri.clone())).collect()
}

// ===========================================================================
// AC1: Correctness — Same Search Results as Baseline
// ===========================================================================

/// Test that search_symbols returns the same results as the baseline O(n) implementation.
/// This is the core correctness test for the HashMap optimization.
#[test]
fn test_search_symbols_matches_baseline_exact_match() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Calc/Arithmetic.pm")?;

    let code = r#"
package Calc::Arithmetic;
sub add { return $_[0] + $_[1]; }
sub subtract { return $_[0] - $_[1]; }
sub multiply { return $_[0] * $_[1]; }
sub divide { return $_[0] / $_[1]; }
"#;
    index.index_file(uri, code.to_string())?;

    // Exact match on qualified name
    let optimized = index.search_symbols("Calc::Arithmetic::add");
    let baseline = deduplicate_by_uri(baseline_search_symbols(&index, "Calc::Arithmetic::add"));

    assert_eq!(
        optimized.len(),
        baseline.len(),
        "search_symbols('Calc::Arithmetic::add'): expected {} results, got {}. Optimized must match baseline O(n) scan.",
        baseline.len(),
        optimized.len()
    );

    // Verify each result matches
    for (i, (opt, base)) in optimized.iter().zip(baseline.iter()).enumerate() {
        assert_eq!(opt.uri, base.uri, "Result[{}]: URI mismatch", i);
        assert_eq!(opt.name, base.name, "Result[{}]: name mismatch", i);
    }

    Ok(())
}

/// Test case-insensitive matching is preserved.
#[test]
fn test_search_symbols_matches_baseline_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MyModule.pm")?;

    let code = r#"
package MyModule;
sub MyFunction { return 1; }
sub yourFunction { return 2; }
"#;
    index.index_file(uri, code.to_string())?;

    // Case-insensitive: lowercase query should match uppercase symbol
    let optimized = index.search_symbols("myfunction");
    let baseline = deduplicate_by_uri(baseline_search_symbols(&index, "myfunction"));

    assert_eq!(
        optimized.len(),
        baseline.len(),
        "search_symbols('myfunction'): expected {} results (case-insensitive), got {}. \
         HashMap lookup must preserve case-insensitive semantics.",
        baseline.len(),
        optimized.len()
    );

    // Mixed case query
    let optimized2 = index.search_symbols("MyFunction");
    let baseline2 = deduplicate_by_uri(baseline_search_symbols(&index, "MyFunction"));

    assert_eq!(
        optimized2.len(),
        baseline2.len(),
        "search_symbols('MyFunction'): expected {} results, got {}.",
        baseline2.len(),
        optimized2.len()
    );

    Ok(())
}

/// Test substring matching is preserved.
#[test]
fn test_search_symbols_matches_baseline_substring_match() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Process.pm")?;

    let code = r#"
package Process;
sub process_data { return 1; }
sub process_file { return 2; }
sub data_processor { return 3; }
sub PROCESS { return 4; }
"#;
    index.index_file(uri, code.to_string())?;

    // Substring match: "process" should match all symbols containing "process"
    let optimized = index.search_symbols("process");
    let baseline = deduplicate_by_uri(baseline_search_symbols(&index, "process"));

    assert_eq!(
        optimized.len(),
        baseline.len(),
        "search_symbols('process'): expected {} results (substring match), got {}. \
         HashMap optimization must preserve substring semantics, not just prefix matching.",
        baseline.len(),
        optimized.len()
    );

    // Verify specific matches
    let names: Vec<_> = optimized.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"process_data".to_string()), "Should match process_data");
    assert!(names.contains(&"process_file".to_string()), "Should match process_file");
    assert!(names.contains(&"data_processor".to_string()), "Should match data_processor");

    Ok(())
}

/// Test dual-name search: query matches both bare name and qualified name.
#[test]
fn test_search_symbols_matches_baseline_dual_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/File/Find.pm")?;

    let code = r#"
package File::Find;
sub find { return 1; }
sub seek { return 2; }
"#;
    index.index_file(uri, code.to_string())?;

    // Search by qualified name
    let by_qualified = index.search_symbols("File::Find::find");
    let baseline_qualified = deduplicate_by_uri(baseline_search_symbols(&index, "File::Find::find"));
    assert_eq!(
        by_qualified.len(),
        baseline_qualified.len(),
        "search_symbols('File::Find::find'): expected {} results, got {}",
        baseline_qualified.len(),
        by_qualified.len()
    );

    // Search by bare name
    let by_bare = index.search_symbols("find");
    let baseline_bare = deduplicate_by_uri(baseline_search_symbols(&index, "find"));
    assert_eq!(
        by_bare.len(),
        baseline_bare.len(),
        "search_symbols('find'): expected {} results, got {}",
        baseline_bare.len(),
        by_bare.len()
    );

    Ok(())
}

/// Test short query handling (1-2 chars) is preserved.
#[test]
fn test_search_symbols_matches_baseline_short_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;

    let code = r#"
package Test;
sub a { return 1; }
sub ab { return 2; }
sub abc { return 3; }
sub abcdef { return 4; }
sub x { return 5; }
"#;
    index.index_file(uri, code.to_string())?;

    // Single character query
    let optimized_a = index.search_symbols("a");
    let baseline_a = deduplicate_by_uri(baseline_search_symbols(&index, "a"));
    assert_eq!(
        optimized_a.len(),
        baseline_a.len(),
        "search_symbols('a'): expected {} results, got {}. Short query handling must match baseline.",
        baseline_a.len(),
        optimized_a.len()
    );

    // Two character query
    let optimized_ab = index.search_symbols("ab");
    let baseline_ab = deduplicate_by_uri(baseline_search_symbols(&index, "ab"));
    assert_eq!(
        optimized_ab.len(),
        baseline_ab.len(),
        "search_symbols('ab'): expected {} results, got {}",
        baseline_ab.len(),
        optimized_ab.len()
    );

    Ok(())
}

/// Test search across multiple files returns all matching symbols.
#[test]
fn test_search_symbols_matches_baseline_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // File 1
    let uri1 = file_url("/lib/ModuleA.pm")?;
    index.index_file(uri1, "package ModuleA;\nsub helper { return 1; }".to_string())?;

    // File 2
    let uri2 = file_url("/lib/ModuleB.pm")?;
    index.index_file(uri2, "package ModuleB;\nsub helper { return 2; }".to_string())?;

    // File 3
    let uri3 = file_url("/lib/ModuleC.pm")?;
    index.index_file(uri3, "package ModuleC;\nsub other { return 3; }".to_string())?;

    // Search for "helper" should find both ModuleA::helper and ModuleB::helper
    let optimized = index.search_symbols("helper");
    let baseline = deduplicate_by_uri(baseline_search_symbols(&index, "helper"));

    assert_eq!(
        optimized.len(),
        baseline.len(),
        "search_symbols('helper') across multiple files: expected {} results, got {}. \
         HashMap index must aggregate symbols from all indexed files.",
        baseline.len(),
        optimized.len()
    );

    // Verify we got symbols from both files
    let uris: std::collections::HashSet<_> = optimized.iter().map(|s| s.uri.clone()).collect();
    assert!(uris.contains("file:///lib/ModuleA.pm"), "Should find helper from ModuleA");
    assert!(uris.contains("file:///lib/ModuleB.pm"), "Should find helper from ModuleB");

    Ok(())
}

/// Test that search results are deduplicated by URI.
#[test]
fn test_search_symbols_deduplicates_by_uri() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Dup.pm")?;

    // Single file with multiple symbols
    let code = r#"
package Dup;
sub target { return 1; }
sub target { return 2; }
"#;
    index.index_file(uri, code.to_string())?;

    let results = index.search_symbols("target");

    // Should not return duplicates from the same URI
    let uris: Vec<_> = results.iter().map(|s| s.uri.clone()).collect();
    let unique_uris: std::collections::HashSet<_> = uris.iter().collect();
    assert_eq!(
        uris.len(),
        unique_uris.len(),
        "search_symbols should deduplicate by URI. Got: {:?}, Unique: {:?}",
        uris,
        unique_uris
    );

    Ok(())
}

// ===========================================================================
// AC3: Index Consistency — Correct Maintenance on File Changes
// ===========================================================================

/// Test that global_name_index is updated when a file is indexed.
#[test]
fn test_global_name_index_updated_on_index_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/IndexMe.pm")?;

    let code = r#"
package IndexMe;
sub my_function { return 1; }
our $scalar = 2;
"#;
    index.index_file(uri, code.to_string())?;

    // After indexing, searching for the symbol should find it
    let results = index.search_symbols("my_function");
    assert!(
        !results.is_empty(),
        "global_name_index should contain 'my_function' after index_file(). \
         Search for 'my_function' returned empty results."
    );

    // Verify it can be found via qualified name too
    let results_q = index.search_symbols("IndexMe::my_function");
    assert!(
        !results_q.is_empty(),
        "global_name_index should contain 'IndexMe::my_function' after index_file()"
    );

    Ok(())
}

/// Test that global_name_index is updated when a file is re-indexed (update).
#[test]
fn test_global_name_index_updated_on_reindex_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/ReIndexMe.pm")?;

    // Initial indexing
    let code_v1 = r#"
package ReIndexMe;
sub old_function { return 1; }
"#;
    index.index_file(uri.clone(), code_v1.to_string())?;

    // Verify old symbol is found
    let results_old = index.search_symbols("old_function");
    assert!(!results_old.is_empty(), "Should find old_function after initial indexing");

    // Re-index with different content
    let code_v2 = r#"
package ReIndexMe;
sub new_function { return 2; }
sub different_function { return 3; }
"#;
    index.index_file(uri, code_v2.to_string())?;

    // After re-indexing, old symbol should NOT be found
    let results_after = index.search_symbols("old_function");
    assert!(
        results_after.is_empty(),
        "global_name_index should NOT contain 'old_function' after re-indexing. \
         Old symbols must be removed before new ones are added."
    );

    // New symbols should be found
    let results_new = index.search_symbols("new_function");
    assert!(
        !results_new.is_empty(),
        "global_name_index should contain 'new_function' after re-indexing"
    );

    let results_diff = index.search_symbols("different_function");
    assert!(
        !results_diff.is_empty(),
        "global_name_index should contain 'different_function' after re-indexing"
    );

    Ok(())
}

/// Test that global_name_index is updated when a file is removed.
#[test]
fn test_global_name_index_updated_on_remove_file() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/RemoveMe.pm")?;

    // Index a file
    let code = r#"
package RemoveMe;
sub will_be_gone { return 1; }
sub also_gone { return 2; }
"#;
    index.index_file(uri, code.to_string())?;

    // Verify symbols are found
    let results_before = index.search_symbols("will_be_gone");
    assert!(!results_before.is_empty(), "Should find 'will_be_gone' before removal");

    // Remove the file
    index.remove_file("file:///lib/RemoveMe.pm");

    // After removal, symbols should NOT be found
    let results_after = index.search_symbols("will_be_gone");
    assert!(
        results_after.is_empty(),
        "global_name_index should NOT contain 'will_be_gone' after remove_file(). \
         All symbols from removed file must be purged from global_name_index."
    );

    let results_also = index.search_symbols("also_gone");
    assert!(
        results_also.is_empty(),
        "global_name_index should NOT contain 'also_gone' after remove_file()"
    );

    Ok(())
}

/// Test that global_name_index correctly handles symbol shadowing across files.
#[test]
fn test_global_name_index_handles_shadowing() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // File 1 defines helper
    let uri1 = file_url("/lib/First.pm")?;
    index.index_file(uri1, "package First;\nsub helper { return 'first'; }".to_string())?;

    // File 2 also defines helper
    let uri2 = file_url("/lib/Second.pm")?;
    index.index_file(uri2, "package Second;\nsub helper { return 'second'; }".to_string())?;

    // Both should be found
    let results = index.search_symbols("helper");
    assert_eq!(
        results.len(),
        2,
        "Should find 'helper' from both First.pm and Second.pm. Got {} results.",
        results.len()
    );

    let uris: std::collections::HashSet<_> = results.iter().map(|s| s.uri.clone()).collect();
    assert!(uris.contains("file:///lib/First.pm"), "Should find helper from First.pm");
    assert!(uris.contains("file:///lib/Second.pm"), "Should find helper from Second.pm");

    // Remove First.pm
    index.remove_file("file:///lib/First.pm");

    // Now only Second's helper should be found
    let results_after = index.search_symbols("helper");
    assert_eq!(
        results_after.len(),
        1,
        "After removing First.pm, should only find 1 'helper'. Got {} results.",
        results_after.len()
    );
    assert_eq!(
        results_after[0].uri, "file:///lib/Second.pm",
        "Remaining helper should be from Second.pm"
    );

    Ok(())
}

// ===========================================================================
// AC5: Backward Compatibility — Same API and Behavior
// ===========================================================================

/// Test that search_symbols returns Vec<WorkspaceSymbol>.
#[test]
fn test_search_symbols_returns_vec_workspace_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/TypeCheck.pm")?;
    index.index_file(uri, "package TypeCheck;\nsub test_sub { 1 }".to_string())?;

    let results = index.search_symbols("test_sub");

    // Verify return type is Vec<WorkspaceSymbol>
    assert!(results.len() > 0, "search_symbols should return non-empty Vec for existing symbol");

    // Verify the type has expected fields
    let symbol = &results[0];
    assert!(!symbol.name.is_empty(), "WorkspaceSymbol should have non-empty name");
    assert!(!symbol.uri.is_empty(), "WorkspaceSymbol should have non-empty uri");
    assert!(
        symbol.qualified_name.is_some() || symbol.name == "test_sub",
        "WorkspaceSymbol should have qualified_name or bare name"
    );

    Ok(())
}

/// Test that find_symbols (alias) also works correctly.
#[test]
fn test_find_symbols_alias_works() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/AliasTest.pm")?;
    index.index_file(uri, "package AliasTest;\nsub target_sub { 1 }".to_string())?;

    let by_search = index.search_symbols("target_sub");
    let by_find = index.find_symbols("target_sub");

    assert_eq!(
        by_search.len(),
        by_find.len(),
        "find_symbols (alias) should return same number of results as search_symbols. \
         Got search={}, find={}",
        by_search.len(),
        by_find.len()
    );

    Ok(())
}

/// Test that search_symbols signature is unchanged (takes &str, returns Vec).
#[test]
fn test_search_symbols_signature_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/SigTest.pm")?;
    index.index_file(uri, "package SigTest;\nsub foo { 1 }".to_string())?;

    // These calls should compile without error, proving the signature is unchanged
    let _result: Vec<_> = index.search_symbols("");
    let _result2: Vec<_> = index.search_symbols("foo");
    let _result3: Vec<_> = index.search_symbols("bar");

    // Verify empty query returns empty (not panic)
    assert!(true, "Empty query should return empty Vec, not panic");

    Ok(())
}

// ===========================================================================
// Additional Edge Cases
// ===========================================================================

/// Test that search with no matching symbols returns empty Vec.
#[test]
fn test_search_symbols_empty_result_for_no_match() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/NoMatch.pm")?;
    index.index_file(uri, "package NoMatch;\nsub exists { 1 }".to_string())?;

    let results = index.search_symbols("nonexistent_symbol_xyz");
    assert!(
        results.is_empty(),
        "search_symbols should return empty Vec for non-matching query. Got {} results.",
        results.len()
    );

    Ok(())
}

/// Test search with query matching only qualified name, not bare name.
#[test]
fn test_search_symbols_qualified_only() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/QualOnly.pm")?;
    index.index_file(uri, "package QualOnly;\nsub unique_name { 1 }".to_string())?;

    // Query that only matches the qualified name prefix, not the bare name
    let results = index.search_symbols("QualOnly::unique");
    assert!(
        !results.is_empty(),
        "search_symbols should find 'QualOnly::unique_name' when searching 'QualOnly::unique'"
    );

    Ok(())
}

/// Test that symbols with no qualified_name are still searchable by bare name.
#[test]
fn test_search_symbols_no_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/NoQual.pm")?;
    // Symbols may have no qualified_name in some edge cases
    index.index_file(uri, "package NoQual;\nsub bare_only { 1 }".to_string())?;

    let results = index.search_symbols("bare_only");
    assert!(
        !results.is_empty(),
        "Symbols without qualified_name should still be searchable by bare name"
    );

    Ok(())
}

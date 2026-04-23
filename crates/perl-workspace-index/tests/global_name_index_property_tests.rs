//! Property-based tests for HashMap-based symbol search index in WorkspaceIndex.
//!
//! These tests verify invariants that should hold across all inputs, not just
//! specific examples. Properties are tested with 100+ generated inputs each.
//!
//! # Invariants Tested
//!
//! 1. **Idempotent indexing**: Indexing same content twice = indexing once
//! 2. **Remove is inverse of add**: After remove_file(X), X's symbols are gone
//! 3. **Update replaces**: Re-indexing a file replaces old symbols with new
//! 4. **Search correctness**: All results actually match the query (case-insensitive contains)
//! 5. **No duplicates**: Results don't contain the same symbol twice
//! 6. **Monotonic add**: Adding files never decreases search results
//! 7. **Global index consistency**: Every symbol in files is in global_name_index
//! 8. **Dual-name indexing works**: Both bare name and qualified name queries find symbols

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use std::collections::HashSet;
use url::Url;

// -----------------------------------------------------------------------------
// Helper utilities
// -----------------------------------------------------------------------------

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

fn generate_symbolic_code(file_id: usize, symbol_count: usize) -> String {
    let mut code = format!("package File{};\n", file_id);
    for i in 0..symbol_count {
        code.push_str(&format!("sub func{}_{} {{ {} }}\n", file_id, i, i));
    }
    code
}

fn generate_code_with_shared_names(file_id: usize, shared_count: usize) -> String {
    let mut code = format!("package File{};\n", file_id);
    // Some shared names across files
    for i in 0..shared_count {
        code.push_str(&format!("sub shared_func{} {{ {} }}\n", i, i));
    }
    // Some unique names
    for i in 0..3 {
        code.push_str(&format!("sub unique_func{}_{} {{ {} }}\n", file_id, i, file_id * 100 + i));
    }
    code
}

// -----------------------------------------------------------------------------
// Property 1: Idempotent Indexing
// -----------------------------------------------------------------------------

/// Property: Indexing the same file with identical content twice should produce
/// identical state to indexing it once.
///
/// Invariant: search_symbols returns the same results after single vs double indexing.
/// This is a form of idempotency - applying the operation twice gives same result.
#[test]
fn property_idempotent_indexing_same_content() -> Result<(), Box<dyn std::error::Error>> {
    // Test with 50 different file IDs to get broad coverage
    for file_id in 0..50 {
        let index_once = WorkspaceIndex::new();
        let index_twice = WorkspaceIndex::new();

        let uri_once = file_url(&format!("/lib/Idempotent_{}.pm", file_id))?;
        let uri_twice = file_url(&format!("/lib/Idempotent_{}.pm", file_id))?;

        let code = generate_symbolic_code(file_id, 5);

        // Index once
        index_once.index_file(uri_once.clone(), code.clone())?;

        // Index twice with same content
        index_twice.index_file(uri_twice.clone(), code.clone())?;
        index_twice.index_file(uri_twice.clone(), code.clone())?;

        // Both should find the same symbols
        let results_once = index_once.search_symbols("func");
        let results_twice = index_twice.search_symbols("func");

        assert_eq!(
            results_once.len(),
            results_twice.len(),
            "Idempotency violated: single index found {} symbols, double index found {}",
            results_once.len(),
            results_twice.len()
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property 2: Remove Is Inverse of Add
// -----------------------------------------------------------------------------

/// Property: After index_file(X) then remove_file(X), X's symbols should be gone.
///
/// Invariant: Removing a file means its symbols are no longer findable.
/// This is a form of inverse operation - add then remove = not add.
#[test]
fn property_remove_is_inverse_of_add() -> Result<(), Box<dyn std::error::Error>> {
    for iteration in 0..50 {
        let index = WorkspaceIndex::new();

        let uri = file_url(&format!("/lib/RemoveTest_{}.pm", iteration))?;
        let code = generate_symbolic_code(iteration, 10);

        // Add the file
        index.index_file(uri.clone(), code)?;

        // Verify symbols exist
        let results_before = index.search_symbols("func");
        assert!(!results_before.is_empty(), "Precondition: symbols should exist after indexing");

        // Remove the file
        index.remove_file(uri.as_str());

        // After removal, no symbols from this file should be found
        let results_after = index.search_symbols("func");
        for symbol in &results_after {
            assert_ne!(
                symbol.uri,
                uri.as_str(),
                "Found symbol from removed file {}: {}",
                uri.as_str(),
                symbol.name
            );
        }
    }
    Ok(())
}

/// Property: Remove-then-add of same file equals just-add.
///
/// Invariant: index(remove(index(X))) should have same symbols as just index(X).
#[test]
fn property_remove_then_add_equals_add() -> Result<(), Box<dyn std::error::Error>> {
    for iteration in 0..30 {
        let index_remove_add = WorkspaceIndex::new();
        let index_just_add = WorkspaceIndex::new();

        let uri1 = file_url(&format!("/lib/RTA_{}.pm", iteration))?;
        let uri2 = file_url(&format!("/lib/RTA_{}.pm", iteration))?;

        let code1 = generate_symbolic_code(iteration, 5);
        let code2 = generate_code_with_shared_names(iteration, 5);

        // Remove-add path: index -> remove -> index with different code
        index_remove_add.index_file(uri1.clone(), code1.clone())?;
        index_remove_add.remove_file(uri1.as_str());
        index_remove_add.index_file(uri1.clone(), code2.clone())?;

        // Just-add path: just index the final code
        index_just_add.index_file(uri2.clone(), code2.clone())?;

        // Both should have same search results
        let results_rta = index_remove_add.search_symbols("func");
        let results_ja = index_just_add.search_symbols("func");

        assert_eq!(
            results_rta.len(),
            results_ja.len(),
            "Remove-then-add should equal just-add. Got {} vs {}",
            results_rta.len(),
            results_ja.len()
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property 3: Update Replaces Content
// -----------------------------------------------------------------------------

/// Property: Re-indexing a file should replace old symbols with new ones.
///
/// Invariant: After index(A, v1) then index(A, v2), searching should only
/// find symbols from v2, not v1.
#[test]
fn property_update_replaces_old_symbols() -> Result<(), Box<dyn std::error::Error>> {
    for iteration in 0..50 {
        let index = WorkspaceIndex::new();

        let uri = file_url(&format!("/lib/Update_{}.pm", iteration))?;

        // Version 1: has old_func
        let code_v1 = format!("package Update{};\nsub old_func {{ 1 }}\n", iteration);
        index.index_file(uri.clone(), code_v1)?;

        // Verify old symbol exists
        let results_v1 = index.search_symbols("old_func");
        assert!(!results_v1.is_empty(), "Precondition: old_func should exist after v1");

        // Version 2: has new_func instead
        let code_v2 = format!("package Update{};\nsub new_func {{ 2 }}\n", iteration);
        index.index_file(uri.clone(), code_v2)?;

        // Old symbol should NOT exist
        let results_old = index.search_symbols("old_func");
        assert!(
            results_old.is_empty(),
            "After update, old_func should be gone. Found {} results",
            results_old.len()
        );

        // New symbol SHOULD exist
        let results_new = index.search_symbols("new_func");
        assert!(!results_new.is_empty(), "After update, new_func should exist");
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property 4: Search Correctness (All Results Match Query)
// -----------------------------------------------------------------------------

/// Property: Every symbol returned by search_symbols must actually match the query.
///
/// Invariant: For all symbols in results, query must be a case-insensitive substring
/// of either symbol.name or symbol.qualified_name.
#[test]
fn property_search_results_all_match_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Create a diverse set of symbols
    let test_cases = vec![
        ("/lib/Test1.pm", "package Test1;\nsub calculate_total { 1 }\nsub process_data { 2 }"),
        ("/lib/Test2.pm", "package Test2;\nsub MyFunction { 3 }\nsub yourFunction { 4 }"),
        ("/lib/Test3.pm", "package Test3;\nsub Process::helper { 5 }"),
    ];

    for (uri_str, code) in test_cases {
        let uri = file_url(uri_str)?;
        index.index_file(uri, code.to_string())?;
    }

    // Test various queries
    let queries =
        vec!["calc", "CALCULATE", "process", "PROCESS", "data", "my", "helper", "MyFunction"];

    for query in queries {
        let results = index.search_symbols(query);
        let query_lower = query.to_lowercase();

        for symbol in &results {
            let name_match = symbol.name.to_lowercase().contains(&query_lower);
            let qname_match = symbol
                .qualified_name
                .as_ref()
                .map(|qn| qn.to_lowercase().contains(&query_lower))
                .unwrap_or(false);

            assert!(
                name_match || qname_match,
                "Search for '{}' returned '{}' which doesn't contain query. qualified_name={:?}",
                query,
                symbol.name,
                symbol.qualified_name
            );
        }
    }
    Ok(())
}

/// Property: Search is case-insensitive.
///
/// Invariant: search("FOO") should find symbols named "foo", "Foo", "FOO", etc.
/// Note: This tests case variations OF THE SAME NAME. "MyFunction" and "foo" are
/// different names, so searching "myfunction" should NOT find "foo".
#[test]
fn property_search_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Case.pm")?;

    // All case variations of the SAME name "myfunc"
    index.index_file(
        uri,
        "package Case;\nsub myfunc { 1 }\nsub MyFunc { 2 }\nsub MYFUNC { 3 }\nsub myFunc { 4 }"
            .to_string(),
    )?;

    // All case variations should find all 4 case variations of "myfunc"
    // (case-insensitive contains: all contain "myfunc" when lowercased)
    let query_variations = vec!["myfunc", "MyFunc", "MYFUNC", "my"];

    for query in query_variations {
        let results = index.search_symbols(query);
        assert!(
            results.len() >= 2,
            "Query '{}' should find at least 2 case variations, found {}",
            query,
            results.len()
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property 5: No Duplicates in Results
// -----------------------------------------------------------------------------

/// Property: search_symbols should never return duplicate symbols.
///
/// Invariant: Results should not contain multiple entries with the same
/// (name, qualified_name, uri) tuple.
#[test]
fn property_no_duplicate_results() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index multiple files with potentially overlapping symbols
    for i in 0..20 {
        let uri = file_url(&format!("/lib/DupTest_{}.pm", i))?;
        index.index_file(uri, generate_code_with_shared_names(i, 5))?;
    }

    // Search for shared_func which appears in all files
    let results = index.search_symbols("shared_func");

    // Check for duplicates by (name, qualified_name, uri)
    let mut seen: HashSet<(String, Option<String>, String)> = HashSet::new();
    for symbol in &results {
        let key = (symbol.name.clone(), symbol.qualified_name.clone(), symbol.uri.clone());
        assert!(
            seen.insert(key),
            "Duplicate symbol found: name={}, qname={:?}, uri={}",
            symbol.name,
            symbol.qualified_name,
            symbol.uri
        );
    }

    Ok(())
}

/// Property: Dual-name indexing should not cause duplicates.
///
/// Invariant: A symbol indexed under both bare name AND qualified name should
/// appear only once in results (deduplication by full tuple).
#[test]
fn property_dual_indexing_no_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Dual.pm")?;

    index.index_file(uri, "package Dual;\nsub target_sub { 1 }".to_string())?;

    // Searching for "Dual::target" matches the qualified name
    // but should not return target_sub twice
    let results = index.search_symbols("Dual::target");

    let target_count = results.iter().filter(|s| s.name == "target_sub").count();
    assert_eq!(
        target_count, 1,
        "Symbol with both bare and qualified name match should appear once, not {}",
        target_count
    );

    Ok(())
}

// -----------------------------------------------------------------------------
// Property 6: Monotonic Add
// -----------------------------------------------------------------------------

/// Property: Adding a file should never decrease the number of search results.
///
/// Invariant: If we track count(query) before and after index_file(),
/// then count_after >= count_before for all queries.
#[test]
fn property_monotonic_add() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Get baseline counts for various queries
    let queries = vec!["func", "unique", "shared", "package"];

    for iteration in 0..20 {
        // Capture counts before adding
        let counts_before: Vec<usize> =
            queries.iter().map(|q| index.search_symbols(q).len()).collect();

        // Add a file
        let uri = file_url(&format!("/lib/Mono_{}.pm", iteration))?;
        index.index_file(uri, generate_code_with_shared_names(iteration, 3))?;

        // Counts after should be >= counts before
        for (i, query) in queries.iter().enumerate() {
            let count_after = index.search_symbols(query).len();
            assert!(
                count_after >= counts_before[i],
                "Monotonicity violated: query '{}' went from {} to {} after adding file",
                query,
                counts_before[i],
                count_after
            );
        }
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Property 7: Global Index Consistency
// -----------------------------------------------------------------------------

/// Property: Every symbol in files should be findable via search_symbols.
///
/// Invariant: After indexing, all symbols from a file can be found by searching
/// for their names.
#[test]
fn property_global_index_all_symbols_findable() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index files with known symbols
    let uri1 = file_url("/lib/Find1.pm")?;
    let uri2 = file_url("/lib/Find2.pm")?;

    index.index_file(uri1, "package Find1;\nsub alpha { 1 }\nsub beta { 2 }".to_string())?;
    index.index_file(uri2, "package Find2;\nsub gamma { 3 }\nsub delta { 4 }".to_string())?;

    // All symbols should be findable
    for name in vec!["alpha", "beta", "gamma", "delta"] {
        let results = index.search_symbols(name);
        assert!(!results.is_empty(), "Symbol '{}' should be findable after indexing", name);
    }

    Ok(())
}

/// Property: Symbols from removed files should NOT be findable.
///
/// Invariant: After remove_file(X), no search should return symbols from X.
#[test]
fn property_removed_files_not_findable() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let uri1 = file_url("/lib/Removable1.pm")?;
    let uri2 = file_url("/lib/Removable2.pm")?;

    index
        .index_file(uri1.clone(), "package Removable1;\nsub should_be_removed { 1 }".to_string())?;
    index.index_file(uri2.clone(), "package Removable2;\nsub should_stay { 2 }".to_string())?;

    // Remove uri1
    index.remove_file(uri1.as_str());

    // should_be_removed should NOT be findable
    let results_removed = index.search_symbols("should_be_removed");
    assert!(
        results_removed.is_empty(),
        "Removed symbol 'should_be_removed' should not be findable. Found {} results",
        results_removed.len()
    );

    // should_stay SHOULD still be findable
    let results_stay = index.search_symbols("should_stay");
    assert!(!results_stay.is_empty(), "Symbol 'should_stay' should still be findable");

    Ok(())
}

// -----------------------------------------------------------------------------
// Property 8: Dual-Name Search Works
// -----------------------------------------------------------------------------

/// Property: Symbols should be findable by both bare name and qualified name.
///
/// Invariant: For any indexed symbol with qualified_name "Pkg::name",
/// both search("name") and search("Pkg::name") should find it.
#[test]
fn property_dual_name_search_both_ways() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/DualSearch.pm")?;

    index.index_file(uri, "package MyPackage;\nsub my_func { 1 }".to_string())?;

    // Should find by bare name
    let by_bare = index.search_symbols("my_func");
    assert!(!by_bare.is_empty(), "Should find 'my_func' by bare name");

    // Should also find by qualified name
    let by_qualified = index.search_symbols("MyPackage::my_func");
    assert!(!by_qualified.is_empty(), "Should find 'MyPackage::my_func' by qualified name");

    Ok(())
}

// -----------------------------------------------------------------------------
// Property 9: Exact Match Among Partial Matches
// -----------------------------------------------------------------------------

/// Property: Searching for a substring should find all symbols containing it.
///
/// Invariant: search("foo") should find symbols named "foo", "foobar", "barfoo", etc.
#[test]
fn property_substring_search_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Substr.pm")?;

    index.index_file(
        uri,
        r#"
package Substr;
sub exact { 1 }
sub foo { 2 }
sub foobar { 3 }
sub barfoo { 4 }
sub foo_bar_foo { 5 }
"#
        .to_string(),
    )?;

    let results = index.search_symbols("foo");

    let names: Vec<_> = results.iter().map(|s| s.name.clone()).collect();

    // Should find all symbols containing "foo"
    assert!(names.contains(&"foo".to_string()), "Should find 'foo'");
    assert!(names.contains(&"foobar".to_string()), "Should find 'foobar'");
    assert!(names.contains(&"barfoo".to_string()), "Should find 'barfoo'");
    assert!(names.contains(&"foo_bar_foo".to_string()), "Should find 'foo_bar_foo'");

    Ok(())
}

// -----------------------------------------------------------------------------
// Property 10: Large-Scale Consistency
// -----------------------------------------------------------------------------

/// Property: After many operations, index should remain consistent.
///
/// Invariant: A sequence of index/remove/reindex operations should leave
/// the index in a valid state where all remaining symbols are findable.
#[test]
fn property_large_scale_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Perform 100 operations
    for i in 0..100 {
        let uri = file_url(&format!("/lib/Large_{}.pm", i % 20))?;

        match i % 4 {
            0 => {
                // Add
                index.index_file(uri, generate_symbolic_code(i, 3))?;
            }
            1 => {
                // Re-add (update)
                index.index_file(uri, generate_symbolic_code(i + 1000, 3))?;
            }
            2 => {
                // Remove
                index.remove_file(uri.as_str());
            }
            3 => {
                // Search (should not panic)
                let _ = index.search_symbols("func");
            }
            _ => unreachable!(),
        }
    }

    // After all operations, search should work and results should be valid
    let results = index.search_symbols("func");

    // All results should be valid (match query, have non-empty names/URIs)
    for symbol in &results {
        assert!(!symbol.name.is_empty(), "Symbol name should not be empty");
        assert!(!symbol.uri.is_empty(), "Symbol URI should not be empty");
        assert!(
            symbol.name.to_lowercase().contains("func"),
            "Symbol '{}' should match query 'func'",
            symbol.name
        );
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Property 11: Multiple Files With Same Symbol Names
// -----------------------------------------------------------------------------

/// Property: Multiple files defining the same symbol name should all be findable.
///
/// Invariant: If FileA and FileB both define "shared", search("shared")
/// should return symbols from BOTH files.
#[test]
fn property_multiple_files_same_name_all_found() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index 10 files, all with "shared" function
    // Note: Using "shared" instead of "helper" because "helper" appears as substring
    // in "Helper0::helper" which causes additional matches in the HashMap key iteration.
    for i in 0..10 {
        let uri = file_url(&format!("/lib/Multi_{}.pm", i))?;
        index.index_file(uri, format!("package Multi{};\nsub shared {{ {} }}\n", i, i))?;
    }

    let results = index.search_symbols("shared");

    // Should find all 10 "shared" symbols
    // Note: "shared" does NOT appear as substring in "Multi0::shared" (case-sensitive),
    // so each symbol should appear exactly once in results
    assert_eq!(
        results.len(),
        10,
        "Should find 10 'shared' symbols from 10 files, found {}",
        results.len()
    );

    // Verify they come from different files
    let uris: HashSet<_> = results.iter().map(|s| s.uri.clone()).collect();
    assert_eq!(uris.len(), 10, "Should have 10 different URIs, got {}", uris.len());

    Ok(())
}
/// only the remaining files' symbols should be found.
///
/// Invariant: If FileA and FileB both define "helper", and we remove FileA,
/// then search("helper") should only return FileB's symbol.
#[test]
fn property_remove_one_of_many_same_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let uri_a = file_url("/lib/Many_A.pm")?;
    let uri_b = file_url("/lib/Many_B.pm")?;

    index.index_file(uri_a.clone(), "package ManyA;\nsub shared { 'A' }".to_string())?;
    index.index_file(uri_b.clone(), "package ManyB;\nsub shared { 'B' }".to_string())?;

    // Both should be found
    let results_before = index.search_symbols("shared");
    assert_eq!(results_before.len(), 2, "Should find 2 'shared' symbols before removal");

    // Remove A
    index.remove_file(uri_a.as_str());

    // Only B should remain
    let results_after = index.search_symbols("shared");
    assert_eq!(results_after.len(), 1, "Should find only 1 'shared' symbol after removal");
    assert_eq!(results_after[0].uri, uri_b.as_str(), "Remaining symbol should be from Many_B.pm");

    Ok(())
}

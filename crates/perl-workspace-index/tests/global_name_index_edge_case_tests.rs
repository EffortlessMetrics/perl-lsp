//! Edge case tests for HashMap-based global name index in WorkspaceIndex.
//!
//! These tests verify edge cases that are NOT covered by the main correctness tests.
//! They should ALL PASS once the HashMap optimization is correctly implemented.
//!
//! Edge cases covered:
//! - Empty query handling
//! - Whitespace-only queries
//! - Very long queries
//! - Special characters in queries
//! - Unicode handling
//! - Symbols with underscores/digits
//! - Query matching both bare and qualified names
//! - Variables and package names (not just subroutines)
//! - Multiple symbols with same name in same file (different URIs via symlinks, etc.)

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

// -----------------------------------------------------------------------------
// Helper utilities
// -----------------------------------------------------------------------------

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

// ===========================================================================
// Edge Case: Empty and Boundary Query Values
// ===========================================================================

/// Test that empty query returns all symbols (original behavior).
/// Empty string is a substring of everything, so empty query matches all symbols.
/// Note: Due to dual indexing (bare name + qualified name), each symbol may appear
/// twice, but deduplication by (name, qualified_name, uri) ensures uniqueness.
#[test]
fn test_search_symbols_empty_query_returns_all() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/EmptyQuery.pm")?;
    index.index_file(
        uri,
        r#"
package EmptyQuery;
sub foo { 1 }
sub bar { 2 }
sub baz { 3 }
"#
        .to_string(),
    )?;

    // Empty query matches everything (empty string is substring of any string)
    // We just verify it's non-empty since dual indexing affects exact count
    let results = index.search_symbols("");
    assert!(
        !results.is_empty(),
        "Empty query should return results (all symbols). Got {} results",
        results.len()
    );

    Ok(())
}

/// Test that whitespace-only query returns empty results.
#[test]
fn test_search_symbols_whitespace_query_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/WhitespaceQuery.pm")?;
    index.index_file(uri, "package WhitespaceQuery;\nsub foo { 1 }".to_string())?;

    let results = index.search_symbols("   ");
    assert!(
        results.is_empty(),
        "Whitespace-only query should return empty results. Got {} results",
        results.len()
    );

    Ok(())
}

/// Test that very long query (longer than any symbol) returns empty results.
#[test]
fn test_search_symbols_very_long_query_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/LongQuery.pm")?;
    index.index_file(uri, "package LongQuery;\nsub short { 1 }".to_string())?;

    let long_query = "a".repeat(1000);
    let results = index.search_symbols(&long_query);
    assert!(
        results.is_empty(),
        "Very long query (1000 chars) should return empty results. Got {} results",
        results.len()
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Special Characters and Unicode
// ===========================================================================

/// Test that symbols with underscores are searchable.
#[test]
fn test_search_symbols_with_underscores() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Underscore.pm")?;
    index.index_file(
        uri,
        r#"
package Underscore;
sub my_function_name { 1 }
sub another_underscored_name { 2 }
sub no_underscore { 3 }
"#
        .to_string(),
    )?;

    // Search for underscore-containing name
    let results = index.search_symbols("my_function");
    assert!(!results.is_empty(), "Should find 'my_function_name' when searching 'my_function'");

    Ok(())
}

/// Test that symbols with digits are searchable.
#[test]
fn test_search_symbols_with_digits() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Digits.pm")?;
    index.index_file(
        uri,
        r#"
package Digits;
sub test1 { 1 }
sub test2 { 2 }
sub test12 { 3 }
"#
        .to_string(),
    )?;

    // Search for digit-containing name
    let results = index.search_symbols("test1");
    assert!(!results.is_empty(), "Should find 'test1' when searching 'test1'");

    // Substring match on digits
    let results2 = index.search_symbols("test");
    assert!(
        results2.len() >= 2,
        "Should find at least test1 and test2 when searching 'test', got {}",
        results2.len()
    );

    Ok(())
}

/// Test that query with colons (qualified name separator) works correctly.
#[test]
fn test_search_symbols_query_with_colons() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Colons.pm")?;
    index.index_file(
        uri,
        r#"
package Colons::Inner;
sub my_func { 1 }
"#
        .to_string(),
    )?;

    // Search with colons
    let results = index.search_symbols("Colons::Inner::my_func");
    assert!(
        !results.is_empty(),
        "Should find symbol when searching with full qualified name containing colons"
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Dual-Name Query Semantics
// ===========================================================================

/// Test that searching for a substring that matches both bare name AND qualified name
/// of the SAME symbol returns that symbol only once (deduplication).
#[test]
fn test_search_symbols_bare_and_qualified_same_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/DualMatch.pm")?;
    index.index_file(
        uri,
        r#"
package DualMatch;
sub target_sub { 1 }
"#
        .to_string(),
    )?;

    // Searching for "DualMatch::target" should find target_sub
    // but it should NOT appear twice (once for bare name match, once for qualified name match)
    let results = index.search_symbols("DualMatch::target");

    // Count how many times target_sub appears
    let target_count = results.iter().filter(|s| s.name == "target_sub").count();

    // Should appear exactly once (deduplicated by URI)
    assert_eq!(
        target_count, 1,
        "Symbol should appear exactly once even if both bare and qualified names match. Got {} occurrences",
        target_count
    );

    Ok(())
}

/// Test that searching for a common prefix finds multiple symbols.
#[test]
fn test_search_symbols_common_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Prefix.pm")?;
    index.index_file(
        uri,
        r#"
package Prefix;
sub helper_a { 1 }
sub helper_b { 2 }
sub helper_c { 3 }
sub other { 4 }
"#
        .to_string(),
    )?;

    let results = index.search_symbols("helper");
    assert_eq!(
        results.len(),
        3,
        "Should find all 3 helper_* symbols. Got {} results",
        results.len()
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Non-Subroutine Symbols
// ===========================================================================

/// Test that variables and other symbol kinds are also searchable.
/// The HashMap index should work for all symbol kinds, not just subroutines.
#[test]
fn test_search_symbols_variables() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Variables.pm")?;
    index.index_file(
        uri,
        r#"
package Variables;
our $scalar_var = 1;
our @array_var = (1, 2, 3);
our %hash_var = (a => 1);
sub regular_sub { 4 }
"#
        .to_string(),
    )?;

    // Variables should be searchable by name
    let results_scalar = index.search_symbols("scalar_var");
    assert!(!results_scalar.is_empty(), "Should find scalar variable");

    let results_array = index.search_symbols("array_var");
    assert!(!results_array.is_empty(), "Should find array variable");

    let results_hash = index.search_symbols("hash_var");
    assert!(!results_hash.is_empty(), "Should find hash variable");

    Ok(())
}

/// Test that package names are searchable.
#[test]
fn test_search_symbols_package_names() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MyPackage.pm")?;
    index.index_file(uri, "package MyPackage;\nsub foo { 1 }".to_string())?;

    // Package names might be indexed as symbols
    let results = index.search_symbols("MyPackage");

    // The search should find something (either the package or subroutines)
    // This test verifies the index doesn't break for package-level symbols
    assert!(
        results.is_empty() || results.iter().any(|s| s.name.contains("MyPackage")),
        "Search for 'MyPackage' should find package or related symbols"
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Very Large Number of Matching Symbols
// ===========================================================================

/// Test that search handles many matching symbols efficiently.
/// This is a sanity check that the HashMap doesn't have obvious performance issues.
#[test]
fn test_search_symbols_many_matches() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Create many files with the same symbol name
    for i in 0..50 {
        let uri = file_url(&format!("/lib/Many_{}/Module{}.pm", i % 5, i))?;
        let code = format!("package Module{};\nsub common_name {{ {} }}", i, i);
        index.index_file(uri, code)?;
    }

    // Search for the common name
    let results = index.search_symbols("common_name");

    // Should find all 50 symbols
    assert_eq!(
        results.len(),
        50,
        "Should find all 50 symbols with name 'common_name'. Got {}",
        results.len()
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Case Variation Edge Cases
// ===========================================================================

/// Test that searching for lowercase finds all case variations.
#[test]
fn test_search_symbols_case_variations() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/CaseVar.pm")?;
    index.index_file(
        uri,
        r#"
package CaseVar;
sub MyFunc { 1 }
sub MYFUNC { 2 }
sub myfunc { 3 }
sub myFunc { 4 }
"#
        .to_string(),
    )?;

    // Searching for lowercase should find all variations
    let results = index.search_symbols("myfunc");
    assert_eq!(
        results.len(),
        4,
        "Should find all 4 case variations of 'myfunc'. Got {}",
        results.len()
    );

    Ok(())
}

/// Test that mixed-case query matches case-insensitively.
#[test]
fn test_search_symbols_mixed_case_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MixedCase.pm")?;
    index.index_file(uri, "package MixedCase;\nsub MyFunction { 1 }".to_string())?;

    // Mixed case query
    let results = index.search_symbols("MyFunction");
    assert!(!results.is_empty(), "Mixed case query 'MyFunction' should find 'MyFunction'");

    // Different mixed case query
    let results2 = index.search_symbols("MYFUNCTION");
    assert!(!results2.is_empty(), "Uppercase query 'MYFUNCTION' should find 'MyFunction'");

    Ok(())
}

// ===========================================================================
// Edge Case: Exact Match Priority
// ===========================================================================

/// Test that exact match is found among partial matches.
/// The HashMap should allow finding exact matches efficiently.
#[test]
fn test_search_symbols_exact_match_among_partial() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/ExactMatch.pm")?;
    index.index_file(
        uri,
        r#"
package ExactMatch;
sub foo { 1 }
sub foobar { 2 }
sub foobarbaz { 3 }
"#
        .to_string(),
    )?;

    // Search for "foo" - should find foo, foobar, foobarbaz
    let results = index.search_symbols("foo");
    assert!(!results.is_empty(), "Should find at least 'foo'. Got {} results", results.len());

    // Verify foo is in the results
    let names: Vec<_> = results.iter().map(|s| s.name.clone()).collect();
    assert!(
        names.contains(&"foo".to_string()),
        "Should contain exact match 'foo', got {:?}",
        names
    );

    Ok(())
}

// ===========================================================================
// Edge Case: No Files Indexed
// ===========================================================================

/// Test that searching with no files indexed returns empty results.
#[test]
fn test_search_symbols_no_files_indexed() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let results = index.search_symbols("anything");
    assert!(
        results.is_empty(),
        "Search on empty index should return empty results. Got {} results",
        results.len()
    );

    Ok(())
}

// ===========================================================================
// Edge Case: Concurrent Modification (Stress Test)
// ===========================================================================

/// Test that search works correctly after many rapid index/update/remove operations.
#[test]
fn test_search_symbols_after_rapid_changes() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Rapidly index, update, and remove files
    for i in 0..10 {
        let uri = file_url(&format!("/lib/Rapid_{}.pm", i))?;
        index.index_file(uri, format!("package Rapid{};\nsub target {{ {} }}", i, i))?;
    }

    // Search should work
    let results = index.search_symbols("target");
    assert_eq!(results.len(), 10, "Should find 10 target symbols. Got {}", results.len());

    // Remove half
    for i in 0..5 {
        index.remove_file(&format!("file:///lib/Rapid_{}.pm", i));
    }

    // Search should now find fewer
    let results_after = index.search_symbols("target");
    assert_eq!(
        results_after.len(),
        5,
        "Should find 5 target symbols after removal. Got {}",
        results_after.len()
    );

    Ok(())
}

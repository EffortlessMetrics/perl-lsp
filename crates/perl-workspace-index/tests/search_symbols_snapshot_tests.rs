//! Snapshot tests for search_symbols output in perl-workspace-index.
//!
//! These tests capture the output of `search_symbols()` for representative inputs,
//! enabling detection of any output changes (whether intentional or bugs).
//!
//! Run with: cargo test -p perl-workspace-index --test search_symbols_snapshot_tests
//! Update snapshots: INSTA_UPDATE=always cargo test -p perl-workspace-index --test search_symbols_snapshot_tests

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

/// Helper to create a file URL
fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

/// Helper to serialize search results for snapshot comparison.
/// Returns a sorted, deterministic representation of search results.
fn serialize_results(
    results: &[perl_workspace::workspace::workspace_index::WorkspaceSymbol],
) -> String {
    use std::collections::BTreeSet;

    // Sort results by (uri, name, qualified_name) for deterministic output
    let mut sorted: Vec<String> = results
        .iter()
        .map(|s| {
            format!(
                "({},{},{},{:?})",
                s.name,
                s.uri,
                s.qualified_name.as_ref().unwrap_or(&"<none>".to_string()),
                s.kind
            )
        })
        .collect();
    sorted.sort();

    // Deduplicate using BTreeSet
    let unique: BTreeSet<String> = sorted.into_iter().collect();

    format!("{:?}", unique.into_iter().collect::<Vec<_>>())
}

// =============================================================================
// Snapshot tests for search_symbols
// =============================================================================

#[test]
fn snapshot_search_exact_name_match() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Calc/Arithmetic.pm")?;

    let code = r#"
package Calc::Arithmetic;
sub add { return $_[0] + $_[1]; }
sub subtract { return $_[0] - $_[1]; }
"#;
    index.index_file(uri, code.to_string())?;

    // Exact match on "add"
    let results = index.search_symbols("add");
    insta::assert_snapshot!("exact_name_match", serialize_results(&results));
    Ok(())
}

#[test]
fn snapshot_search_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/MyModule.pm")?;

    let code = r#"
package MyModule;
sub MyFunction { return 1; }
sub yourFunction { return 2; }
sub LOWERCASE { return 3; }
"#;
    index.index_file(uri, code.to_string())?;

    // Case-insensitive: lowercase query matches uppercase symbol
    let results = index.search_symbols("myfunction");
    insta::assert_snapshot!("case_insensitive_lowercase_query", serialize_results(&results));

    // Case-insensitive: uppercase query matches lowercase symbol
    let results2 = index.search_symbols("MYFUNCTION");
    insta::assert_snapshot!("case_insensitive_uppercase_query", serialize_results(&results2));

    // Mixed case query
    let results3 = index.search_symbols("MyFunction");
    insta::assert_snapshot!("case_insensitive_mixed_query", serialize_results(&results3));

    Ok(())
}

#[test]
fn snapshot_search_substring_match() -> Result<(), Box<dyn std::error::Error>> {
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

    // Substring match: "process" should match multiple symbols
    let results = index.search_symbols("process");
    insta::assert_snapshot!("substring_process", serialize_results(&results));

    Ok(())
}

#[test]
fn snapshot_search_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
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
    insta::assert_snapshot!("qualified_name_full", serialize_results(&by_qualified));

    // Search by bare name
    let by_bare = index.search_symbols("find");
    insta::assert_snapshot!("qualified_name_bare", serialize_results(&by_bare));

    Ok(())
}

#[test]
fn snapshot_search_short_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;

    let code = r#"
package Test;
sub abc { return 1; }
sub abcd { return 2; }
sub abcdef { return 3; }
sub xyz { return 4; }
"#;
    index.index_file(uri, code.to_string())?;

    // Short query "ab" should match abc, abcd, abcdef (all contain "ab")
    let results = index.search_symbols("ab");
    insta::assert_snapshot!("short_query_ab", serialize_results(&results));

    Ok(())
}

#[test]
fn snapshot_search_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
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

    // Search for "helper" should find from both ModuleA and ModuleB
    let results = index.search_symbols("helper");
    insta::assert_snapshot!("multi_file_helper", serialize_results(&results));

    // Search for "other" should find only from ModuleC
    let results2 = index.search_symbols("other");
    insta::assert_snapshot!("multi_file_other", serialize_results(&results2));

    Ok(())
}

#[test]
fn snapshot_search_no_match() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = file_url("/lib/Test.pm")?;

    let code = r#"
package Test;
sub foo { return 1; }
sub bar { return 2; }
"#;
    index.index_file(uri, code.to_string())?;

    // No match should return empty
    let results = index.search_symbols("nonexistent");
    insta::assert_snapshot!("no_match", serialize_results(&results));

    Ok(())
}

#[test]
fn snapshot_search_empty_index() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Empty index should return empty
    let results = index.search_symbols("anything");
    insta::assert_snapshot!("empty_index", serialize_results(&results));

    Ok(())
}

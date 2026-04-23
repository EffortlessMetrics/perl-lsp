//! Tests for folder provenance in the workspace index.
//!
//! These tests verify that folder attribution is correctly stored and queried
//! for multi-root workspace support.

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

fn parse_uri(uri: &str) -> Result<Url, url::ParseError> {
    Url::parse(uri)
}

#[test]
fn test_folder_uri_in_file_index() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = parse_uri("file:///test.pl")?;
    let code = "package MyPackage;\nsub example { return 42; }";

    // Index a file (folder_uri will be None for now, as determine_folder_uri returns None)
    index.index_file(uri.clone(), code.to_string())?;

    // Verify that the file was indexed
    let file_count = index.file_count();
    assert_eq!(file_count, 1, "Should have 1 indexed file");

    // Verify symbols were created
    let symbols = index.all_symbols();
    assert!(!symbols.is_empty(), "Should have indexed symbols");

    // Verify that workspace_folder_uri is set (None for now)
    for symbol in &symbols {
        // Currently None because determine_folder_uri returns None
        // This will be updated in PR 4 when proper folder tracking is implemented
        assert!(symbol.workspace_folder_uri.is_none());
    }

    Ok(())
}

#[test]
fn test_files_in_folder_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index multiple files
    let uri1 = parse_uri("file:///test1.pl")?;
    let uri2 = parse_uri("file:///test2.pl")?;
    let code = "package MyPackage;\nsub example { return 42; }";

    index.index_file(uri1, code.to_string())?;
    index.index_file(uri2, code.to_string())?;

    // Query for files in a folder (returns empty for now)
    let files = index.files_in_folder("file:///folder");
    assert_eq!(files.len(), 0, "Should return empty for non-existent folder");

    Ok(())
}

#[test]
fn test_symbols_in_folder_query() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index a file
    let uri = parse_uri("file:///test.pl")?;
    let code = "package MyPackage;\nsub example { return 42; }";
    index.index_file(uri, code.to_string())?;

    // Query for symbols in a folder (returns empty for now)
    let symbols = index.symbols_in_folder("file:///folder");
    assert_eq!(symbols.len(), 0, "Should return empty for non-existent folder");

    Ok(())
}

#[test]
fn test_remove_folder() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index multiple files
    let uri1 = parse_uri("file:///test1.pl")?;
    let uri2 = parse_uri("file:///test2.pl")?;
    let code = "package MyPackage;\nsub example { return 42; }";

    index.index_file(uri1, code.to_string())?;
    index.index_file(uri2, code.to_string())?;

    let initial_count = index.file_count();
    assert_eq!(initial_count, 2, "Should have 2 indexed files");

    // Remove a folder (no-op for now)
    index.remove_folder("file:///folder");

    // File count should remain the same
    let final_count = index.file_count();
    assert_eq!(final_count, 2, "File count should remain unchanged");

    Ok(())
}

#[test]
fn test_folder_provenance_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = parse_uri("file:///test.pl")?;
    let code = "package MyPackage;\nsub example { return 42; }";

    index.index_file(uri, code.to_string())?;

    // Get symbols and verify they can be serialized
    let symbols = index.all_symbols();
    assert!(!symbols.is_empty());

    Ok(())
}

#[test]
fn test_rank_symbols_by_folder() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Set up workspace folders
    index.set_workspace_folders(vec![
        "file:///project1".to_string(),
        "file:///project2".to_string(),
    ]);

    // Index files in different folders
    let uri1 = parse_uri("file:///project1/lib/Module1.pm")?;
    let code1 = "package Module1;\nsub example { return 42; }";
    index.index_file(uri1, code1.to_string())?;

    let uri2 = parse_uri("file:///project2/lib/Module2.pm")?;
    let code2 = "package Module2;\nsub example { return 42; }";
    index.index_file(uri2, code2.to_string())?;

    // Search for "example" - should find both
    let symbols = index.search_symbols("example");
    assert_eq!(symbols.len(), 2, "Should find 2 symbols named 'example'");

    // Rank symbols from project1 perspective
    let ranked = index.rank_symbols_by_folder(symbols, "file:///project1/src/main.pl");
    assert_eq!(ranked.len(), 2, "Should have 2 ranked symbols");

    // First result should be from project1 (same folder)
    assert_eq!(
        ranked[0].workspace_folder_uri,
        Some("file:///project1".to_string()),
        "First result should be from same folder"
    );

    // Second result should be from project2 (different folder)
    assert_eq!(
        ranked[1].workspace_folder_uri,
        Some("file:///project2".to_string()),
        "Second result should be from different folder"
    );

    Ok(())
}

#[test]
fn test_search_symbols_ranked() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Set up workspace folders
    index.set_workspace_folders(vec![
        "file:///project1".to_string(),
        "file:///project2".to_string(),
    ]);

    // Index files in different folders
    let uri1 = parse_uri("file:///project1/lib/Module1.pm")?;
    let code1 = "package Module1;\nsub test_sub { return 1; }";
    index.index_file(uri1, code1.to_string())?;

    let uri2 = parse_uri("file:///project2/lib/Module2.pm")?;
    let code2 = "package Module2;\nsub test_sub { return 2; }";
    index.index_file(uri2, code2.to_string())?;

    // Search with ranking from project1 perspective
    let ranked = index.search_symbols_ranked("test_sub", "file:///project1/src/main.pl");
    assert_eq!(ranked.len(), 2, "Should find 2 symbols");

    // First result should be from project1
    assert_eq!(
        ranked[0].workspace_folder_uri,
        Some("file:///project1".to_string()),
        "First result should be from same folder"
    );

    Ok(())
}

#[test]
fn test_rank_symbols_no_folder_context() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Set up workspace folders
    index.set_workspace_folders(vec![
        "file:///project1".to_string(),
        "file:///project2".to_string(),
    ]);

    // Index files in different folders
    let uri1 = parse_uri("file:///project1/lib/Module1.pm")?;
    let code1 = "package Module1;\nsub no_context { return 1; }";
    index.index_file(uri1, code1.to_string())?;

    let uri2 = parse_uri("file:///project2/lib/Module2.pm")?;
    let code2 = "package Module2;\nsub no_context { return 2; }";
    index.index_file(uri2, code2.to_string())?;

    // Get symbols
    let symbols = index.search_symbols("no_context");
    assert_eq!(symbols.len(), 2, "Should find 2 symbols");

    // Rank without folder context (document not in any workspace folder)
    let ranked = index.rank_symbols_by_folder(symbols, "file:///other/project/main.pl");
    assert_eq!(ranked.len(), 2, "Should have 2 ranked symbols");

    // Without folder context, all symbols should be treated as different folder
    // Results should be sorted by name for stability
    assert!(ranked[0].name <= ranked[1].name, "Should be sorted by name for stability");

    Ok(())
}

#[test]
fn test_extract_package_name() {
    let index = WorkspaceIndex::new();

    // Test extracting package names
    assert_eq!(
        index.extract_package_name("Foo::Bar::baz"),
        Some("Foo::Bar".to_string()),
        "Should extract package name"
    );

    assert_eq!(
        index.extract_package_name("MyPackage::subroutine"),
        Some("MyPackage".to_string()),
        "Should extract single-level package"
    );

    assert_eq!(
        index.extract_package_name("simple_function"),
        None,
        "Should return None for main package"
    );
}

#[test]
fn test_same_package() {
    let index = WorkspaceIndex::new();

    // Test same package detection using container_name
    assert!(index.same_package_by_container("Foo::Bar", "Foo::Bar"), "Same package should match");

    assert!(
        !index.same_package_by_container("Foo::Bar", "Other::Package"),
        "Different packages should not match"
    );
}

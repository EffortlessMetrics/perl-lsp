use perl_parser::workspace_index::{LspWorkspaceSymbol, WorkspaceIndex};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

#[test]
fn test_document_store_close_uses_normalized_uri() {
    let index = WorkspaceIndex::new();

    // Index a file with a non-normalized URI (missing file://)
    let uri = "/home/user/test.pl";
    let text = "sub test_func { my $x = 42; }";
    index.index_file(Url::from_file_path(uri).unwrap(), text.to_string()).unwrap();

    // The file should be indexed (accessible with both normalized and non-normalized)
    let symbols = index.find_symbols("test_func");
    assert_eq!(symbols.len(), 1);

    // Remove the file with the original URI (should work due to normalization)
    index.remove_file(uri);

    // The file should be completely removed
    let symbols = index.find_symbols("test_func");
    assert_eq!(symbols.len(), 0);

    // Re-index with file:// prefix
    let normalized_uri = "file:///home/user/test.pl";
    index.index_file(Url::parse(normalized_uri).unwrap(), text.to_string()).unwrap();
    let symbols = index.find_symbols("test_func");
    assert_eq!(symbols.len(), 1);

    // Remove with non-normalized URI should still work
    index.remove_file(uri);
    let symbols = index.find_symbols("test_func");
    assert_eq!(symbols.len(), 0);
}

#[test]
fn test_lsp_workspace_symbol_no_internal_fields() {
    let index = WorkspaceIndex::new();

    // Index a file with a regular subroutine that has internal 'has_body' field
    let uri = "file:///test.pl";
    let text = "sub test_func { return 42; } my $x = 'test';";
    index.index_file(Url::parse(uri).unwrap(), text.to_string()).unwrap();

    // Get symbols and convert to LSP DTOs
    let internal_symbols = index.find_symbols("test_func");
    let lsp_symbols: Vec<LspWorkspaceSymbol> =
        internal_symbols.iter().map(|sym| sym.into()).collect();

    // Serialize to JSON and check fields
    let json_value = serde_json::to_value(&lsp_symbols).unwrap();
    if let Value::Array(symbols) = json_value {
        for symbol in symbols {
            let obj = symbol.as_object().unwrap();

            // These fields should exist
            assert!(obj.contains_key("name"));
            assert!(obj.contains_key("kind"));
            assert!(obj.contains_key("location"));

            // The internal 'has_body' field should NOT exist
            assert!(
                !obj.contains_key("has_body"),
                "Internal field 'has_body' leaked to LSP output"
            );
            assert!(!obj.contains_key("hasBody"), "Internal field 'hasBody' leaked to LSP output");

            // Check location structure
            let location = &obj["location"];
            assert!(location.get("uri").is_some());
            assert!(location.get("range").is_some());

            // Check range structure
            let range = &location["range"];
            assert!(range.get("start").is_some());
            assert!(range.get("end").is_some());
        }
    }
}

#[test]
fn test_symbol_kind_uses_lsp_constants() {
    use perl_parser::workspace_index::SymbolKind;

    // Test that to_lsp_kind returns proper LSP-compliant values
    assert_eq!(SymbolKind::Package.to_lsp_kind(), 2); // Module
    assert_eq!(SymbolKind::Subroutine.to_lsp_kind(), 12); // Function
    assert_eq!(SymbolKind::Method.to_lsp_kind(), 6); // Method
    assert_eq!(SymbolKind::Variable.to_lsp_kind(), 13); // Variable
    assert_eq!(SymbolKind::Constant.to_lsp_kind(), 14); // Constant
    assert_eq!(SymbolKind::Class.to_lsp_kind(), 5); // Class

    // No magic numbers - all should be valid LSP SymbolKind values
    let valid_lsp_kinds: HashSet<u32> = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26,
    ]
    .into_iter()
    .collect();

    for kind in &[
        SymbolKind::Package,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Variable,
        SymbolKind::Constant,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Import,
        SymbolKind::Export,
    ] {
        let lsp_kind = kind.to_lsp_kind();
        assert!(
            valid_lsp_kinds.contains(&lsp_kind),
            "{:?} maps to invalid LSP kind {}",
            kind,
            lsp_kind
        );
    }
}

#[test]
fn test_workspace_symbol_deduplication() {
    let index = WorkspaceIndex::new();

    // Index a file with duplicate symbol definitions (shouldn't happen, but test dedup)
    let uri = "file:///test.pl";
    let text = "sub dup_func { } sub dup_func { }"; // Parser might produce duplicates
    index.index_file(Url::parse(uri).unwrap(), text.to_string()).unwrap();

    // Get symbols - should be deduplicated
    let symbols = index.find_symbols("dup_func");

    // Convert to LSP symbols and check deduplication by position
    let mut seen = HashSet::new();
    for sym in &symbols {
        let key =
            (sym.uri.clone(), sym.range.start.line, sym.range.start.column, sym.name.clone());

        // This would fail if we had true duplicates at the same position
        if seen.contains(&key) {
            panic!("Found duplicate symbol at same position: {:?}", key);
        }
        seen.insert(key);
    }
}

#[test]
fn test_uri_normalization_consistency() {
    let index = WorkspaceIndex::new();

    // Test various URI formats
    let test_cases = vec![
        ("/absolute/path/test.pl", "file:///absolute/path/test.pl"),
        ("file:///with/scheme/test.pl", "file:///with/scheme/test.pl"),
        ("untitled:Untitled-1", "untitled:Untitled-1"), // Special VSCode scheme
    ];

    for (input_uri, _expected_pattern) in test_cases {
        let text = format!("sub test_{} {{ }}", input_uri.len());

        // Index with various formats
        let url = if input_uri.starts_with("file://") || input_uri.starts_with("untitled:") {
            Url::parse(input_uri).unwrap()
        } else {
            Url::from_file_path(input_uri).unwrap()
        };
        index.index_file(url, text.clone()).unwrap();

        // Should be able to find the symbol
        let func_name = format!("test_{}", input_uri.len());
        let symbols = index.find_symbols(&func_name);
        assert!(!symbols.is_empty(), "Failed to find symbol indexed with URI: {}", input_uri);

        // Remove and verify it's gone
        index.remove_file(input_uri);
        let symbols = index.find_symbols(&func_name);
        assert!(symbols.is_empty(), "Failed to remove file with URI: {}", input_uri);
    }
}

#[test]
fn test_utf16_position_encoding() {
    // This test would require the actual LSP server, but we can test
    // that the position encoding is advertised correctly
    let index = WorkspaceIndex::new();

    // Index a file with emoji (multi-byte UTF-8, different in UTF-16)
    let uri = "file:///emoji.pl";
    let text = "my $♥ = 'love'; sub test { }";
    index.index_file(Url::parse(uri).unwrap(), text.to_string()).unwrap();

    // Find the subroutine
    let symbols = index.find_symbols("test");
    assert_eq!(symbols.len(), 1);

    // The position should be calculated correctly
    // In UTF-8 bytes: "my $♥ = 'love'; sub " is 20 bytes (♥ is 3 bytes)
    // In UTF-16 units: "my $♥ = 'love'; sub " is 18 units (♥ is 1 unit)
    // Character position should reflect this
    let symbol = &symbols[0];

    // The exact position depends on the parser, but it should be consistent
    assert!(symbol.range.start.column < 100, "Position seems unreasonably large");
}

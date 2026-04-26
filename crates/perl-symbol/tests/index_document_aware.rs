use perl_symbol::index::SymbolIndex;

#[test]
fn removing_document_removes_only_its_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["alpha::one".to_string(), "shared::name".to_string()]);
    index.replace_document_symbols("doc-b", vec!["beta::two".to_string(), "shared::name".to_string()]);

    index.remove_document("doc-a");

    let prefix_results = index.search_prefix("alpha");
    assert!(prefix_results.is_empty(), "alpha prefix should return no results after doc-a is removed");

    let shared_results = index.search_prefix("shared");
    assert_eq!(shared_results, vec!["shared::name".to_string()], "shared prefix should still find symbol from doc-b");

    let fuzzy_results = index.search_fuzzy("alpha");
    assert!(fuzzy_results.is_empty(), "alpha fuzzy should return no results after doc-a is removed");

    Ok(())
}

#[test]
fn replacing_document_symbols_drops_stale_entries() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["OldThing".to_string(), "StillHere".to_string()]);

    index.replace_document_symbols("doc-a", vec!["StillHere".to_string(), "NewThing".to_string()]);

    assert!(index.search_prefix("Old").is_empty(), "Old prefix should be empty after replacement");
    assert_eq!(index.search_prefix("New"), vec!["NewThing".to_string()], "New prefix should find NewThing");
    assert_eq!(index.search_fuzzy("old"), Vec::<String>::new(), "fuzzy 'old' should find nothing after replacement");
    assert!(index.search_fuzzy("new").contains(&"NewThing".to_string()), "fuzzy 'new' should contain NewThing");

    Ok(())
}

#[test]
fn duplicate_symbols_across_documents_remain_until_last_document_removed() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["common_symbol".to_string()]);
    index.replace_document_symbols("doc-b", vec!["common_symbol".to_string()]);

    index.remove_document("doc-a");
    assert_eq!(index.search_prefix("common"), vec!["common_symbol".to_string()], "symbol should remain after first document removed");

    index.remove_document("doc-b");
    assert!(index.search_prefix("common").is_empty(), "symbol should be removed after last document removed");
    assert!(index.search_fuzzy("common").is_empty(), "fuzzy search should be empty after last document removed");

    Ok(())
}

#[test]
fn add_symbol_remains_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = SymbolIndex::new();
    index.add_symbol("legacy_name".to_string());
    index.add_symbol("legacy_name".to_string());

    assert_eq!(index.search_prefix("legacy"), vec!["legacy_name".to_string()], "idempotent adds should return single result");

    Ok(())
}

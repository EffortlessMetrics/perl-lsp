use perl_symbol::index::SymbolIndex;

#[test]
fn removing_document_removes_only_its_symbols() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["alpha::one".to_string(), "shared::name".to_string()]);
    index.replace_document_symbols("doc-b", vec!["beta::two".to_string(), "shared::name".to_string()]);

    index.remove_document("doc-a");

    let prefix_results = index.search_prefix("alpha");
    assert!(prefix_results.is_empty());

    let shared_results = index.search_prefix("shared");
    assert_eq!(shared_results, vec!["shared::name".to_string()]);

    let fuzzy_results = index.search_fuzzy("alpha");
    assert!(fuzzy_results.is_empty());
}

#[test]
fn replacing_document_symbols_drops_stale_entries() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["OldThing".to_string(), "StillHere".to_string()]);

    index.replace_document_symbols("doc-a", vec!["StillHere".to_string(), "NewThing".to_string()]);

    assert!(index.search_prefix("Old").is_empty());
    assert_eq!(index.search_prefix("New"), vec!["NewThing".to_string()]);
    assert_eq!(index.search_fuzzy("old"), Vec::<String>::new());
    assert!(index.search_fuzzy("new").contains(&"NewThing".to_string()));
}

#[test]
fn duplicate_symbols_across_documents_remain_until_last_document_removed() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", vec!["common_symbol".to_string()]);
    index.replace_document_symbols("doc-b", vec!["common_symbol".to_string()]);

    index.remove_document("doc-a");
    assert_eq!(index.search_prefix("common"), vec!["common_symbol".to_string()]);

    index.remove_document("doc-b");
    assert!(index.search_prefix("common").is_empty());
    assert!(index.search_fuzzy("common").is_empty());
}

#[test]
fn add_symbol_remains_idempotent() {
    let mut index = SymbolIndex::new();
    index.add_symbol("legacy_name".to_string());
    index.add_symbol("legacy_name".to_string());

    assert_eq!(index.search_prefix("legacy"), vec!["legacy_name".to_string()]);
}

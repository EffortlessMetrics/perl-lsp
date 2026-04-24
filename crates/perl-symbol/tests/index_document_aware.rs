use perl_symbol::index::SymbolIndex;

#[test]
fn replace_document_symbols_removes_stale_entries() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", ["alpha_old".to_string(), "alpha_keep".to_string()]);

    index.replace_document_symbols("doc-a", ["alpha_new".to_string(), "alpha_keep".to_string()]);

    let prefix = index.search_prefix("alpha_");
    assert!(prefix.contains(&"alpha_new".to_string()));
    assert!(prefix.contains(&"alpha_keep".to_string()));
    assert!(!prefix.contains(&"alpha_old".to_string()));

    let fuzzy = index.search_fuzzy("old");
    assert!(!fuzzy.contains(&"alpha_old".to_string()));
}

#[test]
fn remove_document_only_removes_its_own_occurrences() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", ["shared_name".to_string(), "only_a".to_string()]);
    index.replace_document_symbols("doc-b", ["shared_name".to_string(), "only_b".to_string()]);

    index.remove_document("doc-a");

    let prefix = index.search_prefix("shared");
    assert_eq!(prefix, vec!["shared_name".to_string()]);

    let only_a = index.search_prefix("only_a");
    assert!(only_a.is_empty());

    let only_b = index.search_prefix("only_b");
    assert_eq!(only_b, vec!["only_b".to_string()]);
}

#[test]
fn remove_document_clears_fuzzy_entries_without_stale_drift() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("doc-a", ["parse_document".to_string()]);
    index.replace_document_symbols("doc-b", ["parse_workspace".to_string()]);

    index.remove_document("doc-a");

    let results = index.search_fuzzy("document");
    assert!(results.is_empty());

    let workspace_results = index.search_fuzzy("workspace");
    assert_eq!(workspace_results, vec!["parse_workspace".to_string()]);
}

#[test]
fn add_symbol_compatibility_path_still_dedupes() {
    let mut index = SymbolIndex::new();

    index.add_symbol("legacy_symbol".to_string());
    index.add_symbol("legacy_symbol".to_string());

    let prefix = index.search_prefix("legacy");
    assert_eq!(prefix, vec!["legacy_symbol".to_string()]);
}

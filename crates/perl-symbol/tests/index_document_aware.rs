use perl_symbol::index::SymbolIndex;

#[test]
fn replace_document_symbols_removes_stale_symbols() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols(
        "file:///a.pm",
        vec!["alpha_one".to_string(), "alpha_two".to_string()],
    );

    let initial_prefix = index.search_prefix("alpha");
    assert_eq!(initial_prefix.len(), 2);

    index.replace_document_symbols("file:///a.pm", vec!["alpha_three".to_string()]);

    let updated_prefix = index.search_prefix("alpha");
    assert_eq!(updated_prefix, vec!["alpha_three".to_string()]);
    assert!(index.search_fuzzy("one").is_empty());
    assert!(index.search_fuzzy("two").is_empty());
    assert!(index.search_fuzzy("three").contains(&"alpha_three".to_string()));
}

#[test]
fn remove_document_keeps_shared_symbol_until_last_document_is_removed() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols("file:///a.pm", vec!["shared_name".to_string()]);
    index.replace_document_symbols("file:///b.pm", vec!["shared_name".to_string()]);

    index.remove_document("file:///a.pm");

    let prefix_after_single_remove = index.search_prefix("shared");
    assert_eq!(prefix_after_single_remove, vec!["shared_name".to_string()]);

    index.remove_document("file:///b.pm");

    assert!(index.search_prefix("shared").is_empty());
    assert!(index.search_fuzzy("name").is_empty());
}

#[test]
fn add_symbol_compatibility_path_remains_idempotent() {
    let mut index = SymbolIndex::new();

    index.add_symbol("legacy_entry".to_string());
    index.add_symbol("legacy_entry".to_string());

    let prefix = index.search_prefix("legacy");
    assert_eq!(prefix, vec!["legacy_entry".to_string()]);

    let fuzzy = index.search_fuzzy("entry");
    assert_eq!(fuzzy, vec!["legacy_entry".to_string()]);
}

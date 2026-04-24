use perl_symbol::index::SymbolIndex;

#[test]
fn remove_document_removes_stale_symbols_from_prefix_and_fuzzy() {
    let mut index = SymbolIndex::new();

    index.replace_document_symbols(
        "file:///workspace/lib/Foo.pm",
        ["foo_alpha".to_string(), "foo_beta".to_string()],
    );
    index.replace_document_symbols("file:///workspace/lib/Bar.pm", ["bar_alpha".to_string()]);

    assert!(index.search_prefix("foo_").contains(&"foo_alpha".to_string()));
    assert!(index.search_fuzzy("foo").contains(&"foo_beta".to_string()));

    index.remove_document("file:///workspace/lib/Foo.pm");

    let prefix_results = index.search_prefix("foo_");
    assert!(prefix_results.is_empty(), "removed document symbols must not remain in prefix index");

    let fuzzy_results = index.search_fuzzy("foo");
    assert!(fuzzy_results.is_empty(), "removed document symbols must not remain in fuzzy index");

    assert!(index.search_prefix("bar").contains(&"bar_alpha".to_string()));
}

#[test]
fn replacing_document_symbols_discards_old_names_and_keeps_new_names() {
    let mut index = SymbolIndex::new();
    let doc_id = "file:///workspace/lib/Baz.pm";

    index.replace_document_symbols(doc_id, ["old_name".to_string()]);
    assert!(index.search_prefix("old_").contains(&"old_name".to_string()));

    index.replace_document_symbols(doc_id, ["new_name".to_string(), "new_value".to_string()]);

    assert!(index.search_prefix("old_").is_empty(), "replaced document must drop stale symbols");

    let new_prefix = index.search_prefix("new_");
    assert!(new_prefix.contains(&"new_name".to_string()));
    assert!(new_prefix.contains(&"new_value".to_string()));

    let fuzzy_results = index.search_fuzzy("new");
    assert!(fuzzy_results.contains(&"new_name".to_string()));
    assert!(fuzzy_results.contains(&"new_value".to_string()));
}

#[test]
fn duplicate_symbol_across_documents_survives_single_document_removal() {
    let mut index = SymbolIndex::new();

    let shared = "shared_symbol".to_string();
    index.replace_document_symbols("file:///workspace/lib/One.pm", [shared.clone()]);
    index.replace_document_symbols("file:///workspace/lib/Two.pm", [shared.clone()]);

    let initial = index.search_prefix("shared");
    assert_eq!(initial, vec![shared.clone()]);

    index.remove_document("file:///workspace/lib/One.pm");
    assert_eq!(index.search_prefix("shared"), vec![shared.clone()]);

    index.remove_document("file:///workspace/lib/Two.pm");
    assert!(index.search_prefix("shared").is_empty());
    assert!(index.search_fuzzy("shared").is_empty());
}

#[test]
fn replace_document_symbols_deduplicates_names_within_document() {
    let mut index = SymbolIndex::new();
    index.replace_document_symbols(
        "file:///workspace/lib/Dedupe.pm",
        ["dup_name".to_string(), "dup_name".to_string(), "dup_name".to_string()],
    );

    assert_eq!(index.search_prefix("dup"), vec!["dup_name".to_string()]);
    assert_eq!(index.search_fuzzy("dup"), vec!["dup_name".to_string()]);
}

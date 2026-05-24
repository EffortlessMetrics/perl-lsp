//! Property tests for [`SymbolIndex`] document lifecycle operations.
//!
//! `prop_index_invariants.rs` already covers search shape (prefix / fuzzy
//! match correctness). This file exercises the multi-document state machine:
//! `add_symbol` idempotency, `replace_document_symbols` removing stale
//! entries, `remove_document` reference-counting, and consistency between
//! prefix and fuzzy views.

use perl_symbol::index::SymbolIndex;
use proptest::prelude::*;
use std::collections::HashSet;

fn symbol_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[A-Za-z][A-Za-z0-9_]{0,8}", 1..4).prop_map(|parts| parts.join("::"))
}

fn document_strategy() -> impl Strategy<Value = (String, Vec<String>)> {
    (
        "doc[0-9]{1,3}".prop_map(|s| format!("file:///{s}.pm")),
        prop::collection::vec(symbol_strategy(), 0..10),
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Calling `add_symbol` repeatedly with the same name must leave
    /// `search_prefix` returning that symbol exactly once. Duplicate
    /// completions in the editor (PR #122) regressed when this invariant
    /// broke.
    #[test]
    fn prop_add_symbol_is_idempotent_for_search(
        symbol in symbol_strategy(),
        repeats in 1usize..6,
    ) {
        let mut index = SymbolIndex::new();
        for _ in 0..repeats {
            index.add_symbol(symbol.clone());
        }

        let results = index.search_prefix(&symbol);
        let occurrences = results.iter().filter(|s| **s == symbol).count();
        prop_assert_eq!(
            occurrences,
            1,
            "expected {} exactly once after {} adds, got {} (results: {:?})",
            symbol,
            repeats,
            occurrences,
            results
        );
    }

    /// After `replace_document_symbols(uri, new)` the index must contain
    /// every distinct symbol in `new` and (if no other document added them)
    /// none of the symbols that were in `old` but not `new`.
    #[test]
    fn prop_replace_document_drops_stale_symbols(
        uri in "doc[0-9]{1,3}".prop_map(|s| format!("file:///{s}.pm")),
        old in prop::collection::vec(symbol_strategy(), 0..8),
        new in prop::collection::vec(symbol_strategy(), 0..8),
    ) {
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(&uri, old.clone());
        index.replace_document_symbols(&uri, new.clone());

        let old_set: HashSet<&String> = old.iter().collect();
        let new_set: HashSet<&String> = new.iter().collect();

        for symbol in &new_set {
            let results = index.search_prefix(symbol);
            prop_assert!(
                results.contains(symbol),
                "expected {} to be discoverable after replace, got {:?}",
                symbol,
                results
            );
        }

        for symbol in old_set.difference(&new_set) {
            let results = index.search_prefix(symbol);
            prop_assert!(
                !results.contains(symbol),
                "expected stale {} to be removed, got {:?}",
                symbol,
                results
            );
        }
    }

    /// `remove_document` followed by `search_prefix` for a symbol that was
    /// only present in that document must return no results for it.
    #[test]
    fn prop_remove_document_clears_unique_symbols(
        (uri, symbols) in document_strategy(),
    ) {
        let mut index = SymbolIndex::new();
        index.replace_document_symbols(&uri, symbols.clone());
        index.remove_document(&uri);

        for symbol in &symbols {
            let results = index.search_prefix(symbol);
            prop_assert!(
                !results.contains(symbol),
                "symbol {} survived remove_document({}): {:?}",
                symbol,
                uri,
                results
            );
        }
    }

    /// A symbol referenced by two distinct documents must remain
    /// discoverable when only one of those documents is removed. This
    /// guards the internal reference-counting in `symbol_ref_counts`.
    #[test]
    fn prop_shared_symbol_survives_partial_remove(
        symbol in symbol_strategy(),
        other_a in prop::collection::vec(symbol_strategy(), 0..3),
        other_b in prop::collection::vec(symbol_strategy(), 0..3),
    ) {
        let mut index = SymbolIndex::new();
        let mut doc_a = other_a;
        doc_a.push(symbol.clone());
        let mut doc_b = other_b;
        doc_b.push(symbol.clone());

        index.replace_document_symbols("file:///a.pm", doc_a);
        index.replace_document_symbols("file:///b.pm", doc_b);
        index.remove_document("file:///a.pm");

        let results = index.search_prefix(&symbol);
        prop_assert!(
            results.contains(&symbol),
            "symbol {} should survive when other doc still references it (results: {:?})",
            symbol,
            results
        );
    }

    /// Every symbol added via `add_symbol` must be reachable via fuzzy
    /// search on its first underscore-delimited token. Using snake_case
    /// identifiers keeps the tokenization deterministic: the index's
    /// tokenizer also splits on case changes, so a mixed-case query like
    /// `aA` would otherwise lower to `aa` which isn't a token.
    #[test]
    fn prop_trie_and_inverted_index_stay_in_sync(
        parts in prop::collection::vec("[a-z][a-z0-9]{0,6}", 1..4),
        extra in prop::collection::vec("[a-z][a-z0-9]{0,6}", 0..6),
    ) {
        let symbol = parts.join("_");
        let mut index = SymbolIndex::new();
        index.add_symbol(symbol.clone());
        for other in &extra {
            index.add_symbol(other.clone());
        }

        let token = &parts[0];
        let fuzzy = index.search_fuzzy(token);
        prop_assert!(
            fuzzy.contains(&symbol),
            "symbol {} reachable in trie but not via fuzzy query {:?}: {:?}",
            symbol,
            token,
            fuzzy
        );
    }
}

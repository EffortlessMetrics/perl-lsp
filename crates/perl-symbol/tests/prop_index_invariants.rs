use perl_symbol::index::SymbolIndex;
use proptest::prelude::*;

fn symbol_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[A-Za-z][A-Za-z0-9_]{0,10}", 1..6).prop_map(|parts| parts.join("::"))
}

proptest! {
    #[test]
    fn prop_prefix_search_only_returns_matching_symbols(
        symbols in prop::collection::vec(symbol_strategy(), 1..40),
        prefix_len in 0usize..8,
    ) {
        let mut index = SymbolIndex::new();
        for symbol in &symbols {
            index.add_symbol(symbol.clone());
        }

        let seed = symbols.first().cloned().unwrap_or_default();
        let prefix: String = seed.chars().take(prefix_len).collect();
        let results = index.search_prefix(&prefix);

        for result in &results {
            prop_assert!(
                result.starts_with(&prefix),
                "non-matching symbol {result} returned for prefix {prefix}"
            );
        }
    }

    #[test]
    fn prop_fuzzy_single_token_query_finds_source_symbol(
        parts in prop::collection::vec("[a-z]{1,8}", 1..5),
    ) {
        let mut index = SymbolIndex::new();
        let symbol = parts.join("_");
        index.add_symbol(symbol.clone());

        let query = parts[0].to_lowercase();
        let results = index.search_fuzzy(&query);

        prop_assert!(
            results.contains(&symbol),
            "expected symbol {symbol} to be discoverable via fuzzy query {query}"
        );
    }
}

# perl-symbol-index

Small single-responsibility crate for symbol indexing and lookup.

## Features

- Prefix search via trie-backed index
- Fuzzy token-based symbol search
- Zero runtime dependencies

## Usage

```rust
use perl_symbol_index::SymbolIndex;

let mut index = SymbolIndex::new();
index.add_symbol("calculate_total".to_string());
index.add_symbol("get_user_name".to_string());

let prefix = index.search_prefix("calc");
let fuzzy = index.search_fuzzy("user name");
```

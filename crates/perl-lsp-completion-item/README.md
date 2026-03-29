# perl-lsp-completion-item

Standalone SRP microcrate for completion item domain types and stable deduplicating sort behavior.

## When to use this crate

Use `perl-lsp-completion-item` when you need deterministic completion-item
ordering and deduplication without depending on a full completion provider.

It is useful for:

- provider crates that generate completion candidates
- tests that need stable sort behavior
- adapters that turn completion results into protocol payloads later

## Quick example

```rust
use perl_lsp_completion_item::{CompletionItem, CompletionItemKind, deduplicate_and_sort};

let items = vec![
    CompletionItem {
        label: "print".into(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: Some("020".into()),
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    },
    CompletionItem {
        label: "print".into(),
        kind: CompletionItemKind::Keyword,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: Some("100".into()),
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    },
];

let sorted = deduplicate_and_sort(items);
assert_eq!(sorted.len(), 1);
```

## Public API

- `CompletionItemKind`: crate-local completion taxonomy
- `CompletionItem`: stable completion domain type
- `deduplicate_and_sort`: deterministic deduplication and ordering policy

## License

MIT OR Apache-2.0

# perl-lsp-completion-item

Shared completion-item types and deterministic sorting for the Perl LSP stack.
Use this crate when you need a stable completion payload model without the full
completion engine.

## Use this crate when

Use `perl-lsp-completion-item` for data-model work, tests, or adapters that need
the same item semantics as the completion provider. Use
`perl-lsp-completion` when you want the actual suggestion engine.

## Key exports

- `CompletionItem` - normalized completion payload
- `CompletionItemKind` - completion classification
- `deduplicate_and_sort` - stable ordering and duplicate suppression

## Example

```rust,ignore
use perl_lsp_completion_item::{CompletionItem, CompletionItemKind, deduplicate_and_sort};

let items = vec![
    CompletionItem {
        label: "print".to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: None,
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    },
    CompletionItem {
        label: "printf".to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: None,
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    },
];
let sorted = deduplicate_and_sort(items);
```

## Stack role

This crate sits at the boundary between provider logic and editor payloads. It
keeps the completion result shape consistent across the wider provider stack.

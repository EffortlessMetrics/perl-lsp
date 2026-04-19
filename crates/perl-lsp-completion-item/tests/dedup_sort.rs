//! Mutation-killing tests for deduplicate_and_sort in perl-lsp-completion-item.
//!
//! The inline tests cover basic dedup and empty-label dropping.
//! These tests target the branches and orderings not exercised by the happy path:
//!
//! - Empty input (early return branch)
//! - No duplicates: result is sorted only
//! - sort_text = None falls back to label for comparison
//! - Secondary sort by CompletionItemKind when sort_texts are equal
//! - Tertiary sort by label when kind is also equal
//! - Multiple (3+) items with the same label: best sort_text wins
//! - All items have empty labels: result is empty
//! - Single item with no sort_text: returned unchanged

use perl_lsp_completion_item::{CompletionItem, CompletionItemKind, deduplicate_and_sort};

fn item(label: &str, kind: CompletionItemKind, sort_text: Option<&str>) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: sort_text.map(str::to_string),
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    }
}

// ---------------------------------------------------------------------------
// Empty input branch
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_empty_input_returns_empty() {
    // This exercises the `if completions.is_empty() { return completions; }` branch.
    let result = deduplicate_and_sort(vec![]);
    assert!(result.is_empty(), "empty input must return empty vec");
}

// ---------------------------------------------------------------------------
// No duplicates: sorting only
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_no_duplicates_sorts_by_sort_text() {
    let items = vec![
        item("zoo", CompletionItemKind::Function, Some("300")),
        item("alpha", CompletionItemKind::Function, Some("100")),
        item("middle", CompletionItemKind::Function, Some("200")),
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].label, "alpha", "lowest sort_text first");
    assert_eq!(result[1].label, "middle");
    assert_eq!(result[2].label, "zoo", "highest sort_text last");
}

// ---------------------------------------------------------------------------
// sort_text = None falls back to label
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_none_sort_text_uses_label() {
    // When sort_text is None, label is used for sorting
    let items = vec![
        item("zoo", CompletionItemKind::Function, None),
        item("aardvark", CompletionItemKind::Function, None),
        item("mango", CompletionItemKind::Function, None),
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(
        result[0].label, "aardvark",
        "without sort_text, sorts by label"
    );
    assert_eq!(result[1].label, "mango");
    assert_eq!(result[2].label, "zoo");
}

#[test]
fn deduplicate_and_sort_none_sort_text_compared_against_some() {
    // Item with sort_text="a" vs item with sort_text=None (label="z")
    // "a" < "z" → sort_text item first
    let items = vec![
        item("zebra", CompletionItemKind::Function, None), // sorts as "zebra"
        item("aardvark", CompletionItemKind::Function, Some("aaa")), // sorts as "aaa"
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(
        result[0].label, "aardvark",
        "explicit sort_text 'aaa' < 'zebra'"
    );
    assert_eq!(result[1].label, "zebra");
}

// ---------------------------------------------------------------------------
// Secondary sort by CompletionItemKind (when sort_text equal)
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_secondary_sort_by_kind_when_sort_text_equal() {
    // CompletionItemKind derives Ord: Variable=0, Function=1, Keyword=2, Module=3, ...
    // When sort_text is equal, lower kind value comes first
    let items = vec![
        item("foo", CompletionItemKind::Module, Some("same")),
        item("foo_func", CompletionItemKind::Variable, Some("same")),
        item("foo_kw", CompletionItemKind::Function, Some("same")),
    ];
    let result = deduplicate_and_sort(items);
    // Variable < Function < Module by derive Ord
    assert_eq!(result[0].kind, CompletionItemKind::Variable);
    assert_eq!(result[1].kind, CompletionItemKind::Function);
    assert_eq!(result[2].kind, CompletionItemKind::Module);
}

// ---------------------------------------------------------------------------
// Tertiary sort by label (when sort_text AND kind are equal)
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_tertiary_sort_by_label_when_kind_and_sort_text_equal() {
    let items = vec![
        item("zoo_func", CompletionItemKind::Function, Some("tie")),
        item("alpha_func", CompletionItemKind::Function, Some("tie")),
        item("mango_func", CompletionItemKind::Function, Some("tie")),
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(
        result[0].label, "alpha_func",
        "tertiary: lexicographic by label"
    );
    assert_eq!(result[1].label, "mango_func");
    assert_eq!(result[2].label, "zoo_func");
}

// ---------------------------------------------------------------------------
// Three duplicates of same label: best sort_text wins
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_three_duplicates_keeps_best_sort_text() {
    // "foo" appears 3 times. Best sort_text is "001" (lowest lexicographic).
    let items = vec![
        item("foo", CompletionItemKind::Function, Some("100")),
        item("foo", CompletionItemKind::Variable, Some("001")), // best
        item("foo", CompletionItemKind::Keyword, Some("050")),
        item("bar", CompletionItemKind::Function, Some("200")),
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 2, "3 duplicates must be deduped to 1");
    let foo = result.iter().find(|i| i.label == "foo");
    assert!(foo.is_some(), "foo must be kept");
    let foo = foo.unwrap_or(&result[0]);
    // The one with sort_text "001" (Variable kind) should win
    assert_eq!(
        foo.sort_text.as_deref(),
        Some("001"),
        "best sort_text must win among 3 duplicates"
    );
}

// ---------------------------------------------------------------------------
// All items have empty labels
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_all_empty_labels_returns_empty() {
    let items = vec![
        item("", CompletionItemKind::Function, Some("001")),
        item("", CompletionItemKind::Variable, Some("002")),
        item("", CompletionItemKind::Keyword, None),
    ];
    let result = deduplicate_and_sort(items);
    assert!(result.is_empty(), "all empty labels must be dropped");
}

// ---------------------------------------------------------------------------
// Single item
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_single_item_returned_unchanged() {
    let items = vec![item("only", CompletionItemKind::Function, Some("001"))];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "only");
}

#[test]
fn deduplicate_and_sort_single_item_no_sort_text_returned_unchanged() {
    let items = vec![item("only", CompletionItemKind::Function, None)];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "only");
    assert!(result[0].sort_text.is_none());
}

// ---------------------------------------------------------------------------
// Mixed empty and non-empty labels
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_mixed_empty_and_valid_drops_empty_keeps_valid() {
    let items = vec![
        item("", CompletionItemKind::Function, None),
        item("valid_a", CompletionItemKind::Function, Some("010")),
        item("", CompletionItemKind::Keyword, Some("005")),
        item("valid_b", CompletionItemKind::Function, Some("020")),
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 2, "only 2 valid items should remain");
    let labels: Vec<&str> = result.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"valid_a"), "valid_a should be present");
    assert!(labels.contains(&"valid_b"), "valid_b should be present");
}

// ---------------------------------------------------------------------------
// Dedup with sort_text = None on duplicate: label is the tiebreaker
// ---------------------------------------------------------------------------

#[test]
fn deduplicate_and_sort_dedup_prefers_none_sort_text_lower_label() {
    // Two "foo" items: one with sort_text=None (sorts as "foo"), one with sort_text="zzz"
    // "foo" < "zzz" lexicographically, so the None-sort_text item wins
    let items = vec![
        item("foo", CompletionItemKind::Function, Some("zzz")),
        item("foo", CompletionItemKind::Variable, None), // sorts as "foo"
    ];
    let result = deduplicate_and_sort(items);
    assert_eq!(result.len(), 1, "duplicate 'foo' must be deduped");
    // The item with sort_text=None (effective "foo" < "zzz") must win
    assert!(
        result[0].sort_text.is_none(),
        "item with None sort_text should win (label 'foo' < 'zzz')"
    );
}

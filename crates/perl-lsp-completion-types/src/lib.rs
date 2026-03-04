//! Completion item model and deterministic sorting utilities.

use perl_parser_core::SourceLocation;

/// Type of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    /// Variable (scalar, array, hash)
    Variable,
    /// Function or method
    Function,
    /// Perl keyword
    Keyword,
    /// Package or module
    Module,
    /// File path
    File,
    /// Snippet with placeholders
    Snippet,
    /// Constant value
    Constant,
    /// Property or hash key
    Property,
}

/// A single completion suggestion.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert
    pub label: String,
    /// Kind of completion
    pub kind: CompletionItemKind,
    /// Optional detail text
    pub detail: Option<String>,
    /// Optional documentation
    pub documentation: Option<String>,
    /// Text to insert (if different from label)
    pub insert_text: Option<String>,
    /// Sort priority (lower is better)
    pub sort_text: Option<String>,
    /// Filter text for matching
    pub filter_text: Option<String>,
    /// Additional text edits to apply
    pub additional_edits: Vec<(SourceLocation, String)>,
    /// Range to replace in the document (for proper prefix handling)
    pub text_edit_range: Option<(usize, usize)>, // (start, end) offsets
}

/// Remove duplicates and sort completions with stable, deterministic ordering.
#[must_use]
pub fn deduplicate_and_sort(mut completions: Vec<CompletionItem>) -> Vec<CompletionItem> {
    if completions.is_empty() {
        return completions;
    }

    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut to_remove = std::collections::HashSet::<usize>::new();

    for (i, item) in completions.iter().enumerate() {
        if item.label.is_empty() {
            to_remove.insert(i);
            continue;
        }

        if let Some(&existing_idx) = seen.get(&item.label) {
            let existing_sort = completions[existing_idx]
                .sort_text
                .as_ref()
                .unwrap_or(&completions[existing_idx].label);
            let current_sort = item.sort_text.as_ref().unwrap_or(&item.label);

            if current_sort < existing_sort {
                to_remove.insert(existing_idx);
                seen.insert(item.label.clone(), i);
            } else {
                to_remove.insert(i);
            }
        } else {
            seen.insert(item.label.clone(), i);
        }
    }

    let mut indices: Vec<usize> = to_remove.into_iter().collect();
    indices.sort_by(|a, b| b.cmp(a));
    for idx in indices {
        completions.remove(idx);
    }

    completions.sort_by(|a, b| {
        let a_sort = a.sort_text.as_ref().unwrap_or(&a.label);
        let b_sort = b.sort_text.as_ref().unwrap_or(&b.label);

        match a_sort.cmp(b_sort) {
            std::cmp::Ordering::Equal => match a.kind.cmp(&b.kind) {
                std::cmp::Ordering::Equal => a.label.cmp(&b.label),
                other => other,
            },
            other => other,
        }
    });

    completions
}

#[cfg(test)]
mod tests {
    use super::{CompletionItem, CompletionItemKind, deduplicate_and_sort};

    fn item(label: &str, kind: CompletionItemKind, sort: Option<&str>) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            sort_text: sort.map(ToString::to_string),
            filter_text: None,
            additional_edits: vec![],
            text_edit_range: None,
        }
    }

    #[test]
    fn removes_empty_labels_and_deduplicates_by_best_sort_text() {
        let items = vec![
            item("foo", CompletionItemKind::Function, Some("200")),
            item("", CompletionItemKind::Function, Some("001")),
            item("foo", CompletionItemKind::Function, Some("100")),
        ];

        let sorted = deduplicate_and_sort(items);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].label, "foo");
        assert_eq!(sorted[0].sort_text.as_deref(), Some("100"));
    }

    #[test]
    fn sorts_deterministically_by_sort_kind_then_label() {
        let items = vec![
            item("zeta", CompletionItemKind::Variable, Some("100")),
            item("alpha", CompletionItemKind::Function, Some("100")),
            item("beta", CompletionItemKind::Function, Some("010")),
        ];

        let sorted = deduplicate_and_sort(items);
        let labels: Vec<&str> = sorted.iter().map(|it| it.label.as_str()).collect();
        assert_eq!(labels, vec!["beta", "zeta", "alpha"]);
    }
}

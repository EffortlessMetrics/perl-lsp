#![warn(missing_docs)]
//! Completion item domain types and sorting utilities.
//!
//! This microcrate isolates completion payload representation and deterministic
//! ordering/deduplication policy from provider logic.

use perl_parser_core::SourceLocation;

/// Type of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    /// Variable (scalar, array, hash).
    Variable,
    /// Function or method.
    Function,
    /// Perl keyword.
    Keyword,
    /// Package or module.
    Module,
    /// File path.
    File,
    /// Snippet with placeholders.
    Snippet,
    /// Constant value.
    Constant,
    /// Property or hash key.
    Property,
}

/// A single completion suggestion.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert.
    pub label: String,
    /// Kind of completion.
    pub kind: CompletionItemKind,
    /// Optional detail text.
    pub detail: Option<String>,
    /// Optional documentation.
    pub documentation: Option<String>,
    /// Text to insert (if different from label).
    pub insert_text: Option<String>,
    /// Sort priority (lower is better).
    pub sort_text: Option<String>,
    /// Filter text for matching.
    pub filter_text: Option<String>,
    /// Additional text edits to apply.
    pub additional_edits: Vec<(SourceLocation, String)>,
    /// Range to replace in the document (for proper prefix handling).
    pub text_edit_range: Option<(usize, usize)>, // (start, end) offsets
    /// Commit characters that trigger auto-insertion (LSP 3.0+).
    /// Each entry must be exactly one character per LSP spec.
    pub commit_characters: Option<Vec<String>>,
}

/// Remove duplicates and sort completions with stable, deterministic ordering.
#[must_use]
pub fn deduplicate_and_sort(mut completions: Vec<CompletionItem>) -> Vec<CompletionItem> {
    if completions.is_empty() {
        return completions;
    }

    // Remove duplicates based on label, keeping the one with better sort_text.
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut to_remove = std::collections::HashSet::<usize>::new();

    for (i, item) in completions.iter().enumerate() {
        if item.label.is_empty() {
            // Skip items with empty labels.
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
                // Current item is better, remove the existing one.
                to_remove.insert(existing_idx);
                seen.insert(item.label.clone(), i);
            } else {
                // Existing item is better, remove current one.
                to_remove.insert(i);
            }
        } else {
            seen.insert(item.label.clone(), i);
        }
    }

    // Remove marked duplicates in reverse order to maintain indices.
    let mut indices: Vec<usize> = to_remove.into_iter().collect();
    indices.sort_by(|a, b| b.cmp(a)); // Sort in descending order.
    for idx in indices {
        completions.remove(idx);
    }

    // Sort with stable, deterministic ordering.
    completions.sort_by(|a, b| {
        let a_sort = a.sort_text.as_ref().unwrap_or(&a.label);
        let b_sort = b.sort_text.as_ref().unwrap_or(&b.label);

        // Primary sort: by sort_text/label.
        match a_sort.cmp(b_sort) {
            std::cmp::Ordering::Equal => {
                // Secondary sort: by completion kind for stability.
                match a.kind.cmp(&b.kind) {
                    std::cmp::Ordering::Equal => {
                        // Tertiary sort: by label for full determinism.
                        a.label.cmp(&b.label)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    completions
}

#[cfg(test)]
mod tests {
    use super::{CompletionItem, CompletionItemKind, deduplicate_and_sort};

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

    #[test]
    fn deduplicates_on_label_using_best_sort_text() {
        let items = vec![
            item("foo", CompletionItemKind::Function, Some("200")),
            item("foo", CompletionItemKind::Variable, Some("050")),
            item("bar", CompletionItemKind::Function, Some("100")),
        ];

        let result = deduplicate_and_sort(items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "foo");
        assert_eq!(result[0].kind, CompletionItemKind::Variable);
        assert_eq!(result[1].label, "bar");
    }

    #[test]
    fn drops_empty_labels() {
        let items = vec![
            item("", CompletionItemKind::Function, Some("001")),
            item("ok", CompletionItemKind::Function, Some("002")),
        ];

        let result = deduplicate_and_sort(items);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "ok");
    }
}

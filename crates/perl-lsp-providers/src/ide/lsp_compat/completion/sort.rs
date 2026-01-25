//! Completion sorting utilities
//!
//! Provides deduplication and sorting for completion items.

use crate::ide::lsp_compat::completion::items::CompletionItem;

/// Remove duplicates and sort completions with stable, deterministic ordering
pub fn deduplicate_and_sort(mut completions: Vec<CompletionItem>) -> Vec<CompletionItem> {
    if completions.is_empty() {
        return completions;
    }

    // Remove duplicates based on label, keeping the one with better sort_text
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut to_remove = std::collections::HashSet::<usize>::new();

    for (i, item) in completions.iter().enumerate() {
        if item.label.is_empty() {
            // Skip items with empty labels
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
                // Current item is better, remove the existing one
                to_remove.insert(existing_idx);
                seen.insert(item.label.clone(), i);
            } else {
                // Existing item is better, remove current one
                to_remove.insert(i);
            }
        } else {
            seen.insert(item.label.clone(), i);
        }
    }

    // Remove marked duplicates in reverse order to maintain indices
    let mut indices: Vec<usize> = to_remove.into_iter().collect();
    indices.sort_by(|a, b| b.cmp(a)); // Sort in descending order
    for idx in indices {
        completions.remove(idx);
    }

    // Sort with stable, deterministic ordering
    completions.sort_by(|a, b| {
        let a_sort = a.sort_text.as_ref().unwrap_or(&a.label);
        let b_sort = b.sort_text.as_ref().unwrap_or(&b.label);

        // Primary sort: by sort_text/label
        match a_sort.cmp(b_sort) {
            std::cmp::Ordering::Equal => {
                // Secondary sort: by completion kind for stability
                match a.kind.cmp(&b.kind) {
                    std::cmp::Ordering::Equal => {
                        // Tertiary sort: by label for full determinism
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

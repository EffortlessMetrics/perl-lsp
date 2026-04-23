//! Function completion for Perl
//!
//! Provides completion for user-defined subroutines with scope-distance ranking.
//! Functions defined in the same package rank higher than those from outer scopes.

use super::scope_distance::compute_scope_distance;
use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

/// Add function completions with scope-distance ranking.
///
/// User-defined subroutines are ranked by proximity to the cursor's scope,
/// so locally-defined helpers appear above package-level or outer-scope functions.
pub fn add_function_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    symbol_table: &SymbolTable,
) {
    let prefix_without_amp = context.prefix.trim_start_matches('&');

    for (name, symbols) in &symbol_table.symbols {
        for symbol in symbols {
            if !name.starts_with(prefix_without_amp) {
                continue;
            }

            let (kind, detail, insert_text) = match symbol.kind {
                SymbolKind::Subroutine => (
                    super::items::CompletionItemKind::Function,
                    Some("sub".to_string()),
                    Some(format!("{name}()")),
                ),
                SymbolKind::Constant => (
                    super::items::CompletionItemKind::Constant,
                    Some("constant".to_string()),
                    Some(name.clone()),
                ),
                _ => continue,
            };

            let distance =
                compute_scope_distance(symbol_table, context.cursor_scope_id, symbol.scope_id);
            completions.push(CompletionItem {
                label: name.clone(),
                kind,
                detail,
                documentation: symbol.documentation.clone(),
                insert_text,
                sort_text: Some(format!("2{}_{}", distance.sort_key(), name)),
                filter_text: Some(name.clone()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}

//! Function completion for Perl
//!
//! Provides completion for user-defined subroutines.

use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

/// Add function completions
pub fn add_function_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    symbol_table: &SymbolTable,
) {
    let prefix_without_amp = context.prefix.trim_start_matches('&');

    for (name, symbols) in &symbol_table.symbols {
        for symbol in symbols {
            if symbol.kind == SymbolKind::Subroutine && name.starts_with(prefix_without_amp) {
                completions.push(CompletionItem {
                    label: name.clone(),
                    kind: crate::completion::items::CompletionItemKind::Function,
                    detail: Some("sub".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(format!("{}()", name)),
                    sort_text: Some(format!("2_{}", name)),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
        }
    }
}

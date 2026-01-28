//! Workspace symbol completion for Perl
//!
//! Provides completion for symbols from other files in the workspace using the workspace index.

use super::{context::CompletionContext, items::{CompletionItem, CompletionItemKind}};
use perl_workspace_index::workspace_index::{SymbolKind as WsSymbolKind, VarKind, WorkspaceIndex};
use std::sync::Arc;

/// Add workspace symbol completions for functions and variables
///
/// Queries the workspace index to provide completions for symbols from other files.
/// This enables cross-file completion when the user types a symbol name.
pub fn add_workspace_symbol_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    // Only proceed if we have a workspace index
    let Some(index) = workspace_index else {
        return;
    };

    // Only provide workspace completions if there's a reasonable prefix
    // to avoid overwhelming the user with all workspace symbols
    if context.prefix.is_empty() {
        return;
    }

    // Check if the workspace index has any symbols
    if !index.has_symbols() {
        return;
    }

    // Search for symbols matching the prefix
    let matching_symbols = index.find_symbols(&context.prefix);

    for symbol in matching_symbols {
        // Skip symbols that don't match the prefix
        if !symbol.name.starts_with(&context.prefix)
            && !symbol.qualified_name.as_ref().map_or(false, |qn| qn.contains(&context.prefix)) {
            continue;
        }

        match symbol.kind {
            WsSymbolKind::Subroutine | WsSymbolKind::Method => {
                // Add function completion
                let label = if let Some(ref qname) = symbol.qualified_name {
                    qname.clone()
                } else {
                    symbol.name.clone()
                };

                completions.push(CompletionItem {
                    label: label.clone(),
                    kind: CompletionItemKind::Function,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("3_{}", label)), // Sort after local symbols
                    filter_text: Some(label),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Variable(var_kind) => {
                // Add variable completion with appropriate sigil
                let sigil = match var_kind {
                    VarKind::Scalar => "$",
                    VarKind::Array => "@",
                    VarKind::Hash => "%",
                };

                let label = if let Some(ref qname) = symbol.qualified_name {
                    format!("{}{}", sigil, qname)
                } else {
                    format!("{}{}", sigil, symbol.name)
                };

                // Only suggest if the prefix matches (considering sigil)
                if !label.starts_with(&context.prefix) {
                    continue;
                }

                completions.push(CompletionItem {
                    label: label.clone(),
                    kind: CompletionItemKind::Variable,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(label.clone()),
                    sort_text: Some(format!("3_{}", label)), // Sort after local symbols
                    filter_text: Some(label),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Package => {
                // Add package completion
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: CompletionItemKind::Module,
                    detail: Some("package".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("3_{}", symbol.name)),
                    filter_text: Some(symbol.name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Constant => {
                // Add constant completion
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: CompletionItemKind::Constant,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("3_{}", symbol.name)),
                    filter_text: Some(symbol.name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Export => {
                // Add exported symbol completion
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: CompletionItemKind::Function,
                    detail: Some("exported".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("2_{}", symbol.name)), // Prioritize exports
                    filter_text: Some(symbol.name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            _ => {
                // Skip other symbol types
            }
        }
    }
}

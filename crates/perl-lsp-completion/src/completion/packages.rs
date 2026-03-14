//! Package member completion for Perl
//!
//! Provides completion for package members using workspace index integration.

use super::{context::CompletionContext, items::CompletionItem};
use perl_workspace_index::workspace_index::{
    SymbolKind as WsSymbolKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::sync::Arc;

fn fallback_member_documentation(package_name: &str, symbol: &WorkspaceSymbol) -> String {
    let qualified_name =
        symbol.qualified_name.clone().unwrap_or_else(|| format!("{package_name}::{}", symbol.name));

    match symbol.kind {
        WsSymbolKind::Export => {
            format!("Exported function `{qualified_name}` from package `{package_name}`.")
        }
        WsSymbolKind::Subroutine => {
            format!("Subroutine `{qualified_name}` defined in package `{package_name}`.")
        }
        WsSymbolKind::Method => {
            format!("Method `{qualified_name}` defined in package `{package_name}`.")
        }
        WsSymbolKind::Variable(_) => {
            format!("Package variable `{qualified_name}` declared in `{package_name}`.")
        }
        WsSymbolKind::Constant => {
            format!("Constant `{qualified_name}` declared in `{package_name}`.")
        }
        _ => format!("Package member `{qualified_name}` from `{package_name}`."),
    }
}

fn package_member_documentation(package_name: &str, symbol: &WorkspaceSymbol) -> Option<String> {
    symbol
        .documentation
        .clone()
        .or_else(|| Some(fallback_member_documentation(package_name, symbol)))
}

/// Add package member completions
pub fn add_package_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    // Only proceed if we have a workspace index to query
    let Some(index) = workspace_index else {
        return;
    };

    // Split the prefix into package name and member prefix
    let mut parts: Vec<&str> = context.prefix.split("::").collect();
    if parts.len() < 2 {
        return;
    }
    let member_prefix = parts.pop().unwrap_or("");
    let package_name = parts.join("::");

    // Query workspace index for members of the package
    let members = index.get_package_members(&package_name);
    for symbol in members {
        match symbol.kind {
            WsSymbolKind::Export | WsSymbolKind::Subroutine | WsSymbolKind::Method => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: crate::completion::items::CompletionItemKind::Function,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            WsSymbolKind::Variable(_) => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: crate::completion::items::CompletionItemKind::Variable,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            WsSymbolKind::Constant => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: crate::completion::items::CompletionItemKind::Constant,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            _ => {}
        }
    }
}

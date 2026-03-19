//! Workspace symbol completion for Perl
//!
//! Provides completion for symbols from other files in the workspace using the workspace index.
//! Includes module name completion for `use`/`require` statements, workspace-aware method
//! completion for `->` expressions, general cross-file symbol completion, and auto-import.

use super::{
    auto_import,
    context::CompletionContext,
    items::{CompletionItem, CompletionItemKind},
};
use perl_workspace_index::workspace_index::{SymbolKind as WsSymbolKind, VarKind, WorkspaceIndex};
use std::collections::HashSet;
use std::sync::Arc;

/// Compute auto-import additional edits for a symbol from a given module.
fn auto_import_edits(
    source: &str,
    module_name: &str,
    symbol_name: &str,
) -> Vec<(perl_parser_core::SourceLocation, String)> {
    let use_qw = !symbol_name.contains("::");
    match auto_import::compute_auto_import(source, module_name, symbol_name, use_qw) {
        Some(edit) => vec![(edit.location, edit.text)],
        None => vec![],
    }
}

/// Add workspace symbol completions for functions and variables.
///
/// Queries the workspace index to provide completions for symbols from other files.
/// When a symbol comes from a known module, attaches auto-import edits.
pub fn add_workspace_symbol_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    source: &str,
) {
    let Some(index) = workspace_index else {
        return;
    };
    if context.prefix.is_empty() {
        return;
    }
    if !index.has_symbols() {
        return;
    }

    let matching_symbols = index.find_symbols(&context.prefix);

    for symbol in matching_symbols {
        if !symbol.name.starts_with(&context.prefix)
            && !symbol.qualified_name.as_ref().is_some_and(|qn| qn.contains(&context.prefix))
        {
            continue;
        }

        match symbol.kind {
            WsSymbolKind::Subroutine | WsSymbolKind::Method => {
                let label = symbol.qualified_name.clone().unwrap_or_else(|| symbol.name.clone());
                let additional_edits = symbol
                    .container_name
                    .as_deref()
                    .map(|c| auto_import_edits(source, c, &symbol.name))
                    .unwrap_or_default();
                completions.push(CompletionItem {
                    label: label.clone(),
                    kind: CompletionItemKind::Function,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("3_{}", label)),
                    filter_text: Some(label),
                    additional_edits,
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Variable(var_kind) => {
                let sigil = match var_kind {
                    VarKind::Scalar => "$",
                    VarKind::Array => "@",
                    VarKind::Hash => "%",
                };
                let label = symbol
                    .qualified_name
                    .as_ref()
                    .map(|qn| format!("{sigil}{qn}"))
                    .unwrap_or_else(|| format!("{sigil}{}", symbol.name));
                if !label.starts_with(&context.prefix) {
                    continue;
                }
                completions.push(CompletionItem {
                    label: label.clone(),
                    kind: CompletionItemKind::Variable,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(label.clone()),
                    sort_text: Some(format!("3_{}", label)),
                    filter_text: Some(label),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Package => {
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
                let additional_edits = symbol
                    .container_name
                    .as_deref()
                    .map(|c| auto_import_edits(source, c, &symbol.name))
                    .unwrap_or_default();
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: CompletionItemKind::Constant,
                    detail: symbol.container_name.clone().or_else(|| Some("workspace".to_string())),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("3_{}", symbol.name)),
                    filter_text: Some(symbol.name.clone()),
                    additional_edits,
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            WsSymbolKind::Export => {
                let additional_edits = symbol
                    .container_name
                    .as_deref()
                    .map(|c| auto_import_edits(source, c, &symbol.name))
                    .unwrap_or_default();
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: CompletionItemKind::Function,
                    detail: Some("exported".to_string()),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(symbol.name.clone()),
                    sort_text: Some(format!("2_{}", symbol.name)),
                    filter_text: Some(symbol.name.clone()),
                    additional_edits,
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
            _ => {}
        }
    }
}

/// Add module name completions for `use` and `require` statements.
pub fn add_use_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    let Some(index) = workspace_index else {
        return;
    };
    if !index.has_symbols() {
        return;
    }
    let mut seen = HashSet::new();
    let all_symbols = if context.prefix.is_empty() {
        index.all_symbols()
    } else {
        index.find_symbols(&context.prefix)
    };
    for symbol in all_symbols {
        if symbol.kind != WsSymbolKind::Package {
            continue;
        }
        if !context.prefix.is_empty() && !symbol.name.starts_with(&context.prefix) {
            continue;
        }
        if !seen.insert(symbol.name.clone()) {
            continue;
        }
        completions.push(CompletionItem {
            label: symbol.name.clone(),
            kind: CompletionItemKind::Module,
            detail: Some("module".to_string()),
            documentation: symbol
                .documentation
                .clone()
                .or_else(|| Some(format!("Package `{}`", symbol.name))),
            insert_text: Some(symbol.name.clone()),
            sort_text: Some(format!("1_{}", symbol.name)),
            filter_text: Some(symbol.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

/// Add import completions for symbols inside `use Module qw(...)`.
pub fn add_use_qw_import_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    module_name: &str,
    qw_prefix: &str,
) {
    let Some(index) = workspace_index else {
        return;
    };
    if !index.has_symbols() {
        return;
    }
    let mut seen = HashSet::new();
    let members = index.get_package_members(module_name);
    for symbol in &members {
        match symbol.kind {
            WsSymbolKind::Subroutine
            | WsSymbolKind::Method
            | WsSymbolKind::Export
            | WsSymbolKind::Constant => {}
            _ => continue,
        }
        if !qw_prefix.is_empty() && !symbol.name.starts_with(qw_prefix) {
            continue;
        }
        if !seen.insert(symbol.name.clone()) {
            continue;
        }
        let kind_label = match symbol.kind {
            WsSymbolKind::Constant => "constant",
            WsSymbolKind::Export => "exported",
            _ => "function",
        };
        completions.push(CompletionItem {
            label: symbol.name.clone(),
            kind: match symbol.kind {
                WsSymbolKind::Constant => CompletionItemKind::Constant,
                _ => CompletionItemKind::Function,
            },
            detail: Some(format!("{module_name} {kind_label}")),
            documentation: symbol
                .documentation
                .clone()
                .or_else(|| Some(format!("`{module_name}::{}`", symbol.name))),
            insert_text: Some(symbol.name.clone()),
            sort_text: Some(format!("1_{}", symbol.name)),
            filter_text: Some(symbol.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

fn infer_receiver_package(context: &CompletionContext, source: &str) -> Option<String> {
    let arrow_prefix = context.prefix.trim_end_matches("->");
    if !arrow_prefix.starts_with('$')
        && !arrow_prefix.starts_with('@')
        && !arrow_prefix.starts_with('%')
        && arrow_prefix.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    {
        return Some(arrow_prefix.to_string());
    }
    if arrow_prefix.starts_with('$') {
        let var_name = arrow_prefix;
        let before = &source[..context.position.min(source.len())];
        for line in before.lines().rev() {
            let trimmed = line.trim();
            if let Some(assign_pos) = find_assignment_eq(trimmed) {
                let lhs = trimmed[..assign_pos].trim();
                if lhs.ends_with(var_name) || lhs.contains(&format!("{var_name} ")) {
                    let rhs = trimmed[assign_pos + 1..].trim();
                    if let Some(arrow_pos) = rhs.find("->") {
                        let pkg = rhs[..arrow_pos].trim();
                        if pkg.contains("::")
                            || pkg.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        {
                            return Some(pkg.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Add method completions from the workspace index for `->` expressions.
pub fn add_workspace_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    let Some(index) = workspace_index else {
        return;
    };
    if !index.has_symbols() {
        return;
    }
    let Some(package_name) = infer_receiver_package(context, source) else {
        return;
    };
    let existing_labels: HashSet<String> =
        completions.iter().map(|item| item.label.clone()).collect();
    let method_prefix = context.prefix.rsplit("->").next().unwrap_or("");
    let members = index.get_package_members(&package_name);
    for symbol in members {
        match symbol.kind {
            WsSymbolKind::Subroutine | WsSymbolKind::Method => {}
            _ => continue,
        }
        if !method_prefix.is_empty() && !symbol.name.starts_with(method_prefix) {
            continue;
        }
        if existing_labels.contains(&symbol.name) {
            continue;
        }
        completions.push(CompletionItem {
            label: symbol.name.clone(),
            kind: CompletionItemKind::Function,
            detail: Some(format!("{package_name} method")),
            documentation: symbol.documentation.clone().or_else(|| {
                Some(format!("Method `{}::{}` from workspace index.", package_name, symbol.name))
            }),
            insert_text: Some(format!("{}()", symbol.name)),
            sort_text: Some(format!("2_{}", symbol.name)),
            filter_text: Some(symbol.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

fn find_assignment_eq(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' {
            continue;
        }
        if i > 0 && matches!(bytes[i - 1], b'!' | b'<' | b'>' | b'=') {
            continue;
        }
        if i + 1 < bytes.len() && matches!(bytes[i + 1], b'=' | b'~' | b'>') {
            continue;
        }
        return Some(i);
    }
    None
}

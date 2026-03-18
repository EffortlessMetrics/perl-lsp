//! Workspace symbol completion for Perl
//!
//! Provides completion for symbols from other files in the workspace using the workspace index.
//! Includes module name completion for `use`/`require` statements, workspace-aware method
//! completion for `->` expressions, and general cross-file symbol completion.

use super::{
    context::CompletionContext,
    items::{CompletionItem, CompletionItemKind},
};
use perl_workspace_index::workspace_index::{SymbolKind as WsSymbolKind, VarKind, WorkspaceIndex};
use std::collections::HashSet;
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
            && !symbol.qualified_name.as_ref().is_some_and(|qn| qn.contains(&context.prefix))
        {
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

/// Add module name completions for `use` and `require` statements.
///
/// When the cursor is after `use ` or `require `, suggests package names from the
/// workspace index. This enables discovering available modules as you type.
///
/// For example, typing `use My` will suggest `MyApp`, `MyApp::Config`, etc.
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

    // Search for package symbols matching the prefix
    let all_symbols = if context.prefix.is_empty() {
        index.all_symbols()
    } else {
        index.find_symbols(&context.prefix)
    };

    for symbol in all_symbols {
        if symbol.kind != WsSymbolKind::Package {
            continue;
        }

        // Match against the module name prefix
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
            sort_text: Some(format!("1_{}", symbol.name)), // High priority in use context
            filter_text: Some(symbol.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

/// Add import completions for symbols inside `use Module qw(...)`.
///
/// When the cursor is inside the `qw()` import list of a `use` statement,
/// queries the workspace index for symbols exported by or defined in that
/// module and suggests matching function/variable/constant names.
///
/// For example, typing `use File::Basename qw(bas` will suggest `basename`,
/// `fileparse`, `dirname`, etc.
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

        // Filter by prefix typed inside qw()
        if !qw_prefix.is_empty() && !symbol.name.starts_with(qw_prefix) {
            continue;
        }

        // Deduplicate
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

/// Infer the package type of a `->` receiver from the source context.
///
/// Looks for patterns like `My::Package->method` (static call) or attempts to
/// find the type from variable assignment context like `my $obj = My::Package->new`.
fn infer_receiver_package(context: &CompletionContext, source: &str) -> Option<String> {
    let arrow_prefix = context.prefix.trim_end_matches("->");

    // Case 1: Static method call like `My::Package->meth` or `Package->meth`
    // The prefix already contains the package name (starts with uppercase, no sigil)
    if !arrow_prefix.starts_with('$')
        && !arrow_prefix.starts_with('@')
        && !arrow_prefix.starts_with('%')
        && arrow_prefix.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    {
        return Some(arrow_prefix.to_string());
    }

    // Case 2: Variable method call like `$obj->meth`
    // Try to find the type from assignment context
    if arrow_prefix.starts_with('$') {
        let var_name = arrow_prefix;
        // Look for assignment pattern: `my $var = Package->new`
        // Search backwards in source for the variable assignment
        let before = &source[..context.position.min(source.len())];

        // Find the most recent assignment to this variable
        for line in before.lines().rev() {
            let trimmed = line.trim();
            // Match patterns like: `my $var = Package::Name->new(...)`
            // or `$var = Package::Name->new(...)`
            // We need a single `=` that is not part of `==`, `!=`, `<=`, `>=`, `=~`.
            let assign_pos = find_assignment_eq(trimmed);
            if let Some(assign_pos) = assign_pos {
                let lhs = trimmed[..assign_pos].trim();
                if lhs.ends_with(var_name) || lhs.contains(&format!("{var_name} ")) {
                    let rhs = trimmed[assign_pos + 1..].trim();
                    // Extract package name from `Package::Name->new(...)` pattern
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
///
/// When the user types `$obj->` or `Package->`, queries the workspace index for
/// methods defined in the receiver's package and suggests them.
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

    // Collect labels already present to avoid duplicates with local method completions
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

        // Skip if already provided by local method completion
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
            sort_text: Some(format!("2_{}", symbol.name)), // After local, before generic
            filter_text: Some(symbol.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

/// Find the position of a single assignment `=` in a line, skipping compound
/// operators like `==`, `!=`, `<=`, `>=`, `=~`, and `=>`.
///
/// Returns `None` if no assignment operator is found.
fn find_assignment_eq(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' {
            continue;
        }
        // Skip if preceded by !, <, >, or = (compound operators)
        if i > 0 && matches!(bytes[i - 1], b'!' | b'<' | b'>' | b'=') {
            continue;
        }
        // Skip if followed by = or ~ or > (==, =~, =>)
        if i + 1 < bytes.len() && matches!(bytes[i + 1], b'=' | b'~' | b'>') {
            continue;
        }
        return Some(i);
    }
    None
}

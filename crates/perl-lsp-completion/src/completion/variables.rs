//! Variable completion for Perl
//!
//! Provides completion for scalar, array, and hash variables with scope analysis.

use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

/// Add variable completions with thread-safe symbol table access
pub fn add_variable_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    kind: SymbolKind,
    symbol_table: &SymbolTable,
) {
    let sigil = kind.sigil().unwrap_or("");
    let prefix_without_sigil = context.prefix.trim_start_matches(sigil);

    for (name, symbols) in &symbol_table.symbols {
        for symbol in symbols {
            if symbol.kind == kind && name.starts_with(prefix_without_sigil) {
                let insert_text = format!("{}{}", sigil, name);

                completions.push(CompletionItem {
                    label: insert_text.clone(),
                    kind: crate::completion::items::CompletionItemKind::Variable,
                    detail: Some(
                        format!(
                            "{} {}{}",
                            symbol.declaration.as_deref().unwrap_or(""),
                            sigil,
                            name
                        )
                        .trim()
                        .to_string(),
                    ),
                    documentation: symbol.documentation.clone(),
                    insert_text: Some(insert_text),
                    sort_text: Some(format!("1_{}", name)), // Variables have high priority
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
        }
    }
}

/// Add special Perl variables
pub fn add_special_variables(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    sigil: &str,
) {
    let special_vars = match sigil {
        "$" => vec![
            ("$_", "Default input and pattern-search space"),
            ("$.", "Current line number"),
            ("$,", "Output field separator"),
            ("$/", "Input record separator"),
            ("$\\", "Output record separator"),
            ("$!", "Current errno"),
            ("$@", "Last eval error"),
            ("$$", "Process ID"),
            ("$0", "Program name"),
            ("$1", "First capture group"),
            ("$&", "Last match"),
            ("$`", "Prematch"),
            ("$'", "Postmatch"),
            ("$+", "Last capture group"),
            ("$^O", "Operating system name"),
            ("$^V", "Perl version"),
        ],
        "@" => vec![
            ("@_", "Subroutine arguments"),
            ("@ARGV", "Command line arguments"),
            ("@INC", "Module search paths"),
            ("@ISA", "Base classes"),
            ("@EXPORT", "Exported symbols"),
        ],
        "%" => vec![
            ("%ENV", "Environment variables"),
            ("%INC", "Loaded modules"),
            ("%SIG", "Signal handlers"),
        ],
        _ => vec![],
    };

    for (var, description) in special_vars {
        if var.starts_with(&context.prefix) {
            completions.push(CompletionItem {
                label: var.to_string(),
                kind: crate::completion::items::CompletionItemKind::Variable,
                detail: Some("special variable".to_string()),
                documentation: Some(description.to_string()),
                insert_text: Some(var.to_string()),
                sort_text: Some(format!("0_{}", var)), // Special vars have highest priority
                filter_text: Some(var.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }
}

/// Add all variables without sigils (for interpolation contexts)
pub fn add_all_variables(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    symbol_table: &SymbolTable,
) {
    // Only add if the prefix doesn't already have a sigil
    if !context.prefix.starts_with(['$', '@', '%', '&']) {
        for (name, symbols) in &symbol_table.symbols {
            for symbol in symbols {
                if symbol.kind.is_variable() && name.starts_with(&context.prefix) {
                    let sigil = symbol.kind.sigil().unwrap_or("");
                    completions.push(CompletionItem {
                        label: format!("{}{}", sigil, name),
                        kind: crate::completion::items::CompletionItemKind::Variable,
                        detail: Some(format!(
                            "{} variable",
                            symbol.declaration.as_deref().unwrap_or("")
                        )),
                        documentation: symbol.documentation.clone(),
                        insert_text: Some(format!("{}{}", sigil, name)),
                        sort_text: Some(format!("5_{}", name)),
                        filter_text: Some(name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
        }
    }
}

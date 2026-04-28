//! Variable completion for Perl
//!
//! Provides completion for scalar, array, and hash variables with scope analysis.
//! Variables are ranked by scope distance: immediate scope > parent scope >
//! package level, giving users the most relevant completions first.

use super::scope_distance::compute_scope_distance;
use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};

/// Add variable completions with scope-distance ranking.
///
/// Variables from the immediate scope rank highest, then parent scopes,
/// then package/global scope. This produces more relevant completion
/// lists when the same prefix matches variables at multiple scope depths.
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

                let distance =
                    compute_scope_distance(symbol_table, context.cursor_scope_id, symbol.scope_id);

                completions.push(CompletionItem {
                    label: insert_text.clone(),
                    kind: super::items::CompletionItemKind::Variable,
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
                    sort_text: Some(format!("1{}_{}", distance.sort_key(), name)),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
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
            ("$?", "Child process status (exit code)"),
            ("$$", "Process ID"),
            ("$0", "Program name"),
            ("$1", "First capture group"),
            ("$&", "Last match"),
            ("$`", "Prematch"),
            ("$'", "Postmatch"),
            ("$+", "Last capture group"),
            ("$^O", "Operating system name"),
            ("$^V", "Perl version"),
            ("$^T", "Script start time (epoch seconds)"),
            ("$^A", "Accumulator for format() output"),
            ("$^W", "Warning flag (prefer 'use warnings')"),
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
                kind: super::items::CompletionItemKind::Variable,
                detail: Some("special variable".to_string()),
                documentation: Some(description.to_string()),
                insert_text: Some(var.to_string()),
                sort_text: Some(format!("0_{}", var)), // Special vars have highest priority
                filter_text: Some(var.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}

/// Add all variables without sigils (for interpolation contexts)
///
/// Uses scope-distance ranking so closer variables sort before distant ones,
/// while keeping the `5x_` prefix to rank below sigil-prefixed completions.
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
                    let distance = compute_scope_distance(
                        symbol_table,
                        context.cursor_scope_id,
                        symbol.scope_id,
                    );
                    completions.push(CompletionItem {
                        label: format!("{}{}", sigil, name),
                        kind: super::items::CompletionItemKind::Variable,
                        detail: Some(format!(
                            "{} variable",
                            symbol.declaration.as_deref().unwrap_or("")
                        )),
                        documentation: symbol.documentation.clone(),
                        insert_text: Some(format!("{}{}", sigil, name)),
                        sort_text: Some(format!("5{}_{}", distance.sort_key(), name)),
                        filter_text: Some(name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                        commit_characters: None,
                    });
                }
            }
        }
    }
}

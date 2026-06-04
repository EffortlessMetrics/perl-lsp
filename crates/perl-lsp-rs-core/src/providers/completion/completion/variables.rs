//! Variable completion for Perl
//!
//! Provides completion for scalar, array, and hash variables with scope analysis.
//! Variables are ranked by scope distance: immediate scope > parent scope >
//! package level, giving users the most relevant completions first.

use super::scope_distance::compute_scope_sort_key;
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

                let scope_sort_key =
                    compute_scope_sort_key(symbol_table, context.cursor_scope_id, symbol.scope_id);

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
                    sort_text: Some(format!("1{scope_sort_key}_{name}")),
                    filter_text: Some(name.clone()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                    label_details: None,
                });
            }
        }
    }
}

/// Add special Perl variables
///
/// Offers a curated set of Perl magic/special variables (from perlvar) as
/// completion items keyed by sigil. Each entry carries a one-line description
/// so editors can display a tooltip without requiring the user to consult
/// the documentation separately.
pub fn add_special_variables(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    sigil: &str,
) {
    let special_vars: &[(&str, &str)] = match sigil {
        "$" => &[
            // Topic / default variable
            ("$_", "Default input and pattern-search space (topic variable)"),
            // I/O and formatting
            ("$.", "Current line number of the last filehandle read"),
            ("$,", "Output field separator for print"),
            ("$/", "Input record separator (undef to slurp)"),
            ("$\\", "Output record separator appended by print"),
            ("$|", "Output auto-flush: set to 1 to disable buffering"),
            ("$\"", "List separator for interpolated arrays (default: space)"),
            ("$;", "Subscript separator for multi-dimensional hashes"),
            // Error and status
            ("$!", "Errno / last OS error message (POSIX::strerror)"),
            ("$@", "Error from the last eval block or do-file"),
            ("$?", "Child process status (wait status from system/backtick)"),
            // Process info
            ("$$", "Process ID of the current Perl process"),
            ("$0", "Name of the running program (can be assigned)"),
            // Regex capture groups
            ("$1", "First regex capture group from last successful match"),
            ("$2", "Second regex capture group from last successful match"),
            ("$3", "Third regex capture group from last successful match"),
            ("$4", "Fourth regex capture group from last successful match"),
            ("$5", "Fifth regex capture group from last successful match"),
            ("$6", "Sixth regex capture group from last successful match"),
            ("$7", "Seventh regex capture group from last successful match"),
            ("$8", "Eighth regex capture group from last successful match"),
            ("$9", "Ninth regex capture group from last successful match"),
            // Regex match strings
            ("$&", "Entire string matched by the last successful regex"),
            ("$`", "String preceding the last successful regex match"),
            ("$'", "String following the last successful regex match"),
            ("$+", "Last bracket matched by the last successful regex"),
            // Control variables
            ("$^O", "Operating system name (e.g. 'linux', 'MSWin32')"),
            ("$^V", "Perl interpreter version as a v-string"),
            ("$^T", "Script start time in seconds since the epoch"),
            ("$^W", "Global warning flag (prefer 'use warnings' instead)"),
            ("$^A", "Accumulator variable for write/format output"),
            ("$^I", "In-place edit extension (e.g. '.bak' with -i flag)"),
            ("$^F", "Maximum system file descriptor (default: 2)"),
            ("$^X", "Path to the current Perl interpreter executable"),
            ("$^D", "Debugging flags (numeric, set by -D flag)"),
            ("$^P", "Internal debugger flag; true when under debugger"),
            ("$^S", "Current interpreter state: true inside eval"),
            ("$^E", "OS-specific extended error information"),
            ("$^H", "Compile-time hints bitmask (internal)"),
            ("$^M", "Emergency memory pool for out-of-memory handler"),
            ("$^N", "Most recently matched capture group in the current regex"),
            ("$^R", "Result of the last successful (?{...}) assertion"),
        ],
        "@" => &[
            ("@_", "Subroutine arguments (passed by reference)"),
            ("@ARGV", "Command-line arguments to the script"),
            ("@INC", "Module search paths (@INC for use/require)"),
            ("@ISA", "List of base classes for the current package"),
            ("@EXPORT", "Symbols exported by default from an Exporter module"),
            ("@EXPORT_OK", "Symbols exported on request from an Exporter module"),
        ],
        "%" => &[
            ("%ENV", "Environment variables (read/write)"),
            ("%INC", "Map of loaded module file paths keyed by module name"),
            ("%SIG", "Signal handlers keyed by signal name"),
            ("%+", "Named capture buffers from the last successful regex"),
            ("%-", "All named capture buffers (multi-valued) from last regex"),
        ],
        _ => &[],
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
                label_details: None,
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
                    let scope_sort_key = compute_scope_sort_key(
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
                        sort_text: Some(format!("5{scope_sort_key}_{name}")),
                        filter_text: Some(name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                        commit_characters: None,
                        label_details: None,
                    });
                }
            }
        }
    }
}

//! Method completion for Perl
//!
//! Provides context-aware method completion including DBI methods.

use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};
use std::collections::HashSet;

/// DBI database handle methods
pub const DBI_DB_METHODS: &[(&str, &str)] = &[
    ("do", "Execute a single SQL statement"),
    ("prepare", "Prepare a SQL statement"),
    ("prepare_cached", "Prepare and cache a SQL statement"),
    ("selectrow_array", "Execute and fetch a single row as array"),
    ("selectrow_arrayref", "Execute and fetch a single row as arrayref"),
    ("selectrow_hashref", "Execute and fetch a single row as hashref"),
    ("selectall_arrayref", "Execute and fetch all rows as arrayref"),
    ("selectall_hashref", "Execute and fetch all rows as hashref"),
    ("begin_work", "Begin a database transaction"),
    ("commit", "Commit the current transaction"),
    ("rollback", "Rollback the current transaction"),
    ("disconnect", "Disconnect from the database"),
    ("last_insert_id", "Get the last inserted row ID"),
    ("quote", "Quote a string for SQL"),
    ("quote_identifier", "Quote an identifier for SQL"),
    ("ping", "Check if database connection is alive"),
];

/// DBI statement handle methods
pub const DBI_ST_METHODS: &[(&str, &str)] = &[
    ("bind_param", "Bind a parameter to the statement"),
    ("bind_param_inout", "Bind an in/out parameter"),
    ("execute", "Execute the prepared statement"),
    ("fetch", "Fetch the next row as arrayref"),
    ("fetchrow_array", "Fetch the next row as array"),
    ("fetchrow_arrayref", "Fetch the next row as arrayref"),
    ("fetchrow_hashref", "Fetch the next row as hashref"),
    ("fetchall_arrayref", "Fetch all remaining rows as arrayref"),
    ("fetchall_hashref", "Fetch all remaining rows as hashref of hashrefs"),
    ("finish", "Finish the statement handle"),
    ("rows", "Get the number of rows affected"),
];

/// Infer receiver type from context (for DBI method completion)
pub fn infer_receiver_type(context: &CompletionContext, source: &str) -> Option<String> {
    // Look backwards from the position to find the receiver
    let prefix = context.prefix.trim_end_matches("->");

    // Simple heuristics for DBI types based on variable name
    if prefix.ends_with("$dbh") {
        return Some("DBI::db".to_string());
    }
    if prefix.ends_with("$sth") {
        return Some("DBI::st".to_string());
    }

    // Look at the broader context - check if variable was assigned from DBI->connect
    if let Some(var_pos) = source.rfind(prefix) {
        // Look backwards for assignment
        let before_var = &source[..var_pos];
        if let Some(assign_pos) = before_var.rfind('=') {
            let assignment = &source[assign_pos..var_pos + prefix.len()];

            // Check if this looks like DBI->connect result
            if assignment.contains("DBI") && assignment.contains("connect") {
                return Some("DBI::db".to_string());
            }

            // Check if this looks like prepare/prepare_cached result
            if assignment.contains("prepare") {
                return Some("DBI::st".to_string());
            }
        }
    }

    None
}

/// Add method completions
pub fn add_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    symbol_table: &SymbolTable,
) {
    let mut seen = HashSet::new();

    // Prefer discovered in-file methods first (including synthesized framework accessors).
    let method_prefix = context.prefix.rsplit("->").next().unwrap_or(&context.prefix);
    for (name, symbols) in &symbol_table.symbols {
        let is_callable = symbols
            .iter()
            .any(|symbol| matches!(symbol.kind, SymbolKind::Subroutine | SymbolKind::Method));
        if !is_callable {
            continue;
        }

        if !method_prefix.is_empty() && !name.starts_with(method_prefix) {
            continue;
        }

        let documentation = symbols.iter().find_map(|symbol| symbol.documentation.clone());
        if seen.insert(name.clone()) {
            completions.push(CompletionItem {
                label: name.clone(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some("method".to_string()),
                documentation,
                insert_text: Some(format!("{}()", name)),
                sort_text: Some(format!("1_{}", name)),
                filter_text: Some(name.clone()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }

    // Try to infer the receiver type from context
    let receiver_type = infer_receiver_type(context, source);

    // Choose methods based on inferred type
    let methods: Vec<(&str, &str)> = match receiver_type.as_deref() {
        Some("DBI::db") => DBI_DB_METHODS.to_vec(),
        Some("DBI::st") => DBI_ST_METHODS.to_vec(),
        _ => {
            // Default common object methods
            vec![
                ("new", "Constructor"),
                ("isa", "Check if object is of given class"),
                ("can", "Check if object can call method"),
                ("DOES", "Check if object does role"),
                ("VERSION", "Get version"),
            ]
        }
    };

    for (method, desc) in methods {
        let method_name = method.to_string();
        if seen.insert(method_name.clone()) {
            completions.push(CompletionItem {
                label: method_name.clone(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some("method".to_string()),
                documentation: Some(desc.to_string()),
                insert_text: Some(format!("{}()", method)),
                sort_text: Some(format!("2_{}", method)),
                filter_text: Some(method_name),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }

    // If we have a DBI type, also add common methods at lower priority
    if receiver_type.as_deref() == Some("DBI::db") || receiver_type.as_deref() == Some("DBI::st") {
        for (method, desc) in [
            ("isa", "Check if object is of given class"),
            ("can", "Check if object can call method"),
        ] {
            let method_name = method.to_string();
            if seen.insert(method_name.clone()) {
                completions.push(CompletionItem {
                    label: method_name.clone(),
                    kind: crate::completion::items::CompletionItemKind::Function,
                    detail: Some("method".to_string()),
                    documentation: Some(desc.to_string()),
                    insert_text: Some(format!("{}()", method)),
                    sort_text: Some(format!("9_{}", method)), // Lower priority
                    filter_text: Some(method_name),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
        }
    }
}

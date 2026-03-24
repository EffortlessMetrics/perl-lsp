//! Method completion for Perl
//!
//! Provides context-aware method completion including DBI methods.

use super::{auto_import, context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::symbol::{SymbolKind, SymbolTable};
use std::collections::HashSet;

/// Extract the receiver module name from the completion prefix for static calls.
///
/// For `LWP::UserAgent->ge` the prefix is `LWP::UserAgent->ge` and we extract
/// `LWP::UserAgent`.  Returns `None` when the receiver is a variable (`$obj->`)
/// or when the prefix has no `->`.
fn static_receiver_module(prefix: &str) -> Option<&str> {
    let arrow = prefix.rfind("->")?;
    let receiver = prefix[..arrow].trim();
    // Static receivers start with an uppercase ASCII letter and contain no sigil.
    if !receiver.starts_with('$')
        && !receiver.starts_with('@')
        && !receiver.starts_with('%')
        && receiver.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    {
        Some(receiver)
    } else {
        None
    }
}

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

/// Parameter signatures for DBI database-handle methods.
///
/// Each entry is `(name, signature, description)`.
pub const DBI_DB_METHOD_SIGS: &[(&str, &str, &str)] = &[
    ("do", "do($statement, \\@attr?, @bind_values?)", "Execute a single SQL statement"),
    ("prepare", "prepare($statement, \\@attr?)", "Prepare a SQL statement for execution"),
    (
        "prepare_cached",
        "prepare_cached($statement, \\@attr?, $if_active?)",
        "Prepare and cache a SQL statement",
    ),
    (
        "selectrow_array",
        "selectrow_array($statement, \\@attr?, @bind)",
        "Execute and return first row as list",
    ),
    (
        "selectrow_arrayref",
        "selectrow_arrayref($statement, \\@attr?, @bind)",
        "Execute and return first row as arrayref",
    ),
    (
        "selectrow_hashref",
        "selectrow_hashref($statement, \\@attr?, @bind)",
        "Execute and return first row as hashref",
    ),
    (
        "selectall_arrayref",
        "selectall_arrayref($statement, \\@attr?, @bind)",
        "Execute and return all rows as arrayref",
    ),
    (
        "selectall_hashref",
        "selectall_hashref($statement, $key_field, \\@attr?, @bind)",
        "Execute and return all rows as hashref",
    ),
    ("begin_work", "begin_work()", "Begin a database transaction"),
    ("commit", "commit()", "Commit the current transaction"),
    ("rollback", "rollback()", "Rollback the current transaction"),
    ("disconnect", "disconnect()", "Disconnect from the database"),
    (
        "last_insert_id",
        "last_insert_id($catalog, $schema, $table, $field, \\@attr?)",
        "Get the last inserted row ID",
    ),
    ("quote", "quote($value, $data_type?)", "Quote a string value for use in SQL"),
    ("quote_identifier", "quote_identifier($name)", "Quote an identifier for SQL"),
    ("ping", "ping()", "Check if the database connection is still alive"),
];

/// Parameter signatures for DBI statement-handle methods.
///
/// Each entry is `(name, signature, description)`.
pub const DBI_ST_METHOD_SIGS: &[(&str, &str, &str)] = &[
    (
        "bind_param",
        "bind_param($param_num, $bind_value, \\@attr?)",
        "Bind a value to a placeholder",
    ),
    (
        "bind_param_inout",
        "bind_param_inout($param_num, \\$bind_value, $max_len)",
        "Bind an in/out parameter",
    ),
    ("execute", "execute(@bind_values?)", "Execute the prepared statement"),
    ("fetch", "fetch()", "Fetch the next row as arrayref (alias for fetchrow_arrayref)"),
    ("fetchrow_array", "fetchrow_array()", "Fetch the next row as a list"),
    ("fetchrow_arrayref", "fetchrow_arrayref()", "Fetch the next row as an arrayref"),
    ("fetchrow_hashref", "fetchrow_hashref($name?)", "Fetch the next row as a hashref"),
    (
        "fetchall_arrayref",
        "fetchall_arrayref($slice?, $max_rows?)",
        "Fetch all remaining rows as arrayref",
    ),
    (
        "fetchall_hashref",
        "fetchall_hashref($key_field)",
        "Fetch all remaining rows as hashref of hashrefs",
    ),
    ("finish", "finish()", "Indicate no more rows will be fetched"),
    ("rows", "rows()", "Return the number of rows affected or returned"),
];

/// Look up DBI method documentation by receiver hint and method name.
///
/// `receiver_hint` is the variable name or token before `->` (e.g. `"$dbh"`, `"$sth"`).
/// Returns `(signature, description)` or `None` if not a known DBI method.
///
/// When the receiver is ambiguous, database-handle methods take priority.
pub fn get_dbi_method_documentation(
    receiver_hint: &str,
    method_name: &str,
) -> Option<(&'static str, &'static str)> {
    let is_db = receiver_hint.ends_with("dbh")
        || receiver_hint.contains("DBI")
        || receiver_hint.contains("connect");
    let is_st = receiver_hint.ends_with("sth");

    let table: &[(&str, &str, &str)] = if is_db {
        DBI_DB_METHOD_SIGS
    } else if is_st {
        DBI_ST_METHOD_SIGS
    } else {
        // Unknown receiver — check db table first, then st table
        if let Some(entry) = DBI_DB_METHOD_SIGS.iter().find(|(n, _, _)| *n == method_name) {
            return Some((entry.1, entry.2));
        }
        DBI_ST_METHOD_SIGS
    };

    table.iter().find(|(n, _, _)| *n == method_name).map(|(_, sig, desc)| (*sig, *desc))
}

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

/// Build rich documentation for a Moo/Moose accessor from its symbol attributes.
///
/// Attributes are stored as `key=value` strings (e.g. `"is=ro"`, `"isa=Str"`).
/// This function formats them into a human-readable documentation string that
/// surfaces the type constraint and access mode prominently.
fn moo_accessor_documentation(name: &str, attributes: &[String]) -> String {
    let mut isa_value: Option<&str> = None;
    let mut is_value: Option<&str> = None;
    let mut extra_parts: Vec<&str> = Vec::new();

    for attr in attributes {
        if let Some((key, value)) = attr.split_once('=') {
            match key {
                "isa" => isa_value = Some(value),
                "is" => is_value = Some(value),
                _ => extra_parts.push(attr),
            }
        }
    }

    let mut doc = format!("Moo/Moose accessor `{name}`");

    if let Some(isa) = isa_value {
        doc.push_str(&format!("\n\n**Type**: `{isa}`"));
    }
    if let Some(is) = is_value {
        let mode = match is {
            "ro" => "read-only",
            "rw" => "read-write",
            "rwp" => "read-write private",
            "lazy" => "lazy",
            other => other,
        };
        doc.push_str(&format!("\n\n**Access**: {mode}"));
    }
    if !extra_parts.is_empty() {
        doc.push_str(&format!("\n\n**Options**: {}", extra_parts.join(", ")));
    }

    doc
}

/// Add method completions
pub fn add_method_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    symbol_table: &SymbolTable,
) {
    let mut seen: HashSet<&str> = HashSet::new();

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

        // Check if this is a synthesized Moo/Moose accessor (declaration == "has")
        let callable_symbol = symbols
            .iter()
            .find(|symbol| matches!(symbol.kind, SymbolKind::Subroutine | SymbolKind::Method));

        let is_moo_accessor =
            callable_symbol.and_then(|s| s.declaration.as_deref()).is_some_and(|d| d == "has");

        let (detail, documentation) = if is_moo_accessor {
            let attrs = callable_symbol.map(|s| s.attributes.as_slice()).unwrap_or(&[]);
            ("Moo/Moose accessor".to_string(), Some(moo_accessor_documentation(name, attrs)))
        } else {
            let doc = symbols.iter().find_map(|symbol| symbol.documentation.clone());
            ("method".to_string(), doc)
        };

        if seen.insert(name.as_str()) {
            completions.push(CompletionItem {
                label: name.clone(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some(detail),
                documentation,
                insert_text: Some(format!("{}()", name)),
                sort_text: Some(format!("1_{}", name)),
                filter_text: Some(name.clone()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }

    // Try to infer the receiver type from context
    let receiver_type = infer_receiver_type(context, source);

    // Determine module for auto-import:
    // - For static calls like `LWP::UserAgent->` use the prefix receiver.
    // - For DBI-inferred types use "DBI".
    let import_module: Option<&str> =
        static_receiver_module(&context.prefix).or(match receiver_type.as_deref() {
            Some("DBI::db") | Some("DBI::st") => Some("DBI"),
            _ => None,
        });

    // Build an auto-import edit once so all items in this batch share it.
    let auto_import_edit =
        import_module.and_then(|m| auto_import::build_auto_import_edit(source, m));

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
        if seen.insert(method) {
            let additional_edits =
                auto_import_edit.as_ref().map(|e| vec![e.clone()]).unwrap_or_default();
            completions.push(CompletionItem {
                label: method.to_string(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some("method".to_string()),
                documentation: Some(desc.to_string()),
                insert_text: Some(format!("{}()", method)),
                sort_text: Some(format!("2_{}", method)),
                filter_text: Some(method.to_string()),
                additional_edits,
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }

    // If we have a DBI type, also add common methods at lower priority
    if receiver_type.as_deref() == Some("DBI::db") || receiver_type.as_deref() == Some("DBI::st") {
        for (method, desc) in [
            ("isa", "Check if object is of given class"),
            ("can", "Check if object can call method"),
        ] {
            if seen.insert(method) {
                let additional_edits =
                    auto_import_edit.as_ref().map(|e| vec![e.clone()]).unwrap_or_default();
                completions.push(CompletionItem {
                    label: method.to_string(),
                    kind: crate::completion::items::CompletionItemKind::Function,
                    detail: Some("method".to_string()),
                    documentation: Some(desc.to_string()),
                    insert_text: Some(format!("{}()", method)),
                    sort_text: Some(format!("9_{}", method)), // Lower priority
                    filter_text: Some(method.to_string()),
                    additional_edits,
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
        }
    }
}

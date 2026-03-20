//! Method completion for Perl
//!
//! Provides context-aware method completion including DBI methods.

use super::{context::CompletionContext, items::CompletionItem};
use perl_semantic_analyzer::class_model::{AccessorType, ClassModel, Framework};
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
        if seen.insert(method) {
            completions.push(CompletionItem {
                label: method.to_string(),
                kind: crate::completion::items::CompletionItemKind::Function,
                detail: Some("method".to_string()),
                documentation: Some(desc.to_string()),
                insert_text: Some(format!("{}()", method)),
                sort_text: Some(format!("2_{}", method)),
                filter_text: Some(method.to_string()),
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
            if seen.insert(method) {
                completions.push(CompletionItem {
                    label: method.to_string(),
                    kind: crate::completion::items::CompletionItemKind::Function,
                    detail: Some("method".to_string()),
                    documentation: Some(desc.to_string()),
                    insert_text: Some(format!("{}()", method)),
                    sort_text: Some(format!("9_{}", method)), // Lower priority
                    filter_text: Some(method.to_string()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                });
            }
        }
    }
}

/// Format framework name for display.
fn framework_label(framework: Framework) -> &'static str {
    match framework {
        Framework::Moose => "Moose",
        Framework::Moo => "Moo",
        Framework::Mouse => "Mouse",
        Framework::ClassAccessor => "Class::Accessor",
        Framework::ObjectPad => "Object::Pad",
        Framework::Native | Framework::None => "Perl",
    }
}

/// Format accessor type for display.
fn accessor_type_label(is: Option<AccessorType>) -> &'static str {
    match is {
        Some(AccessorType::Ro) => "ro",
        Some(AccessorType::Rw) => "rw",
        Some(AccessorType::Lazy) => "lazy",
        Some(AccessorType::Bare) => "bare",
        None => "accessor",
    }
}

/// Add completions from ClassModel data (Moose/Moo attributes and methods).
///
/// This supplements the symbol-table-based completions with richer information
/// from the ClassModel, including accessor type, type constraints, and
/// framework-specific details. Items already present (by label) are skipped.
pub fn add_class_model_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    class_models: &[ClassModel],
) {
    let method_prefix = context.prefix.rsplit("->").next().unwrap_or(&context.prefix);

    // Collect labels already added to avoid duplicates
    let existing: HashSet<String> = completions.iter().map(|c| c.label.clone()).collect();

    // Find the class model for the current package
    let current_model = class_models.iter().find(|m| m.name == context.current_package);

    let Some(model) = current_model else {
        return;
    };

    let fw = framework_label(model.framework);

    // Add attribute accessors
    for attr in &model.attributes {
        // Skip bare accessors (no accessor generated)
        if attr.is == Some(AccessorType::Bare) {
            continue;
        }

        let accessor = &attr.accessor_name;
        if !method_prefix.is_empty() && !accessor.starts_with(method_prefix) {
            continue;
        }
        if existing.contains(accessor) {
            continue;
        }

        let mode = accessor_type_label(attr.is);
        let detail = format!("{fw} attribute ({mode})");

        let mut doc = format!("`{fw}` accessor `{accessor}`");
        if let Some(ref isa) = attr.isa {
            doc.push_str(&format!("\n\n**Type**: `{isa}`"));
        }
        if attr.required {
            doc.push_str("\n\n**Required**");
        }

        completions.push(CompletionItem {
            label: accessor.clone(),
            kind: crate::completion::items::CompletionItemKind::Property,
            detail: Some(detail),
            documentation: Some(doc),
            insert_text: Some(accessor.clone()),
            sort_text: Some(format!("0_{accessor}")),
            filter_text: Some(accessor.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }

    // Add methods from the class model
    for method in &model.methods {
        if !method_prefix.is_empty() && !method.name.starts_with(method_prefix) {
            continue;
        }
        if existing.contains(&method.name) {
            continue;
        }

        completions.push(CompletionItem {
            label: method.name.clone(),
            kind: crate::completion::items::CompletionItemKind::Function,
            detail: Some(format!("{fw} method")),
            documentation: None,
            insert_text: Some(format!("{}()", method.name)),
            sort_text: Some(format!("1_{}", method.name)),
            filter_text: Some(method.name.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

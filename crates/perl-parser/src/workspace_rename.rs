//! Workspace Rename Provider for LSP
//!
//! Provides cross-file renaming functionality using the workspace index.

use crate::workspace_index::{SymKind, SymbolKey, WorkspaceIndex};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Represents a text edit for a single document
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub start: (u32, u32), // (line, character) in UTF-16
    pub end: (u32, u32),
    pub new_text: String,
}

/// Represents edits to a single document
#[derive(Debug, Clone)]
pub struct RenameEdit {
    pub uri: String,
    pub edits: Vec<TextEdit>,
}

/// Build a rename edit across the workspace
pub fn build_rename_edit(
    idx: &WorkspaceIndex,
    key: &SymbolKey,
    new_name_bare: &str,
) -> Vec<RenameEdit> {
    // 1) Get all references across the workspace
    let mut locs = idx.find_refs(key);

    // 2) Also include the definition itself
    if let Some(def) = idx.find_def(key) {
        locs.push(def);
    }

    // 3) Group edits by URI and compute replacement text
    let mut grouped: BTreeMap<String, Vec<TextEdit>> = BTreeMap::new();

    for loc in locs {
        let start_line = loc.range.start.line;
        let start_char = loc.range.start.character;
        let end_line = loc.range.end.line;
        let end_char = loc.range.end.character;

        // Compute replacement text based on symbol kind
        let replacement = match key.kind {
            SymKind::Var => {
                // Preserve the sigil for variables
                let sigil = key.sigil.unwrap_or('$');
                format!("{}{}", sigil, new_name_bare)
            }
            SymKind::Sub => {
                // For subroutines, use the bare name
                // TODO: Preserve qualifiers if present in original
                new_name_bare.to_string()
            }
            SymKind::Pack => {
                // Package names are replaced as-is
                new_name_bare.to_string()
            }
        };

        grouped.entry(loc.uri.clone()).or_default().push(TextEdit {
            start: (start_line, start_char),
            end: (end_line, end_char),
            new_text: replacement,
        });
    }

    // Convert to RenameEdit structs
    grouped.into_iter().map(|(uri, edits)| RenameEdit { uri, edits }).collect()
}

/// Convert RenameEdit to LSP WorkspaceEdit JSON
pub fn to_workspace_edit(edits: Vec<RenameEdit>) -> Value {
    let mut changes: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for rename_edit in edits {
        let text_edits: Vec<Value> = rename_edit
            .edits
            .into_iter()
            .map(|te| {
                json!({
                    "range": {
                        "start": { "line": te.start.0, "character": te.start.1 },
                        "end": { "line": te.end.0, "character": te.end.1 }
                    },
                    "newText": te.new_text
                })
            })
            .collect();

        changes.insert(rename_edit.uri, text_edits);
    }

    json!({ "changes": changes })
}

/// Check if a rename is valid for the given symbol
pub fn validate_rename(_key: &SymbolKey, new_name: &str) -> Result<(), String> {
    // Basic validation
    if new_name.is_empty() {
        return Err("New name cannot be empty".to_string());
    }

    // Check for valid Perl identifier
    if !new_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(
            "Invalid identifier: must contain only alphanumeric characters and underscores"
                .to_string(),
        );
    }

    // Check first character is not a digit
    if new_name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err("Identifier cannot start with a digit".to_string());
    }

    Ok(())
}

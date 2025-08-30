//! Workspace Rename Provider for LSP
//!
//! Provides cross-file renaming functionality using the workspace index.

use crate::workspace_index::{Location, SymKind, SymbolKey, WorkspaceIndex};
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

        // Inspect original text for qualified names
        let qualifier = extract_qualifier(idx, &loc);

        // Compute replacement text based on symbol kind
        let replacement = match key.kind {
            SymKind::Var => {
                // Preserve the sigil for variables
                let sigil = key.sigil.unwrap_or('$');
                format!("{}{}", sigil, new_name_bare)
            }
            SymKind::Sub | SymKind::Pack => {
                // Preserve module qualifiers if present
                if let Some(q) = qualifier {
                    format!("{}{}", q, new_name_bare)
                } else {
                    new_name_bare.to_string()
                }
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

// Extract the module qualifier (e.g., "Some::Module::") from the original text
fn extract_qualifier(idx: &WorkspaceIndex, loc: &Location) -> Option<String> {
    let mut doc = idx.document_store().get(&loc.uri)?;
    let start =
        doc.line_index.position_to_offset(loc.range.start.line, loc.range.start.character)?;
    let end = doc.line_index.position_to_offset(loc.range.end.line, loc.range.end.character)?;
    let original = &doc.text[start..end];
    if let Some(pos) = original.rfind("::") { Some(original[..pos + 2].to_string()) } else { None }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use url::Url;

    #[test]
    fn rename_preserves_qualified_sub() {
        let index = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        let code = r#"package Some::Module;

sub my_sub {}
Some::Module::my_sub();
"#;
        index.index_file(Url::parse(uri).unwrap(), code.to_string()).unwrap();

        let key = SymbolKey {
            pkg: Arc::from("Some::Module"),
            name: Arc::from("my_sub"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&index, &key, "renamed");
        let file_edits = edits.into_iter().find(|e| e.uri == uri).expect("file edits");
        let new_texts: Vec<String> = file_edits.edits.iter().map(|e| e.new_text.clone()).collect();

        assert!(new_texts.contains(&"Some::Module::renamed".to_string()));
        assert!(new_texts.contains(&"renamed".to_string()));
    }
}

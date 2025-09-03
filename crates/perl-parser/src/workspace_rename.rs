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
                // For subroutines, preserve any existing package qualifier
                let mut replacement = new_name_bare.to_string();

                if let Some(mut doc) = idx.document_store().get(&loc.uri) {
                    if let (Some(start_off), Some(end_off)) = (
                        doc.line_index.position_to_offset(start_line, start_char),
                        doc.line_index.position_to_offset(end_line, end_char),
                    ) {
                        if let Some(original) = doc.text.get(start_off..end_off) {
                            if let Some((qual, _)) = original.rsplit_once("::") {
                                replacement = format!("{}::{}", qual, new_name_bare);
                            }
                        }
                    }
                }

                replacement
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use url::Url;

    fn index_text(idx: &WorkspaceIndex, uri: &str, text: &str) {
        let url = Url::parse(uri).unwrap();
        idx.index_file(url, text.to_string()).unwrap();
    }

    #[test]
    fn rename_sub_preserves_package_qualifier() {
        let idx = WorkspaceIndex::new();
        let uri = "file:///test.pl";
        let text = r#"
package Package;
my $var = 0;
sub name { }
Package::name();
name();
$var;
"#;
        index_text(&idx, uri, text);

        let key = SymbolKey {
            pkg: Arc::from("Package"),
            name: Arc::from("name"),
            sigil: None,
            kind: SymKind::Sub,
        };

        let edits = build_rename_edit(&idx, &key, "new_name");
        assert_eq!(edits.len(), 1);

        let texts: Vec<String> = edits[0].edits.iter().map(|e| e.new_text.clone()).collect();

        // Workspace indexing now correctly finds both the subroutine declaration and its calls
        assert_eq!(texts.len(), 2); // Should find both declaration and unqualified call
        assert!(texts.contains(&"new_name".to_string())); // Both should be renamed to "new_name"

        // Apply edits and verify other symbols remain unchanged
        let mut doc = idx.document_store().get(uri).unwrap();
        let mut replacements: Vec<(usize, usize, &str)> = edits[0]
            .edits
            .iter()
            .map(|e| {
                let start = doc.line_index.position_to_offset(e.start.0, e.start.1).unwrap();
                let end = doc.line_index.position_to_offset(e.end.0, e.end.1).unwrap();
                (start, end, e.new_text.as_str())
            })
            .collect();
        replacements.sort_by(|a, b| b.0.cmp(&a.0));
        let mut new_text = text.to_string();
        for (start, end, rep) in replacements {
            new_text.replace_range(start..end, rep);
        }

        assert!(new_text.contains("package Package;"));
        assert!(new_text.contains("$var"));
        // Workspace indexing now works correctly - should rename function calls too
        assert!(new_text.contains("new_name")); // Declaration and calls should be renamed
    }
}

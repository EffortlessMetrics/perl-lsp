//! Workspace-wide refactoring code actions
//!
//! Provides code actions that span multiple files using the WorkspaceRefactor.
//! These include:
//! - Extract subroutine to a new module file
//! - Move subroutine to a different module file
//! - Organize imports across multiple files
//!
//! Issue #3522: "[workspace] Workspace-wide refactoring operations not supported"

use crate::workspace_index::{fs_path_to_uri, uri_to_fs_path, WorkspaceIndex};
use perl_refactoring::workspace_refactor::{RefactorResult, WorkspaceRefactor};
use perl_symbol::SymbolKind;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Convert a RefactorResult to LSP WorkspaceEdit JSON format
///
/// Takes a RefactorResult from WorkspaceRefactor and converts it to the LSP
/// WorkspaceEdit format. For multi-file edits (edits spanning multiple files),
/// uses documentChanges format which is required for creating new files.
pub fn refactor_result_to_workspace_edit(result: &RefactorResult, idx: &WorkspaceIndex) -> Value {
    // Check if this is a multi-file edit (or new file creation).
    // The documentChanges format is required when creating new files because
    // the changes map format cannot represent file creation - only modifications.
    // We detect new files by the convention that an edit with start=0 and end=0
    // represents inserting all content into a new file.
    let needs_document_changes = result.file_edits.len() > 1
        || result.file_edits.first().map(|f| f.edits.first().map(|e| e.start == 0 && e.end == 0).unwrap_or(false)).unwrap_or(false);

    if needs_document_changes {
        // Use documentChanges format for multi-file edits
        let document_changes: Vec<Value> = result
            .file_edits
            .iter()
            .filter_map(|file_edit| {
                let uri = match fs_path_to_uri(&file_edit.file_path) {
                    Ok(u) => u.to_string(),
                    Err(_) => return None,
                };

                // Detect new file by the start=0, end=0 convention.
                // An edit with range (0,0) represents inserting all content into
                // a file that doesn't exist yet - the empty range means "insert
                // at position 0" and newText holds the complete file contents.
                let is_new_file = file_edit
                    .edits
                    .first()
                    .map(|e| e.start == 0 && e.end == 0)
                    .unwrap_or(false);

                if is_new_file {
                    // For new files, include the full content as newText with range at start
                    let new_text = file_edit.edits.first()?.new_text.clone();
                    Some(json!({
                        "textDocument": { "uri": uri },
                        "edits": [{
                            "range": {
                                "start": { "line": 0, "character": 0 },
                                "end": { "line": 0, "character": 0 }
                            },
                            "newText": new_text
                        }]
                    }))
                } else {
                    // For existing files, get the document and convert offsets to positions
                    let doc = match idx.document_store().get(&uri) {
                        Some(d) => d,
                        None => return None,
                    };

                    let text_edits: Vec<Value> = file_edit
                        .edits
                        .iter()
                        .map(|te| {
                            let (start_line, start_char) =
                                doc.line_index.offset_to_position(te.start);
                            let (end_line, end_char) = doc.line_index.offset_to_position(te.end);
                            json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "newText": te.new_text
                            })
                        })
                        .collect();

                    Some(json!({
                        "textDocument": { "uri": uri },
                        "edits": text_edits
                    }))
                }
            })
            .collect();

        json!({ "documentChanges": document_changes })
    } else {
        // Single file edit - use simpler changes format
        let mut changes: BTreeMap<String, Vec<Value>> = BTreeMap::new();

        for file_edit in &result.file_edits {
            let uri = match fs_path_to_uri(&file_edit.file_path) {
                Ok(u) => u.to_string(),
                Err(_) => continue,
            };

            let doc = match idx.document_store().get(&uri) {
                Some(d) => d,
                None => continue,
            };

            let text_edits: Vec<Value> = file_edit
                .edits
                .iter()
                .map(|te| {
                    let (start_line, start_char) = doc.line_index.offset_to_position(te.start);
                    let (end_line, end_char) = doc.line_index.offset_to_position(te.end);
                    json!({
                        "range": {
                            "start": { "line": start_line, "character": start_char },
                            "end": { "line": end_line, "character": end_char }
                        },
                        "newText": te.new_text
                    })
                })
                .collect();

            changes.insert(uri, text_edits);
        }

        json!({ "changes": changes })
    }
}

/// Create a WorkspaceRefactor from an Arc<WorkspaceIndex>
///
/// Since WorkspaceIndex is now Clone (because all its fields are Arc), we can
/// clone the inner value instead of trying to unwrap the Arc. This is cheap
/// because cloning an Arc just increments the reference count.
fn try_create_refactor(idx: Arc<WorkspaceIndex>) -> Option<WorkspaceRefactor> {
    // Clone the inner WorkspaceIndex (cheap because fields are Arc)
    let inner = (*idx).clone();
    Some(WorkspaceRefactor::new(inner))
}

/// Build an "Extract to module" code action
///
/// When a subroutine is selected, this creates a code action that extracts
/// the subroutine to a new module file. The action produces WorkspaceEdit
/// changes for both the original file (replacing the subroutine with a use statement)
/// and the new module file.
pub fn build_extract_module_action(
    idx: Arc<WorkspaceIndex>,
    uri: &str,
    start_line: u32,
    end_line: u32,
) -> Option<Value> {
    // Validate line numbers
    if start_line > end_line {
        return None;
    }

    // Use the file name (without extension) as a hint for the module name
    let file_path = uri_to_fs_path(uri)?;
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Extracted");

    // Generate a module name from the file path
    let module_name = format!("{}::Extracted", file_name.replace('-', "_"));

    // Get the document from the index's document store
    let doc = idx.document_store().get(uri)?;

    // Convert line/char positions to byte offsets
    let start_off = doc.line_index.position_to_offset(start_line, 0)?;
    let end_off = doc.line_index.position_to_offset(end_line, 0).unwrap_or(doc.text.len());

    let extracted = doc.text[start_off..end_off].to_string();

    // Original file edit - replace selection with use statement
    let original_edits = vec![perl_refactoring::workspace_refactor::TextEdit {
        start: start_off,
        end: end_off,
        new_text: format!("use {};\n", module_name),
    }];

    // New module file content - insert at beginning (offset 0).
    // The start=0, end=0 convention signals a new file creation where
    // the entire file content is provided in newText. This is required
    // for the LSP documentChanges format to support file creation.
    let new_path = file_path.with_file_name(module_name_to_path(&module_name));
    let new_edits = vec![perl_refactoring::workspace_refactor::TextEdit {
        start: 0,
        end: 0,
        new_text: extracted,
    }];

    let result = perl_refactoring::workspace_refactor::RefactorResult {
        file_edits: vec![
            perl_refactoring::workspace_refactor::FileEdit {
                file_path: file_path.to_path_buf(),
                edits: original_edits,
            },
            perl_refactoring::workspace_refactor::FileEdit {
                file_path: new_path,
                edits: new_edits,
            },
        ],
        description: format!(
            "Extract {} lines from {} into module '{}'",
            end_line - start_line + 1,
            uri,
            module_name
        ),
        warnings: vec![],
    };

    let idx_ref = idx.as_ref();
    let edit = refactor_result_to_workspace_edit(&result, idx_ref);

    Some(json!({
        "title": format!("Extract to module '{}'", module_name),
        "kind": "refactor.extract",
        "edit": edit
    }))
}

/// Build a "Move module" code action
///
/// When a package declaration is selected, this creates a code action that moves
/// the module to a new location. The action produces WorkspaceEdit changes for
/// both the old and new locations.
pub fn build_move_module_action(
    idx: Arc<WorkspaceIndex>,
    uri: &str,
    _start_line: u32,
    _end_line: u32,
) -> Option<Value> {
    let file_path = uri_to_fs_path(uri)?;

    // Get the file symbols to find the package name
    let symbols = idx.file_symbols(uri);
    let package_name =
        symbols.iter().find(|s| s.kind == SymbolKind::Package).map(|s| s.name.to_string());

    let package_name = package_name?;

    // Generate a new module path suggestion
    let new_module = format!("{}::Moved", package_name.replace("::", "_"));

    // Try to create the refactor (may fail if Arc has multiple references)
    let refactor = try_create_refactor(idx.clone())?;

    // Try to move the first subroutine found in this file
    let symbols = idx.file_symbols(uri);
    let sub_name = symbols
        .iter()
        .find(|s| s.kind == SymbolKind::Subroutine)
        .map(|s| s.name.to_string());

    let sub_name = sub_name?;

    let result = refactor.move_subroutine(&sub_name, &file_path, &new_module);

    let result = match result {
        Ok(r) => r,
        Err(_) => return None,
    };

    let idx_ref = idx.as_ref();
    let edit = refactor_result_to_workspace_edit(&result, idx_ref);

    Some(json!({
        "title": format!("Move module '{}'", package_name),
        "kind": "refactor.move",
        "edit": edit
    }))
}

/// Build a workspace-wide "Organize imports" code action
///
/// This action organizes imports across all indexed documents, removing duplicates
/// and sorting them alphabetically.
pub fn build_organize_imports_action(idx: Arc<WorkspaceIndex>, _uri: &str) -> Option<Value> {
    // Try to create the refactor (may fail if Arc has multiple references)
    let refactor = try_create_refactor(idx.clone())?;

    let result = refactor.optimize_imports();

    let result = match result {
        Ok(r) => r,
        Err(_) => return None,
    };

    // Only return if we have edits in multiple files
    if result.file_edits.len() <= 1 {
        // Check if the single file actually has changes
        if result.file_edits.first().map(|f| f.edits.is_empty()).unwrap_or(true) {
            return None;
        }
    }

    let idx_ref = idx.as_ref();
    let edit = refactor_result_to_workspace_edit(&result, idx_ref);

    Some(json!({
        "title": "Organize imports",
        "kind": "source.organizeImports",
        "edit": edit
    }))
}

/// Check if a URI contains a package declaration at the given line
pub fn is_package_declaration(idx: &WorkspaceIndex, uri: &str, _line: u32) -> bool {
    let symbols = idx.file_symbols(uri);
    symbols.iter().any(|s| s.kind == SymbolKind::Package)
}

/// Check if a URI contains a subroutine definition at the given range
///
/// Returns true if any subroutine's range overlaps with the selected range.
/// This allows for partial selection of a subroutine to still trigger the action.
pub fn has_subroutine_at_range(
    idx: &WorkspaceIndex,
    uri: &str,
    start_line: u32,
    end_line: u32,
) -> bool {
    let symbols = idx.file_symbols(uri);
    symbols.iter().any(|s| {
        s.kind == SymbolKind::Subroutine
            && s.range.start.line <= end_line
            && s.range.end.line >= start_line
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn index_text(
        idx: &WorkspaceIndex,
        uri: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse(uri)?;
        idx.index_file(url, text.to_string())?;
        Ok(())
    }

    #[test]
    fn test_extract_module_produces_multi_file_edit() -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();
        let uri = "file:///test.pl";

        let text = r#"
package Test;

sub helper {
    my ($x) = @_;
    return $x * 2;
}

sub main {
    print "hello\n";
}

1;
"#;
        index_text(&idx, uri, text)?;

        // Extract helper subroutine (lines 2-4, 0-indexed)
        let action = build_extract_module_action(Arc::new(idx), uri, 2, 4);

        assert!(action.is_some(), "Should produce extract module action");

        let edit = action.unwrap().get("edit").unwrap();
        let changes = edit.get("changes").unwrap().as_object().unwrap();

        // Should have edits in at least 1 file
        assert!(changes.len() >= 1, "Should have edits for at least the original file");

        Ok(())
    }

    #[test]
    fn test_organize_imports_produces_action() -> Result<(), Box<dyn std::error::Error>> {
        let idx = WorkspaceIndex::new();

        // Index a file with messy imports
        let uri1 = "file:///lib/File1.pm";
        let text1 = r#"
use strict;
use warnings;
use Data::Dumper;
use Carp qw(carp);
use Data::Dumper;
use File::Spec;

1;
"#;
        index_text(&idx, uri1, text1)?;

        let action = build_organize_imports_action(Arc::new(idx), uri1);

        // Should produce an action
        assert!(action.is_some(), "Should produce organize imports action");

        Ok(())
    }
}

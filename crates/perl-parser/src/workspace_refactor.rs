//! Workspace-wide refactoring operations (stub implementation)
//!
//! This module provides refactoring capabilities that span multiple files.
//! Currently a stub implementation to demonstrate the architecture.

use crate::workspace_index::{
    SymKind, SymbolKey, WorkspaceIndex, fs_path_to_uri, normalize_var, uri_to_fs_path,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A file edit as part of a refactoring operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    pub file_path: PathBuf,
    pub edits: Vec<TextEdit>,
}

/// A single text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub new_text: String,
}

/// Result of a refactoring operation
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorResult {
    pub file_edits: Vec<FileEdit>,
    pub description: String,
    pub warnings: Vec<String>,
}

/// Workspace-wide refactoring provider
pub struct WorkspaceRefactor {
    _index: WorkspaceIndex,
}

impl WorkspaceRefactor {
    pub fn new(index: WorkspaceIndex) -> Self {
        Self { _index: index }
    }

    /// Rename a symbol across all files
    pub fn rename_symbol(
        &self,
        old_name: &str,
        new_name: &str,
        _file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, String> {
        // Infer symbol kind and bare name
        let (sigil, bare) = normalize_var(old_name);
        let kind = if sigil.is_some() { SymKind::Var } else { SymKind::Sub };

        // For now assume package 'main'
        let key = SymbolKey {
            pkg: Arc::from("main".to_string()),
            name: Arc::from(bare.to_string()),
            sigil,
            kind,
        };

        let mut edits: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();

        // Find all references including definition
        let mut locations = self._index.find_refs(&key);
        if let Some(def) = self._index.find_def(&key) {
            locations.push(def);
        }

        let store = self._index.document_store();

        if locations.is_empty() {
            // Fallback naive search
            for doc in store.all_documents() {
                let mut idx = doc.line_index.clone();
                let mut pos = 0;
                while let Some(found) = doc.text[pos..].find(old_name) {
                    let start = pos + found;
                    let end = start + old_name.len();
                    let (start_line, start_col) = idx.offset_to_position(start);
                    let (end_line, end_col) = idx.offset_to_position(end);
                    locations.push(crate::workspace_index::Location {
                        uri: doc.uri.clone(),
                        range: lsp_types::Range {
                            start: lsp_types::Position { line: start_line, character: start_col },
                            end: lsp_types::Position { line: end_line, character: end_col },
                        },
                    });
                    pos = end;
                }
            }
        }

        for loc in locations {
            let path = uri_to_fs_path(&loc.uri).ok_or("invalid uri")?;
            if let Some(mut doc) = store.get(&loc.uri) {
                if let (Some(start_off), Some(end_off)) = (
                    doc.line_index
                        .position_to_offset(loc.range.start.line, loc.range.start.character),
                    doc.line_index.position_to_offset(loc.range.end.line, loc.range.end.character),
                ) {
                    let replacement = match kind {
                        SymKind::Var => {
                            let sig = sigil.unwrap_or('$');
                            format!("{}{}", sig, new_name)
                        }
                        _ => new_name.to_string(),
                    };
                    edits.entry(path).or_default().push(TextEdit {
                        start: start_off,
                        end: end_off,
                        new_text: replacement,
                    });
                }
            }
        }

        let file_edits =
            edits.into_iter().map(|(file_path, edits)| FileEdit { file_path, edits }).collect();

        Ok(RefactorResult {
            file_edits,
            description: format!("Rename '{}' to '{}'", old_name, new_name),
            warnings: vec![],
        })
    }

    /// Extract selected code into a new module
    pub fn extract_module(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        module_name: &str,
    ) -> Result<RefactorResult, String> {
        let uri = fs_path_to_uri(file_path).map_err(|e| e.to_string())?;
        let store = self._index.document_store();
        let doc = store.get(&uri).ok_or_else(|| "document not indexed".to_string())?;
        let mut idx = doc.line_index.clone();

        // Determine byte offsets for lines
        let start_off =
            idx.position_to_offset(start_line as u32 - 1, 0).ok_or("invalid start line")?;
        let end_off = idx.position_to_offset(end_line as u32, 0).unwrap_or_else(|| doc.text.len());

        let extracted = doc.text[start_off..end_off].to_string();

        // Original file edit - replace selection with use statement
        let original_edits = vec![TextEdit {
            start: start_off,
            end: end_off,
            new_text: format!("use {};\n", module_name),
        }];

        // New module file content
        let new_path = file_path.with_file_name(format!("{}.pm", module_name));
        let new_edits = vec![TextEdit { start: 0, end: 0, new_text: extracted }];

        let file_edits = vec![
            FileEdit { file_path: file_path.to_path_buf(), edits: original_edits },
            FileEdit { file_path: new_path.clone(), edits: new_edits },
        ];

        Ok(RefactorResult {
            file_edits,
            description: format!(
                "Extract {} lines from {} into module '{}'",
                end_line - start_line + 1,
                file_path.display(),
                module_name
            ),
            warnings: vec![],
        })
    }

    /// Optimize imports across the workspace
    pub fn optimize_imports(&self) -> Result<RefactorResult, String> {
        let store = self._index.document_store();
        let mut edits: Vec<FileEdit> = Vec::new();

        for doc in store.all_documents() {
            let deps = self._index.file_dependencies(&doc.uri);
            if deps.is_empty() {
                continue;
            }

            // Collect existing use lines
            let lines: Vec<&str> = doc.text.lines().collect();
            let mut use_lines: Vec<usize> = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("use ") {
                    use_lines.push(i);
                }
            }

            if use_lines.is_empty() {
                continue;
            }

            let start_line = *use_lines.first().unwrap();
            let end_line = *use_lines.last().unwrap();
            let mut idx = doc.line_index.clone();
            let start_off = idx.position_to_offset(start_line as u32, 0).ok_or("offset")?;
            let end_off =
                idx.position_to_offset(end_line as u32 + 1, 0).unwrap_or_else(|| doc.text.len());

            let mut deps_vec: Vec<String> = deps.into_iter().collect();
            deps_vec.sort();
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            for d in deps_vec {
                if seen.insert(d.clone()) {
                    unique.push(format!("use {};", d));
                }
            }
            let new_block = unique.join("\n") + "\n";

            edits.push(FileEdit {
                file_path: uri_to_fs_path(&doc.uri).ok_or("invalid uri")?,
                edits: vec![TextEdit { start: start_off, end: end_off, new_text: new_block }],
            });
        }

        Ok(RefactorResult {
            file_edits: edits,
            description: "Optimize imports across workspace".to_string(),
            warnings: vec![],
        })
    }

    /// Move a subroutine to another module
    pub fn move_subroutine(
        &self,
        sub_name: &str,
        from_file: &Path,
        to_module: &str,
    ) -> Result<RefactorResult, String> {
        let uri = fs_path_to_uri(from_file).map_err(|e| e.to_string())?;
        let symbols = self._index.file_symbols(&uri);
        let sym = symbols
            .into_iter()
            .find(|s| s.name == sub_name)
            .ok_or_else(|| "subroutine not found".to_string())?;

        let store = self._index.document_store();
        let doc = store.get(&uri).ok_or("document not indexed")?;
        let mut idx = doc.line_index.clone();
        let start_off = idx
            .position_to_offset(sym.range.start.line, sym.range.start.character)
            .ok_or("start")?;
        let end_off =
            idx.position_to_offset(sym.range.end.line, sym.range.end.character).ok_or("end")?;
        let sub_text = doc.text[start_off..end_off].to_string();

        // Remove from original file
        let mut file_edits = vec![FileEdit {
            file_path: from_file.to_path_buf(),
            edits: vec![TextEdit { start: start_off, end: end_off, new_text: String::new() }],
        }];

        // Append to new module file
        let target_path = from_file.with_file_name(format!("{}.pm", to_module));
        let target_uri = fs_path_to_uri(&target_path).map_err(|e| e.to_string())?;
        let target_doc = store.get(&target_uri);
        let insertion_offset = target_doc.as_ref().map(|d| d.text.len()).unwrap_or(0);

        file_edits.push(FileEdit {
            file_path: target_path.clone(),
            edits: vec![TextEdit {
                start: insertion_offset,
                end: insertion_offset,
                new_text: sub_text,
            }],
        });

        Ok(RefactorResult {
            file_edits,
            description: format!(
                "Move subroutine '{}' from {} to module '{}'",
                sub_name,
                from_file.display(),
                to_module
            ),
            warnings: vec![],
        })
    }

    /// Inline a variable across its scope
    pub fn inline_variable(
        &self,
        var_name: &str,
        file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, String> {
        let (sigil, bare) = normalize_var(var_name);
        let _key = SymbolKey {
            pkg: Arc::from("main".to_string()),
            name: Arc::from(bare.to_string()),
            sigil,
            kind: SymKind::Var,
        };

        let uri = fs_path_to_uri(file_path).map_err(|e| e.to_string())?;
        let store = self._index.document_store();
        let doc = store.get(&uri).ok_or("document not indexed")?;
        let mut idx = doc.line_index.clone();

        // Naively find definition line
        let def_line_idx = doc
            .text
            .lines()
            .position(|l| l.contains(&format!("{} =", var_name)))
            .ok_or("definition not found")?;
        let def_line_start = idx.position_to_offset(def_line_idx as u32, 0).ok_or("start")?;
        let def_line_end =
            idx.position_to_offset(def_line_idx as u32 + 1, 0).unwrap_or_else(|| doc.text.len());
        let def_line = doc.text.lines().nth(def_line_idx).unwrap_or("");
        let expr = def_line
            .split('=')
            .nth(1)
            .map(|s| s.trim().trim_end_matches(';'))
            .ok_or("no initializer")?
            .to_string();

        let mut edits_map: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();

        // Remove definition line
        edits_map.entry(file_path.to_path_buf()).or_default().push(TextEdit {
            start: def_line_start,
            end: def_line_end,
            new_text: String::new(),
        });

        // Replace remaining occurrences
        let mut search_pos = def_line_end;
        while let Some(found) = doc.text[search_pos..].find(var_name) {
            let start = search_pos + found;
            let end = start + var_name.len();
            edits_map.entry(file_path.to_path_buf()).or_default().push(TextEdit {
                start,
                end,
                new_text: expr.clone(),
            });
            search_pos = end;
        }

        let file_edits =
            edits_map.into_iter().map(|(file_path, edits)| FileEdit { file_path, edits }).collect();

        Ok(RefactorResult {
            file_edits,
            description: format!("Inline variable '{}' in {}", var_name, file_path.display()),
            warnings: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_index(files: Vec<(&str, &str)>) -> (WorkspaceIndex, Vec<PathBuf>) {
        let dir = tempdir().unwrap();
        let mut paths = Vec::new();
        let index = WorkspaceIndex::new();
        for (name, content) in files {
            let path = dir.path().join(name);
            std::fs::write(&path, content).unwrap();
            index.index_file_str(path.to_str().unwrap(), content).unwrap();
            paths.push(path);
        }
        (index, paths)
    }

    #[test]
    fn test_rename_symbol() {
        let (index, paths) =
            setup_index(vec![("a.pl", "my $foo = 1; print $foo;"), ("b.pl", "print $foo;")]);
        let refactor = WorkspaceRefactor::new(index);
        let result = refactor.rename_symbol("$foo", "bar", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());
    }

    #[test]
    fn test_extract_module() {
        let (index, paths) = setup_index(vec![("a.pl", "my $x = 1;\nprint $x;\n")]);
        let refactor = WorkspaceRefactor::new(index);
        let res = refactor.extract_module(&paths[0], 2, 2, "Extracted").unwrap();
        assert_eq!(res.file_edits.len(), 2);
    }

    #[test]
    fn test_optimize_imports() {
        let (index, _paths) = setup_index(vec![
            ("a.pl", "use B;\nuse A;\nuse B;\n"),
            ("b.pl", "use C;\nuse A;\nuse C;\n"),
        ]);
        let refactor = WorkspaceRefactor::new(index);
        let res = refactor.optimize_imports().unwrap();
        assert_eq!(res.file_edits.len(), 2);
    }

    #[test]
    fn test_move_subroutine() {
        let (index, paths) = setup_index(vec![("a.pl", "sub foo {1}\n"), ("b.pm", "")]);
        let refactor = WorkspaceRefactor::new(index);
        let res = refactor.move_subroutine("foo", &paths[0], "b").unwrap();
        assert_eq!(res.file_edits.len(), 2);
    }

    #[test]
    fn test_inline_variable() {
        let (index, paths) =
            setup_index(vec![("a.pl", "my $x = 42;\nmy $y = $x + 1;\nprint $y;\n")]);
        let refactor = WorkspaceRefactor::new(index);
        let res = refactor.inline_variable("$x", &paths[0], (0, 0)).unwrap();
        assert!(!res.file_edits.is_empty());
    }
}

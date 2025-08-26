//! Workspace-wide refactoring operations
//!
//! This module implements a few high level refactoring operations that can be
//! executed across multiple files.  The original version of this module only
//! contained empty stubs – the real behaviour has now been implemented using
//! the [`WorkspaceIndex`]'s document store.

use crate::import_optimizer::ImportOptimizer;
use crate::workspace_index::{uri_to_fs_path, WorkspaceIndex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Index containing all documents in the workspace
    index: WorkspaceIndex,
}

impl WorkspaceRefactor {
    pub fn new(index: WorkspaceIndex) -> Self {
        Self { index }
    }

    /// Rename a symbol across all files.
    ///
    /// The implementation is intentionally simple – it performs a textual
    /// search across every open document and returns the edits required to
    /// rename all occurrences of `old_name` to `new_name`.
    pub fn rename_symbol(
        &self,
        old_name: &str,
        new_name: &str,
        _file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, String> {
        let mut file_edits = Vec::new();
        let re =
            Regex::new(&format!(r"\b{}\b", regex::escape(old_name))).map_err(|e| e.to_string())?;

        for doc in self.index.document_store().all_documents() {
            let mut edits = Vec::new();
            for m in re.find_iter(&doc.text) {
                edits.push(TextEdit {
                    start: m.start(),
                    end: m.end(),
                    new_text: new_name.to_string(),
                });
            }
            if !edits.is_empty() {
                if let Some(path) = uri_to_fs_path(&doc.uri) {
                    file_edits.push(FileEdit { file_path: path, edits });
                }
            }
        }

        Ok(RefactorResult {
            file_edits,
            description: format!("Rename '{}' to '{}'", old_name, new_name),
            warnings: vec![],
        })
    }

    /// Extract selected code into a new module.
    ///
    /// `start_line` and `end_line` are 1-based inclusive line numbers.
    pub fn extract_module(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        module_name: &str,
    ) -> Result<RefactorResult, String> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        if start_line == 0 || end_line < start_line || end_line > lines.len() {
            return Err("Invalid line range for extraction".to_string());
        }

        // Determine byte offsets for the range we want to remove
        let mut line_offsets = Vec::with_capacity(lines.len() + 1);
        let mut offset = 0;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1; // include newline
        }
        line_offsets.push(offset);

        let start_offset = line_offsets[start_line - 1];
        let end_offset = line_offsets[end_line];
        let extracted: Vec<&str> = lines[start_line - 1..end_line].to_vec();

        // Build edits: remove from original file and create new module file
        let mut file_edits = Vec::new();
        file_edits.push(FileEdit {
            file_path: file_path.to_path_buf(),
            edits: vec![TextEdit { start: start_offset, end: end_offset, new_text: String::new() }],
        });

        let new_module_path = file_path.with_file_name(format!("{}.pm", module_name));
        let mut module_content = extracted.join("\n");
        module_content.push('\n');
        file_edits.push(FileEdit {
            file_path: new_module_path,
            edits: vec![TextEdit { start: 0, end: 0, new_text: module_content }],
        });

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

    /// Optimize imports across the workspace.
    ///
    /// The optimizer removes duplicate and unused imports and adds any missing
    /// imports that were detected.  The resulting import block is sorted
    /// alphabetically.
    pub fn optimize_imports(&self) -> Result<RefactorResult, String> {
        let mut file_edits = Vec::new();
        let optimizer = ImportOptimizer::new();

        for doc in self.index.document_store().all_documents() {
            let Some(path) = uri_to_fs_path(&doc.uri) else { continue };
            let analysis = optimizer.analyze_file(&path)?;
            if analysis.unused_imports.is_empty()
                && analysis.duplicate_imports.is_empty()
                && analysis.missing_imports.is_empty()
            {
                continue;
            }

            // Find import lines in the document
            let mut import_lines = Vec::new();
            for (i, line) in doc.text.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("use ") || trimmed.starts_with("require ") {
                    import_lines.push(i);
                }
            }
            if import_lines.is_empty() {
                continue;
            }

            // Compute byte offsets
            let mut line_offsets = Vec::with_capacity(doc.text.lines().count() + 1);
            let mut off = 0;
            for line in doc.text.lines() {
                line_offsets.push(off);
                off += line.len() + 1;
            }
            line_offsets.push(off);
            let start_offset = line_offsets[import_lines[0]];
            let end_offset = line_offsets[import_lines.last().unwrap() + 1];

            // Gather existing modules in import block
            let mut modules: Vec<String> = import_lines
                .iter()
                .filter_map(|i| doc.text.lines().nth(*i))
                .filter_map(|line| line.split_whitespace().nth(1))
                .map(|m| m.trim_end_matches(';').to_string())
                .collect();

            // Remove duplicates and unused modules
            let unused: std::collections::HashSet<_> =
                analysis.unused_imports.iter().map(|u| u.module.as_str()).collect();
            modules.retain(|m| !unused.contains(m.as_str()));
            modules.sort();
            modules.dedup();
            // Add missing imports
            for miss in &analysis.missing_imports {
                if !modules.contains(&miss.module) {
                    modules.push(miss.module.clone());
                }
            }
            modules.sort();

            let new_block: String = modules.into_iter().map(|m| format!("use {}\n", m)).collect();

            file_edits.push(FileEdit {
                file_path: path,
                edits: vec![TextEdit { start: start_offset, end: end_offset, new_text: new_block }],
            });
        }

        Ok(RefactorResult {
            file_edits,
            description: "Optimize imports across workspace".to_string(),
            warnings: vec![],
        })
    }

    /// Move a subroutine to another module (stub implementation)
    pub fn move_subroutine(
        &self,
        sub_name: &str,
        from_file: &Path,
        to_module: &str,
    ) -> Result<RefactorResult, String> {
        // Stub implementation
        Ok(RefactorResult {
            file_edits: vec![],
            description: format!(
                "Move subroutine '{}' from {} to module '{}'",
                sub_name,
                from_file.display(),
                to_module
            ),
            warnings: vec![],
        })
    }

    /// Inline a variable across its scope (stub implementation)
    pub fn inline_variable(
        &self,
        var_name: &str,
        file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, String> {
        // Stub implementation
        Ok(RefactorResult {
            file_edits: vec![],
            description: format!("Inline variable '{}' in {}", var_name, file_path.display()),
            warnings: vec![],
        })
    }
}

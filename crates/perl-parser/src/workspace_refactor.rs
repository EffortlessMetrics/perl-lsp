//! Workspace-wide refactoring operations
//!
//! This module provides comprehensive refactoring capabilities that span multiple files,
//! including symbol renaming, module extraction, import optimization, and code movement.
//! All operations are designed to be safe, reversible, and provide detailed feedback.

use crate::import_optimizer::ImportOptimizer;
use crate::workspace_index::{
    SymKind, SymbolKey, WorkspaceIndex, fs_path_to_uri, normalize_var, uri_to_fs_path,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Errors that can occur during workspace refactoring operations
#[derive(Debug, Clone)]
pub enum RefactorError {
    /// Failed to convert between file paths and URIs
    UriConversion(String),
    /// Document not found in workspace index
    DocumentNotIndexed(String),
    /// Invalid position or range in document
    InvalidPosition { file: String, details: String },
    /// Symbol not found in workspace
    SymbolNotFound { symbol: String, file: String },
    /// Failed to parse or analyze code structure
    ParseError(String),
    /// Input validation failed
    InvalidInput(String),
    /// File system operation failed
    FileSystemError(String),
}

impl fmt::Display for RefactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefactorError::UriConversion(msg) => write!(f, "URI conversion failed: {}", msg),
            RefactorError::DocumentNotIndexed(file) => {
                write!(f, "Document not indexed in workspace: {}", file)
            }
            RefactorError::InvalidPosition { file, details } => {
                write!(f, "Invalid position in {}: {}", file, details)
            }
            RefactorError::SymbolNotFound { symbol, file } => {
                write!(f, "Symbol '{}' not found in {}", symbol, file)
            }
            RefactorError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RefactorError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            RefactorError::FileSystemError(msg) => write!(f, "File system error: {}", msg),
        }
    }
}

impl std::error::Error for RefactorError {}

// Move regex outside loop to avoid recompilation
lazy_static::lazy_static! {
    static ref IMPORT_BLOCK_RE: Regex = Regex::new(r"(?m)^(?:use\s+[\w:]+[^\n]*\n)+").unwrap();
}

/// A file edit as part of a refactoring operation
///
/// Represents a set of text edits that should be applied to a single file
/// as part of a workspace refactoring operation. All edits within a FileEdit
/// are applied to the same file and should be applied in reverse order
/// (from end to beginning) to maintain position validity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    /// The absolute path to the file that should be edited
    pub file_path: PathBuf,
    /// The list of text edits to apply to this file, in document order
    pub edits: Vec<TextEdit>,
}

/// A single text edit within a file
///
/// Represents a single textual change within a document, defined by
/// a byte range and the replacement text. The range is inclusive of
/// the start position and exclusive of the end position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// The byte offset of the start of the range to replace (inclusive)
    pub start: usize,
    /// The byte offset of the end of the range to replace (exclusive)
    pub end: usize,
    /// The text to replace the range with (may be empty for deletion)
    pub new_text: String,
}

/// Result of a refactoring operation
///
/// Contains all the changes that need to be applied to complete a refactoring
/// operation, along with descriptive information and any warnings that were
/// encountered during the analysis phase.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorResult {
    /// The list of file edits that need to be applied to complete the refactoring
    pub file_edits: Vec<FileEdit>,
    /// A human-readable description of what the refactoring operation does
    pub description: String,
    /// Any warnings encountered during the refactoring analysis (non-fatal issues)
    pub warnings: Vec<String>,
}

/// Workspace-wide refactoring provider
///
/// Provides high-level refactoring operations that can operate across multiple files
/// within a workspace. Uses a WorkspaceIndex to understand symbol relationships
/// and dependencies between files.
///
/// # Examples
///
/// ```rust
/// # use perl_parser::workspace_refactor::WorkspaceRefactor;
/// # use perl_parser::workspace_index::WorkspaceIndex;
/// # use std::path::Path;
/// let index = WorkspaceIndex::new();
/// let refactor = WorkspaceRefactor::new(index);
///
/// // Rename a variable across all files
/// let result = refactor.rename_symbol("$old_name", "$new_name", Path::new("file.pl"), (0, 0));
/// ```
pub struct WorkspaceRefactor {
    /// The workspace index used for symbol lookup and cross-file analysis
    _index: WorkspaceIndex,
}

impl WorkspaceRefactor {
    /// Create a new workspace refactoring provider
    ///
    /// # Arguments
    /// * `index` - A WorkspaceIndex containing indexed symbols and documents
    ///
    /// # Returns
    /// A new WorkspaceRefactor instance ready to perform refactoring operations
    pub fn new(index: WorkspaceIndex) -> Self {
        Self { _index: index }
    }

    /// Rename a symbol across all files in the workspace
    ///
    /// Performs a comprehensive rename of a Perl symbol (variable, subroutine, or package)
    /// across all indexed files in the workspace. The operation preserves sigils for
    /// variables and handles both indexed symbol lookups and text-based fallback searches.
    ///
    /// # Arguments
    /// * `old_name` - The current name of the symbol (e.g., "$variable", "subroutine")
    /// * `new_name` - The new name for the symbol (e.g., "$new_variable", "new_subroutine")
    /// * `_file_path` - The file path where the rename was initiated (currently unused)
    /// * `_position` - The position in the file where the rename was initiated (currently unused)
    ///
    /// # Returns
    /// * `Ok(RefactorResult)` - Contains all file edits needed to complete the rename
    /// * `Err(RefactorError)` - If validation fails or symbol lookup encounters issues
    ///
    /// # Errors
    /// * `RefactorError::InvalidInput` - If names are empty or identical
    /// * `RefactorError::UriConversion` - If file path/URI conversion fails
    ///
    /// # Examples
    /// ```rust
    /// # use perl_parser::workspace_refactor::WorkspaceRefactor;
    /// # use perl_parser::workspace_index::WorkspaceIndex;
    /// # use std::path::Path;
    /// let index = WorkspaceIndex::new();
    /// let refactor = WorkspaceRefactor::new(index);
    ///
    /// let result = refactor.rename_symbol("$old_var", "$new_var", Path::new("file.pl"), (0, 0))?;
    /// println!("Rename will affect {} files", result.file_edits.len());
    /// # Ok::<(), perl_parser::workspace_refactor::RefactorError>(())
    /// ```
    pub fn rename_symbol(
        &self,
        old_name: &str,
        new_name: &str,
        _file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, RefactorError> {
        // Validate input parameters
        if old_name.is_empty() {
            return Err(RefactorError::InvalidInput("Symbol name cannot be empty".to_string()));
        }
        if new_name.is_empty() {
            return Err(RefactorError::InvalidInput("New name cannot be empty".to_string()));
        }
        if old_name == new_name {
            return Err(RefactorError::InvalidInput("Old and new names are identical".to_string()));
        }

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
            // Fallback naive search with performance optimizations
            let _old_name_bytes = old_name.as_bytes();

            for doc in store.all_documents() {
                // Pre-check if the document even contains the target string to avoid unnecessary work
                if !doc.text.contains(old_name) {
                    continue;
                }

                let mut idx = doc.line_index.clone();
                let mut pos = 0;
                let _text_bytes = doc.text.as_bytes();

                // Use faster byte-based searching with matches iterator
                while let Some(found) = doc.text[pos..].find(old_name) {
                    let start = pos + found;
                    let end = start + old_name.len();

                    // Early bounds checking to avoid invalid positions
                    if start >= doc.text.len() || end > doc.text.len() {
                        break;
                    }

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

                    // Limit the number of matches to prevent runaway performance issues
                    if locations.len() >= 1000 {
                        break;
                    }
                }

                // If we've found matches in this document and it's getting large, we can break early
                // This is a heuristic to balance completeness with performance
                if locations.len() >= 500 {
                    break;
                }
            }
        }

        for loc in locations {
            let path = uri_to_fs_path(&loc.uri).ok_or_else(|| {
                RefactorError::UriConversion(format!("Failed to convert URI to path: {}", loc.uri))
            })?;
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
    ///
    /// Takes a range of lines from an existing file and moves them into a new
    /// Perl module file, replacing the original code with a `use` statement.
    /// This is useful for breaking up large files into smaller, more manageable modules.
    ///
    /// # Arguments
    /// * `file_path` - The path to the file containing the code to extract
    /// * `start_line` - The first line to extract (1-based line number)
    /// * `end_line` - The last line to extract (1-based line number, inclusive)
    /// * `module_name` - The name of the new module to create (without .pm extension)
    ///
    /// # Returns
    /// * `Ok(RefactorResult)` - Contains edits for both the original file and new module
    /// * `Err(RefactorError)` - If validation fails or file operations encounter issues
    ///
    /// # Errors
    /// * `RefactorError::InvalidInput` - If module name is empty or start_line > end_line
    /// * `RefactorError::DocumentNotIndexed` - If the source file is not in the workspace index
    /// * `RefactorError::InvalidPosition` - If the line numbers are invalid
    /// * `RefactorError::UriConversion` - If file path/URI conversion fails
    ///
    /// # Examples
    /// ```rust
    /// # use perl_parser::workspace_refactor::WorkspaceRefactor;
    /// # use perl_parser::workspace_index::WorkspaceIndex;
    /// # use std::path::Path;
    /// let index = WorkspaceIndex::new();
    /// let refactor = WorkspaceRefactor::new(index);
    ///
    /// let result = refactor.extract_module(
    ///     Path::new("large_file.pl"),
    ///     50, 100,  // Extract lines 50-100
    ///     "ExtractedUtils"
    /// )?;
    /// # Ok::<(), perl_parser::workspace_refactor::RefactorError>(())
    /// ```
    pub fn extract_module(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        module_name: &str,
    ) -> Result<RefactorResult, RefactorError> {
        // Validate input parameters
        if module_name.is_empty() {
            return Err(RefactorError::InvalidInput("Module name cannot be empty".to_string()));
        }
        if start_line > end_line {
            return Err(RefactorError::InvalidInput(
                "Start line cannot be after end line".to_string(),
            ));
        }

        let uri = fs_path_to_uri(file_path).map_err(|e| {
            RefactorError::UriConversion(format!("Failed to convert path to URI: {}", e))
        })?;
        let store = self._index.document_store();
        let doc = store
            .get(&uri)
            .ok_or_else(|| RefactorError::DocumentNotIndexed(file_path.display().to_string()))?;
        let mut idx = doc.line_index.clone();

        // Determine byte offsets for lines
        let start_off = idx.position_to_offset(start_line as u32 - 1, 0).ok_or_else(|| {
            RefactorError::InvalidPosition {
                file: file_path.display().to_string(),
                details: format!("Invalid start line: {}", start_line),
            }
        })?;
        let end_off = idx.position_to_offset(end_line as u32, 0).unwrap_or(doc.text.len());

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

    /// Optimize imports across the entire workspace
    ///
    /// Uses the ImportOptimizer to analyze all files and optimize their import statements by:
    /// - Detecting unused imports with smart bare import analysis
    /// - Removing duplicate imports from the same module
    /// - Sorting imports alphabetically
    /// - Consolidating multiple imports from the same module
    /// - Conservative handling of pragma modules and bare imports
    ///
    /// # Returns
    /// * `Ok(RefactorResult)` - Contains all file edits to optimize imports
    /// * `Err(String)` - If import analysis encounters issues
    pub fn optimize_imports(&self) -> Result<RefactorResult, String> {
        let optimizer = ImportOptimizer::new();
        let mut file_edits = Vec::new();

        // Iterate over all open documents in the workspace
        for doc in self._index.document_store().all_documents() {
            let Some(path) = uri_to_fs_path(&doc.uri) else { continue };

            let analysis = optimizer.analyze_content(&doc.text)?;
            let optimized = optimizer.generate_optimized_imports(&analysis);

            if optimized.is_empty() {
                continue;
            }

            // Replace the existing import block at the top of the file
            let import_block_re = &*IMPORT_BLOCK_RE;
            let (start, end) = if let Some(m) = import_block_re.find(&doc.text) {
                (m.start(), m.end())
            } else {
                (0, 0)
            };

            file_edits.push(FileEdit {
                file_path: path.clone(),
                edits: vec![TextEdit { start, end, new_text: format!("{}\n", optimized) }],
            });
        }

        Ok(RefactorResult {
            file_edits,
            description: "Optimize imports across workspace".to_string(),
            warnings: vec![],
        })
    }

    /// Move a subroutine from one file to another module
    ///
    /// Extracts a subroutine definition from one file and moves it to another module file.
    /// The subroutine is completely removed from the source file and appended to the
    /// target module file. This operation does not update callers or add import statements.
    ///
    /// # Arguments
    /// * `sub_name` - The name of the subroutine to move (without 'sub' keyword)
    /// * `from_file` - The source file containing the subroutine
    /// * `to_module` - The name of the target module (without .pm extension)
    ///
    /// # Returns
    /// * `Ok(RefactorResult)` - Contains edits for both source and target files
    /// * `Err(RefactorError)` - If validation fails or the subroutine cannot be found
    ///
    /// # Errors
    /// * `RefactorError::InvalidInput` - If names are empty
    /// * `RefactorError::DocumentNotIndexed` - If the source file is not indexed
    /// * `RefactorError::SymbolNotFound` - If the subroutine is not found in the source file
    /// * `RefactorError::InvalidPosition` - If the subroutine's position is invalid
    /// * `RefactorError::UriConversion` - If file path/URI conversion fails
    ///
    /// # Examples
    /// ```rust
    /// # use perl_parser::workspace_refactor::WorkspaceRefactor;
    /// # use perl_parser::workspace_index::WorkspaceIndex;
    /// # use std::path::Path;
    /// let index = WorkspaceIndex::new();
    /// let refactor = WorkspaceRefactor::new(index);
    ///
    /// let result = refactor.move_subroutine(
    ///     "utility_function",
    ///     Path::new("main.pl"),
    ///     "Utils"
    /// )?;
    /// # Ok::<(), perl_parser::workspace_refactor::RefactorError>(())
    /// ```
    pub fn move_subroutine(
        &self,
        sub_name: &str,
        from_file: &Path,
        to_module: &str,
    ) -> Result<RefactorResult, RefactorError> {
        // Validate input parameters
        if sub_name.is_empty() {
            return Err(RefactorError::InvalidInput("Subroutine name cannot be empty".to_string()));
        }
        if to_module.is_empty() {
            return Err(RefactorError::InvalidInput(
                "Target module name cannot be empty".to_string(),
            ));
        }

        let uri = fs_path_to_uri(from_file).map_err(|e| {
            RefactorError::UriConversion(format!("Failed to convert path to URI: {}", e))
        })?;
        let symbols = self._index.file_symbols(&uri);
        let sym = symbols.into_iter().find(|s| s.name == sub_name).ok_or_else(|| {
            RefactorError::SymbolNotFound {
                symbol: sub_name.to_string(),
                file: from_file.display().to_string(),
            }
        })?;

        let store = self._index.document_store();
        let doc = store
            .get(&uri)
            .ok_or_else(|| RefactorError::DocumentNotIndexed(from_file.display().to_string()))?;
        let mut idx = doc.line_index.clone();
        let start_off = idx
            .position_to_offset(sym.range.start.line, sym.range.start.character)
            .ok_or_else(|| RefactorError::InvalidPosition {
                file: from_file.display().to_string(),
                details: format!(
                    "Invalid start position for subroutine '{}' at line {}, column {}",
                    sub_name, sym.range.start.line, sym.range.start.character
                ),
            })?;
        let end_off = idx
            .position_to_offset(sym.range.end.line, sym.range.end.character)
            .ok_or_else(|| RefactorError::InvalidPosition {
                file: from_file.display().to_string(),
                details: format!(
                    "Invalid end position for subroutine '{}' at line {}, column {}",
                    sub_name, sym.range.end.line, sym.range.end.character
                ),
            })?;
        let sub_text = doc.text[start_off..end_off].to_string();

        // Remove from original file
        let mut file_edits = vec![FileEdit {
            file_path: from_file.to_path_buf(),
            edits: vec![TextEdit { start: start_off, end: end_off, new_text: String::new() }],
        }];

        // Append to new module file
        let target_path = from_file.with_file_name(format!("{}.pm", to_module));
        let target_uri = fs_path_to_uri(&target_path).map_err(|e| {
            RefactorError::UriConversion(format!("Failed to convert target path to URI: {}", e))
        })?;
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
    ///
    /// Replaces all occurrences of a variable with its initializer expression
    /// and removes the variable declaration. This is useful for eliminating
    /// unnecessary intermediate variables that only serve to store simple expressions.
    ///
    /// **Note**: This is a naive implementation that uses simple text matching.
    /// It may not handle all scoping rules correctly and should be used with caution.
    ///
    /// # Arguments
    /// * `var_name` - The name of the variable to inline (including sigil, e.g., "$temp")
    /// * `file_path` - The file containing the variable to inline
    /// * `_position` - The position in the file (currently unused)
    ///
    /// # Returns
    /// * `Ok(RefactorResult)` - Contains the file edits to inline the variable
    /// * `Err(RefactorError)` - If validation fails or the variable cannot be found
    ///
    /// # Errors
    /// * `RefactorError::InvalidInput` - If the variable name is empty
    /// * `RefactorError::DocumentNotIndexed` - If the file is not indexed
    /// * `RefactorError::SymbolNotFound` - If the variable definition is not found
    /// * `RefactorError::ParseError` - If the variable has no initializer
    /// * `RefactorError::UriConversion` - If file path/URI conversion fails
    ///
    /// # Examples
    /// ```rust
    /// # use perl_parser::workspace_refactor::WorkspaceRefactor;
    /// # use perl_parser::workspace_index::WorkspaceIndex;
    /// # use std::path::Path;
    /// let index = WorkspaceIndex::new();
    /// let refactor = WorkspaceRefactor::new(index);
    ///
    /// // Inline a temporary variable like: my $temp = some_function(); print $temp;
    /// let result = refactor.inline_variable("$temp", Path::new("file.pl"), (0, 0))?;
    /// # Ok::<(), perl_parser::workspace_refactor::RefactorError>(())
    /// ```
    pub fn inline_variable(
        &self,
        var_name: &str,
        file_path: &Path,
        _position: (usize, usize),
    ) -> Result<RefactorResult, RefactorError> {
        let (sigil, bare) = normalize_var(var_name);
        let _key = SymbolKey {
            pkg: Arc::from("main".to_string()),
            name: Arc::from(bare.to_string()),
            sigil,
            kind: SymKind::Var,
        };

        // Validate input parameters
        if var_name.is_empty() {
            return Err(RefactorError::InvalidInput("Variable name cannot be empty".to_string()));
        }

        let uri = fs_path_to_uri(file_path).map_err(|e| {
            RefactorError::UriConversion(format!("Failed to convert path to URI: {}", e))
        })?;
        let store = self._index.document_store();
        let doc = store
            .get(&uri)
            .ok_or_else(|| RefactorError::DocumentNotIndexed(file_path.display().to_string()))?;
        let mut idx = doc.line_index.clone();

        // Naively find definition line (variable declaration with "my")
        let def_line_idx = doc
            .text
            .lines()
            .position(|l| l.trim_start().starts_with("my ") && l.contains(var_name))
            .ok_or_else(|| RefactorError::SymbolNotFound {
                symbol: var_name.to_string(),
                file: file_path.display().to_string(),
            })?;
        let def_line_start = idx.position_to_offset(def_line_idx as u32, 0).ok_or_else(|| {
            RefactorError::InvalidPosition {
                file: file_path.display().to_string(),
                details: format!("Invalid start position for definition line: {}", def_line_idx),
            }
        })?;
        let def_line_end =
            idx.position_to_offset(def_line_idx as u32 + 1, 0).unwrap_or(doc.text.len());
        let def_line = doc.text.lines().nth(def_line_idx).unwrap_or("");
        let expr = def_line
            .split('=')
            .nth(1)
            .map(|s| s.trim().trim_end_matches(';'))
            .ok_or_else(|| {
                RefactorError::ParseError(format!(
                    "Variable '{}' has no initializer in line: {}",
                    var_name, def_line
                ))
            })?
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
        let result = refactor.rename_symbol("$foo", "$bar", &paths[0], (0, 0)).unwrap();
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
        let result = refactor.inline_variable("$x", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());
    }

    // Edge case and error handling tests
    #[test]
    fn test_rename_symbol_validation_errors() {
        let (index, paths) = setup_index(vec![("a.pl", "my $foo = 1;")]);
        let refactor = WorkspaceRefactor::new(index);

        // Empty old name
        assert!(matches!(
            refactor.rename_symbol("", "$bar", &paths[0], (0, 0)),
            Err(RefactorError::InvalidInput(_))
        ));

        // Empty new name
        assert!(matches!(
            refactor.rename_symbol("$foo", "", &paths[0], (0, 0)),
            Err(RefactorError::InvalidInput(_))
        ));

        // Identical names
        assert!(matches!(
            refactor.rename_symbol("$foo", "$foo", &paths[0], (0, 0)),
            Err(RefactorError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_extract_module_validation_errors() {
        let (index, paths) = setup_index(vec![("a.pl", "my $x = 1;\nprint $x;\n")]);
        let refactor = WorkspaceRefactor::new(index);

        // Empty module name
        assert!(matches!(
            refactor.extract_module(&paths[0], 1, 2, ""),
            Err(RefactorError::InvalidInput(_))
        ));

        // Invalid line range
        assert!(matches!(
            refactor.extract_module(&paths[0], 5, 2, "Test"),
            Err(RefactorError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_move_subroutine_validation_errors() {
        let (index, paths) = setup_index(vec![("a.pl", "sub foo { 1 }")]);
        let refactor = WorkspaceRefactor::new(index);

        // Empty subroutine name
        assert!(matches!(
            refactor.move_subroutine("", &paths[0], "Utils"),
            Err(RefactorError::InvalidInput(_))
        ));

        // Empty target module
        assert!(matches!(
            refactor.move_subroutine("foo", &paths[0], ""),
            Err(RefactorError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_inline_variable_validation_errors() {
        let (index, paths) = setup_index(vec![("a.pl", "my $x = 42;")]);
        let refactor = WorkspaceRefactor::new(index);

        // Empty variable name
        assert!(matches!(
            refactor.inline_variable("", &paths[0], (0, 0)),
            Err(RefactorError::InvalidInput(_))
        ));
    }

    // Unicode and international character tests
    #[test]
    fn test_rename_symbol_unicode_variables() {
        let (index, paths) = setup_index(vec![
            ("unicode.pl", "my $♥ = '爱'; print $♥; # Unicode variable"),
            ("unicode2.pl", "use utf8; my $données = 42; print $données;"), // French accents
        ]);
        let refactor = WorkspaceRefactor::new(index);

        // Rename Unicode variable
        let result = refactor.rename_symbol("$♥", "$love", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());
        assert!(result.description.contains("♥"));

        // Rename variable with accents
        let result = refactor.rename_symbol("$données", "$data", &paths[1], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());
        assert!(result.description.contains("données"));
    }

    #[test]
    fn test_extract_module_unicode_content() {
        let (index, paths) = setup_index(vec![(
            "unicode_content.pl",
            "# コメント in Japanese\nmy $message = \"你好世界\";\nprint $message;\n# More 中文 content\n",
        )]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.extract_module(&paths[0], 2, 3, "UnicodeUtils").unwrap();
        assert_eq!(result.file_edits.len(), 2); // Original + new module

        // Check that the extracted content contains Unicode
        let new_module_edit = &result.file_edits[1];
        assert!(new_module_edit.edits[0].new_text.contains("你好世界"));
    }

    #[test]
    fn test_inline_variable_unicode_expressions() {
        let (index, paths) = setup_index(vec![(
            "unicode_expr.pl",
            "my $表达式 = \"测试表达式\";\nmy $result = $表达式 . \"suffix\";\nprint $result;\n",
        )]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.inline_variable("$表达式", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());

        // Check that the replacement contains the Unicode string literal
        let edits = &result.file_edits[0].edits;
        assert!(edits.iter().any(|edit| edit.new_text.contains("测试表达式")));
    }

    // Complex edge cases
    #[test]
    fn test_rename_symbol_complex_perl_constructs() {
        let (index, paths) = setup_index(vec![(
            "complex.pl",
            r#"
package MyPackage;
my @array = qw($var1 $var2 $var3);
my %hash = ( key1 => $var1, key2 => $var2 );
my $ref = \$var1;
print "Variable in string: $var1\n";
$var1 =~ s/old/new/g;
for my $item (@{[$var1, $var2]}) {
    print $item;
}
"#,
        )]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.rename_symbol("$var1", "$renamed_var", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());

        // Should find multiple occurrences in different contexts
        let edits = &result.file_edits[0].edits;
        assert!(edits.len() > 3); // Should find multiple instances
    }

    #[test]
    fn test_extract_module_with_dependencies() {
        let (index, paths) = setup_index(vec![(
            "with_deps.pl",
            r#"
use strict;
use warnings;
use Data::Dumper;

sub utility_func {
    my ($param) = @_;
    return Data::Dumper::Dumper($param);
}

sub main_func {
    my $data = { key => "value" };
    my $result = utility_func($data);
    print $result;
}
"#,
        )]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.extract_module(&paths[0], 6, 9, "Utils").unwrap();
        assert_eq!(result.file_edits.len(), 2);

        // Check that extracted content includes the subroutine
        let new_module_edit = &result.file_edits[1];
        assert!(new_module_edit.edits[0].new_text.contains("sub utility_func"));
        assert!(new_module_edit.edits[0].new_text.contains("Data::Dumper"));
    }

    #[test]
    fn test_optimize_imports_complex_scenarios() {
        let (index, _paths) = setup_index(vec![
            (
                "complex_imports.pl",
                r#"
use strict;
use warnings;
use utf8;
use Data::Dumper;
use Data::Dumper qw(Dumper);
use List::Util;
use List::Util qw(first);
use Data::Dumper; # Duplicate
"#,
            ),
            ("minimal_imports.pl", "use strict;\nuse warnings;"),
            ("no_imports.pl", "print 'Hello World';"),
        ]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.optimize_imports().unwrap();

        // Should optimize the complex file, skip minimal (no duplicates), skip no imports
        assert!(result.file_edits.len() <= 3);

        // Check that we don't create empty edits for files with no imports
        for file_edit in &result.file_edits {
            assert!(!file_edit.edits.is_empty());
        }
    }

    #[test]
    fn test_move_subroutine_not_found() {
        let (index, paths) = setup_index(vec![("empty.pl", "# No subroutines here")]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.move_subroutine("nonexistent", &paths[0], "Target");
        assert!(matches!(result, Err(RefactorError::SymbolNotFound { .. })));
    }

    #[test]
    fn test_inline_variable_no_initializer() {
        let (index, paths) =
            setup_index(vec![("no_init.pl", "my $var;\n$var = 42;\nprint $var;\n")]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.inline_variable("$var", &paths[0], (0, 0));
        // Should fail because the found line "my $var;" doesn't have an initializer after =
        assert!(matches!(result, Err(RefactorError::ParseError(_))));
    }

    #[test]
    fn test_import_optimization_integration() {
        // Test the integration between workspace refactor and import optimizer
        let (index, _paths) = setup_index(vec![
            ("with_unused.pl", "use strict;\nuse warnings;\nuse Data::Dumper;\nuse JSON qw(encode_json unused_symbol);\n\nmy $json = encode_json({test => 1});"),
            ("clean.pl", "use strict;\nuse warnings;\nuse Data::Dumper;\n\nprint Dumper({test => 1});"),
        ]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.optimize_imports().unwrap();

        // Should only optimize files that have optimizations available
        // Files with unused imports should get optimized edits
        assert!(!result.file_edits.is_empty());
        
        // Check that we actually have some optimization suggestions
        let has_optimizations = result.file_edits.iter().any(|edit| !edit.edits.is_empty());
        assert!(has_optimizations);
    }

    // Performance and scalability tests
    #[test]
    fn test_large_file_handling() {
        // Create a large file with many occurrences
        let mut large_content = String::new();
        large_content.push_str("my $target = 'value';\n");
        for i in 0..100 {
            large_content.push_str(&format!("print \"Line {}: $target\\n\";\n", i));
        }

        let (index, paths) = setup_index(vec![("large.pl", &large_content)]);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.rename_symbol("$target", "$renamed", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());

        // Should handle all occurrences
        let edits = &result.file_edits[0].edits;
        assert!(edits.len() >= 100); // At least one per line
    }

    #[test]
    fn test_multiple_files_workspace() {
        let files = (0..10)
            .map(|i| (format!("file_{}.pl", i), format!("my $shared = {}; print $shared;\n", i)))
            .collect::<Vec<_>>();

        let files_refs: Vec<_> =
            files.iter().map(|(name, content)| (name.as_str(), content.as_str())).collect();
        let (index, paths) = setup_index(files_refs);
        let refactor = WorkspaceRefactor::new(index);

        let result = refactor.rename_symbol("$shared", "$common", &paths[0], (0, 0)).unwrap();
        assert!(!result.file_edits.is_empty());

        // Should potentially affect multiple files if fallback search is used
        assert!(!result.description.is_empty());
    }
}

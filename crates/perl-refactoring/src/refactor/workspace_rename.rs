//! Workspace-wide rename refactoring for Perl symbols
//!
//! This module implements comprehensive symbol renaming across entire workspaces,
//! supporting variables, subroutines, and packages with full LSP integration.
//!
//! # LSP Workflow Integration
//!
//! Workspace rename operates across the complete LSP pipeline:
//! - **Parse**: Extract symbols from Perl source files
//! - **Index**: Utilize dual indexing for qualified and bare symbol lookup
//! - **Navigate**: Resolve cross-file references
//! - **Complete**: Validate new names and detect conflicts
//! - **Analyze**: Perform scope analysis and semantic validation
//!
//! # Features
//!
//! - **Cross-file rename**: Identify and rename symbols across entire workspace
//! - **Atomic operations**: All-or-nothing changes with automatic rollback
//! - **Scope-aware**: Respects Perl package namespaces and lexical scoping
//! - **Dual indexing**: Finds both qualified (`Package::sub`) and bare (`sub`) references
//! - **Progress reporting**: Real-time feedback during large operations
//! - **Backup support**: Optional backup creation for safety
//!
//! # Example
//!
//! ```rust,ignore
//! use perl_refactoring::workspace_rename::{WorkspaceRename, WorkspaceRenameConfig};
//! use perl_workspace::workspace_index::WorkspaceIndex;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let index = WorkspaceIndex::new();
//! let config = WorkspaceRenameConfig::default();
//! let rename_engine = WorkspaceRename::new(index, config);
//!
//! let result = rename_engine.rename_symbol(
//!     "old_function",
//!     "new_function",
//!     Path::new("lib/Utils.pm"),
//!     (5, 4), // Line 5, column 4
//! )?;
//!
//! println!("Renamed {} occurrences across {} files",
//!          result.statistics.total_changes,
//!          result.statistics.files_modified);
//! # Ok(())
//! # }
//! ```

use crate::refactor::refactoring::BackupInfo;
use crate::refactor::workspace_refactor::{FileEdit, TextEdit};
use crate::workspace_index::WorkspaceIndex;
use perl_parser_core::qualified_name::split_qualified_name;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Configuration for workspace-wide rename operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRenameConfig {
    /// Enable atomic transaction with rollback (default: true)
    pub atomic_mode: bool,

    /// Create backups before modification (default: true)
    pub create_backups: bool,

    /// Operation timeout in seconds (default: 60)
    pub operation_timeout: u64,

    /// Enable parallel file processing (default: true)
    pub parallel_processing: bool,

    /// Number of files per batch in parallel mode (default: 10)
    pub batch_size: usize,

    /// Maximum number of files to process (0 = unlimited) (default: 0)
    pub max_files: usize,

    /// Enable progress reporting (default: true)
    pub report_progress: bool,

    /// Validate syntax after each file edit (default: true)
    pub validate_syntax: bool,
}

impl Default for WorkspaceRenameConfig {
    fn default() -> Self {
        Self {
            atomic_mode: true,
            create_backups: true,
            operation_timeout: 60,
            parallel_processing: true,
            batch_size: 10,
            max_files: 0,
            report_progress: true,
            validate_syntax: true,
        }
    }
}

/// Result of a workspace rename operation
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceRenameResult {
    /// File edits to apply
    pub file_edits: Vec<FileEdit>,
    /// Backup information for rollback
    pub backup_info: Option<BackupInfo>,
    /// Human-readable description
    pub description: String,
    /// Non-fatal warnings
    pub warnings: Vec<String>,
    /// Operation statistics
    pub statistics: RenameStatistics,
}

/// Statistics for a rename operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameStatistics {
    /// Number of files modified
    pub files_modified: usize,
    /// Total number of changes made
    pub total_changes: usize,
    /// Operation duration in milliseconds
    pub elapsed_ms: u64,
}

/// Progress events during rename operation
#[derive(Debug, Clone)]
pub enum Progress {
    /// Workspace scan started
    Scanning {
        /// Total files to scan
        total: usize,
    },
    /// Processing a file
    Processing {
        /// Current file index
        current: usize,
        /// Total files
        total: usize,
        /// File being processed
        file: PathBuf,
    },
    /// Operation complete
    Complete {
        /// Files modified
        files_modified: usize,
        /// Total changes
        changes: usize,
    },
}

/// Errors specific to workspace rename operations
#[derive(Debug, Clone)]
pub enum WorkspaceRenameError {
    /// Symbol not found in workspace
    SymbolNotFound {
        /// Symbol name
        symbol: String,
        /// File path
        file: String,
    },

    /// Name conflict detected in scope
    NameConflict {
        /// New name that conflicts
        new_name: String,
        /// Locations of conflicts
        conflicts: Vec<ConflictLocation>,
    },

    /// Operation timed out
    Timeout {
        /// Elapsed seconds
        elapsed_seconds: u64,
        /// Files processed before timeout
        files_processed: usize,
        /// Total files
        total_files: usize,
    },

    /// File system operation failed
    FileSystemError {
        /// Operation name
        operation: String,
        /// File path
        file: PathBuf,
        /// Error message
        error: String,
    },

    /// Rollback failed (critical)
    RollbackFailed {
        /// Original error
        original_error: String,
        /// Rollback error
        rollback_error: String,
        /// Backup directory
        backup_dir: PathBuf,
    },

    /// Index update failed
    IndexUpdateFailed {
        /// Error message
        error: String,
        /// Affected files
        affected_files: Vec<PathBuf>,
    },

    /// Security violation
    SecurityError {
        /// Error message
        message: String,
        /// Offending path
        path: Option<PathBuf>,
    },

    /// Feature not yet implemented
    NotImplemented {
        /// Description of unimplemented feature
        feature: String,
    },
}

impl std::fmt::Display for WorkspaceRenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceRenameError::SymbolNotFound { symbol, file } => {
                write!(f, "Symbol '{}' not found in {}", symbol, file)
            }
            WorkspaceRenameError::NameConflict { new_name, conflicts } => {
                write!(f, "Name '{}' conflicts with {} existing symbols", new_name, conflicts.len())
            }
            WorkspaceRenameError::Timeout { elapsed_seconds, files_processed, total_files } => {
                write!(
                    f,
                    "Operation timed out after {}s ({}/{} files)",
                    elapsed_seconds, files_processed, total_files
                )
            }
            WorkspaceRenameError::FileSystemError { operation, file, error } => {
                write!(f, "File system error during {}: {} - {}", operation, file.display(), error)
            }
            WorkspaceRenameError::RollbackFailed { original_error, rollback_error, backup_dir } => {
                write!(
                    f,
                    "Rollback failed - original: {}, rollback: {}, backup: {}",
                    original_error,
                    rollback_error,
                    backup_dir.display()
                )
            }
            WorkspaceRenameError::IndexUpdateFailed { error, affected_files } => {
                write!(f, "Index update failed: {} ({} files)", error, affected_files.len())
            }
            WorkspaceRenameError::SecurityError { message, path } => {
                if let Some(p) = path {
                    write!(f, "Security error: {} ({})", message, p.display())
                } else {
                    write!(f, "Security error: {}", message)
                }
            }
            WorkspaceRenameError::NotImplemented { feature } => {
                write!(f, "Feature not yet implemented: {}", feature)
            }
        }
    }
}

impl std::error::Error for WorkspaceRenameError {}

/// Location of a name conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictLocation {
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Existing symbol name
    pub existing_symbol: String,
}

/// Workspace rename engine
///
/// Provides comprehensive symbol renaming across entire workspace with atomic
/// operations, backup support, and progress reporting.
pub struct WorkspaceRename {
    /// Workspace index for symbol lookup
    index: WorkspaceIndex,
    /// Configuration
    config: WorkspaceRenameConfig,
}

impl WorkspaceRename {
    /// Create a new workspace rename engine
    ///
    /// # Arguments
    /// * `index` - Workspace index for symbol lookup
    /// * `config` - Rename configuration
    ///
    /// # Returns
    /// A new `WorkspaceRename` instance
    pub fn new(index: WorkspaceIndex, config: WorkspaceRenameConfig) -> Self {
        Self { index, config }
    }

    /// Get a reference to the workspace index
    pub fn index(&self) -> &WorkspaceIndex {
        &self.index
    }

    /// Rename a symbol across the workspace
    ///
    /// # Arguments
    /// * `old_name` - Current symbol name
    /// * `new_name` - New symbol name
    /// * `file_path` - File containing the symbol
    /// * `position` - Position of the symbol (line, column)
    ///
    /// # Returns
    /// * `Ok(WorkspaceRenameResult)` - Rename result with edits and statistics
    /// * `Err(WorkspaceRenameError)` - Error during rename operation
    ///
    /// # Errors
    /// * `SymbolNotFound` - Symbol not found in workspace
    /// * `NameConflict` - New name conflicts with existing symbol
    /// * `Timeout` - Operation exceeded configured timeout
    /// * `FileSystemError` - File I/O error
    pub fn rename_symbol(
        &self,
        old_name: &str,
        new_name: &str,
        file_path: &Path,
        _position: (usize, usize),
    ) -> Result<WorkspaceRenameResult, WorkspaceRenameError> {
        self.rename_symbol_impl(old_name, new_name, file_path, None)
    }

    /// Rename a symbol with progress reporting
    ///
    /// # Arguments
    /// * `old_name` - Current symbol name
    /// * `new_name` - New symbol name
    /// * `file_path` - File containing the symbol
    /// * `position` - Position of the symbol (line, column)
    /// * `progress_tx` - Channel for progress events
    ///
    /// # Returns
    /// * `Ok(WorkspaceRenameResult)` - Rename result with edits and statistics
    /// * `Err(WorkspaceRenameError)` - Error during rename operation
    pub fn rename_symbol_with_progress(
        &self,
        old_name: &str,
        new_name: &str,
        file_path: &Path,
        _position: (usize, usize),
        progress_tx: std::sync::mpsc::Sender<Progress>,
    ) -> Result<WorkspaceRenameResult, WorkspaceRenameError> {
        self.rename_symbol_impl(old_name, new_name, file_path, Some(progress_tx))
    }

    /// Core rename implementation shared between rename_symbol and rename_symbol_with_progress
    fn rename_symbol_impl(
        &self,
        old_name: &str,
        new_name: &str,
        file_path: &Path,
        progress_tx: Option<std::sync::mpsc::Sender<Progress>>,
    ) -> Result<WorkspaceRenameResult, WorkspaceRenameError> {
        let start = Instant::now();
        let timeout = std::time::Duration::from_secs(self.config.operation_timeout);

        // Extract the bare name and optional package qualifier from old_name
        let (old_package, old_bare) = split_qualified_name(old_name);
        let (new_package, new_bare) = split_qualified_name(new_name);

        // AC:AC2 - Name conflict validation
        // Check if any symbol already exists with the new name
        self.check_name_conflicts(new_bare, old_package)?;

        // AC:AC1 - Workspace symbol identification using dual indexing
        // Find definition first
        let definition = self.index.find_definition(old_name);

        // Get the package context for scope-aware rename
        // Only use scope filtering when the user explicitly provided a qualified name
        let scope_package = old_package.map(|p| p.to_string());

        // Collect all references using dual indexing
        let mut all_references = self.index.find_references(old_name);

        // If qualified, also find bare references
        if let Some(_pkg) = &scope_package {
            let qualified = format!("{}::{}", _pkg, old_bare);
            let qualified_refs = self.index.find_references(&qualified);
            for r in qualified_refs {
                if !all_references
                    .iter()
                    .any(|existing| existing.uri == r.uri && existing.range == r.range)
                {
                    all_references.push(r);
                }
            }
            // Also search bare form
            let bare_refs = self.index.find_references(old_bare);
            for r in bare_refs {
                if !all_references
                    .iter()
                    .any(|existing| existing.uri == r.uri && existing.range == r.range)
                {
                    all_references.push(r);
                }
            }
        }

        // Add the definition location if not already present
        if let Some(ref def) = definition {
            if !all_references.iter().any(|r| r.uri == def.uri && r.range == def.range) {
                all_references.push(def.clone());
            }
        }

        // Also try text-based fallback search across all indexed documents
        let store = self.index.document_store();
        let all_docs = store.all_documents();
        let total_files = all_docs.len();

        // Emit scanning progress
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(Progress::Scanning { total: total_files });
        }

        // AC:AC4 - Perl scoping rules
        // For scope-aware rename, we search for the old_bare name in document text
        // but only replace it when it matches the correct scope
        let mut edits_by_file: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();
        let mut files_processed = 0;

        for (idx, doc) in all_docs.iter().enumerate() {
            // Check timeout
            if start.elapsed() > timeout {
                return Err(WorkspaceRenameError::Timeout {
                    elapsed_seconds: start.elapsed().as_secs(),
                    files_processed,
                    total_files,
                });
            }

            // Check max_files limit
            if self.config.max_files > 0 && files_processed >= self.config.max_files {
                break;
            }

            let doc_path = crate::workspace_index::uri_to_fs_path(&doc.uri);

            // Emit processing progress
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(Progress::Processing {
                    current: idx + 1,
                    total: total_files,
                    file: doc_path.clone().unwrap_or_default(),
                });
            }

            // Search for the old name in this document's text
            let text = &doc.text;
            if !text.contains(old_bare) {
                files_processed += 1;
                continue;
            }

            let line_index = &doc.line_index;
            let mut search_pos = 0;
            let mut file_edits = Vec::new();

            while let Some(found) = text[search_pos..].find(old_bare) {
                let match_start = search_pos + found;
                let match_end = match_start + old_bare.len();

                // Bounds check
                if match_end > text.len() {
                    break;
                }

                // Verify this is a word boundary match (not a substring of a larger identifier).
                // Walk chars (not bytes) so UTF-8 continuation bytes aren't mistaken for
                // word boundaries (#956).
                let is_word_start = is_word_boundary_before(text, match_start);
                let is_word_end = is_word_boundary_after(text, match_end);

                // Allow the match if it is either in plain code or is an
                // interpolated variable reference inside a double-quoted string.
                // The second condition catches `"$var"`, `"${var}"`, `"@arr"` etc.
                // while correctly skipping bare literal text like `"hello var"`.
                if is_word_start
                    && is_word_end
                    && (is_rename_code_position(text, match_start)
                        || is_interpolated_in_double_quote(text, match_start))
                {
                    // AC:AC4 - Scope check: if we have a package context, verify this reference
                    // is in the correct scope
                    let in_scope = if let Some(ref pkg) = scope_package {
                        // Check if the reference is qualified with the correct package
                        let before = &text[..match_start];
                        let is_qualified_with_pkg = before.ends_with(&format!("{}::", pkg))
                            || before.ends_with(&format!("{pkg}'"));

                        // Check if we're within the right package scope
                        let current_package = find_package_at_offset(text, match_start);
                        let in_package_scope = current_package.as_deref() == Some(pkg.as_str());

                        is_qualified_with_pkg || in_package_scope
                    } else {
                        true
                    };

                    if in_scope {
                        // Also replace the package qualifier if it precedes the match
                        let (edit_start, replacement) = build_replacement_text(
                            text,
                            match_start,
                            &scope_package,
                            new_package,
                            new_bare,
                        );

                        let (start_line, start_col) = line_index.offset_to_position(edit_start);
                        let (end_line, end_col) = line_index.offset_to_position(match_end);

                        if let (Some(start_byte), Some(end_byte)) = (
                            line_index.position_to_offset(start_line, start_col),
                            line_index.position_to_offset(end_line, end_col),
                        ) {
                            file_edits.push(TextEdit {
                                start: start_byte,
                                end: end_byte,
                                new_text: replacement,
                            });
                        }
                    }
                }

                search_pos = match_end;

                // Safety limit
                if file_edits.len() >= 1000 {
                    break;
                }
            }

            if !file_edits.is_empty() {
                if let Some(path) = doc_path {
                    edits_by_file.entry(path).or_default().extend(file_edits);
                }
            }

            files_processed += 1;
        }

        // If no edits found, the symbol wasn't found
        if edits_by_file.is_empty() {
            return Err(WorkspaceRenameError::SymbolNotFound {
                symbol: old_name.to_string(),
                file: file_path.display().to_string(),
            });
        }

        // Build file edits, sorting each file's edits in reverse order for safe application
        let file_edits: Vec<FileEdit> = edits_by_file
            .into_iter()
            .map(|(file_path, mut edits)| {
                edits.sort_by_key(|e| std::cmp::Reverse(e.start));
                FileEdit { file_path, edits }
            })
            .collect();

        let total_changes: usize = file_edits.iter().map(|fe| fe.edits.len()).sum();
        let files_modified = file_edits.len();

        // AC:AC5 - Backup creation
        let backup_info =
            if self.config.create_backups { self.create_backup(&file_edits).ok() } else { None };

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Emit completion progress
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(Progress::Complete { files_modified, changes: total_changes });
        }

        Ok(WorkspaceRenameResult {
            file_edits,
            backup_info,
            description: format!("Rename '{}' to '{}'", old_name, new_name),
            warnings: vec![],
            statistics: RenameStatistics { files_modified, total_changes, elapsed_ms },
        })
    }

    /// Check for name conflicts in the workspace
    fn check_name_conflicts(
        &self,
        new_bare_name: &str,
        scope_package: Option<&str>,
    ) -> Result<(), WorkspaceRenameError> {
        let all_symbols = self.index.all_symbols();

        let mut conflicts = Vec::new();
        for symbol in &all_symbols {
            let matches_bare = symbol.name == new_bare_name;
            let matches_qualified = if let Some(pkg) = scope_package {
                let qualified = format!("{}::{}", pkg, new_bare_name);
                symbol.qualified_name.as_deref() == Some(&qualified) || symbol.name == qualified
            } else {
                false
            };

            if matches_bare || matches_qualified {
                conflicts.push(ConflictLocation {
                    file: crate::workspace_index::uri_to_fs_path(&symbol.uri).unwrap_or_default(),
                    line: symbol.range.start.line,
                    column: symbol.range.start.column,
                    existing_symbol: symbol
                        .qualified_name
                        .clone()
                        .unwrap_or_else(|| symbol.name.clone()),
                });
            }
        }

        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(WorkspaceRenameError::NameConflict {
                new_name: new_bare_name.to_string(),
                conflicts,
            })
        }
    }

    /// Create backups of files that will be modified
    fn create_backup(&self, file_edits: &[FileEdit]) -> Result<BackupInfo, WorkspaceRenameError> {
        // Use nanos + thread ID for uniqueness across parallel operations
        let ts =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let backup_dir = std::env::temp_dir().join(format!(
            "perl_rename_backup_{}_{}_{:?}",
            ts.as_secs(),
            ts.subsec_nanos(),
            std::thread::current().id()
        ));

        std::fs::create_dir_all(&backup_dir).map_err(|e| {
            WorkspaceRenameError::FileSystemError {
                operation: "create_backup_dir".to_string(),
                file: backup_dir.clone(),
                error: e.to_string(),
            }
        })?;

        let mut file_mappings = HashMap::new();

        for (idx, file_edit) in file_edits.iter().enumerate() {
            if file_edit.file_path.exists() {
                // Use index prefix + filename for uniqueness within a single backup
                let file_name = file_edit
                    .file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let backup_name = format!("{}_{}", idx, file_name);
                let backup_path = backup_dir.join(&backup_name);

                std::fs::copy(&file_edit.file_path, &backup_path).map_err(|e| {
                    WorkspaceRenameError::FileSystemError {
                        operation: "backup_copy".to_string(),
                        file: file_edit.file_path.clone(),
                        error: e.to_string(),
                    }
                })?;

                file_mappings.insert(file_edit.file_path.clone(), backup_path);
            }
        }

        Ok(BackupInfo { backup_dir, file_mappings })
    }

    /// Apply file edits atomically with rollback support
    ///
    /// # AC:AC3 - Atomic multi-file changes
    pub fn apply_edits(&self, result: &WorkspaceRenameResult) -> Result<(), WorkspaceRenameError> {
        let mut written_files = Vec::new();

        for file_edit in &result.file_edits {
            // Read original content
            let content = std::fs::read_to_string(&file_edit.file_path).map_err(|e| {
                // Rollback already-written files before returning error
                if let Some(ref backup) = result.backup_info {
                    let _ = self.rollback_from_backup(&written_files, backup);
                }
                WorkspaceRenameError::FileSystemError {
                    operation: "read".to_string(),
                    file: file_edit.file_path.clone(),
                    error: e.to_string(),
                }
            })?;

            // Apply edits in reverse order (edits are already sorted end-to-start)
            let mut new_content = content;
            for edit in &file_edit.edits {
                if edit.start <= new_content.len() && edit.end <= new_content.len() {
                    new_content = format!(
                        "{}{}{}",
                        &new_content[..edit.start],
                        edit.new_text,
                        &new_content[edit.end..],
                    );
                }
            }

            // Write modified content
            std::fs::write(&file_edit.file_path, &new_content).map_err(|e| {
                // Rollback already-written files
                if let Some(ref backup) = result.backup_info {
                    let _ = self.rollback_from_backup(&written_files, backup);
                }
                WorkspaceRenameError::FileSystemError {
                    operation: "write".to_string(),
                    file: file_edit.file_path.clone(),
                    error: e.to_string(),
                }
            })?;

            written_files.push(file_edit.file_path.clone());
        }

        Ok(())
    }

    /// Rollback files from backup
    fn rollback_from_backup(
        &self,
        files: &[PathBuf],
        backup: &BackupInfo,
    ) -> Result<(), WorkspaceRenameError> {
        for file in files {
            if let Some(backup_path) = backup.file_mappings.get(file) {
                std::fs::copy(backup_path, file).map_err(|e| {
                    WorkspaceRenameError::RollbackFailed {
                        original_error: "file write failed".to_string(),
                        rollback_error: format!("failed to restore {}: {}", file.display(), e),
                        backup_dir: backup.backup_dir.clone(),
                    }
                })?;
            }
        }
        Ok(())
    }

    /// Update the workspace index after a rename operation
    ///
    /// # AC:AC8 - Dual indexing update
    pub fn update_index_after_rename(
        &self,
        old_name: &str,
        new_name: &str,
        file_edits: &[FileEdit],
    ) -> Result<(), WorkspaceRenameError> {
        // Re-index each modified file with new content
        for file_edit in file_edits {
            let content = std::fs::read_to_string(&file_edit.file_path).map_err(|e| {
                WorkspaceRenameError::IndexUpdateFailed {
                    error: format!("Failed to read {}: {}", file_edit.file_path.display(), e),
                    affected_files: vec![file_edit.file_path.clone()],
                }
            })?;

            let uri_str =
                crate::workspace_index::fs_path_to_uri(&file_edit.file_path).map_err(|e| {
                    WorkspaceRenameError::IndexUpdateFailed {
                        error: format!("URI conversion failed: {}", e),
                        affected_files: vec![file_edit.file_path.clone()],
                    }
                })?;

            // Remove old index entries and re-index with new content
            self.index.remove_file(&uri_str);

            let url =
                url::Url::parse(&uri_str).map_err(|e| WorkspaceRenameError::IndexUpdateFailed {
                    error: format!("URL parse failed: {}", e),
                    affected_files: vec![file_edit.file_path.clone()],
                })?;

            self.index.index_file(url, content).map_err(|e| {
                WorkspaceRenameError::IndexUpdateFailed {
                    error: format!(
                        "Re-indexing failed for '{}' -> '{}': {}",
                        old_name, new_name, e
                    ),
                    affected_files: vec![file_edit.file_path.clone()],
                }
            })?;
        }

        Ok(())
    }
}

fn build_replacement_text(
    text: &str,
    match_start: usize,
    scope_package: &Option<String>,
    new_package: Option<&str>,
    new_bare: &str,
) -> (usize, String) {
    if let Some(pkg) = scope_package {
        let prefix = format!("{pkg}::");
        if match_start >= prefix.len() && text[match_start - prefix.len()..match_start] == *prefix {
            let target_package = new_package.unwrap_or(pkg.as_str());
            return (match_start - prefix.len(), format!("{target_package}::{new_bare}"));
        }

        let legacy_prefix = format!("{pkg}'");
        if match_start >= legacy_prefix.len()
            && text[match_start - legacy_prefix.len()..match_start] == *legacy_prefix
        {
            let target_package = new_package.unwrap_or(pkg.as_str());
            return (match_start - legacy_prefix.len(), format!("{target_package}::{new_bare}"));
        }
    }

    (match_start, new_bare.to_string())
}

/// Perl string context at a given byte offset.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StringContext {
    /// Executable code (rename allowed).
    Code,
    /// Inside a `'...'` single-quoted string (no interpolation).
    SingleQuoted,
    /// Inside a `"..."` double-quoted string (interpolation allowed for sigil-preceded names).
    DoubleQuoted,
    /// Inside a `# ...` line comment (never rename).
    LineComment,
}

/// Scan `text` up to `offset` and return the Perl string context at that position.
fn scan_string_context(text: &str, offset: usize) -> StringContext {
    let mut state = StringContext::Code;
    let mut escaped = false;

    for (idx, byte) in text.bytes().enumerate() {
        if idx >= offset {
            return state;
        }

        match state {
            StringContext::Code => match byte {
                b'\'' => state = StringContext::SingleQuoted,
                b'"' => state = StringContext::DoubleQuoted,
                b'#' => state = StringContext::LineComment,
                _ => {}
            },
            StringContext::SingleQuoted => {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'\'' {
                    state = StringContext::Code;
                }
            }
            StringContext::DoubleQuoted => {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    state = StringContext::Code;
                }
            }
            StringContext::LineComment => {
                if byte == b'\n' {
                    state = StringContext::Code;
                }
            }
        }
    }

    state
}

/// Check whether a byte offset is in executable Perl code rather than trivia.
///
/// Returns `true` for plain code; `false` for single-quoted strings, double-quoted
/// strings, and line comments.  To check interpolated references inside double-quoted
/// strings, use [`is_interpolated_in_double_quote`] in addition.
fn is_rename_code_position(text: &str, offset: usize) -> bool {
    scan_string_context(text, offset) == StringContext::Code
}

/// Returns `true` when `offset` is inside a double-quoted Perl string AND the
/// token at `offset` is immediately preceded by a Perl interpolation sigil
/// (`$`, `@`, `%`) or by `{` that is itself preceded by a sigil (covering
/// `"${var}"`, `"@{arr}"`, etc.).
///
/// This is the complement to [`is_rename_code_position`]: together they ensure
/// that every real variable reference — in plain code *and* in double-quoted
/// string interpolation — is included in a rename, while literal text inside
/// strings (`"hello var text"`) is correctly excluded.
///
/// Single-quoted strings (`'...'`) are never interpolated and are not matched.
fn is_interpolated_in_double_quote(text: &str, offset: usize) -> bool {
    if scan_string_context(text, offset) != StringContext::DoubleQuoted {
        return false;
    }

    // Look at the byte(s) immediately before `offset`.
    if offset == 0 {
        return false;
    }

    let before = &text[..offset];
    // Last character before the identifier token.
    let last_char = match before.chars().next_back() {
        Some(c) => c,
        None => return false,
    };

    // Direct sigil: `$var`, `@arr`, `%hash`
    if matches!(last_char, '$' | '@' | '%') {
        return true;
    }

    // Braced form: `${var}`, `@{arr}`, `%{hash}` — last char is `{`
    if last_char == '{' {
        let before_brace = &before[..before.len() - 1]; // strip the `{`
        let sigil = before_brace.chars().next_back();
        return matches!(sigil, Some('$') | Some('@') | Some('%'));
    }

    false
}

/// Check if a char is a valid Perl identifier character.
/// Operates on chars (not bytes) so UTF-8 continuation bytes are handled correctly (#956).
fn is_perl_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns `true` when the position immediately before `byte_offset` in `text`
/// is a word boundary (no preceding Perl identifier character).
///
/// Char-aware replacement for the old byte-level check that treated UTF-8
/// continuation bytes as boundaries (#956).
fn is_word_boundary_before(text: &str, byte_offset: usize) -> bool {
    if byte_offset == 0 {
        return true;
    }
    text[..byte_offset].chars().next_back().is_none_or(|c| !is_perl_ident_char(c))
}

/// Returns `true` when the position immediately at `byte_offset` in `text`
/// is a word boundary (no following Perl identifier character).
///
/// Char-aware replacement for the old byte-level check that treated UTF-8
/// lead bytes as boundaries (#956).
fn is_word_boundary_after(text: &str, byte_offset: usize) -> bool {
    if byte_offset >= text.len() {
        return true;
    }
    text[byte_offset..].chars().next().is_none_or(|c| !is_perl_ident_char(c))
}

/// Find the current package scope at a given byte offset in Perl source
fn find_package_at_offset(text: &str, offset: usize) -> Option<String> {
    let before = &text[..offset];
    // Search backwards for the most recent "package NAME" declaration
    let mut last_package = None;
    let mut search_pos = 0;
    while let Some(found) = before[search_pos..].find("package ") {
        let pkg_start = search_pos + found + "package ".len();
        // Extract the package name (until ; or { or whitespace)
        let remaining = &before[pkg_start..];
        let pkg_end = remaining
            .find(|c: char| c == ';' || c == '{' || c.is_whitespace())
            .unwrap_or(remaining.len());
        let pkg_name = remaining[..pkg_end].trim();
        if !pkg_name.is_empty() {
            last_package = Some(pkg_name.to_string());
        }
        search_pos = pkg_start;
    }
    last_package
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = WorkspaceRenameConfig::default();
        assert!(config.atomic_mode);
        assert!(config.create_backups);
        assert_eq!(config.operation_timeout, 60);
        assert!(config.parallel_processing);
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.max_files, 0);
        assert!(config.report_progress);
        assert!(config.validate_syntax);
    }

    #[test]
    fn test_split_qualified_name() {
        assert_eq!(split_qualified_name("process"), (None, "process"));
        assert_eq!(split_qualified_name("Utils::process"), (Some("Utils"), "process"));
        assert_eq!(split_qualified_name("A::B::process"), (Some("A::B"), "process"));
    }

    #[test]
    fn test_is_perl_ident_char() {
        assert!(is_perl_ident_char('a'));
        assert!(is_perl_ident_char('Z'));
        assert!(is_perl_ident_char('0'));
        assert!(is_perl_ident_char('_'));
        assert!(!is_perl_ident_char(' '));
        assert!(!is_perl_ident_char(':'));
        assert!(!is_perl_ident_char(';'));
        // Unicode alphanumerics are valid Perl identifier chars under `use utf8`
        assert!(is_perl_ident_char('α'));
        assert!(is_perl_ident_char('変'));
        // UTF-8 continuation bytes must NOT be treated as ident chars
        assert!(!is_perl_ident_char('\u{B0}')); // not alphanumeric
    }

    #[test]
    fn test_find_package_at_offset() {
        let text = "package Foo;\nsub bar { 1 }\npackage Bar;\nsub baz { 2 }\n";
        assert_eq!(find_package_at_offset(text, 20), Some("Foo".to_string()));
        assert_eq!(find_package_at_offset(text, 45), Some("Bar".to_string()));
        assert_eq!(find_package_at_offset(text, 0), None);
    }

    #[test]
    fn test_workspace_rename_leaves_comments_and_strings_unchanged()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("main.pl");
        let content = concat!(
            "sub target_name { target_name(); }\n",
            "# target_name in comment must stay descriptive\n",
            "my $single = 'target_name in single quoted text';\n",
            "my $double = \"target_name in double quoted text\";\n",
        );
        std::fs::write(&path, content)?;

        let index = WorkspaceIndex::new();
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| format!("failed to create URL for {}", path.display()))?;
        index.index_file_str(uri.as_str(), content)?;

        let config = WorkspaceRenameConfig { create_backups: false, ..Default::default() };
        let rename_engine = WorkspaceRename::new(index, config);
        let result = rename_engine.rename_symbol("target_name", "renamed_name", &path, (0, 4))?;

        assert_eq!(result.statistics.total_changes, 2);
        rename_engine.apply_edits(&result)?;

        let renamed = std::fs::read_to_string(&path)?;
        assert!(renamed.contains("sub renamed_name { renamed_name(); }"));
        assert!(renamed.contains("# target_name in comment must stay descriptive"));
        assert!(renamed.contains("'target_name in single quoted text'"));
        assert!(renamed.contains("\"target_name in double quoted text\""));

        Ok(())
    }

    #[test]
    fn test_rename_code_position_skips_comments_and_strings() -> Result<(), String> {
        let text = concat!(
            "sub old_name { old_name(); }\n",
            "# old_name in comment must stay descriptive\n",
            "my $single = 'old_name in single quoted text';\n",
            "my $double = \"old_name in double quoted text\";\n",
            "old_name();\n",
        );

        let definition = text
            .find("old_name")
            .ok_or_else(|| "definition occurrence should exist".to_string())?;
        let comment = text
            .find("old_name in comment")
            .ok_or_else(|| "comment occurrence should exist".to_string())?;
        let single = text
            .find("old_name in single")
            .ok_or_else(|| "single-quoted occurrence should exist".to_string())?;
        let double = text
            .find("old_name in double")
            .ok_or_else(|| "double-quoted occurrence should exist".to_string())?;
        let final_call = text
            .rfind("old_name")
            .ok_or_else(|| "final call occurrence should exist".to_string())?;

        assert!(is_rename_code_position(text, definition));
        assert!(!is_rename_code_position(text, comment));
        assert!(!is_rename_code_position(text, single));
        assert!(!is_rename_code_position(text, double));
        assert!(is_rename_code_position(text, final_call));

        Ok(())
    }

    #[test]
    fn test_rename_code_position_respects_escaped_quotes() -> Result<(), String> {
        let text = "my $text = \"escaped \\\" old_name still string\";\nold_name();\n";
        let string_occurrence = text
            .find("old_name still string")
            .ok_or_else(|| "string occurrence should exist".to_string())?;
        let code_occurrence =
            text.rfind("old_name").ok_or_else(|| "code occurrence should exist".to_string())?;

        assert!(!is_rename_code_position(text, string_occurrence));
        assert!(is_rename_code_position(text, code_occurrence));

        Ok(())
    }

    #[test]
    fn test_build_replacement_text_qualified_uses_new_package_when_provided() {
        let text = "Old::process()";
        let scope_package = Some("Old".to_string());
        let (start, replacement) =
            build_replacement_text(text, "Old::".len(), &scope_package, Some("New"), "execute");

        assert_eq!(start, 0);
        assert_eq!(replacement, "New::execute");
    }

    #[test]
    fn test_build_replacement_text_qualified_keeps_old_package_when_new_is_unqualified() {
        let text = "Old::process()";
        let scope_package = Some("Old".to_string());
        let (start, replacement) =
            build_replacement_text(text, "Old::".len(), &scope_package, None, "execute");

        assert_eq!(start, 0);
        assert_eq!(replacement, "Old::execute");
    }

    #[test]
    fn test_build_replacement_text_unqualified_replaces_bare_symbol_only() {
        let text = "process()";
        let scope_package = Some("Old".to_string());
        let (start, replacement) =
            build_replacement_text(text, 0, &scope_package, Some("New"), "execute");

        assert_eq!(start, 0);
        assert_eq!(replacement, "execute");
    }

    #[test]
    fn test_build_replacement_text_legacy_qualified_rewrites_to_double_colon() {
        let text = "Old'process()";
        let scope_package = Some("Old".to_string());
        let (start, replacement) =
            build_replacement_text(text, "Old'".len(), &scope_package, Some("New"), "execute");

        assert_eq!(start, 0);
        assert_eq!(replacement, "New::execute");
    }

    // -------------------------------------------------------------------------
    // Char-boundary helpers (#956) - lib-level coverage for Codecov patch gate
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_word_boundary_before_ascii() {
        // At offset 0: always a boundary
        assert!(is_word_boundary_before("foo", 0));
        // Preceded by space: boundary
        assert!(is_word_boundary_before("x foo", 2));
        // Preceded by ASCII ident char: NOT a boundary
        assert!(!is_word_boundary_before("xfoo", 1));
        assert!(!is_word_boundary_before("_foo", 1));
    }

    #[test]
    fn test_is_word_boundary_before_unicode() {
        // "変数foo" — each kanji is 3 bytes; byte offset of 'f' is 6
        let text = "変数foo";
        let foo_start = text.find("foo").unwrap();
        // The char before 'f' is '数' (alphanumeric) — NOT a boundary
        assert!(!is_word_boundary_before(text, foo_start));

        // "αfoo" — α is 2 bytes (U+03B1); 'f' is at byte 2
        let text2 = "αfoo";
        let foo_start2 = text2.find("foo").unwrap();
        assert!(!is_word_boundary_before(text2, foo_start2));

        // "$foo" — '$' is not alphanumeric — IS a boundary
        assert!(is_word_boundary_before("$foo", 1));
    }

    #[test]
    fn test_is_word_boundary_after_ascii() {
        let text = "foo bar";
        // At text.len(): always a boundary
        assert!(is_word_boundary_after(text, text.len()));
        // After "foo": next char is space — boundary
        assert!(is_word_boundary_after(text, 3));
        // After 'f': next char is 'o' — NOT a boundary
        assert!(!is_word_boundary_after(text, 1));
    }

    #[test]
    fn test_is_word_boundary_after_unicode() {
        // "fooα" — α is U+03B1 (2 bytes); byte 3 starts α
        let text = "fooα";
        // The char at byte 3 is 'α' (alphanumeric) — NOT a boundary
        assert!(!is_word_boundary_after(text, 3));

        // "foo変" — '変' is 3 bytes; byte 3 starts '変'
        let text2 = "foo変";
        assert!(!is_word_boundary_after(text2, 3));

        // "foo " — byte 3 is space — boundary
        assert!(is_word_boundary_after("foo ", 3));
    }

    // -------------------------------------------------------------------------
    // String-interpolation helpers — lib-level coverage for Codecov patch gate
    // -------------------------------------------------------------------------

    /// `scan_string_context` must return `Code` for plain code positions.
    #[test]
    fn test_scan_string_context_code_positions() {
        let text = "sub foo { foo(); }\n";
        let foo_pos = text.find("foo").unwrap_or(0);
        assert_eq!(scan_string_context(text, foo_pos), StringContext::Code);
        assert_eq!(scan_string_context(text, text.len()), StringContext::Code);
    }

    /// `scan_string_context` must return `SingleQuoted` inside `'...'`.
    #[test]
    fn test_scan_string_context_single_quoted() {
        let text = "my $x = 'hello';";
        // offset of 'h' inside the single-quoted string
        let h_pos = text.find("hello").unwrap_or(0);
        assert_eq!(scan_string_context(text, h_pos), StringContext::SingleQuoted);
    }

    /// `scan_string_context` must return `DoubleQuoted` inside `"..."`.
    #[test]
    fn test_scan_string_context_double_quoted() {
        let text = "my $x = \"hello\";";
        let h_pos = text.find("hello").unwrap_or(0);
        assert_eq!(scan_string_context(text, h_pos), StringContext::DoubleQuoted);
    }

    /// `scan_string_context` must return `LineComment` inside `#...` comments.
    #[test]
    fn test_scan_string_context_line_comment() {
        let text = "foo(); # bar in comment\n";
        let bar_pos = text.find("bar").unwrap_or(0);
        assert_eq!(scan_string_context(text, bar_pos), StringContext::LineComment);
    }

    /// `is_interpolated_in_double_quote` must be `true` for `$var`, `@arr`, `%hash` sigils.
    #[test]
    fn test_is_interpolated_direct_sigils() {
        // "$var" — 'v' is at offset 1, '$' is at offset 0
        let dollar = "\"$var\"";
        let v_pos = dollar.find("var").unwrap_or(0);
        assert!(is_interpolated_in_double_quote(dollar, v_pos), "$var must be interpolated");

        // "@arr" — 'a' is at offset 1, '@' at offset 0
        let at = "\"@arr\"";
        let a_pos = at.find("arr").unwrap_or(0);
        assert!(is_interpolated_in_double_quote(at, a_pos), "@arr must be interpolated");

        // "%hash" — 'h' at offset 1, '%' at offset 0
        let percent = "\"%hash\"";
        let h_pos = percent.find("hash").unwrap_or(0);
        assert!(is_interpolated_in_double_quote(percent, h_pos), "%hash must be interpolated");
    }

    /// `is_interpolated_in_double_quote` must be `true` for braced `${var}`, `@{arr}`.
    #[test]
    fn test_is_interpolated_braced_sigils() {
        // "${var}" inside a double-quoted string
        let braced_dollar = "\"${var}\"";
        let v_pos = braced_dollar.find("var").unwrap_or(0);
        assert!(
            is_interpolated_in_double_quote(braced_dollar, v_pos),
            "${{var}} must be interpolated"
        );

        // "@{arr}" inside a double-quoted string
        let braced_at = "\"@{arr}\"";
        let a_pos = braced_at.find("arr").unwrap_or(0);
        assert!(is_interpolated_in_double_quote(braced_at, a_pos), "@{{arr}} must be interpolated");
    }

    /// `is_interpolated_in_double_quote` must be `false` for bare text in strings.
    #[test]
    fn test_is_interpolated_bare_text_in_string_not_interpolated() {
        // `"hello var text"` — 'v' has space before it, not a sigil
        let text = "\"hello var text\"";
        let var_pos = text.find("var").unwrap_or(0);
        assert!(
            !is_interpolated_in_double_quote(text, var_pos),
            "bare text in string must NOT be treated as interpolated"
        );
    }

    /// `is_interpolated_in_double_quote` must be `false` for code positions.
    #[test]
    fn test_is_interpolated_not_in_string_returns_false() {
        // `$foo` in code (not in a string)
        let text = "my $foo = 1;";
        let foo_pos = text.find("foo").unwrap_or(0);
        assert!(
            !is_interpolated_in_double_quote(text, foo_pos),
            "code position must NOT be treated as interpolated"
        );
    }

    /// `is_interpolated_in_double_quote` must be `false` inside single-quoted strings.
    #[test]
    fn test_is_interpolated_single_quoted_returns_false() {
        // Perl single-quoted strings never interpolate
        let text = "my $x = '$foo';";
        let foo_pos = text.find("foo").unwrap_or(0);
        assert!(
            !is_interpolated_in_double_quote(text, foo_pos),
            "single-quoted string must never be treated as interpolated"
        );
    }

    /// End-to-end: renaming a variable finds occurrences in double-quoted
    /// interpolation (`"$var"`, `"${var}"`) but not in bare text or single quotes.
    #[test]
    fn test_rename_includes_double_quote_interpolation() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("interp.pl");
        // 3 code occurrences: declaration, bare call, interpolated in double-quoted string
        // 2 occurrences that must NOT be renamed: single-quoted string, bare text in dq-string
        let content = concat!(
            "my $tgt = 1;\n",                     // line 1 — code declaration
            "print $tgt;\n",                      // line 2 — code use
            "my $msg = \"value: $tgt\";\n",       // line 3 — dq interpolation (rename)
            "my $braced = \"${tgt} done\";\n",    // line 4 — braced dq interpolation (rename)
            "my $skip1 = 'literal $tgt here';\n", // line 5 — single-quoted (no rename)
            "my $skip2 = \"bare tgt text\";\n",   // line 6 — bare text in dq (no rename)
        );
        std::fs::write(&path, content)?;

        let index = WorkspaceIndex::new();
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| format!("failed to create URL for {}", path.display()))?;
        index.index_file_str(uri.as_str(), content)?;

        let config = WorkspaceRenameConfig { create_backups: false, ..Default::default() };
        let rename_engine = WorkspaceRename::new(index, config);
        let result = rename_engine.rename_symbol("tgt", "renamed", &path, (0, 3))?;

        // Should find: line 1 ($tgt decl), line 2 ($tgt use), line 3 ($tgt interpolated),
        //              line 4 (${tgt} braced interpolation) = 4 occurrences.
        // Should NOT find: line 5 (single-quoted), line 6 (bare text in dq string).
        assert_eq!(
            result.statistics.total_changes, 4,
            "expected 4 renames (2 code + 2 interpolated dq), got {}",
            result.statistics.total_changes
        );

        rename_engine.apply_edits(&result)?;
        let after = std::fs::read_to_string(&path)?;

        // Interpolated occurrences were renamed
        assert!(after.contains("\"value: $renamed\""), "dq interpolation must be renamed");
        assert!(after.contains("\"${renamed} done\""), "braced dq interpolation must be renamed");

        // Non-interpolated occurrences were NOT renamed
        assert!(after.contains("'literal $tgt here'"), "single-quoted must be unchanged");
        assert!(after.contains("\"bare tgt text\""), "bare text in dq string must be unchanged");

        Ok(())
    }
}

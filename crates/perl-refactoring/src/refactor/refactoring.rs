//! Unified refactoring engine for Perl code transformations
//!
//! This module provides a comprehensive refactoring engine that combines workspace-level
//! operations with modern code transformations. It serves as the primary entry point
//! for all refactoring operations in the Perl LSP ecosystem.
//!
//! ## LSP Workflow Integration
//!
//! The refactoring engine operates within the standard LSP workflow:
//! **Parse → Index → Navigate → Complete → Analyze**
//!
//! - **Parse Stage**: Analyzes Perl syntax to understand code structure
//! - **Index Stage**: Builds cross-file symbol relationships for safe refactoring
//! - **Navigate Stage**: Updates references and maintains navigation integrity
//! - **Complete Stage**: Ensures completion accuracy after refactoring changes
//! - **Analyze Stage**: Validates refactoring results and provides feedback
//!
//! ## Performance Characteristics
//!
//! Optimized for enterprise Perl development with large codebases:
//! - **Memory Efficiency**: Streaming approach for large file processing
//! - **Incremental Updates**: Only processes changed portions during refactoring
//! - **Parallel Operations**: Thread-safe refactoring for multi-file changes
//!
//! ## Architecture
//!
//! The unified engine integrates existing specialized refactoring modules:
//! - workspace_refactor: Cross-file operations and symbol management
//! - modernize: Code modernization and best practice application
//! - import_optimizer: Import statement optimization and cleanup

use crate::error::{ParseError, ParseResult};
use crate::import_optimizer::ImportOptimizer;
#[cfg(feature = "modernize")]
use crate::modernize::PerlModernizer as ModernizeEngine;
#[cfg(feature = "workspace_refactor")]
use crate::workspace_index::WorkspaceIndex;
#[cfg(feature = "workspace_refactor")]
use crate::workspace_refactor::WorkspaceRefactor;
use perl_parser_core::line_index::LineIndex;
use perl_parser_core::{Node, NodeKind, Parser, SourceLocation};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Unified refactoring engine that coordinates all refactoring operations
///
/// Provides a single interface for all types of code transformations,
/// from simple symbol renames to complex workspace restructuring.
pub struct RefactoringEngine {
    /// Workspace-level refactoring operations (architectural placeholder for future implementation)
    #[cfg(feature = "workspace_refactor")]
    #[allow(dead_code)]
    workspace_refactor: WorkspaceRefactor,
    #[cfg(not(feature = "workspace_refactor"))]
    #[allow(dead_code)]
    workspace_refactor: temp_stubs::WorkspaceRefactor,
    /// Code modernization engine for updating legacy Perl patterns
    #[cfg(feature = "modernize")]
    modernize: crate::modernize::PerlModernizer,
    /// Code modernization engine stub (feature disabled)
    #[cfg(not(feature = "modernize"))]
    modernize: temp_stubs::ModernizeEngine,
    /// Import optimization engine for cleaning up use statements
    import_optimizer: ImportOptimizer,
    /// Configuration for refactoring operations
    config: RefactoringConfig,
    /// Cache of recent operations for rollback support
    operation_history: Vec<RefactoringOperation>,
}

/// Configuration for refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringConfig {
    /// Enable safe mode (validate before applying changes)
    pub safe_mode: bool,
    /// Maximum number of files to process in a single operation
    pub max_files_per_operation: usize,
    /// Enable automatic backup creation
    pub create_backups: bool,
    /// Timeout for individual refactoring operations (seconds)
    pub operation_timeout: u64,
    /// Enable parallel processing for multi-file operations
    pub parallel_processing: bool,
    /// Maximum number of backup directories to retain (0 = unlimited)
    pub max_backup_retention: usize,
    /// Maximum age of backup directories in seconds (0 = no age limit)
    pub backup_max_age_seconds: u64,
    /// Custom backup root directory (defaults to temp_dir/perl_refactor_backups)
    #[serde(skip)]
    pub backup_root: Option<PathBuf>,
}

impl Default for RefactoringConfig {
    fn default() -> Self {
        Self {
            safe_mode: true,
            max_files_per_operation: 100,
            create_backups: true,
            operation_timeout: 60,
            parallel_processing: true,
            max_backup_retention: 10,
            backup_max_age_seconds: 7 * 24 * 60 * 60, // 7 days
            backup_root: None,
        }
    }
}

/// Types of refactoring operations supported by the engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    /// Rename symbols across workspace
    SymbolRename {
        /// Original symbol name to find
        old_name: String,
        /// New name to replace with
        new_name: String,
        /// Scope of the rename operation
        scope: RefactoringScope,
    },
    /// Extract methods from existing code
    ExtractMethod {
        /// Name for the extracted method
        method_name: String,
        /// Start position (line, column) of code to extract
        start_position: (usize, usize),
        /// End position (line, column) of code to extract
        end_position: (usize, usize),
    },
    /// Move code between files
    MoveCode {
        /// Source file containing the code to move
        source_file: PathBuf,
        /// Destination file for the moved code
        target_file: PathBuf,
        /// Names of elements (subs, packages) to move
        elements: Vec<String>,
    },
    /// Modernize legacy code patterns
    Modernize {
        /// Modernization patterns to apply
        patterns: Vec<ModernizationPattern>,
    },
    /// Optimize imports across files
    OptimizeImports {
        /// Remove unused import statements
        remove_unused: bool,
        /// Sort imports alphabetically
        sort_alphabetically: bool,
        /// Group imports by type (core, CPAN, local)
        group_by_type: bool,
    },
    /// Inline variables or methods
    Inline {
        /// Name of the symbol to inline
        symbol_name: String,
        /// Whether to inline all occurrences or just the selected one
        all_occurrences: bool,
    },
}

/// Scope of refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringScope {
    /// Single file operation
    File(PathBuf),
    /// Workspace-wide operation
    Workspace,
    /// Specific directory tree
    Directory(PathBuf),
    /// Custom set of files
    FileSet(Vec<PathBuf>),
    /// Package scope within a file
    Package {
        /// File containing the package declaration.
        file: PathBuf,
        /// Package name to scope the operation to.
        name: String,
    },
    /// Function scope within a file
    Function {
        /// File containing the function definition.
        file: PathBuf,
        /// Function name to scope the operation to.
        name: String,
    },
    /// Arbitrary block scope within a file (start, end positions)
    Block {
        /// File containing the block.
        file: PathBuf,
        /// Start position as (line, column).
        start: (u32, u32),
        /// End position as (line, column).
        end: (u32, u32),
    },
}

/// Modernization patterns for legacy code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModernizationPattern {
    /// Convert old-style subroutine calls to modern syntax
    SubroutineCalls,
    /// Add missing use strict/warnings
    StrictWarnings,
    /// Replace deprecated operators
    DeprecatedOperators,
    /// Modernize variable declarations
    VariableDeclarations,
    /// Update package declarations
    PackageDeclarations,
}

/// Record of a refactoring operation for rollback support
#[derive(Debug, Clone)]
pub struct RefactoringOperation {
    /// Unique identifier for the operation
    pub id: String,
    /// Type of operation performed
    pub operation_type: RefactoringType,
    /// Files modified during the operation
    pub modified_files: Vec<PathBuf>,
    /// Timestamp when operation was performed
    pub timestamp: std::time::SystemTime,
    /// Backup information for rollback
    pub backup_info: Option<BackupInfo>,
}

/// Backup information for operation rollback
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupInfo {
    /// Directory containing backup files
    pub backup_dir: PathBuf,
    /// Mapping of original files to backup locations
    pub file_mappings: HashMap<PathBuf, PathBuf>,
}

/// Result of backup cleanup operation
#[derive(Debug, Clone)]
pub struct BackupCleanupResult {
    /// Number of backup directories removed
    pub directories_removed: usize,
    /// Total space reclaimed in bytes
    pub space_reclaimed: u64,
}

/// Metadata for a backup directory used during cleanup
#[derive(Debug, Clone)]
#[allow(dead_code)] // size field reserved for future cleanup policy implementations
struct BackupDirMetadata {
    /// Path to the backup directory
    path: PathBuf,
    /// Last modification time
    modified: std::time::SystemTime,
    /// Total size in bytes
    size: u64,
}

/// Result of a refactoring operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of files modified
    pub files_modified: usize,
    /// Number of changes made
    pub changes_made: usize,
    /// Warning messages from the operation
    pub warnings: Vec<String>,
    /// Error messages if operation failed
    pub errors: Vec<String>,
    /// Operation identifier for rollback
    pub operation_id: Option<String>,
}

impl RefactoringEngine {
    /// Create a new refactoring engine with default configuration
    pub fn new() -> Self {
        Self::with_config(RefactoringConfig::default())
    }

    /// Create a new refactoring engine with custom configuration
    pub fn with_config(config: RefactoringConfig) -> Self {
        Self {
            #[cfg(feature = "workspace_refactor")]
            workspace_refactor: WorkspaceRefactor::new(WorkspaceIndex::default()),
            #[cfg(not(feature = "workspace_refactor"))]
            workspace_refactor: temp_stubs::WorkspaceRefactor::new(),
            #[cfg(feature = "modernize")]
            modernize: ModernizeEngine::new(),
            #[cfg(not(feature = "modernize"))]
            modernize: temp_stubs::ModernizeEngine::new(),
            import_optimizer: ImportOptimizer::new(),
            config,
            operation_history: Vec::new(),
        }
    }

    /// Perform a refactoring operation
    pub fn refactor(
        &mut self,
        operation_type: RefactoringType,
        files: Vec<PathBuf>,
    ) -> ParseResult<RefactoringResult> {
        let operation_id = self.generate_operation_id();

        // Validate operation if in safe mode
        if self.config.safe_mode {
            self.validate_operation(&operation_type, &files)?;
        }

        // Create backup if enabled
        let backup_info = if self.config.create_backups {
            Some(self.create_backup(&files, &operation_id)?)
        } else {
            None
        };

        // Perform the operation
        let result = match operation_type.clone() {
            RefactoringType::SymbolRename { old_name, new_name, scope } => {
                self.perform_symbol_rename(&old_name, &new_name, &scope)
            }
            RefactoringType::ExtractMethod { method_name, start_position, end_position } => {
                self.perform_extract_method(&method_name, start_position, end_position, &files)
            }
            RefactoringType::MoveCode { source_file, target_file, elements } => {
                self.perform_move_code(&source_file, &target_file, &elements)
            }
            RefactoringType::Modernize { patterns } => self.perform_modernize(&patterns, &files),
            RefactoringType::OptimizeImports {
                remove_unused,
                sort_alphabetically,
                group_by_type,
            } => self.perform_optimize_imports(
                remove_unused,
                sort_alphabetically,
                group_by_type,
                &files,
            ),
            RefactoringType::Inline { symbol_name, all_occurrences } => {
                self.perform_inline(&symbol_name, all_occurrences, &files)
            }
        };

        // Record operation in history
        let operation = RefactoringOperation {
            id: operation_id.clone(),
            operation_type,
            modified_files: files,
            timestamp: std::time::SystemTime::now(),
            backup_info,
        };
        self.operation_history.push(operation);

        // Return result with operation ID
        match result {
            Ok(mut res) => {
                res.operation_id = Some(operation_id);
                Ok(res)
            }
            Err(e) => Err(e),
        }
    }

    /// Rollback a previous refactoring operation
    pub fn rollback(&mut self, operation_id: &str) -> ParseResult<RefactoringResult> {
        // Find the operation in history
        let operation =
            self.operation_history.iter().find(|op| op.id == operation_id).ok_or_else(|| {
                ParseError::SyntaxError {
                    message: format!("Operation {} not found", operation_id),
                    location: 0,
                }
            })?;

        if let Some(backup_info) = &operation.backup_info {
            // Restore files from backup
            let mut restored_count = 0;
            for (original, backup) in &backup_info.file_mappings {
                if backup.exists() {
                    std::fs::copy(backup, original).map_err(|e| ParseError::SyntaxError {
                        message: format!("Failed to restore {}: {}", original.display(), e),
                        location: 0,
                    })?;
                    restored_count += 1;
                }
            }

            Ok(RefactoringResult {
                success: true,
                files_modified: restored_count,
                changes_made: restored_count,
                warnings: vec![],
                errors: vec![],
                operation_id: None,
            })
        } else {
            Err(ParseError::SyntaxError {
                message: "No backup available for rollback".to_string(),
                location: 0,
            })
        }
    }

    /// Get list of recent operations
    pub fn get_operation_history(&self) -> &[RefactoringOperation] {
        &self.operation_history
    }

    /// Clear operation history and cleanup backups
    pub fn clear_history(&mut self) -> ParseResult<BackupCleanupResult> {
        let cleanup_result = self.cleanup_backup_directories()?;
        self.operation_history.clear();
        Ok(cleanup_result)
    }

    /// Index a file for workspace-aware refactoring operations
    pub fn index_file(&mut self, path: &Path, content: &str) -> ParseResult<()> {
        #[cfg(feature = "workspace_refactor")]
        {
            let uri_str = crate::workspace_index::fs_path_to_uri(path).map_err(|e| {
                ParseError::SyntaxError {
                    message: format!("URI conversion failed: {}", e),
                    location: 0,
                }
            })?;
            let url = url::Url::parse(&uri_str).map_err(|e| ParseError::SyntaxError {
                message: format!("URL parsing failed: {}", e),
                location: 0,
            })?;
            self.workspace_refactor._index.index_file(url, content.to_string()).map_err(|e| {
                ParseError::SyntaxError { message: format!("Indexing failed: {}", e), location: 0 }
            })?;
        }
        let _ = content; // Acknowledge when feature disabled
        Ok(())
    }

    // Private implementation methods

    fn generate_operation_id(&self) -> String {
        let duration =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        format!("refactor_{}_{}", duration.as_secs(), duration.subsec_nanos())
    }

    fn validate_operation(
        &self,
        operation_type: &RefactoringType,
        files: &[PathBuf],
    ) -> ParseResult<()> {
        // Check file count limit
        if files.len() > self.config.max_files_per_operation {
            return Err(ParseError::SyntaxError {
                message: format!(
                    "Operation exceeds maximum file limit: {} files provided, {} allowed",
                    files.len(),
                    self.config.max_files_per_operation
                ),
                location: 0,
            });
        }

        // Operation-specific validation
        match operation_type {
            RefactoringType::SymbolRename { old_name, new_name, scope } => {
                self.validate_perl_identifier(old_name, "old_name")?;
                self.validate_perl_identifier(new_name, "new_name")?;

                // old_name and new_name must be different
                if old_name == new_name {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "SymbolRename: old_name and new_name must be different (got '{}')",
                            old_name
                        ),
                        location: 0,
                    });
                }

                // Sigil consistency: if old_name has a sigil, new_name must have the same sigil
                let old_sigil = Self::extract_sigil(old_name);
                let new_sigil = Self::extract_sigil(new_name);
                if old_sigil != new_sigil {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "SymbolRename: sigil mismatch - old_name '{}' has sigil {:?}, new_name '{}' has sigil {:?}",
                            old_name, old_sigil, new_name, new_sigil
                        ),
                        location: 0,
                    });
                }

                // Validate scope-specific file requirements
                match scope {
                    RefactoringScope::File(path) => {
                        self.validate_file_exists(path)?;
                    }
                    RefactoringScope::Directory(path) => {
                        self.validate_directory_exists(path)?;
                    }
                    RefactoringScope::FileSet(paths) => {
                        if paths.is_empty() {
                            return Err(ParseError::SyntaxError {
                                message: "FileSet scope requires at least one file".to_string(),
                                location: 0,
                            });
                        }
                        // Enforce max_files_per_operation on FileSet scope
                        if paths.len() > self.config.max_files_per_operation {
                            return Err(ParseError::SyntaxError {
                                message: format!(
                                    "FileSet scope exceeds maximum file limit: {} files provided, {} allowed",
                                    paths.len(),
                                    self.config.max_files_per_operation
                                ),
                                location: 0,
                            });
                        }
                        for path in paths {
                            self.validate_file_exists(path)?;
                        }
                    }
                    RefactoringScope::Workspace => {
                        // Workspace scope doesn't require specific files
                    }
                    RefactoringScope::Package { file, .. }
                    | RefactoringScope::Function { file, .. }
                    | RefactoringScope::Block { file, .. } => {
                        self.validate_file_exists(file)?;
                    }
                }
            }

            RefactoringType::ExtractMethod { method_name, start_position, end_position } => {
                self.validate_perl_subroutine_name(method_name)?;

                // ExtractMethod generates `sub name {}`, so method_name must be a bare identifier
                // (no leading '&' sigil, which would produce invalid Perl like `sub &foo {}`)
                if method_name.starts_with('&') {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "ExtractMethod method_name must be a bare identifier (no leading '&'): got '{}'",
                            method_name
                        ),
                        location: 0,
                    });
                }

                // ExtractMethod requires exactly one file
                if files.is_empty() {
                    return Err(ParseError::SyntaxError {
                        message: "ExtractMethod requires a target file".to_string(),
                        location: 0,
                    });
                }
                if files.len() > 1 {
                    return Err(ParseError::SyntaxError {
                        message: "ExtractMethod operates on a single file".to_string(),
                        location: 0,
                    });
                }
                self.validate_file_exists(&files[0])?;

                // Validate position ordering
                if start_position >= end_position {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "Invalid extraction range: start {:?} must be before end {:?}",
                            start_position, end_position
                        ),
                        location: 0,
                    });
                }
            }

            RefactoringType::MoveCode { source_file, target_file, elements } => {
                self.validate_file_exists(source_file)?;

                // Reject moving code to the same file
                if source_file == target_file {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "MoveCode: source_file and target_file must be different (got '{}')",
                            source_file.display()
                        ),
                        location: 0,
                    });
                }

                // Target file may not exist yet (will be created)
                if let Some(parent) = target_file.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        return Err(ParseError::SyntaxError {
                            message: format!(
                                "Target directory does not exist: {}",
                                parent.display()
                            ),
                            location: 0,
                        });
                    }
                }

                if elements.is_empty() {
                    return Err(ParseError::SyntaxError {
                        message: "MoveCode requires at least one element to move".to_string(),
                        location: 0,
                    });
                }

                // Validate element names (subs or packages)
                for element in elements {
                    self.validate_perl_qualified_name(element)?;
                }
            }

            RefactoringType::Modernize { patterns } => {
                if patterns.is_empty() {
                    return Err(ParseError::SyntaxError {
                        message: "Modernize requires at least one pattern".to_string(),
                        location: 0,
                    });
                }
                // Modernize can work on explicit files or scan workspace
                for file in files {
                    self.validate_file_exists(file)?;
                }
            }

            RefactoringType::OptimizeImports { .. } => {
                // OptimizeImports can work on explicit files or scan workspace
                for file in files {
                    self.validate_file_exists(file)?;
                }
            }

            RefactoringType::Inline { symbol_name, .. } => {
                self.validate_perl_identifier(symbol_name, "symbol_name")?;

                // Inline requires at least one file
                if files.is_empty() {
                    return Err(ParseError::SyntaxError {
                        message: "Inline requires at least one target file".to_string(),
                        location: 0,
                    });
                }
                for file in files {
                    self.validate_file_exists(file)?;
                }
            }
        }

        Ok(())
    }

    /// Validates a Perl identifier (variable, subroutine, or package name).
    ///
    /// Perl identifiers can have sigils ($, @, %, &, *) and the name portion
    /// must start with a letter or underscore, followed by alphanumerics/underscores.
    fn validate_perl_identifier(&self, name: &str, param_name: &str) -> ParseResult<()> {
        if name.is_empty() {
            return Err(ParseError::SyntaxError {
                message: format!("{} cannot be empty", param_name),
                location: 0,
            });
        }

        // Strip optional sigil
        let bare_name = name.strip_prefix(['$', '@', '%', '&', '*']).unwrap_or(name);

        if bare_name.is_empty() {
            return Err(ParseError::SyntaxError {
                message: format!("{} cannot be only a sigil", param_name),
                location: 0,
            });
        }

        // Handle qualified names (Package::name)
        // Allow leading :: (for main package or absolute names), but reject:
        // - trailing :: (like "Foo::")
        // - double :: in the middle (like "Foo::::Bar")
        let parts: Vec<&str> = bare_name.split("::").collect();
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                // Allow empty only at position 0 (leading ::)
                if i == 0 {
                    continue;
                }
                // Reject trailing :: or double ::
                return Err(ParseError::SyntaxError {
                    message: format!(
                        "Invalid Perl identifier in {}: '{}' (contains empty segment - trailing or double ::)",
                        param_name, name
                    ),
                    location: 0,
                });
            }
            if !Self::is_valid_identifier_part(part) {
                return Err(ParseError::SyntaxError {
                    message: format!(
                        "Invalid Perl identifier in {}: '{}' (must start with letter/underscore)",
                        param_name, name
                    ),
                    location: 0,
                });
            }
        }

        Ok(())
    }

    /// Validates a Perl subroutine name (no sigil allowed, but & is optional).
    fn validate_perl_subroutine_name(&self, name: &str) -> ParseResult<()> {
        if name.is_empty() {
            return Err(ParseError::SyntaxError {
                message: "Subroutine name cannot be empty".to_string(),
                location: 0,
            });
        }

        // Strip optional & sigil (only valid sigil for subs)
        let bare_name = name.strip_prefix('&').unwrap_or(name);

        // Reject other sigils
        if bare_name.starts_with(['$', '@', '%', '*']) {
            return Err(ParseError::SyntaxError {
                message: format!("Invalid sigil for subroutine name: '{}'", name),
                location: 0,
            });
        }

        if !Self::is_valid_identifier_part(bare_name) {
            return Err(ParseError::SyntaxError {
                message: format!(
                    "Invalid subroutine name: '{}' (must start with letter/underscore)",
                    name
                ),
                location: 0,
            });
        }

        Ok(())
    }

    /// Validates a qualified Perl name (Package::Subpackage::name).
    /// Used for MoveCode elements - does not allow sigils, leading ::, trailing ::, or double ::.
    fn validate_perl_qualified_name(&self, name: &str) -> ParseResult<()> {
        if name.is_empty() {
            return Err(ParseError::SyntaxError {
                message: "Qualified name cannot be empty".to_string(),
                location: 0,
            });
        }

        // Reject sigils - qualified names are for packages/subs, not variables
        if name.starts_with(['$', '@', '%', '&', '*']) {
            return Err(ParseError::SyntaxError {
                message: format!("Invalid qualified name: '{}' cannot start with a sigil", name),
                location: 0,
            });
        }

        // Each segment must be a valid identifier
        // Reject leading ::, trailing ::, or double ::
        let parts: Vec<&str> = name.split("::").collect();
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                return Err(ParseError::SyntaxError {
                    message: format!(
                        "Invalid qualified name: '{}' (contains empty segment at position {})",
                        name, i
                    ),
                    location: 0,
                });
            }
            if !Self::is_valid_identifier_part(part) {
                return Err(ParseError::SyntaxError {
                    message: format!(
                        "Invalid qualified name: '{}' contains invalid segment '{}'",
                        name, part
                    ),
                    location: 0,
                });
            }
        }

        Ok(())
    }

    /// Checks if a string is a valid Perl identifier component (no sigil, no ::).
    ///
    /// Perl allows Unicode identifiers (e.g., `$π`, `Müller::Util`), so we use
    /// `is_alphabetic`/`is_alphanumeric` rather than ASCII-only checks.
    fn is_valid_identifier_part(s: &str) -> bool {
        let mut chars = s.chars();
        match chars.next() {
            Some(c) if c.is_alphabetic() || c == '_' => {
                chars.all(|c| c.is_alphanumeric() || c == '_')
            }
            _ => false,
        }
    }

    /// Extracts the sigil from a Perl identifier, if present.
    fn extract_sigil(name: &str) -> Option<char> {
        let first_char = name.chars().next()?;
        if matches!(first_char, '$' | '@' | '%' | '&' | '*') { Some(first_char) } else { None }
    }

    fn validate_file_exists(&self, path: &Path) -> ParseResult<()> {
        if !path.exists() {
            return Err(ParseError::SyntaxError {
                message: format!("File does not exist: {}", path.display()),
                location: 0,
            });
        }
        if !path.is_file() {
            return Err(ParseError::SyntaxError {
                message: format!("Path is not a file: {}", path.display()),
                location: 0,
            });
        }
        Ok(())
    }

    fn validate_directory_exists(&self, path: &Path) -> ParseResult<()> {
        if !path.exists() {
            return Err(ParseError::SyntaxError {
                message: format!("Directory does not exist: {}", path.display()),
                location: 0,
            });
        }
        if !path.is_dir() {
            return Err(ParseError::SyntaxError {
                message: format!("Path is not a directory: {}", path.display()),
                location: 0,
            });
        }
        Ok(())
    }

    fn create_backup(&self, files: &[PathBuf], operation_id: &str) -> ParseResult<BackupInfo> {
        let backup_dir = self.backup_root().join(operation_id);

        if !backup_dir.exists() {
            std::fs::create_dir_all(&backup_dir).map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to create backup directory: {}", e),
                location: 0,
            })?;
        }

        let mut file_mappings = HashMap::new();

        for (i, file) in files.iter().enumerate() {
            if file.exists() {
                // Use index and extension to create a unique, safe filename
                let extension = file.extension().and_then(|s| s.to_str()).unwrap_or("");
                let backup_filename = if extension.is_empty() {
                    format!("file_{}", i)
                } else {
                    format!("file_{}.{}", i, extension)
                };

                let backup_path = backup_dir.join(backup_filename);

                std::fs::copy(file, &backup_path).map_err(|e| ParseError::SyntaxError {
                    message: format!("Failed to create backup for {}: {}", file.display(), e),
                    location: 0,
                })?;

                file_mappings.insert(file.clone(), backup_path);
            }
        }

        Ok(BackupInfo { backup_dir, file_mappings })
    }

    /// Returns the backup root directory, using the configured path or the default temp location.
    fn backup_root(&self) -> PathBuf {
        self.config
            .backup_root
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("perl_refactor_backups"))
    }

    fn cleanup_backup_directories(&self) -> ParseResult<BackupCleanupResult> {
        let backup_root = self.backup_root();

        if !backup_root.exists() {
            return Ok(BackupCleanupResult { directories_removed: 0, space_reclaimed: 0 });
        }

        // Collect all backup directories with metadata
        let mut backup_dirs = self.collect_backup_directories(&backup_root)?;

        // Apply retention policies
        let dirs_to_remove = self.apply_retention_policies(&mut backup_dirs)?;

        // Remove selected backup directories and calculate space reclaimed
        let (directories_removed, space_reclaimed) =
            self.remove_backup_directories(&dirs_to_remove)?;

        Ok(BackupCleanupResult { directories_removed, space_reclaimed })
    }

    fn collect_backup_directories(
        &self,
        backup_root: &PathBuf,
    ) -> ParseResult<Vec<BackupDirMetadata>> {
        let mut backup_dirs = Vec::new();

        let entries = std::fs::read_dir(backup_root).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to read backup directory: {}", e),
            location: 0,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to read directory entry: {}", e),
                location: 0,
            })?;

            let path = entry.path();
            if path.is_dir() {
                // Validate backup directory structure
                if self.validate_backup_directory(&path)? {
                    let metadata =
                        std::fs::metadata(&path).map_err(|e| ParseError::SyntaxError {
                            message: format!(
                                "Failed to read metadata for {}: {}",
                                path.display(),
                                e
                            ),
                            location: 0,
                        })?;

                    let modified = metadata.modified().map_err(|e| ParseError::SyntaxError {
                        message: format!(
                            "Failed to get modification time for {}: {}",
                            path.display(),
                            e
                        ),
                        location: 0,
                    })?;

                    let size = self.calculate_directory_size(&path)?;

                    backup_dirs.push(BackupDirMetadata { path, modified, size });
                }
            }
        }

        Ok(backup_dirs)
    }

    fn validate_backup_directory(&self, dir: &PathBuf) -> ParseResult<bool> {
        // Check if directory name starts with "refactor_" (expected pattern)
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !dir_name.starts_with("refactor_") {
            return Ok(false);
        }

        // Ensure it's a directory and not a symlink (security check)
        let metadata = std::fs::symlink_metadata(dir).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to read symlink metadata for {}: {}", dir.display(), e),
            location: 0,
        })?;

        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Ok(false);
        }

        Ok(true)
    }

    fn calculate_directory_size(&self, dir: &PathBuf) -> ParseResult<u64> {
        let mut total_size = 0u64;

        let entries = std::fs::read_dir(dir).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to read directory {}: {}", dir.display(), e),
            location: 0,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to read entry: {}", e),
                location: 0,
            })?;

            let metadata = entry.metadata().map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to read entry metadata: {}", e),
                location: 0,
            })?;

            if metadata.is_file() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    fn apply_retention_policies(
        &self,
        backup_dirs: &mut Vec<BackupDirMetadata>,
    ) -> ParseResult<Vec<PathBuf>> {
        let mut dirs_to_remove = Vec::new();

        // Sort by modification time (oldest first)
        backup_dirs.sort_by_key(|d| d.modified);

        let now = std::time::SystemTime::now();

        // Apply age-based retention policy
        if self.config.backup_max_age_seconds > 0 {
            let max_age = std::time::Duration::from_secs(self.config.backup_max_age_seconds);

            backup_dirs.retain(|dir| {
                if let Ok(age) = now.duration_since(dir.modified) {
                    if age > max_age {
                        dirs_to_remove.push(dir.path.clone());
                        return false;
                    }
                }
                true
            });
        }

        // Apply count-based retention policy
        // max_backup_retention = 0 means "remove all", > 0 means "keep at most N"
        if self.config.max_backup_retention == 0 {
            // Remove all remaining backups
            for dir in backup_dirs.iter() {
                if !dirs_to_remove.contains(&dir.path) {
                    dirs_to_remove.push(dir.path.clone());
                }
            }
        } else if backup_dirs.len() > self.config.max_backup_retention {
            let excess_count = backup_dirs.len() - self.config.max_backup_retention;
            for dir in backup_dirs.iter().take(excess_count) {
                if !dirs_to_remove.contains(&dir.path) {
                    dirs_to_remove.push(dir.path.clone());
                }
            }
        }

        Ok(dirs_to_remove)
    }

    fn remove_backup_directories(&self, dirs_to_remove: &[PathBuf]) -> ParseResult<(usize, u64)> {
        let mut directories_removed = 0;
        let mut space_reclaimed = 0u64;

        for dir in dirs_to_remove {
            let size = self.calculate_directory_size(dir)?;

            std::fs::remove_dir_all(dir).map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to remove backup directory {}: {}", dir.display(), e),
                location: 0,
            })?;

            directories_removed += 1;
            space_reclaimed += size;
        }

        Ok((directories_removed, space_reclaimed))
    }

    fn perform_symbol_rename(
        &mut self,
        old_name: &str,
        new_name: &str,
        scope: &RefactoringScope,
    ) -> ParseResult<RefactoringResult> {
        #[cfg(feature = "workspace_refactor")]
        {
            let rename_result = match scope {
                RefactoringScope::Workspace
                | RefactoringScope::File(_)
                | RefactoringScope::Directory(_)
                | RefactoringScope::FileSet(_)
                | RefactoringScope::Package { .. }
                | RefactoringScope::Function { .. }
                | RefactoringScope::Block { .. } => {
                    // For workspace scope or any other scope, we use rename_symbol
                    // The underlying WorkspaceRefactor uses the WorkspaceIndex to find all occurrences
                    // based on the symbol key (pkg + name + sigil).
                    let target_file = match scope {
                        RefactoringScope::File(path) => path,
                        RefactoringScope::Package { file, .. } => file,
                        RefactoringScope::Function { file, .. } => file,
                        RefactoringScope::Block { file, .. } => file,
                        _ => Path::new(""),
                    };

                    self.workspace_refactor.rename_symbol(old_name, new_name, target_file, (0, 0))
                }
            };

            match rename_result {
                Ok(result) => {
                    let files_modified = self.apply_file_edits(&result.file_edits)?;
                    let changes_made = result.file_edits.iter().map(|e| e.edits.len()).sum();
                    println!(
                        "perform_symbol_rename DEBUG: result.success=true, files_modified={}, changes_made={}",
                        files_modified, changes_made
                    );

                    let refac_result = RefactoringResult {
                        success: true,
                        files_modified,
                        changes_made,
                        warnings: vec![],
                        errors: vec![],
                        operation_id: None,
                    };
                    println!("perform_symbol_rename DEBUG: returning result: {:?}", refac_result);
                    Ok(refac_result)
                }
                Err(e) => Ok(RefactoringResult {
                    success: false,
                    files_modified: 0,
                    changes_made: 0,
                    warnings: vec![],
                    errors: vec![format!("Rename failed: {}", e)],
                    operation_id: None,
                }),
            }
        }

        #[cfg(not(feature = "workspace_refactor"))]
        {
            Ok(RefactoringResult {
                success: false,
                files_modified: 0,
                changes_made: 0,
                warnings: vec!["Workspace refactoring feature disabled".to_string()],
                errors: vec![],
                operation_id: None,
            })
        }
    }

    fn perform_extract_method(
        &mut self,
        method_name: &str,
        start_position: (usize, usize),
        end_position: (usize, usize),
        files: &[PathBuf],
    ) -> ParseResult<RefactoringResult> {
        let file_path = if let Some(f) = files.first() {
            f
        } else {
            return Err(ParseError::SyntaxError {
                message: "No file specified for extraction".to_string(),
                location: 0,
            });
        };

        let source_code = std::fs::read_to_string(file_path).map_err(|e| {
            ParseError::SyntaxError { message: format!("Failed to read file: {}", e), location: 0 }
        })?;

        let line_ending = if source_code.contains("\r\n") { "\r\n" } else { "\n" };

        // Calculate offsets
        let line_index = LineIndex::new(source_code.clone());
        let start_offset = line_index
            .position_to_offset(start_position.0 as u32, start_position.1 as u32)
            .ok_or_else(|| ParseError::SyntaxError {
                message: "Invalid start position".to_string(),
                location: 0,
            })?;
        let end_offset = line_index
            .position_to_offset(end_position.0 as u32, end_position.1 as u32)
            .ok_or_else(|| ParseError::SyntaxError {
                message: "Invalid end position".to_string(),
                location: 0,
            })?;

        if start_offset >= end_offset {
            return Err(ParseError::SyntaxError {
                message: "Start position must be before end position".to_string(),
                location: 0,
            });
        }

        // Parse
        let mut parser = Parser::new(&source_code);
        let ast = parser.parse()?;

        // Analyze variables
        let analysis = analyze_extraction(&ast, start_offset, end_offset);

        // Generate Code
        let extracted_code = &source_code[start_offset..end_offset];

        let mut new_sub = format!(
            "{}# Extracted from lines {}-{} {}sub {} {{{}",
            line_ending,
            start_position.0 + 1,
            end_position.0, // end position is exclusive in display usually if it's (line, 0)
            line_ending,
            method_name,
            line_ending
        );

        // Handle inputs
        if !analysis.inputs.is_empty() {
            new_sub.push_str(
                &format!("    my ({}) = @_;\n", analysis.inputs.join(", "))
                    .replace('\n', line_ending),
            );
        }

        // Body
        new_sub.push_str("    ");
        new_sub.push_str(extracted_code.trim());
        new_sub.push_str(line_ending);

        // Handle outputs
        if !analysis.outputs.is_empty() {
            new_sub.push_str(
                &format!("    return ({});\n", analysis.outputs.join(", "))
                    .replace('\n', line_ending),
            );
        }
        new_sub.push_str("}\n".replace('\n', line_ending).as_str());

        // Identify indentation for the call site
        let mut indentation = String::new();
        if let Some(first_line) = extracted_code.lines().find(|l| !l.trim().is_empty()) {
            let trimmed = first_line.trim_start();
            indentation = first_line[..first_line.len() - trimmed.len()].to_string();
        } else if let Some(line_start) = source_code[..start_offset].rfind('\n') {
            let prefix = &source_code[line_start + 1..start_offset];
            if prefix.trim().is_empty() {
                indentation = prefix.to_string();
            }
        }

        // Generate Call
        let inputs_str = analysis.inputs.join(", ");
        let mut call = format!("{}({})", method_name, inputs_str);

        if !analysis.outputs.is_empty() {
            let outputs_str = analysis.outputs.join(", ");
            call = format!("({}) = {}", outputs_str, call);
        }
        call.push(';');

        // Add indentation and newline if appropriate
        let mut call_with_indent = format!("{}{}", indentation, call);
        if source_code[start_offset..end_offset].ends_with('\n') {
            call_with_indent.push_str(line_ending);
        }

        // Apply changes
        let mut final_source = String::new();
        let prefix_len =
            if source_code[..start_offset].ends_with(&indentation) { indentation.len() } else { 0 };
        final_source.push_str(&source_code[..start_offset - prefix_len]);
        final_source.push_str(&call_with_indent);
        final_source.push_str(&source_code[end_offset..]);

        // Find smart placement for the new subroutine
        let insert_pos = if let Some(idx) = final_source.rfind(&format!("{}1;", line_ending)) {
            // Place before the final 1;
            idx + line_ending.len()
        } else if let Some(idx) = final_source.rfind(&format!("{}__DATA__", line_ending)) {
            idx + line_ending.len()
        } else if let Some(idx) = final_source.rfind(&format!("{}__END__", line_ending)) {
            idx + line_ending.len()
        } else {
            final_source.len()
        };

        final_source.insert_str(insert_pos, &new_sub);

        if !self.config.safe_mode {
            std::fs::write(file_path, final_source).map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to write file: {}", e),
                location: 0,
            })?;
        }

        Ok(RefactoringResult {
            success: true,
            files_modified: 1,
            changes_made: 2, // call + sub
            warnings: vec![],
            errors: vec![],
            operation_id: None,
        })
    }

    fn perform_move_code(
        &mut self,
        source_file: &Path,
        target_file: &Path,
        elements: &[String],
    ) -> ParseResult<RefactoringResult> {
        // Validate that source and target are different files
        let source_path = fs::canonicalize(source_file).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to resolve source path: {}", e),
            location: 0,
        })?;
        let target_path = fs::canonicalize(target_file).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to resolve target path: {}", e),
            location: 0,
        })?;

        if source_path == target_path {
            return Err(ParseError::SyntaxError {
                message: "Source and target files must be different".to_string(),
                location: 0,
            });
        }

        // Read files first to prevent partial failure/data loss
        let source_content =
            fs::read_to_string(&source_path).map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to read source file: {}", e),
                location: 0,
            })?;

        let mut target_content =
            fs::read_to_string(&target_path).map_err(|e| ParseError::SyntaxError {
                message: format!("Failed to read target file: {}", e),
                location: 0,
            })?;

        // Parse source file to find elements
        let mut parser = Parser::new(&source_content);
        let ast = parser.parse().map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to parse source file: {}", e),
            location: 0,
        })?;

        // Store location AND content for each element
        struct ElementToMove {
            location: SourceLocation,
            content: String,
        }

        let mut elements_to_move: Vec<ElementToMove> = Vec::new();
        let mut warnings = Vec::new();

        // Find elements in the AST
        let mut found_names: HashSet<String> = HashSet::new();
        ast.for_each_child(|child| {
            if let NodeKind::Subroutine { name, .. } = &child.kind {
                if let Some(sub_name) = name {
                    if elements.contains(sub_name) {
                        found_names.insert(sub_name.clone());
                        elements_to_move.push(ElementToMove {
                            location: child.location,
                            content: source_content[child.location.start..child.location.end]
                                .to_string(),
                        });
                    }
                }
            }
        });

        // Warn about elements that weren't found
        for element in elements {
            if !found_names.contains(element) {
                warnings.push(format!("Subroutine '{}' not found in source file", element));
            }
        }

        if elements_to_move.is_empty() {
            return Ok(RefactoringResult {
                success: false,
                files_modified: 0,
                changes_made: 0,
                warnings: vec!["No elements found to move".to_string()],
                errors: vec![],
                operation_id: None,
            });
        }

        // Sort by start position descending for safe removal from source
        elements_to_move.sort_by(|a, b| b.location.start.cmp(&a.location.start));

        let mut modified_source = source_content.clone();

        // Remove from source (in descending order)
        for element in &elements_to_move {
            let start = element.location.start;
            let end = element.location.end;

            // Check for trailing newline to remove
            let remove_end =
                if end < modified_source.len() && modified_source.as_bytes()[end] == b'\n' {
                    end + 1
                } else {
                    end
                };

            modified_source.replace_range(start..remove_end, "");
        }

        // Sort by start position ascending for correct append order
        elements_to_move.sort_by(|a, b| a.location.start.cmp(&b.location.start));

        // Construct moved content
        let mut moved_content = String::new();
        for element in &elements_to_move {
            moved_content.push_str(&element.content);
            moved_content.push('\n');
        }

        // Calculate insertion point in target
        let insertion_index = if let Some(idx) = target_content.rfind("\n1;") {
            idx + 1 // Insert after the newline, before 1;
        } else if let Some(idx) = target_content.rfind("\nreturn 1;") {
            idx + 1
        } else {
            target_content.len()
        };

        if insertion_index < target_content.len() {
            // moved_content already ends with newline from loop above
            target_content.insert_str(insertion_index, &moved_content);
        } else {
            target_content.push('\n');
            target_content.push_str(&moved_content);
        }

        // Write files - Target first, then Source (safer)
        fs::write(&target_path, target_content).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to write to target file: {}", e),
            location: 0,
        })?;

        fs::write(&source_path, modified_source).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to write source file: {}", e),
            location: 0,
        })?;

        // Add warning about missing dependency analysis
        warnings.push("Warning: Imports and references were not updated. Please review the moved code for missing dependencies.".to_string());

        Ok(RefactoringResult {
            success: true,
            files_modified: 2,
            changes_made: elements_to_move.len(),
            warnings,
            errors: vec![],
            operation_id: None,
        })
    }

    fn perform_modernize(
        &mut self,
        patterns: &[ModernizationPattern],
        files: &[PathBuf],
    ) -> ParseResult<RefactoringResult> {
        // Delegate to modernize engine
        let mut total_changes = 0;
        let mut modified_files = 0;
        let mut warnings = Vec::new();

        for file in files {
            if let Ok(changes) = self.modernize.modernize_file(file, patterns) {
                if changes > 0 {
                    modified_files += 1;
                    total_changes += changes;
                }
            } else {
                warnings.push(format!("Failed to modernize {}", file.display()));
            }
        }

        Ok(RefactoringResult {
            success: true,
            files_modified: modified_files,
            changes_made: total_changes,
            warnings,
            errors: vec![],
            operation_id: None,
        })
    }

    fn perform_optimize_imports(
        &mut self,
        remove_unused: bool,
        sort_alphabetically: bool,
        group_by_type: bool,
        files: &[PathBuf],
    ) -> ParseResult<RefactoringResult> {
        // Delegate to import optimizer
        let mut total_changes = 0;
        let mut modified_files = 0;

        for file in files {
            let analysis = self
                .import_optimizer
                .analyze_file(file)
                .map_err(|e| ParseError::SyntaxError { message: e, location: 0 })?;
            let mut changes_made = 0;

            if remove_unused && !analysis.unused_imports.is_empty() {
                changes_made += analysis.unused_imports.len();
            }

            if sort_alphabetically {
                changes_made += 1; // Count sorting as one change per file
            }

            if group_by_type {
                changes_made += 1; // Count grouping as one change per file
            }

            if changes_made > 0 {
                modified_files += 1;
                total_changes += changes_made;
            }
        }

        Ok(RefactoringResult {
            success: true,
            files_modified: modified_files,
            changes_made: total_changes,
            warnings: vec![],
            errors: vec![],
            operation_id: None,
        })
    }

    fn perform_inline(
        &mut self,
        symbol_name: &str,
        all_occurrences: bool, // AC1: Implement multi-file occurrence inlining
        files: &[PathBuf],
    ) -> ParseResult<RefactoringResult> {
        let mut warnings = Vec::new();

        // Variable inlining - only supported for variables with sigils
        if symbol_name.starts_with('$')
            || symbol_name.starts_with('@')
            || symbol_name.starts_with('%')
        {
            #[cfg(feature = "workspace_refactor")]
            {
                let mut files_modified = 0;
                let mut changes_made = 0;
                let mut applied = false;

                for file in files {
                    // Try to find and inline variable in this file
                    match self.workspace_refactor.inline_variable(symbol_name, file, (0, 0)) {
                        Ok(refactor_result) => {
                            let edits = refactor_result.file_edits;
                            if !edits.is_empty() {
                                let mod_count = self.apply_file_edits(&edits)?;
                                if mod_count > 0 {
                                    files_modified += mod_count;
                                    changes_made +=
                                        edits.iter().map(|e| e.edits.len()).sum::<usize>();
                                    applied = true;
                                    // AC1: If not all_occurrences, stop after first successful inlining
                                    if !all_occurrences {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(crate::workspace_refactor::RefactorError::SymbolNotFound {
                            ..
                        }) => continue,
                        Err(e) => {
                            warnings.push(format!("Error checking {}: {}", file.display(), e));
                        }
                    }
                }

                if !applied && warnings.is_empty() {
                    warnings.push(format!(
                        "Symbol '{}' definition not found in provided files",
                        symbol_name
                    ));
                }

                return Ok(RefactoringResult {
                    success: applied,
                    files_modified,
                    changes_made,
                    warnings,
                    errors: vec![],
                    operation_id: None,
                });
            }

            #[cfg(not(feature = "workspace_refactor"))]
            {
                let _ = files; // Acknowledge parameter when feature is disabled
                warnings.push("Workspace refactoring feature is disabled".to_string());
            }
        } else {
            let _ = files; // Acknowledge parameter for non-variable symbols
            warnings.push(format!(
                "Inlining for symbol '{}' not implemented (only variables supported)",
                symbol_name
            ));
        }

        Ok(RefactoringResult {
            success: false,
            files_modified: 0,
            changes_made: 0,
            warnings,
            errors: vec![],
            operation_id: None,
        })
    }

    #[cfg(feature = "workspace_refactor")]
    fn apply_file_edits(
        &self,
        file_edits: &[crate::workspace_refactor::FileEdit],
    ) -> ParseResult<usize> {
        let mut files_modified = 0;

        for file_edit in file_edits {
            if !file_edit.file_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&file_edit.file_path).map_err(|e| {
                ParseError::SyntaxError {
                    message: format!(
                        "Failed to read file {}: {}",
                        file_edit.file_path.display(),
                        e
                    ),
                    location: 0,
                }
            })?;

            // Clone and sort edits by start position in descending order to apply them safely
            // (applying from end to start preserves earlier byte positions)
            let mut edits = file_edit.edits.clone();
            edits.sort_by(|a, b| b.start.cmp(&a.start));

            // Clone content for comparison after modifications
            let mut new_content = content.clone();
            for edit in edits {
                if edit.end > new_content.len() {
                    return Err(ParseError::SyntaxError {
                        message: format!(
                            "Edit out of bounds for {}: range {}..{} in content len {}",
                            file_edit.file_path.display(),
                            edit.start,
                            edit.end,
                            new_content.len()
                        ),
                        location: 0,
                    });
                }
                new_content.replace_range(edit.start..edit.end, &edit.new_text);
            }

            if new_content != content {
                std::fs::write(&file_edit.file_path, new_content).map_err(|e| {
                    ParseError::SyntaxError {
                        message: format!(
                            "Failed to write file {}: {}",
                            file_edit.file_path.display(),
                            e
                        ),
                        location: 0,
                    }
                })?;
                files_modified += 1;
            }
        }

        Ok(files_modified)
    }
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Temporary stub implementations for missing dependencies
mod temp_stubs {
    use super::*;

    #[allow(dead_code)]
    #[derive(Debug)]
    /// Workspace refactor stub used when the `workspace_refactor` feature is disabled.
    pub(super) struct WorkspaceRefactor;
    #[allow(dead_code)]
    impl WorkspaceRefactor {
        /// Create a new stub workspace refactor instance.
        pub(super) fn new() -> Self {
            Self
        }
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    /// Modernization engine stub used when the `modernize` feature is disabled.
    pub(super) struct ModernizeEngine;
    #[allow(dead_code)]
    impl ModernizeEngine {
        /// Create a new stub modernizer instance.
        pub(super) fn new() -> Self {
            Self
        }

        /// Placeholder modernization hook that reports no changes.
        pub(super) fn modernize_file(
            &mut self,
            _file: &Path,
            _patterns: &[ModernizationPattern],
        ) -> ParseResult<usize> {
            Ok(0)
        }
    }
}

struct ExtractionAnalysis {
    inputs: Vec<String>,
    outputs: Vec<String>,
}

fn analyze_extraction(ast: &Node, start: usize, end: usize) -> ExtractionAnalysis {
    let mut inputs = HashSet::new();
    let mut outputs = HashSet::new();
    let mut declared_in_scope = HashSet::new();
    let mut declared_in_range = HashSet::new();

    visit_node(
        ast,
        start,
        end,
        &mut inputs,
        &mut outputs,
        &mut declared_in_scope,
        &mut declared_in_range,
    );

    let mut inputs_vec: Vec<_> = inputs.into_iter().collect();
    inputs_vec.sort();
    let mut outputs_vec: Vec<_> = outputs.into_iter().collect();
    outputs_vec.sort();

    ExtractionAnalysis { inputs: inputs_vec, outputs: outputs_vec }
}

fn visit_node(
    node: &Node,
    start: usize,
    end: usize,
    inputs: &mut HashSet<String>,
    outputs: &mut HashSet<String>,
    declared_in_scope: &mut HashSet<String>,
    declared_in_range: &mut HashSet<String>,
) {
    let in_range = node.location.start >= start && node.location.end <= end;

    match &node.kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            if declarator == "my" || declarator == "state" {
                let name = extract_var_name(variable);
                if in_range {
                    declared_in_range.insert(name);
                } else {
                    declared_in_scope.insert(name);
                }
            }
            if let Some(init) = initializer {
                visit_node(init, start, end, inputs, outputs, declared_in_scope, declared_in_range);
            }
        }
        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            if declarator == "my" || declarator == "state" {
                for var in variables {
                    let name = extract_var_name(var);
                    if in_range {
                        declared_in_range.insert(name);
                    } else {
                        declared_in_scope.insert(name);
                    }
                }
            }
            if let Some(init) = initializer {
                visit_node(init, start, end, inputs, outputs, declared_in_scope, declared_in_range);
            }
        }
        NodeKind::MandatoryParameter { variable }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => {
            let name = extract_var_name(variable);
            if in_range {
                declared_in_range.insert(name);
            } else {
                declared_in_scope.insert(name);
            }
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            let name = extract_var_name(variable);
            if in_range {
                declared_in_range.insert(name);
            } else {
                declared_in_scope.insert(name);
            }
            visit_node(
                default_value,
                start,
                end,
                inputs,
                outputs,
                declared_in_scope,
                declared_in_range,
            );
        }
        NodeKind::Variable { sigil, name } => {
            let full_name = format!("{}{}", sigil, name);
            if in_range {
                // If not declared in range, check if declared in outer scope.
                if !declared_in_range.contains(&full_name) && declared_in_scope.contains(&full_name)
                {
                    inputs.insert(full_name.clone());
                }
            } else if node.location.start >= end {
                // Usage after range
                // If declared in range OR used in range (input), it might have changed and is used after.
                if declared_in_range.contains(&full_name) || inputs.contains(&full_name) {
                    outputs.insert(full_name);
                }
            }
        }
        NodeKind::Block { statements } => {
            let mut inner_scope = declared_in_scope.clone();
            for stmt in statements {
                visit_node(stmt, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
        }
        NodeKind::Subroutine { signature, body, .. } => {
            let mut inner_scope = declared_in_scope.clone();
            if let Some(sig) = signature {
                visit_node(sig, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
            visit_node(body, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            visit_node(body, start, end, inputs, outputs, declared_in_scope, declared_in_range);
            for (var, catch_body) in catch_blocks {
                let mut inner_scope = declared_in_scope.clone();
                if let Some(v_name) = var {
                    // Check if v_name has sigil, if not assume $
                    let full_name = if v_name.starts_with(['$', '@', '%']) {
                        v_name.clone()
                    } else {
                        format!("${}", v_name)
                    };
                    if in_range {
                        declared_in_range.insert(full_name);
                    } else {
                        declared_in_scope.insert(full_name);
                    }
                }
                visit_node(
                    catch_body,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
            }
            if let Some(finally) = finally_block {
                visit_node(
                    finally,
                    start,
                    end,
                    inputs,
                    outputs,
                    declared_in_scope,
                    declared_in_range,
                );
            }
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            // Visit list with outer scope
            visit_node(list, start, end, inputs, outputs, declared_in_scope, declared_in_range);

            // Visit continue block if present
            if let Some(cb) = continue_block {
                visit_node(cb, start, end, inputs, outputs, declared_in_scope, declared_in_range);
            }

            // Create inner scope for variable and body
            let mut inner_scope = declared_in_scope.clone();
            visit_node(variable, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            visit_node(body, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            let mut inner_scope = declared_in_scope.clone();
            if let Some(n) = init {
                visit_node(n, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
            if let Some(n) = condition {
                visit_node(n, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
            if let Some(n) = update {
                visit_node(n, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
            visit_node(body, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            if let Some(n) = continue_block {
                visit_node(n, start, end, inputs, outputs, &mut inner_scope, declared_in_range);
            }
        }
        _ => {
            for child in node.children() {
                visit_node(
                    child,
                    start,
                    end,
                    inputs,
                    outputs,
                    declared_in_scope,
                    declared_in_range,
                );
            }
        }
    }
}

fn extract_var_name(node: &Node) -> String {
    match &node.kind {
        NodeKind::Variable { sigil, name } => format!("{}{}", sigil, name),
        NodeKind::VariableWithAttributes { variable, .. } => extract_var_name(variable),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_operation_id_generation() {
        let engine = RefactoringEngine::new();
        let id1 = engine.generate_operation_id();
        let id2 = engine.generate_operation_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("refactor_"));
    }

    #[test]
    fn test_config_defaults() {
        let config = RefactoringConfig::default();
        assert!(config.safe_mode);
        assert_eq!(config.max_files_per_operation, 100);
        assert!(config.create_backups);
        assert_eq!(config.operation_timeout, 60);
        assert!(config.parallel_processing);
    }

    #[test]
    fn test_extract_method_basic() {
        use std::io::Write;
        let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
        let code = r#"
sub test {
    my $x = 1;
    my $y = 2;
    # Start extraction
    print $x;
    my $z = $x + $y;
    print $z;
    # End extraction
    return $z;
}
"#;
        must(write!(file, "{}", code));
        let path = file.path().to_path_buf();

        let mut engine = RefactoringEngine::new();
        engine.config.safe_mode = false;

        // Lines are 0-indexed.
        // Line 5: "    print $x;\n"
        // Line 8: "    # End extraction\n"
        let result = must(engine.perform_extract_method(
            "extracted_sub",
            (5, 0),
            (8, 0),
            std::slice::from_ref(&path),
        ));

        assert!(result.success);

        let new_code = must(std::fs::read_to_string(&path));
        println!("New code:\n{}", new_code);

        // Inputs: $x, $y (used in range, declared before)
        // Outputs: $z (declared in range, used after)

        assert!(new_code.contains("sub extracted_sub {"));
        assert!(new_code.contains("my ($x, $y) = @_;"));
        assert!(new_code.contains("return ($z);"));
        // Call verification order depends on how we generate it
        assert!(new_code.contains("($z) = extracted_sub($x, $y);"));
    }

    #[test]
    fn test_extract_method_with_placement() {
        use std::io::Write;
        let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
        let code = r#"
package MyModule;
use strict;
use warnings;

sub existing {
    my $val = 10;
    # start
    print $val;
    my $new_val = $val * 2;
    # end
    return $new_val;
}

1;
"#;
        must(write!(file, "{}", code));
        let path = file.path().to_path_buf();

        let mut engine = RefactoringEngine::new();
        engine.config.safe_mode = false;

        // selection should include lines 8 and 9 (0-indexed)
        // Line 8: "    print $val;\n"
        // Line 9: "    my $new_val = $val * 2;\n"
        let result = must(engine.perform_extract_method(
            "helper",
            (8, 0),
            (10, 0),
            std::slice::from_ref(&path),
        ));

        assert!(result.success);

        let new_code = must(std::fs::read_to_string(&path));
        println!("New code with placement:\n{}", new_code);

        // Check placement: helper should be before 1;
        assert!(new_code.contains("sub helper {"));
        assert!(must_some(new_code.find("sub helper {")) < must_some(new_code.find("1;")));

        assert!(new_code.contains("my ($val) = @_;"));
        assert!(new_code.contains("return ($new_val);"));
        assert!(new_code.contains("($new_val) = helper($val);"));
    }

    #[test]
    fn test_extract_method_complex_vars() {
        use std::io::Write;
        let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
        let code = r#"
sub complex {
    my $sum = 0;
    my @items = (1..10);
    # start
    foreach my $item (@items) {
        $sum += $item;
    }
    state $call_count = 0;
    $call_count++;
    # end
    return ($sum, $call_count);
}
"#;
        must(write!(file, "{}", code));
        let path = file.path().to_path_buf();

        let mut engine = RefactoringEngine::new();
        engine.config.safe_mode = false;

        // Line 5: "    foreach my $item (@items) {"
        // Line 10: "    # end"
        let result = must(engine.perform_extract_method(
            "do_math",
            (5, 0),
            (10, 0),
            std::slice::from_ref(&path),
        ));

        assert!(result.success);
        let new_code = must(std::fs::read_to_string(&path));
        println!("New code complex:\n{}", new_code);

        // check if sub created
        assert!(new_code.contains("sub do_math {"));
        // check inputs
        assert!(new_code.contains("my ($sum, @items) = @_;"));
        // check outputs
        assert!(new_code.contains("return ($call_count, $sum);"));
        // check call
        assert!(new_code.contains("($call_count, $sum) = do_math($sum, @items);"));
        // check indentation of call
        assert!(new_code.contains("    ($call_count, $sum) = do_math($sum, @items);"));
    }

    // ============================================================
    // Validation tests for validate_operation
    // ============================================================

    mod validation_tests {
        use super::*;
        use perl_tdd_support::{must, must_err};
        use serial_test::serial;

        // --- Perl identifier validation tests ---

        #[test]
        fn test_validate_identifier_bare_name() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_identifier("foo", "test").is_ok());
            assert!(engine.validate_perl_identifier("_private", "test").is_ok());
            assert!(engine.validate_perl_identifier("CamelCase", "test").is_ok());
            assert!(engine.validate_perl_identifier("name_with_123", "test").is_ok());
        }

        #[test]
        fn test_validate_identifier_with_sigils() {
            let engine = RefactoringEngine::new();
            // All valid Perl sigils should be accepted
            assert!(engine.validate_perl_identifier("$scalar", "test").is_ok());
            assert!(engine.validate_perl_identifier("@array", "test").is_ok());
            assert!(engine.validate_perl_identifier("%hash", "test").is_ok());
            assert!(engine.validate_perl_identifier("&sub", "test").is_ok());
            assert!(engine.validate_perl_identifier("*glob", "test").is_ok());
        }

        #[test]
        fn test_validate_identifier_qualified_names() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_identifier("Package::name", "test").is_ok());
            assert!(engine.validate_perl_identifier("$Package::var", "test").is_ok());
            assert!(engine.validate_perl_identifier("@Deep::Nested::array", "test").is_ok());
            assert!(engine.validate_perl_identifier("::main_package", "test").is_ok());
        }

        #[test]
        fn test_validate_identifier_empty_rejected() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_identifier("", "test").is_err());
        }

        #[test]
        fn test_validate_identifier_sigil_only_rejected() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_identifier("$", "test").is_err());
            assert!(engine.validate_perl_identifier("@", "test").is_err());
            assert!(engine.validate_perl_identifier("%", "test").is_err());
        }

        #[test]
        fn test_validate_identifier_invalid_start_char() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_identifier("123abc", "test").is_err());
            assert!(engine.validate_perl_identifier("$123abc", "test").is_err());
            assert!(engine.validate_perl_identifier("-invalid", "test").is_err());
        }

        // --- Subroutine name validation tests ---

        #[test]
        fn test_validate_subroutine_name_valid() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_subroutine_name("my_sub").is_ok());
            assert!(engine.validate_perl_subroutine_name("_private_sub").is_ok());
            assert!(engine.validate_perl_subroutine_name("&explicit_sub").is_ok());
        }

        #[test]
        fn test_validate_subroutine_name_invalid_sigils() {
            let engine = RefactoringEngine::new();
            // Subs cannot have $, @, %, * sigils
            assert!(engine.validate_perl_subroutine_name("$not_a_sub").is_err());
            assert!(engine.validate_perl_subroutine_name("@not_a_sub").is_err());
            assert!(engine.validate_perl_subroutine_name("%not_a_sub").is_err());
        }

        #[test]
        fn test_validate_subroutine_name_empty() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_subroutine_name("").is_err());
        }

        // --- Qualified name validation tests ---

        #[test]
        fn test_validate_qualified_name_valid() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_qualified_name("Package").is_ok());
            assert!(engine.validate_perl_qualified_name("Package::Sub").is_ok());
            assert!(engine.validate_perl_qualified_name("Deep::Nested::Name").is_ok());
        }

        #[test]
        fn test_validate_qualified_name_empty_rejected() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_qualified_name("").is_err());
            assert!(engine.validate_perl_qualified_name("::").is_err());
        }

        #[test]
        fn test_validate_qualified_name_invalid_segment() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_qualified_name("Package::123invalid").is_err());
        }

        // --- File count limit validation tests ---

        #[test]
        fn test_validate_file_count_limit() {
            let engine = RefactoringEngine::new();
            // Create more files than allowed
            let files: Vec<PathBuf> =
                (0..150).map(|i| PathBuf::from(format!("/fake/{}.pl", i))).collect();

            let op = RefactoringType::OptimizeImports {
                remove_unused: true,
                sort_alphabetically: true,
                group_by_type: false,
            };

            let result = engine.validate_operation(&op, &files);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("exceeds maximum file limit"));
        }

        // --- ExtractMethod validation tests ---

        #[test]
        fn test_extract_method_requires_file() {
            let engine = RefactoringEngine::new();
            let op = RefactoringType::ExtractMethod {
                method_name: "new_method".to_string(),
                start_position: (1, 0),
                end_position: (5, 0),
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("requires a target file"));
        }

        #[test]
        fn test_extract_method_single_file_only() {
            let file1: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
            let file2: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());

            let engine = RefactoringEngine::new();
            let op = RefactoringType::ExtractMethod {
                method_name: "new_method".to_string(),
                start_position: (1, 0),
                end_position: (5, 0),
            };

            let result = engine
                .validate_operation(&op, &[file1.path().to_path_buf(), file2.path().to_path_buf()]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("operates on a single file"));
        }

        #[test]
        fn test_extract_method_invalid_range() {
            let file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());

            let engine = RefactoringEngine::new();
            let op = RefactoringType::ExtractMethod {
                method_name: "new_method".to_string(),
                start_position: (10, 0),
                end_position: (5, 0), // end before start
            };

            let result = engine.validate_operation(&op, &[file.path().to_path_buf()]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("must be before end"));
        }

        #[test]
        fn test_extract_method_invalid_subroutine_name() {
            let file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());

            let engine = RefactoringEngine::new();
            let op = RefactoringType::ExtractMethod {
                method_name: "$invalid".to_string(), // sigil not allowed for sub names
                start_position: (1, 0),
                end_position: (5, 0),
            };

            let result = engine.validate_operation(&op, &[file.path().to_path_buf()]);
            assert!(result.is_err());
        }

        // --- MoveCode validation tests ---

        #[test]
        fn test_move_code_requires_elements() {
            use std::io::Write;
            let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
            must(write!(file, "# source"));

            let engine = RefactoringEngine::new();
            let op = RefactoringType::MoveCode {
                source_file: file.path().to_path_buf(),
                target_file: PathBuf::from("target.pl"),
                elements: vec![], // empty
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("requires at least one element"));
        }

        // --- SymbolRename validation tests ---

        #[test]
        fn test_symbol_rename_accepts_sigils() {
            use std::io::Write;
            let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
            must(write!(file, "my $old = 1;"));

            let engine = RefactoringEngine::new();
            let op = RefactoringType::SymbolRename {
                old_name: "$old_var".to_string(),
                new_name: "$new_var".to_string(),
                scope: RefactoringScope::File(file.path().to_path_buf()),
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_ok());
        }

        #[test]
        fn test_symbol_rename_workspace_scope_no_files_required() {
            let engine = RefactoringEngine::new();
            let op = RefactoringType::SymbolRename {
                old_name: "old_sub".to_string(),
                new_name: "new_sub".to_string(),
                scope: RefactoringScope::Workspace,
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_ok());
        }

        #[test]
        fn test_symbol_rename_fileset_requires_files() {
            let engine = RefactoringEngine::new();
            let op = RefactoringType::SymbolRename {
                old_name: "old_sub".to_string(),
                new_name: "new_sub".to_string(),
                scope: RefactoringScope::FileSet(vec![]), // empty
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("requires at least one file"));
        }

        // --- Inline validation tests ---

        #[test]
        fn test_inline_requires_files() {
            let engine = RefactoringEngine::new();
            let op =
                RefactoringType::Inline { symbol_name: "$var".to_string(), all_occurrences: true };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("requires at least one target file"));
        }

        // --- Modernize validation tests ---

        #[test]
        fn test_modernize_requires_patterns() {
            let engine = RefactoringEngine::new();
            let op = RefactoringType::Modernize { patterns: vec![] };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("requires at least one pattern"));
        }

        // --- Sigil consistency tests ---

        #[test]
        fn test_symbol_rename_sigil_consistency_required() {
            let engine = RefactoringEngine::new();
            // $foo -> @foo should fail (different sigils)
            let op = RefactoringType::SymbolRename {
                old_name: "$foo".to_string(),
                new_name: "@foo".to_string(),
                scope: RefactoringScope::Workspace,
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("sigil mismatch"));
        }

        #[test]
        fn test_symbol_rename_sigil_consistency_no_sigil_to_sigil() {
            let engine = RefactoringEngine::new();
            // bare name -> sigiled name should fail
            let op = RefactoringType::SymbolRename {
                old_name: "foo".to_string(),
                new_name: "$foo".to_string(),
                scope: RefactoringScope::Workspace,
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("sigil mismatch"));
        }

        #[test]
        fn test_symbol_rename_same_name_rejected() {
            let engine = RefactoringEngine::new();
            let op = RefactoringType::SymbolRename {
                old_name: "$foo".to_string(),
                new_name: "$foo".to_string(),
                scope: RefactoringScope::Workspace,
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("must be different"));
        }

        // --- Double separator and trailing :: tests ---

        #[test]
        fn test_validate_identifier_double_separator_rejected() {
            let engine = RefactoringEngine::new();
            // Double :: should be rejected
            assert!(engine.validate_perl_identifier("Foo::::Bar", "test").is_err());
            assert!(engine.validate_perl_identifier("$Foo::::Bar", "test").is_err());
        }

        #[test]
        fn test_validate_identifier_trailing_separator_rejected() {
            let engine = RefactoringEngine::new();
            // Trailing :: should be rejected
            assert!(engine.validate_perl_identifier("Foo::", "test").is_err());
            assert!(engine.validate_perl_identifier("$Foo::Bar::", "test").is_err());
        }

        #[test]
        fn test_validate_identifier_leading_separator_allowed() {
            let engine = RefactoringEngine::new();
            // Leading :: should be allowed (for main package/absolute names)
            assert!(engine.validate_perl_identifier("::Foo", "test").is_ok());
            assert!(engine.validate_perl_identifier("::Foo::Bar", "test").is_ok());
            assert!(engine.validate_perl_identifier("$::Foo", "test").is_ok());
        }

        #[test]
        fn test_validate_qualified_name_double_separator_rejected() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_qualified_name("Foo::::Bar").is_err());
        }

        #[test]
        fn test_validate_qualified_name_trailing_separator_rejected() {
            let engine = RefactoringEngine::new();
            assert!(engine.validate_perl_qualified_name("Foo::").is_err());
            assert!(engine.validate_perl_qualified_name("Foo::Bar::").is_err());
        }

        #[test]
        fn test_validate_qualified_name_leading_separator_rejected() {
            let engine = RefactoringEngine::new();
            // For qualified names (MoveCode elements), leading :: is also rejected
            assert!(engine.validate_perl_qualified_name("::Foo").is_err());
        }

        #[test]
        fn test_validate_qualified_name_sigil_rejected() {
            let engine = RefactoringEngine::new();
            // Qualified names (for MoveCode) should not have sigils
            assert!(engine.validate_perl_qualified_name("$foo").is_err());
            assert!(engine.validate_perl_qualified_name("@array").is_err());
        }

        // --- Unicode identifier tests ---

        #[test]
        fn test_validate_identifier_unicode_allowed() {
            let engine = RefactoringEngine::new();
            // Perl supports Unicode identifiers
            assert!(engine.validate_perl_identifier("$π", "test").is_ok());
            assert!(engine.validate_perl_identifier("$αβγ", "test").is_ok());
            assert!(engine.validate_perl_identifier("日本語", "test").is_ok());
        }

        #[test]
        fn test_validate_qualified_name_unicode_allowed() {
            let engine = RefactoringEngine::new();
            // Unicode package names should be allowed
            assert!(engine.validate_perl_qualified_name("Müller").is_ok());
            assert!(engine.validate_perl_qualified_name("Müller::Util").is_ok());
            assert!(engine.validate_perl_qualified_name("日本::パッケージ").is_ok());
        }

        // --- ExtractMethod '&' prefix tests ---

        #[test]
        fn test_extract_method_ampersand_prefix_rejected() {
            let file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());

            let engine = RefactoringEngine::new();
            let op = RefactoringType::ExtractMethod {
                method_name: "&foo".to_string(), // leading & should be rejected
                start_position: (1, 0),
                end_position: (5, 0),
            };

            let result = engine.validate_operation(&op, &[file.path().to_path_buf()]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("bare identifier"));
            assert!(err_msg.contains("no leading '&'"));
        }

        // --- MoveCode same-file tests ---

        #[test]
        fn test_move_code_same_file_rejected() {
            use std::io::Write;
            let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
            must(write!(file, "# source"));

            let engine = RefactoringEngine::new();
            let op = RefactoringType::MoveCode {
                source_file: file.path().to_path_buf(),
                target_file: file.path().to_path_buf(), // same as source
                elements: vec!["some_sub".to_string()],
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("must be different"));
        }

        // --- FileSet scope max_files tests ---

        #[test]
        fn test_fileset_scope_max_files_limit() {
            // Create temp files for the test
            let files: Vec<tempfile::NamedTempFile> =
                (0..5).map(|_| must(tempfile::NamedTempFile::new())).collect();
            let paths: Vec<_> = files.iter().map(|f| f.path().to_path_buf()).collect();

            // Create engine with low max_files limit
            let config =
                RefactoringConfig { max_files_per_operation: 3, ..RefactoringConfig::default() };
            let engine = RefactoringEngine::with_config(config);

            let op = RefactoringType::SymbolRename {
                old_name: "old_sub".to_string(),
                new_name: "new_sub".to_string(),
                scope: RefactoringScope::FileSet(paths), // 5 files, but limit is 3
            };

            let result = engine.validate_operation(&op, &[]);
            assert!(result.is_err());
            let err_msg = format!("{:?}", must_err(result));
            assert!(err_msg.contains("exceeds maximum file limit"));
        }

        // --- Backup cleanup tests ---

        #[test]
        fn test_cleanup_no_backups() {
            let mut engine = RefactoringEngine::new();
            let result = must(engine.clear_history());
            assert_eq!(result.directories_removed, 0);
            assert_eq!(result.space_reclaimed, 0);
        }

        #[test]
        #[serial]
        fn test_cleanup_backup_directories() {
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Manually create a backup directory
            let backup = backup_root.join("refactor_100_0");
            must(fs::create_dir_all(&backup));
            must(fs::write(backup.join("file.pl"), "sub test {}"));

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                max_backup_retention: 0, // Remove all
                ..RefactoringConfig::default()
            };
            let mut engine = RefactoringEngine::with_config(config);
            let result = must(engine.clear_history());

            // Should have removed at least one directory
            assert!(result.directories_removed >= 1);
            assert_eq!(engine.operation_history.len(), 0);
        }

        #[test]
        #[serial]
        fn test_cleanup_respects_retention_count() {
            use std::io::Write;

            let config = RefactoringConfig {
                create_backups: true,
                max_backup_retention: 2,
                backup_max_age_seconds: 0, // Disable age-based retention
                ..RefactoringConfig::default()
            };

            let mut engine = RefactoringEngine::with_config(config);

            // Create multiple backups
            for i in 0..4 {
                let mut file: tempfile::NamedTempFile = must(tempfile::NamedTempFile::new());
                must(writeln!(file, "sub test{} {{ }}", i));
                let path = file.path().to_path_buf();

                let op = RefactoringType::SymbolRename {
                    old_name: format!("test{}", i),
                    new_name: format!("renamed_test{}", i),
                    scope: RefactoringScope::File(path.clone()),
                };

                let _ = engine.refactor(op, vec![path]);
                std::thread::sleep(std::time::Duration::from_millis(100)); // Ensure different timestamps
            }

            // Clean up with retention policy
            let result = must(engine.clear_history());

            // Should have removed excess directories (4 created - 2 retained = 2 removed)
            assert!(result.directories_removed >= 2);
        }

        #[test]
        #[serial]
        fn test_cleanup_respects_age_limit() {
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();
            must(fs::create_dir_all(&backup_root));

            // Create an old backup directory manually
            let old_backup = backup_root.join("refactor_1000_0");
            must(fs::create_dir_all(&old_backup));

            // Create a test file in the old backup
            let test_file = old_backup.join("file_0.pl");
            must(fs::write(&test_file, "sub old_backup { }"));

            // Wait until filesystem metadata reports the backup is older than the age threshold.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut reached_age_limit = false;
            while std::time::Instant::now() < deadline {
                if let Ok(metadata) = fs::metadata(&old_backup)
                    && let Ok(modified) = metadata.modified()
                    && let Ok(age) = std::time::SystemTime::now().duration_since(modified)
                    && age > std::time::Duration::from_secs(1)
                {
                    reached_age_limit = true;
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(
                reached_age_limit,
                "backup directory did not age past threshold within test timeout"
            );

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                backup_max_age_seconds: 1, // 1 second age limit
                ..RefactoringConfig::default()
            };

            let mut engine = RefactoringEngine::with_config(config);

            // Run cleanup
            let result = engine.clear_history();
            assert!(result.is_ok());

            // The old backup should be cleaned up
            let cleanup_result = must(result);
            assert!(cleanup_result.directories_removed >= 1);
        }

        #[test]
        fn test_validate_backup_directory_structure() {
            let engine = RefactoringEngine::new();

            let backup_root = std::env::temp_dir().join("perl_refactor_backups");
            let _ = std::fs::create_dir_all(&backup_root);

            // Valid backup directory
            let valid_backup = backup_root.join("refactor_123_456");
            let _ = std::fs::create_dir_all(&valid_backup);
            assert!(must(engine.validate_backup_directory(&valid_backup)));

            // Invalid backup directory (wrong prefix)
            let invalid_backup = backup_root.join("invalid_backup");
            let _ = std::fs::create_dir_all(&invalid_backup);
            assert!(!must(engine.validate_backup_directory(&invalid_backup)));

            // Cleanup
            let _ = std::fs::remove_dir_all(&backup_root);
        }

        #[test]
        fn test_calculate_directory_size() {
            let engine = RefactoringEngine::new();

            let temp_dir = must(tempfile::tempdir());
            let dir_path = temp_dir.path().to_path_buf();

            // Create test files with known sizes
            let file1 = dir_path.join("file1.txt");
            let file2 = dir_path.join("file2.txt");

            must(std::fs::write(&file1, "hello")); // 5 bytes
            must(std::fs::write(&file2, "world!")); // 6 bytes

            let total_size = must(engine.calculate_directory_size(&dir_path));
            assert_eq!(total_size, 11);
        }

        #[test]
        #[serial]
        fn test_backup_cleanup_result_space_reclaimed() {
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Create backup directory with files of known size
            let backup = backup_root.join("refactor_100_0");
            must(fs::create_dir_all(&backup));

            let test_content = "sub test { print 'hello world'; }"; // 33 bytes
            must(fs::write(backup.join("file.pl"), test_content));

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                max_backup_retention: 0, // Remove all
                ..RefactoringConfig::default()
            };
            let mut engine = RefactoringEngine::with_config(config);

            // Clean up and verify space was reclaimed
            let result = must(engine.clear_history());
            assert!(result.space_reclaimed > 0);
        }

        // --- Robust backup cleanup tests (non-flaky) ---

        #[test]
        #[serial]
        fn cleanup_test_identifies_all_backup_directories() {
            // AC1: When clear_history() is called, all backup directories are identified
            // AC5: Method returns count of backup directories removed
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Manually create backup directories
            let backup1 = backup_root.join("refactor_100_0");
            let backup2 = backup_root.join("refactor_200_0");
            must(fs::create_dir_all(&backup1));
            must(fs::create_dir_all(&backup2));

            // Create test files in backups
            must(fs::write(backup1.join("file1.pl"), "sub test1 {}"));
            must(fs::write(backup2.join("file2.pl"), "sub test2 {}"));

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                max_backup_retention: 0, // Remove all
                ..RefactoringConfig::default()
            };
            let mut engine = RefactoringEngine::with_config(config);
            let result = must(engine.clear_history());

            // Should have removed both directories
            assert_eq!(result.directories_removed, 2);
            assert_eq!(engine.operation_history.len(), 0);

            // Verify directories are actually removed
            assert!(!backup1.exists());
            assert!(!backup2.exists());
        }

        #[test]
        #[serial]
        fn cleanup_test_respects_retention_count() {
            // AC2: Backup cleanup removes backup files older than a configurable retention period
            // AC3: Operation provides option to keep recent backups (e.g., last N operations)
            use std::fs;
            use std::thread;
            use std::time::Duration;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Manually create 4 backup directories with different timestamps
            let backups = [
                backup_root.join("refactor_100_0"),
                backup_root.join("refactor_200_0"),
                backup_root.join("refactor_300_0"),
                backup_root.join("refactor_400_0"),
            ];

            for (i, backup) in backups.iter().enumerate() {
                must(fs::create_dir_all(backup));
                must(fs::write(backup.join("file.pl"), format!("sub test{} {{}}", i)));
                // Sleep to ensure different modification times
                thread::sleep(Duration::from_millis(50));
            }

            let config = RefactoringConfig {
                create_backups: true,
                max_backup_retention: 2,
                backup_max_age_seconds: 0, // Disable age-based retention
                backup_root: Some(backup_root),
                ..RefactoringConfig::default()
            };

            let mut engine = RefactoringEngine::with_config(config);
            let result = must(engine.clear_history());

            // Should have removed 2 oldest directories, kept 2 newest
            assert_eq!(result.directories_removed, 2);

            // Verify oldest two are removed
            assert!(!backups[0].exists());
            assert!(!backups[1].exists());
            // temp_dir cleanup is automatic
        }

        #[test]
        #[serial]
        fn cleanup_test_respects_age_limit() {
            // AC2: Backup cleanup removes backup files older than a configurable retention period
            // AC6: Errors during cleanup are logged but don't prevent operation history clearing
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Create an old backup directory manually
            let old_backup = backup_root.join("refactor_1000_0");
            must(fs::create_dir_all(&old_backup));

            // Create a test file in the old backup
            let test_file = old_backup.join("file_0.pl");
            must(fs::write(&test_file, "sub old_backup { }"));

            // Poll filesystem metadata until the backup is older than the age threshold,
            // matching the pattern used by `test_cleanup_respects_age_limit` above.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut reached_age_limit = false;
            while std::time::Instant::now() < deadline {
                if let Ok(metadata) = fs::metadata(&old_backup)
                    && let Ok(modified) = metadata.modified()
                    && let Ok(age) = std::time::SystemTime::now().duration_since(modified)
                    && age > std::time::Duration::from_secs(1)
                {
                    reached_age_limit = true;
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(
                reached_age_limit,
                "backup directory did not age past threshold within test timeout"
            );

            let config = RefactoringConfig {
                backup_max_age_seconds: 1, // 1 second age limit
                backup_root: Some(backup_root),
                ..RefactoringConfig::default()
            };

            let mut engine = RefactoringEngine::with_config(config);

            // Run cleanup
            let result = engine.clear_history();
            assert!(result.is_ok());

            // The old backup should be cleaned up
            let cleanup_result = must(result);
            assert_eq!(cleanup_result.directories_removed, 1);

            // Verify directory is actually removed
            assert!(!old_backup.exists());
            // temp_dir cleanup is automatic
        }

        #[test]
        #[serial]
        fn cleanup_test_space_reclaimed() {
            // AC5: Method returns count of backup directories removed and total disk space reclaimed
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Create backup directory with files of known size
            let backup = backup_root.join("refactor_100_0");
            must(fs::create_dir_all(&backup));

            let test_content = "sub test { print 'hello world'; }"; // 33 bytes
            must(fs::write(backup.join("file1.pl"), test_content));
            must(fs::write(backup.join("file2.pl"), test_content));

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                max_backup_retention: 0, // Remove all
                ..RefactoringConfig::default()
            };
            let mut engine = RefactoringEngine::with_config(config);

            // Clean up and verify space was reclaimed
            let result = must(engine.clear_history());
            assert_eq!(result.directories_removed, 1);
            assert_eq!(result.space_reclaimed, 66); // 33 * 2 bytes

            // Verify directory is actually removed
            assert!(!backup.exists());
        }

        #[test]
        #[serial]
        fn cleanup_test_only_removes_refactor_backups() {
            // AC8: Cleanup respects backup directory naming convention and only removes refactoring engine backups
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Create valid refactor backup
            let refactor_backup = backup_root.join("refactor_100_0");
            must(fs::create_dir_all(&refactor_backup));
            must(fs::write(refactor_backup.join("file.pl"), "test"));

            // Create non-refactor directory (should not be removed)
            let other_dir = backup_root.join("other_backup");
            must(fs::create_dir_all(&other_dir));
            must(fs::write(other_dir.join("file.pl"), "test"));

            let config = RefactoringConfig {
                backup_root: Some(backup_root),
                max_backup_retention: 0, // Remove all
                ..RefactoringConfig::default()
            };
            let mut engine = RefactoringEngine::with_config(config);
            let result = must(engine.clear_history());

            // Should only remove refactor backup, not other directory
            assert_eq!(result.directories_removed, 1);
            assert!(!refactor_backup.exists());
            assert!(other_dir.exists()); // Should still exist
            // temp_dir cleanup is automatic
        }

        #[test]
        #[serial]
        fn cleanup_test_with_zero_retention_removes_all() {
            // AC2: When max_backup_retention is 0, all backups are removed
            use std::fs;

            let temp_dir = must(tempfile::tempdir());
            let backup_root = temp_dir.path().to_path_buf();

            // Create multiple backup directories
            for i in 0..3 {
                let backup = backup_root.join(format!("refactor_{}_0", i * 100));
                must(fs::create_dir_all(&backup));
                must(fs::write(backup.join("file.pl"), "test"));
            }

            let config = RefactoringConfig {
                max_backup_retention: 0, // Remove all
                backup_max_age_seconds: 0,
                backup_root: Some(backup_root),
                ..RefactoringConfig::default()
            };

            let mut engine = RefactoringEngine::with_config(config);
            let result = must(engine.clear_history());

            // All backups should be removed
            assert_eq!(result.directories_removed, 3);
            // temp_dir cleanup is automatic
        }

        #[test]
        #[serial]
        fn comprehensive_backup_cleanup_all_acs() {
            // Comprehensive test covering all ACs to avoid race conditions from multiple tests
            // AC1: Identifies all backup directories
            // AC2: Respects configurable retention period and age limits
            // AC3: Keeps recent backups when configured
            // AC4: Validates directory structure before deletion
            // AC5: Returns count of directories removed and space reclaimed
            // AC6: Errors don't prevent history clearing
            // AC7: Configuration options work
            // AC8: Only removes refactoring engine backups
            use std::fs;
            use std::thread;
            use std::time::Duration;

            // Test AC4 & AC8: Validation and selective removal
            let temp_dir1 = must(tempfile::tempdir());
            let backup_root1 = temp_dir1.path().to_path_buf();

            let valid_backup = backup_root1.join("refactor_test_1");
            let invalid_backup = backup_root1.join("other_backup");
            must(fs::create_dir_all(&valid_backup));
            must(fs::create_dir_all(&invalid_backup));
            must(fs::write(valid_backup.join("file.pl"), "test"));
            must(fs::write(invalid_backup.join("file.pl"), "test"));

            let config1 = RefactoringConfig {
                backup_root: Some(backup_root1.clone()),
                max_backup_retention: 0, // Remove all for this test
                ..RefactoringConfig::default()
            };
            let engine = RefactoringEngine::with_config(config1.clone());
            assert!(must(engine.validate_backup_directory(&valid_backup)));
            assert!(!must(engine.validate_backup_directory(&invalid_backup)));

            // Test AC1 & AC5: Identifies and removes with space calculation
            let mut engine2 = RefactoringEngine::with_config(config1);
            let result1 = must(engine2.clear_history());
            assert_eq!(result1.directories_removed, 1); // Only valid backup removed
            assert_eq!(result1.space_reclaimed, 4); // "test" = 4 bytes
            assert!(!valid_backup.exists());
            assert!(invalid_backup.exists()); // AC8: Other dir still exists

            // Test AC2 & AC3: Retention count
            let temp_dir2 = must(tempfile::tempdir());
            let backup_root2 = temp_dir2.path().to_path_buf();

            for i in 0..4 {
                let backup = backup_root2.join(format!("refactor_retention_{}", i));
                must(fs::create_dir_all(&backup));
                must(fs::write(backup.join("file.pl"), "x"));
                thread::sleep(Duration::from_millis(50));
            }

            let config2 = RefactoringConfig {
                max_backup_retention: 2,
                backup_max_age_seconds: 0,
                backup_root: Some(backup_root2),
                ..RefactoringConfig::default()
            };
            let mut engine3 = RefactoringEngine::with_config(config2);
            let result2 = must(engine3.clear_history());
            assert_eq!(result2.directories_removed, 2); // Oldest 2 removed

            // Test AC2: Age-based retention
            let temp_dir3 = must(tempfile::tempdir());
            let backup_root3 = temp_dir3.path().to_path_buf();

            let old_backup = backup_root3.join("refactor_age_test");
            must(fs::create_dir_all(&old_backup));
            must(fs::write(old_backup.join("file.pl"), "old"));

            let config3 = RefactoringConfig {
                backup_max_age_seconds: 1,
                max_backup_retention: 0,
                backup_root: Some(backup_root3),
                ..RefactoringConfig::default()
            };
            let mut engine4 = RefactoringEngine::with_config(config3);
            thread::sleep(Duration::from_secs(2));

            let result3 = must(engine4.clear_history());
            assert_eq!(result3.directories_removed, 1);
            assert!(!old_backup.exists());
            // temp_dir cleanup is automatic
        }
    }
}

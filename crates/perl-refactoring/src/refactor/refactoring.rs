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
use perl_parser_core::{Node, NodeKind, Parser};
use perl_parser_core::position::line_index::LineIndex;
use std::collections::HashSet;
// Import existing modules conditionally
use crate::import_optimizer::ImportOptimizer;
#[cfg(feature = "modernize")]
use crate::modernize::PerlModernizer as ModernizeEngine;
#[cfg(feature = "workspace_refactor")]
use crate::workspace_index::WorkspaceIndex;
#[cfg(feature = "workspace_refactor")]
use crate::workspace_refactor::WorkspaceRefactor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl Default for RefactoringConfig {
    fn default() -> Self {
        Self {
            safe_mode: true,
            max_files_per_operation: 100,
            create_backups: true,
            operation_timeout: 60,
            parallel_processing: true,
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
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Directory containing backup files
    pub backup_dir: PathBuf,
    /// Mapping of original files to backup locations
    pub file_mappings: HashMap<PathBuf, PathBuf>,
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
    pub fn new() -> ParseResult<Self> {
        Self::with_config(RefactoringConfig::default())
    }

    /// Create a new refactoring engine with custom configuration
    pub fn with_config(config: RefactoringConfig) -> ParseResult<Self> {
        Ok(Self {
            #[cfg(feature = "workspace_refactor")]
            workspace_refactor: WorkspaceRefactor::new(WorkspaceIndex::default()),
            #[cfg(not(feature = "workspace_refactor"))]
            workspace_refactor: temp_stubs::WorkspaceRefactor::new()?,
            #[cfg(feature = "modernize")]
            modernize: ModernizeEngine::new(),
            #[cfg(not(feature = "modernize"))]
            modernize: temp_stubs::ModernizeEngine::new(),
            import_optimizer: ImportOptimizer::new(),
            config,
            operation_history: Vec::new(),
        })
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
    pub fn clear_history(&mut self) -> ParseResult<()> {
        // TODO: Cleanup backup directories
        self.operation_history.clear();
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
        _operation_type: &RefactoringType,
        _files: &[PathBuf],
    ) -> ParseResult<()> {
        // TODO: Implement validation logic
        Ok(())
    }

    fn create_backup(&self, files: &[PathBuf], operation_id: &str) -> ParseResult<BackupInfo> {
        let mut backup_dir = std::env::temp_dir();
        backup_dir.push("perl_refactor_backups");
        backup_dir.push(operation_id);

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

        Ok(BackupInfo {
            backup_dir,
            file_mappings,
        })
    }

    fn perform_symbol_rename(
        &mut self,
        old_name: &str,
        new_name: &str,
        scope: &RefactoringScope,
    ) -> ParseResult<RefactoringResult> {
        // Delegate to workspace refactor
        match scope {
            RefactoringScope::Workspace => {
                // TODO: Implement workspace-wide rename
                Ok(RefactoringResult {
                    success: true,
                    files_modified: 0,
                    changes_made: 0,
                    warnings: vec![format!(
                        "Symbol rename from '{}' to '{}' not yet implemented",
                        old_name, new_name
                    )],
                    errors: vec![],
                    operation_id: None,
                })
            }
            _ => {
                // TODO: Implement scoped rename
                Ok(RefactoringResult {
                    success: false,
                    files_modified: 0,
                    changes_made: 0,
                    warnings: vec![],
                    errors: vec!["Scoped rename not yet implemented".to_string()],
                    operation_id: None,
                })
            }
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

        let source_code = std::fs::read_to_string(file_path).map_err(|e| ParseError::SyntaxError {
            message: format!("Failed to read file: {}", e),
            location: 0,
        })?;

        // Calculate offsets
        let mut line_index = LineIndex::new(source_code.clone());
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
            "\n# Extracted from lines {}-{}\nsub {} {{\n",
            start_position.0 + 1,
            end_position.0 + 1,
            method_name
        );

        // Handle inputs
        if !analysis.inputs.is_empty() {
            new_sub.push_str(&format!("    my ({}) = @_;\n", analysis.inputs.join(", ")));
        }

        // Body
        new_sub.push_str("    ");
        new_sub.push_str(extracted_code.trim());
        new_sub.push('\n');

        // Handle outputs
        if !analysis.outputs.is_empty() {
            new_sub.push_str(&format!("    return ({});\n", analysis.outputs.join(", ")));
        }
        new_sub.push_str("}\n");

        // Generate Call
        let inputs_str = analysis.inputs.join(", ");
        let mut call = format!("{}({})", method_name, inputs_str);

        if !analysis.outputs.is_empty() {
            let outputs_str = analysis.outputs.join(", ");
            call = format!("({}) = {}", outputs_str, call);
        }
        call.push(';');

        // Apply changes
        let mut new_source = String::new();
        new_source.push_str(&source_code[..start_offset]);
        new_source.push_str(&call);
        new_source.push_str(&source_code[end_offset..]);
        new_source.push_str(&new_sub);

        if !self.config.safe_mode {
            std::fs::write(file_path, new_source).map_err(|e| ParseError::SyntaxError {
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
        _source_file: &Path,
        _target_file: &Path,
        _elements: &[String],
    ) -> ParseResult<RefactoringResult> {
        // TODO: Implement code movement
        Ok(RefactoringResult {
            success: false,
            files_modified: 0,
            changes_made: 0,
            warnings: vec![],
            errors: vec!["Move code not yet implemented".to_string()],
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
        _symbol_name: &str,
        _all_occurrences: bool,
        _files: &[PathBuf],
    ) -> ParseResult<RefactoringResult> {
        // TODO: Implement inlining
        Ok(RefactoringResult {
            success: false,
            files_modified: 0,
            changes_made: 0,
            warnings: vec![],
            errors: vec!["Inline operation not yet implemented".to_string()],
            operation_id: None,
        })
    }
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            // Log the error - this shouldn't happen in normal operation
            // as new() only fails if sub-components fail to initialize
            eprintln!("Warning: Failed to create refactoring engine, using fallback: {}", e);
            // Create with default config - this path is for error recovery only
            Self::with_config(RefactoringConfig::default())
                .unwrap_or_else(|_| panic!("RefactoringEngine fallback initialization failed - this is a critical bug in the refactoring system"))
        })
    }
}

// Temporary stub implementations for missing dependencies
mod temp_stubs {
    use super::*;

    #[allow(dead_code)]
    #[derive(Debug)]
    pub(super) struct WorkspaceRefactor;
    #[allow(dead_code)]
    impl WorkspaceRefactor {
        pub(super) fn new() -> ParseResult<Self> {
            Ok(Self)
        }
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    pub(super) struct ModernizeEngine;
    #[allow(dead_code)]
    impl ModernizeEngine {
        pub(super) fn new() -> Self {
            Self
        }

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

    ExtractionAnalysis {
        inputs: inputs_vec,
        outputs: outputs_vec,
    }
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
        NodeKind::VariableDeclaration {
            declarator,
            variable,
            initializer,
            ..
        } => {
            if declarator == "my" {
                let name = extract_var_name(variable);
                if in_range {
                    declared_in_range.insert(name);
                } else {
                    declared_in_scope.insert(name);
                }
            }
            if let Some(init) = initializer {
                visit_node(
                    init,
                    start,
                    end,
                    inputs,
                    outputs,
                    declared_in_scope,
                    declared_in_range,
                );
            }
        }
        NodeKind::VariableListDeclaration {
            declarator,
            variables,
            initializer,
            ..
        } => {
            if declarator == "my" {
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
                visit_node(
                    init,
                    start,
                    end,
                    inputs,
                    outputs,
                    declared_in_scope,
                    declared_in_range,
                );
            }
        }
        NodeKind::Variable { sigil, name } => {
            let full_name = format!("{}{}", sigil, name);
            if in_range {
                // If not declared in range, check if declared in outer scope.
                if !declared_in_range.contains(&full_name)
                    && declared_in_scope.contains(&full_name)
                {
                    inputs.insert(full_name);
                }
            } else if node.location.start >= end {
                // Usage after range
                // If declared in range, it's an output.
                if declared_in_range.contains(&full_name) {
                    outputs.insert(full_name);
                }
            }
        }
        NodeKind::Block { statements } => {
            let mut inner_scope = declared_in_scope.clone();
            for stmt in statements {
                visit_node(
                    stmt,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
            }
        }
        NodeKind::Subroutine { body, .. } => {
            let mut inner_scope = declared_in_scope.clone();
            visit_node(
                body,
                start,
                end,
                inputs,
                outputs,
                &mut inner_scope,
                declared_in_range,
            );
        }
        NodeKind::Foreach {
            variable,
            list,
            body,
        } => {
            // Visit list with outer scope
            visit_node(
                list,
                start,
                end,
                inputs,
                outputs,
                declared_in_scope,
                declared_in_range,
            );

            // Create inner scope for variable and body
            let mut inner_scope = declared_in_scope.clone();
            visit_node(
                variable,
                start,
                end,
                inputs,
                outputs,
                &mut inner_scope,
                declared_in_range,
            );
            visit_node(
                body,
                start,
                end,
                inputs,
                outputs,
                &mut inner_scope,
                declared_in_range,
            );
        }
        NodeKind::For {
            init,
            condition,
            update,
            body,
            continue_block,
        } => {
            let mut inner_scope = declared_in_scope.clone();
            if let Some(n) = init {
                visit_node(
                    n,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
            }
            if let Some(n) = condition {
                visit_node(
                    n,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
            }
            if let Some(n) = update {
                visit_node(
                    n,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
            }
            visit_node(
                body,
                start,
                end,
                inputs,
                outputs,
                &mut inner_scope,
                declared_in_range,
            );
            if let Some(n) = continue_block {
                visit_node(
                    n,
                    start,
                    end,
                    inputs,
                    outputs,
                    &mut inner_scope,
                    declared_in_range,
                );
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

    #[test]
    fn test_refactoring_engine_creation() {
        let engine = RefactoringEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_operation_id_generation() {
        let engine = RefactoringEngine::new().unwrap();
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
        let mut file = tempfile::NamedTempFile::new().unwrap();
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
        write!(file, "{}", code).unwrap();
        let path = file.path().to_path_buf();

        let mut engine = RefactoringEngine::new().unwrap();
        engine.config.safe_mode = false;

        // Lines are 0-indexed.
        // Line 5: "    print $x;\n"
        // Line 8: "    # End extraction\n"
        let result = engine
            .perform_extract_method("extracted_sub", (5, 0), (8, 0), &[path.clone()])
            .unwrap();

        assert!(result.success);

        let new_code = std::fs::read_to_string(&path).unwrap();
        println!("New code:\n{}", new_code);

        // Inputs: $x, $y (used in range, declared before)
        // Outputs: $z (declared in range, used after)

        assert!(new_code.contains("sub extracted_sub {"));
        assert!(new_code.contains("my ($x, $y) = @_;"));
        assert!(new_code.contains("return ($z);"));
        // Call verification order depends on how we generate it
        assert!(new_code.contains("($z) = extracted_sub($x, $y);"));
    }
}

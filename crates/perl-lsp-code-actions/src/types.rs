//! Code action types
//!
//! Defines the core types for representing code actions and their edits.

use perl_lsp_rename::TextEdit;

/// A diagnostic with byte offset range for quick fix processing
///
/// This is a simplified diagnostic type used internally by the quick fix
/// system. It uses byte offsets instead of line/column positions for
/// efficient source text manipulation.
#[derive(Debug, Clone)]
pub struct QuickFixDiagnostic {
    /// The byte offset range (start, end) in the source
    pub range: (usize, usize),
    /// The diagnostic message
    pub message: String,
    /// The diagnostic code (e.g., "undefined-variable")
    #[allow(dead_code)]
    pub code: Option<String>,
}

/// A code action that can be applied to fix an issue
///
/// Code actions represent automated fixes for common issues and refactoring
/// operations that can be applied to Perl source code.
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Human-readable title describing the action
    pub title: String,
    /// The kind/category of code action
    pub kind: CodeActionKind,
    /// Diagnostic codes this action fixes
    pub diagnostics: Vec<String>,
    /// The edit operations to apply
    pub edit: CodeActionEdit,
    /// Whether this action is the preferred choice
    pub is_preferred: bool,
}

/// Kind of code action
///
/// Categorizes the type of code action to help editors organize and present
/// actions to users appropriately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionKind {
    /// Quick fix for a diagnostic issue
    QuickFix,
    /// General refactoring operation
    Refactor,
    /// Extract code into a new construct
    RefactorExtract,
    /// Inline a construct into its usage sites
    RefactorInline,
    /// Rewrite code using a different pattern
    RefactorRewrite,
    /// Source code organization action
    Source,
    /// Organize imports action
    SourceOrganizeImports,
    /// Fix all issues action
    SourceFixAll,
}

/// Edit to apply for a code action
///
/// Contains the specific text changes needed to apply a code action.
#[derive(Debug, Clone)]
pub struct CodeActionEdit {
    /// List of text edits to apply
    pub changes: Vec<TextEdit>,
}

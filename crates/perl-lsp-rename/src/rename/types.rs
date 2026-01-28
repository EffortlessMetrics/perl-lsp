//! Rename type definitions
//!
//! This module contains the core types used for rename operations.

use perl_parser_core::SourceLocation;

/// A text edit to apply during rename
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Location to edit
    pub location: SourceLocation,
    /// New text to insert
    pub new_text: String,
}

/// Result of a rename operation
#[derive(Debug)]
pub struct RenameResult {
    /// All edits to apply
    pub edits: Vec<TextEdit>,
    /// Whether the rename is valid
    pub is_valid: bool,
    /// Error message if not valid
    pub error: Option<String>,
}

/// Options for rename operation
#[derive(Debug, Clone)]
pub struct RenameOptions {
    /// Whether to rename in comments
    pub rename_in_comments: bool,
    /// Whether to rename in strings
    pub rename_in_strings: bool,
    /// Whether to validate the new name
    pub validate_new_name: bool,
}

impl Default for RenameOptions {
    fn default() -> Self {
        RenameOptions {
            rename_in_comments: false,
            rename_in_strings: false,
            validate_new_name: true,
        }
    }
}

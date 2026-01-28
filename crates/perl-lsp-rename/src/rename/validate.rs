//! Rename validation logic
//!
//! This module provides validation for rename operations.

use perl_semantic_analyzer::symbol::SymbolKind;
use perl_semantic_analyzer::symbol::SymbolTable;

/// Check if a symbol can be renamed
pub fn can_rename_symbol(name: &str, _kind: SymbolKind) -> bool {
    // Don't rename special variables
    let special_vars = [
        "_", ".", ",", "/", "\\", "!", "@", "$", "%", "0", "1", "2", "3", "4", "5", "6", "7", "8",
        "9", "&", "`", "'", "+", "[", "]", "{", "}", "^O", "^V", "^W", "^X",
    ];

    if special_vars.contains(&name) {
        return false;
    }

    // Don't rename built-in functions
    let builtins = [
        "print", "say", "printf", "sprintf", "open", "close", "read", "write", "push", "pop",
        "shift", "unshift", "map", "grep", "sort", "reverse", "split", "join", "chomp", "chop",
        "die", "warn", "eval", "exit", "require", "use", "package", "sub",
    ];

    if builtins.contains(&name) {
        return false;
    }

    true
}

/// Validate a new name
pub fn validate_name(
    name: &str,
    kind: SymbolKind,
    symbol_table: &SymbolTable,
) -> Result<(), String> {
    // Check if empty
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    // Check if it starts with a number
    if let Some(first_char) = name.chars().next() {
        if first_char.is_numeric() {
            return Err("Name cannot start with a number".to_string());
        }
    }

    // Check if it contains only valid characters
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Name can only contain letters, numbers, and underscores".to_string());
    }

    // Check if it's a keyword
    let keywords = [
        "my", "our", "local", "state", "if", "elsif", "else", "unless", "while", "until", "for",
        "foreach", "sub", "package", "use", "require", "return", "last", "next", "redo", "and",
        "or", "not", "eq", "ne",
    ];

    if keywords.contains(&name) {
        return Err("Cannot use a keyword as a name".to_string());
    }

    // Check for naming conflicts
    if kind != SymbolKind::Subroutine {
        // Variables can shadow, so this is okay
    } else {
        // Check if a sub with this name already exists
        if symbol_table.symbols.contains_key(name) {
            return Err(format!("A symbol named '{}' already exists", name));
        }
    }

    Ok(())
}

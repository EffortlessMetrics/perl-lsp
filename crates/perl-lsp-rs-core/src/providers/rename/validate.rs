//! Rename validation logic
//!
//! This module provides validation for rename operations.

use perl_lexer::is_rename_keyword;
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
    if let Some(first_char) = name.chars().next()
        && first_char.is_ascii_digit()
    {
        return Err("Name cannot start with a number".to_string());
    }

    // Check if it contains only valid characters
    if !name.is_ascii() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Name can only contain letters, numbers, and underscores".to_string());
    }

    // Keyword check is context-sensitive:
    //   - Variables (scalar/array/hash): keyword names are allowed — Perl permits
    //     `$if`, `@while`, `%for`, etc. as valid variable names distinguished by sigil.
    //   - Subroutines and methods: keyword names are rejected — `sub if { }` is a
    //     Perl syntax error that would break the file.
    //   - Other symbol kinds (Package, Constant, etc.): keywords are rejected.
    if !kind.is_variable() && is_rename_keyword(name) {
        return Err(if kind.is_callable() {
            format!("'{}' is a reserved Perl keyword; subroutine names cannot be keywords", name)
        } else {
            format!("Cannot use reserved keyword '{}' as a name", name)
        });
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
